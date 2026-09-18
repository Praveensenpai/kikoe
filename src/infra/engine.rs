//! Spoken Language Identification (LID) inference engine using ONNX Runtime.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;

use crate::domain::{AudioSample, LanguageScore, SlicePrediction};
use crate::error::{KikoeError, Result};

#[derive(Debug, Deserialize)]
struct LangEntry {
    iso: String,
    name: String,
}

static MODEL_LANG_MAP: LazyLock<Vec<(String, String)>> = LazyLock::new(|| {
    let raw = include_str!("../../models/lang_map.json");
    let entries: HashMap<String, LangEntry> = serde_json::from_str(raw).unwrap_or_default();
    let mut vec: Vec<(usize, String, String)> = entries
        .into_iter()
        .filter_map(|(k, v)| k.parse::<usize>().ok().map(|idx| (idx, v.iso, v.name)))
        .collect();
    vec.sort_by_key(|(idx, _, _)| *idx);
    vec.into_iter().map(|(_, iso, name)| (iso, name)).collect()
});

static ISO_NAME_CACHE: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    MODEL_LANG_MAP
        .iter()
        .map(|(iso, name)| (iso.to_lowercase(), name.clone()))
        .collect()
});

/// Spoken language metadata and human-readable names.
pub struct LanguageRegistry;

impl LanguageRegistry {
    /// Maps 2-letter or 3-letter language code to human-readable name.
    pub fn name_for_code(code: &str) -> String {
        let code_lower = code.to_lowercase();
        if let Some(name) = ISO_NAME_CACHE.get(&code_lower) {
            return name.clone();
        }
        match code_lower.as_str() {
            "jpn" => "Japanese".into(),
            "eng" => "English".into(),
            "tam" => "Tamil".into(),
            "tel" => "Telugu".into(),
            "mal" => "Malayalam".into(),
            "hin" => "Hindi".into(),
            "kan" => "Kannada".into(),
            "kor" => "Korean".into(),
            "zho" | "chi" => "Chinese".into(),
            "spa" => "Spanish".into(),
            "fra" | "fre" => "French".into(),
            "deu" | "ger" => "German".into(),
            _ => "Unknown".into(),
        }
    }
}

/// Spoken Language Identification (LID) inference engine.
pub struct LidModelEngine {
    session: Mutex<Session>,
}

impl LidModelEngine {
    /// Creates a new model engine resolving the ONNX model from standard locations.
    pub fn new() -> Result<Self> {
        let model_path = Self::resolve_model_path()?;
        Self::from_file(&model_path)
    }

    /// Initializes the ONNX session directly from a specified file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let p = path.as_ref();
        let session = Session::builder()
            .map_err(|e| KikoeError::Model(format!("Failed to create ORT session builder: {e}")))?
            .commit_from_file(p)
            .map_err(|e| {
                KikoeError::Model(format!(
                    "Failed to load ONNX model from {}: {e}",
                    p.display()
                ))
            })?;
        Ok(Self {
            session: Mutex::new(session),
        })
    }

    /// Resolves the model path across local project dirs, user cache, and env vars.
    pub fn resolve_model_path() -> Result<PathBuf> {
        if let Ok(env_path) = std::env::var("KIKOE_MODEL_PATH") {
            let p = PathBuf::from(env_path);
            if p.exists() {
                return Ok(p);
            }
        }
        let local = PathBuf::from("models/voxlingua107.onnx");
        if local.exists() {
            return Ok(local);
        }
        if let Ok(home) = std::env::var("HOME") {
            let cache_model = PathBuf::from(home).join(".cache/kikoe/voxlingua107.onnx");
            if cache_model.exists() {
                return Ok(cache_model);
            }
        }
        Self::ensure_cached_model()
    }

    /// Downloads the acoustic model to user cache if not present.
    fn ensure_cached_model() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| KikoeError::Model("HOME environment variable not set".into()))?;
        let cache_dir = PathBuf::from(home).join(".cache/kikoe");
        let target = cache_dir.join("voxlingua107.onnx");
        if target.exists() {
            return Ok(target);
        }

        eprintln!("📥 voxlingua107.onnx not found. Downloading acoustic model (~83MB)...");
        let _ = std::fs::create_dir_all(&cache_dir);
        let download_url =
            "https://github.com/Praveensenpai/kikoe/releases/download/v0.1.0/voxlingua107.onnx";
        let status = std::process::Command::new("curl")
            .args(["-fsSL", download_url, "-o"])
            .arg(&target)
            .status();

        match status {
            Ok(s) if s.success() && target.exists() => Ok(target),
            _ => {
                let _ = std::fs::remove_file(&target);
                Err(KikoeError::Model(format!(
                    "Acoustic model not found. Place 'voxlingua107.onnx' in '{}' or set KIKOE_MODEL_PATH.",
                    target.display()
                )))
            }
        }
    }

    /// Predicts language distribution from a pre-extracted 16kHz PCM audio sample.
    pub fn predict_slice(&self, sample: &AudioSample) -> Result<SlicePrediction> {
        if !sample.is_speech || sample.pcm_samples.is_empty() {
            return Err(KikoeError::InsufficientSpeech);
        }

        let num_samples = sample.pcm_samples.len();
        let tensor = Tensor::from_array(([1, num_samples], sample.pcm_samples.clone()))
            .map_err(|e| KikoeError::Model(format!("Failed to build input tensor: {e}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| KikoeError::Model("Inference session mutex poisoned".into()))?;

        let outputs = session
            .run(ort::inputs![tensor])
            .map_err(|e| KikoeError::Model(format!("ONNX execution failure: {e}")))?;

        let (_shape, logits) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| KikoeError::Model(format!("Failed to extract output logits: {e}")))?;

        let scores = Self::compute_softmax_distribution(logits);

        let top_match = scores
            .first()
            .cloned()
            .ok_or_else(|| KikoeError::Model("Empty predictions from model".into()))?;

        Ok(SlicePrediction {
            timestamp_secs: sample.timestamp_secs,
            top_language: top_match.code,
            confidence: top_match.confidence,
            candidates: scores,
            rms_energy: sample.rms_energy,
        })
    }

    /// Computes numerically stable softmax and maps logits to ranked language scores.
    fn compute_softmax_distribution(logits: &[f32]) -> Vec<LanguageScore> {
        let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = logits.iter().map(|&l| (l - max_logit).exp()).sum();

        let mut scores: Vec<LanguageScore> = logits
            .iter()
            .enumerate()
            .filter_map(|(idx, &l)| {
                let prob = (l - max_logit).exp() / exp_sum;
                MODEL_LANG_MAP.get(idx).map(|(iso, name)| LanguageScore {
                    code: iso.clone(),
                    name: name.clone(),
                    confidence: prob,
                })
            })
            .collect();

        scores.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scores
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization_and_inference() {
        let engine = LidModelEngine::new().expect("Engine should initialize");
        let dummy_pcm = vec![0.01f32; 16000 * 3];
        let sample = AudioSample {
            timestamp_secs: 10.0,
            pcm_samples: dummy_pcm,
            rms_energy: 0.05,
            is_speech: true,
        };

        let pred = engine
            .predict_slice(&sample)
            .expect("Slice inference failed");
        assert!(!pred.top_language.is_empty());
        assert!(pred.confidence > 0.0);
        assert_eq!(pred.candidates.len(), MODEL_LANG_MAP.len());
    }
}
