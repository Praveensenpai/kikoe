//! ⛩️ 聴こえ (kikoe) — Temporal multi-slice audio language identification in pure Rust.

use std::collections::HashMap;
use std::path::Path;

pub mod cli;
pub mod domain;
pub mod error;
pub mod infra;

pub use domain::{LidConfig, LidResult, SlicePrediction, StreamInfo};
pub use error::{KikoeError, Result};
use infra::{AudioSampler, LanguageRegistry, LidModelEngine, MediaProbe};

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
        let mut total_weight = 0.0f32;

        for slice in active_slices {
            let weight = slice.confidence * slice.confidence;
            total_weight += weight;
            for cand in slice.candidates.iter().take(5) {
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

        ranked
            .first()
            .cloned()
            .unwrap_or_else(|| ("und".to_string(), 0.0))
    }
}
