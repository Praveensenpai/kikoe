//! ⛩️ 聴こえ (kikoe) — Temporal multi-slice audio language identification in pure Rust.

use std::collections::HashMap;
use std::path::Path;

pub mod cli;
pub mod domain;
pub mod error;
pub mod infra;

pub use domain::{LanguageScore, LidConfig, LidResult, SlicePrediction, StreamInfo};
pub use error::{KikoeError, Result};
pub use infra::LanguageRegistry;
use infra::{AudioSampler, LidModelEngine, MediaProbe};

/// Primary high-level engine for inspecting containers and classifying audio track languages.
pub struct KikoeEngine {
    config: LidConfig,
    model: LidModelEngine,
}

impl KikoeEngine {
    /// Creates a new KikoeEngine with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(LidConfig::default())
    }

    /// Creates an engine with custom LidConfig options.
    pub fn with_config(config: LidConfig) -> Result<Self> {
        let model = LidModelEngine::new()?;
        Ok(Self { config, model })
    }

    /// Inspects and lists all available audio streams inside a media container.
    pub fn inspect_streams<P: AsRef<Path>>(path: P) -> Result<Vec<StreamInfo>> {
        MediaProbe::probe_audio_streams(path)
    }

    /// Runs multi-slice temporal sampling and soft-voting on the designated audio stream.
    pub fn detect_stream_language<P: AsRef<Path>>(
        &self,
        path: P,
        stream_index: usize,
    ) -> Result<LidResult> {
        let duration = MediaProbe::probe_duration(&path)?;
        let timestamps = Self::calculate_sampling_timestamps(duration, self.config.sample_count);

        let mut slices: Vec<SlicePrediction> = Vec::new();
        let mut voiced_count = 0;

        for &ts in &timestamps {
            let sample_res = AudioSampler::extract_slice(
                &path,
                stream_index,
                ts,
                self.config.clip_duration_secs,
                self.config.silence_threshold_rms,
            );

            match sample_res {
                Ok(sample) if sample.is_speech => {
                    voiced_count += 1;
                    if let Ok(pred) = self.model.predict_slice(&sample) {
                        slices.push(pred);
                    }
                }
                _ => continue,
            }
        }

        if slices.is_empty() {
            return Err(KikoeError::InsufficientSpeech);
        }

        let voiced_ratio = voiced_count as f32 / timestamps.len() as f32;
        let (winner_code, overall_confidence) = Self::aggregate_votes(&slices);
        let winner_name = LanguageRegistry::name_for_code(&winner_code);

        Ok(LidResult {
            stream_index,
            winner_code,
            winner_name,
            overall_confidence,
            duration_secs: duration,
            slices,
            voiced_ratio,
        })
    }

    /// Analyzes all audio streams in a media container using synchronized temporal slices.
    /// Employs cross-track correlation to suppress shared background music (BGM) noise.
    pub fn detect_all_streams<P: AsRef<Path>>(&self, path: P) -> Result<Vec<LidResult>> {
        let streams = Self::inspect_streams(&path)?;
        if streams.is_empty() {
            return Err(KikoeError::StreamNotFound(0));
        }

        let duration = MediaProbe::probe_duration(&path)?;
        let timestamps = Self::calculate_sampling_timestamps(duration, self.config.sample_count);

        let mut raw_stream_slices = Vec::with_capacity(streams.len());
        for s in &streams {
            let slices = self.sample_stream_slices(&path, s.index, &timestamps);
            raw_stream_slices.push((s.index, slices));
        }

        let bgm_noise_flags = Self::identify_bgm_timestamps(&raw_stream_slices, timestamps.len());
        let mut results = Vec::with_capacity(streams.len());

        for (stream_index, slices) in raw_stream_slices {
            let mut valid_slices = Vec::new();
            let mut voiced_count = 0;

            for (idx, pred_opt) in slices.into_iter().enumerate() {
                if let Some(pred) = pred_opt {
                    voiced_count += 1;
                    if !bgm_noise_flags[idx] {
                        valid_slices.push(pred);
                    }
                }
            }

            if valid_slices.is_empty() {
                return Err(KikoeError::InsufficientSpeech);
            }

            let voiced_ratio = voiced_count as f32 / timestamps.len() as f32;
            let (winner_code, overall_confidence) = Self::aggregate_votes(&valid_slices);
            let winner_name = LanguageRegistry::name_for_code(&winner_code);

            results.push(LidResult {
                stream_index,
                winner_code,
                winner_name,
                overall_confidence,
                duration_secs: duration,
                slices: valid_slices,
                voiced_ratio,
            });
        }

        Ok(results)
    }

    /// Samples slices for a single stream across predetermined timestamps.
    fn sample_stream_slices<P: AsRef<Path>>(
        &self,
        path: P,
        stream_index: usize,
        timestamps: &[f64],
    ) -> Vec<Option<SlicePrediction>> {
        timestamps
            .iter()
            .map(|&ts| {
                AudioSampler::extract_slice(
                    &path,
                    stream_index,
                    ts,
                    self.config.clip_duration_secs,
                    self.config.silence_threshold_rms,
                )
                .ok()
                .filter(|sample| sample.is_speech)
                .and_then(|sample| self.model.predict_slice(&sample).ok())
            })
            .collect()
    }

    /// Detects shared soundtrack music by finding timestamps where >= 60% of streams agree.
    fn identify_bgm_timestamps(
        stream_slices: &[(usize, Vec<Option<SlicePrediction>>)],
        total_timestamps: usize,
    ) -> Vec<bool> {
        let mut bgm_flags = vec![false; total_timestamps];
        if stream_slices.len() < 3 {
            return bgm_flags;
        }

        for ts_idx in 0..total_timestamps {
            let mut lang_counts: HashMap<String, usize> = HashMap::new();
            let mut voiced_count = 0;

            for (_, slices) in stream_slices {
                if let Some(pred) = &slices[ts_idx] {
                    voiced_count += 1;
                    *lang_counts.entry(pred.top_language.clone()).or_default() += 1;
                }
            }

            if voiced_count >= 3 {
                for count in lang_counts.values() {
                    if (*count as f32 / voiced_count as f32) >= 0.60 {
                        bgm_flags[ts_idx] = true;
                        break;
                    }
                }
            }
        }
        bgm_flags
    }

    /// Calculates strategically placed timestamps avoiding intro and outro dead zones.
    pub fn calculate_sampling_timestamps(duration_secs: f64, count: usize) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![duration_secs * 0.5];
        }

        // Avoid first 15% and final 15% (themes, logos, credits)
        let start_pct = 0.15;
        let end_pct = 0.85;
        let step = (end_pct - start_pct) / (count - 1) as f64;

        (0..count)
            .map(|i| duration_secs * (start_pct + i as f64 * step))
            .collect()
    }

    /// Computes soft-voting aggregate confidence across all valid temporal slices.
    fn aggregate_votes(slices: &[SlicePrediction]) -> (String, f32) {
        if slices.is_empty() {
            return ("und".to_string(), 0.0);
        }

        // Tiered threshold: isolate dialogue slices (>= 0.70) from ambient music noise
        let thresholds = [0.70, 0.50, 0.35];
        let mut active_slices: Vec<&SlicePrediction> = Vec::new();
        for &thresh in &thresholds {
            let filtered: Vec<&SlicePrediction> =
                slices.iter().filter(|s| s.confidence >= thresh).collect();
            if !filtered.is_empty() {
                active_slices = filtered;
                break;
            }
        }
        if active_slices.is_empty() {
            active_slices = slices.iter().collect();
        }

        let mut score_accum: HashMap<String, f32> = HashMap::new();
        let mut winner_slice_scores: HashMap<String, Vec<f32>> = HashMap::new();
        let mut total_weight = 0.0f32;

        for slice in &active_slices {
            let pooled_cands = Self::pool_sister_candidates(&slice.candidates);
            if let Some(top) = pooled_cands.first() {
                winner_slice_scores
                    .entry(top.code.clone())
                    .or_default()
                    .push(top.confidence);
            }

            let weight = slice.confidence * slice.confidence;
            total_weight += weight;
            for cand in pooled_cands.iter().take(5) {
                *score_accum.entry(cand.code.clone()).or_default() += cand.confidence * weight;
            }
        }

        if total_weight <= 0.0 {
            total_weight = 1.0;
        }

        let mut ranked: Vec<(String, f32)> = score_accum
            .into_iter()
            .map(|(code, total_score)| (code, (total_score / total_weight).min(1.0)))
            .collect();

        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((winner_code, _)) = ranked.first() {
            let conf = if let Some(winning_scores) = winner_slice_scores.get(winner_code) {
                let sum: f32 = winning_scores.iter().sum();
                sum / winning_scores.len() as f32
            } else {
                ranked[0].1
            };
            (winner_code.clone(), conf.min(1.0))
        } else {
            ("und".to_string(), 0.0)
        }
    }

    /// Pools acoustically identical sister languages (e.g. Hindustani Hindi & Urdu) under the leading class.
    fn pool_sister_candidates(candidates: &[LanguageScore]) -> Vec<LanguageScore> {
        let mut pooled = candidates.to_vec();
        let hi_idx = pooled.iter().position(|c| c.code == "hi");
        let ur_idx = pooled.iter().position(|c| c.code == "ur");

        if let (Some(h), Some(u)) = (hi_idx, ur_idx) {
            if pooled[h].confidence >= pooled[u].confidence {
                pooled[h].confidence += pooled[u].confidence;
                pooled.remove(u);
            } else {
                pooled[u].confidence += pooled[h].confidence;
                pooled.remove(h);
            }
            pooled.sort_by(|a, b| {
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        pooled
    }
}
