//! Domain models representing audio streams, samples, and classification results.

use serde::{Deserialize, Serialize};

/// Basic stream metadata extracted via container probing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Zero-based audio stream index within the container.
    pub index: usize,
    /// Codec name (e.g. "eac3", "aac", "ac3", "flac").
    pub codec_name: String,
    /// Number of audio channels (e.g. 2 for stereo, 6 for 5.1).
    pub channels: u32,
    /// Native sample rate in Hz.
    pub sample_rate: u32,
    /// Existing metadata language tag (e.g. "und", "tam", "jpn").
    pub metadata_lang: Option<String>,
    /// Track title if available in metadata.
    pub track_title: Option<String>,
}

/// Extracted in-memory audio slice with energy metrics.
#[derive(Debug, Clone)]
pub struct AudioSample {
    /// Timestamp offset within the media file in seconds.
    pub timestamp_secs: f64,
    /// Mono 16kHz PCM audio samples normalized to [-1.0, 1.0].
    pub pcm_samples: Vec<f32>,
    /// Root Mean Square (RMS) energy metric.
    pub rms_energy: f32,
    /// Flag indicating whether human voice activity is likely present.
    pub is_speech: bool,
}

/// Language prediction score for an individual audio slice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageScore {
    /// ISO language code (e.g. "ja", "en", "ta", "te", "ml", "hi").
    pub code: String,
    /// Human-readable language name.
    pub name: String,
    /// Confidence probability in range [0.0, 1.0].
    pub confidence: f32,
}

/// Detailed prediction for an individual temporal slice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlicePrediction {
    /// Timestamp offset where the slice was extracted.
    pub timestamp_secs: f64,
    /// Highest-scoring language code for this slice.
    pub top_language: String,
    /// Probability of the top language.
    pub confidence: f32,
    /// Top language candidates ranked by probability.
    pub candidates: Vec<LanguageScore>,
    /// RMS audio energy of this slice.
    pub rms_energy: f32,
}

/// Aggregated multi-slice language prediction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidResult {
    /// Target audio stream index.
    pub stream_index: usize,
    /// Overall winning language code determined by ensemble voting.
    pub winner_code: String,
    /// Human-readable winning language name.
    pub winner_name: String,
    /// Ensemble aggregate confidence score.
    pub overall_confidence: f32,
    /// Total duration of the media file in seconds.
    pub duration_secs: f64,
    /// Individual slice predictions used in voting.
    pub slices: Vec<SlicePrediction>,
    /// Ratio of slices that contained valid human speech energy.
    pub voiced_ratio: f32,
}
