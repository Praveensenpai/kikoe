//! Container probing utility using ffprobe.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

use crate::domain::StreamInfo;
use crate::error::{KikoeError, Result};

/// Probe wrapper inspecting audio streams and total container duration.
pub struct MediaProbe;

impl MediaProbe {
    /// Probes media container duration in seconds.
    pub fn probe_duration<P: AsRef<Path>>(path: P) -> Result<f64> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
            ])
            .arg(path.as_ref())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KikoeError::Ffmpeg(format!(
                "Failed to probe duration: {stderr}"
            )));
        }

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let duration: f64 = stdout_str
            .trim()
            .parse()
            .map_err(|_| KikoeError::Ffmpeg(format!("Invalid duration string: {stdout_str}")))?;

        Ok(duration)
    }

    /// Queries all audio streams present in the container.
    pub fn probe_audio_streams<P: AsRef<Path>>(path: P) -> Result<Vec<StreamInfo>> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "a",
                "-show_entries",
                "stream=index,codec_name,channels,sample_rate:stream_tags=language,title",
                "-of",
                "json",
            ])
            .arg(path.as_ref())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(KikoeError::Ffmpeg(format!(
                "Failed to probe audio streams: {stderr}"
            )));
        }

        let json_val: Value = serde_json::from_slice(&output.stdout)?;
        let mut streams = Vec::new();

        if let Some(stream_arr) = json_val.get("streams").and_then(Value::as_array) {
            for (idx, stream_obj) in stream_arr.iter().enumerate() {
                let codec = stream_obj
                    .get("codec_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let channels = stream_obj
                    .get("channels")
                    .and_then(Value::as_u64)
                    .unwrap_or(2) as u32;
                let sample_rate = stream_obj
                    .get("sample_rate")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(48000);

                let tags = stream_obj.get("tags");
                let metadata_lang = tags
                    .and_then(|t| t.get("language"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                let track_title = tags
                    .and_then(|t| t.get("title"))
                    .and_then(Value::as_str)
                    .map(ToString::to_string);

                streams.push(StreamInfo {
                    index: idx,
                    codec_name: codec,
                    channels,
                    sample_rate,
                    metadata_lang,
                    track_title,
                });
            }
        }

        Ok(streams)
    }
}
