//! Infrastructure layer: FFmpeg probing, in-memory audio extraction, and ONNX engine.

pub mod engine;
pub mod probe;
pub mod sampler;

pub use engine::{LanguageRegistry, LidModelEngine};
pub use probe::MediaProbe;
pub use sampler::AudioSampler;
