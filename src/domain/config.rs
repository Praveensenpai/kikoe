//! Configuration settings for multi-slice audio language identification.

use serde::{Deserialize, Serialize};

/// Configuration options controlling the audio slicing and classification heuristics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidConfig {
    /// Total number of temporal slices to sample across the file duration.
    pub sample_count: usize,

    /// Duration of each slice in seconds (recommended 2.0 - 3.0s).
    pub clip_duration_secs: f64,

    /// Minimum confidence threshold for declaring a definitive language winner.
    pub min_confidence: f32,

    /// Minimum RMS audio energy threshold required to classify a clip as speech.
    pub silence_threshold_rms: f32,
}

impl Default for LidConfig {
    fn default() -> Self {
        Self {
            sample_count: 10,
            clip_duration_secs: 6.0,
            min_confidence: 0.60,
            silence_threshold_rms: 0.020,
        }
    }
}
