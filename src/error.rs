//! Centralized typed error definitions for kikoe.

use thiserror::Error;

/// Core domain error enumeration for kikoe operations.
#[derive(Debug, Error)]
pub enum KikoeError {
    /// Standard I/O or subprocess communication failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization failure.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// FFmpeg or FFprobe probe failure.
    #[error("FFmpeg execution failed: {0}")]
    Ffmpeg(String),

    /// Requested audio stream not found in container.
    #[error("Audio stream #{0} was not found in container")]
    StreamNotFound(usize),

    /// Neural network model loading or inference error.
    #[error("Model inference error: {0}")]
    Model(String),

    /// Audio samples did not contain enough voiced speech to classify.
    #[error("Insufficient speech detected in audio stream")]
    InsufficientSpeech,
}

/// Convenience Result alias for kikoe.
pub type Result<T> = std::result::Result<T, KikoeError>;
