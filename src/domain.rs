//! Domain entities, configuration, and data structures.

pub mod config;
pub mod models;

pub use config::LidConfig;
pub use models::{AudioSample, LanguageScore, LidResult, SlicePrediction, StreamInfo};
