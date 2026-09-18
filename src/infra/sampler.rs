//! In-memory audio extraction and voice activity / RMS energy checking.

use std::path::Path;
use std::process::Command;

use crate::domain::AudioSample;
use crate::error::{KikoeError, Result};

/// Sampler extracting 16kHz mono f32le PCM audio slices directly into memory.
pub struct AudioSampler;

impl AudioSampler {
    /// Extracts a slice of audio from the specified stream into normalized f32 PCM.
    pub fn extract_slice<P: AsRef<Path>>(
        path: P,
        audio_stream_idx: usize,
        timestamp_secs: f64,
        duration_secs: f64,
        silence_threshold_rms: f32,
    ) -> Result<AudioSample> {
        let stream_selector = format!("0:a:{audio_stream_idx}");
        let timestamp_str = format!("{timestamp_secs:.3}");
        let duration_str = format!("{duration_secs:.3}");

        let output = Command::new("ffmpeg")
            .args(["-ss", &timestamp_str, "-i"])
            .arg(path.as_ref())
            .args([
                "-map",
                &stream_selector,
                "-t",
                &duration_str,
                "-ac",
                "1",
                "-ar",
                "16000",
                "-f",
                "f32le",
                "-v",
                "quiet",
                "pipe:1",
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KikoeError::Ffmpeg(format!(
                "FFmpeg slice extraction failed: {stderr}"
            )));
        }

        let pcm_samples = Self::bytes_to_f32(&output.stdout);
        let rms_energy = Self::calculate_rms(&pcm_samples);
        let is_speech = rms_energy >= silence_threshold_rms;

        Ok(AudioSample {
            timestamp_secs,
            pcm_samples,
            rms_energy,
            is_speech,
        })
    }

    /// Converts raw little-endian bytes to normalized f32 audio samples.
    fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|chunk| {
                let arr: [u8; 4] = [chunk[0], chunk[1], chunk[2], chunk[3]];
                f32::from_le_bytes(arr)
            })
            .collect()
    }

    /// Computes Root Mean Square (RMS) energy metric across samples.
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }
}
