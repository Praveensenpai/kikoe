# CODEBASE.md: kikoe Semantic Digest

> **Notice**: This file is an AI-optimized semantic index. Do not write narrative prose. Keep token density high.

## 1. System Topology & Data Flow
```text
CLI (src/main.rs / src/cli/args.rs)
  │
  ▼
KikoeEngine (src/lib.rs)
  ├── 1. MediaProbe::probe_audio_streams / probe_duration (src/infra/probe.rs via ffprobe)
  ├── 2. calculate_sampling_timestamps (6 slices between 15% - 85% runtime)
  ├── 3. AudioSampler::extract_slice (src/infra/sampler.rs via ffmpeg f32le PCM + RMS VAD)
  ├── 4. LidModelEngine::predict_slice (src/infra/engine.rs via ONNX Runtime VoxLingua107)
  └── 5. aggregate_votes (tiered confidence-weighted dialogue aggregation)
```

## 2. Global Constraints & Architecture Patterns
- **Primary Language & Edition**: Rust 2024 Edition (`kikoe = "0.1.0"`)
- **Architectural Paradigm**: Role-based domain architecture (`domain/`, `infra/`, `cli/`, root `lib.rs` and `main.rs`)
- **Hard Constraints**: <400 lines/file, <60 lines/fn, zero production `unwrap()`/`expect()`, 0 warnings (`-D warnings`).
- **Dependencies**: `ort` (ONNX Runtime), `clap`, `serde`, `serde_json`, `thiserror`.

## 3. Module & Interface Skeleton

### `src/domain/config.rs` (Role: domain, Lines: 30)
- **Responsibility**: Runtime parameters for sampling count, clip durations, and thresholds.
- **Types**:
  - `pub struct LidConfig { pub sample_count: usize, pub clip_duration_secs: f64, pub min_confidence: f32, pub silence_threshold_rms: f32 }`

### `src/domain/models.rs` (Role: domain, Lines: 78)
- **Responsibility**: Typed data representations for streams, samples, and language detection results.
- **Types**:
  - `pub struct StreamInfo { pub index: usize, pub codec_name: String, pub channels: usize, pub sample_rate: u32, pub metadata_lang: Option<String>, pub track_title: Option<String> }`
  - `pub struct AudioSample { pub timestamp_secs: f64, pub pcm_samples: Vec<f32>, pub rms_energy: f32, pub is_speech: bool }`
  - `pub struct LanguageScore { pub code: String, pub name: String, pub confidence: f32 }`
  - `pub struct SlicePrediction { pub timestamp_secs: f64, pub top_language: String, pub confidence: f32, pub candidates: Vec<LanguageScore>, pub rms_energy: f32 }`
  - `pub struct LidResult { pub stream_index: usize, pub winner_code: String, pub winner_name: String, pub overall_confidence: f32, pub duration_secs: f64, pub slices: Vec<SlicePrediction>, pub voiced_ratio: f32 }`

### `src/error.rs` (Role: domain, Lines: 34)
- **Responsibility**: Typed Kikoe error domain.
- **Types**:
  - `pub enum KikoeError { Io, Json, Ffmpeg(String), StreamNotFound(usize), Model(String), InsufficientSpeech }`
  - `pub type Result<T> = std::result::Result<T, KikoeError>;`

### `src/infra/probe.rs` (Role: infra, Lines: 111)
- **Responsibility**: Subprocess wrappers for `ffprobe` to query container duration and audio stream metadata.
- **Functions**:
  - `MediaProbe::probe_duration(path: P) -> Result<f64>`
  - `MediaProbe::probe_audio_streams(path: P) -> Result<Vec<StreamInfo>>`

### `src/infra/sampler.rs` (Role: infra, Lines: 83)
- **Responsibility**: In-memory 16kHz mono `f32le` PCM extraction via `ffmpeg` and RMS energy calculation.
- **Functions**:
  - `AudioSampler::extract_slice(path: P, audio_stream_idx: usize, timestamp_secs: f64, duration_secs: f64, silence_threshold_rms: f32) -> Result<AudioSample>`

### `src/infra/engine.rs` (Role: infra, Lines: 235)
- **Responsibility**: Model loading, execution, softmax computation, and 107-class language mapping.
- **Types**:
  - `pub struct LanguageRegistry;`
  - `pub struct LidModelEngine { session: Mutex<Session> }`
- **Functions**:
  - `LanguageRegistry::name_for_code(code: &str) -> String`
  - `LidModelEngine::new() -> Result<Self>`
  - `LidModelEngine::from_file<P: AsRef<Path>>(path: P) -> Result<Self>`
  - `LidModelEngine::resolve_model_path() -> Result<PathBuf>`
  - `LidModelEngine::predict_slice(&self, sample: &AudioSample) -> Result<SlicePrediction>`

### `src/lib.rs` (Role: api, Lines: 156)
- **Responsibility**: Public crate interface and temporal voting aggregator.
- **Types**:
  - `pub struct KikoeEngine { config: LidConfig, model: LidModelEngine }`
- **Functions**:
  - `KikoeEngine::new() -> Result<Self>`
  - `KikoeEngine::with_config(config: LidConfig) -> Result<Self>`
  - `KikoeEngine::inspect_streams<P: AsRef<Path>>(path: P) -> Result<Vec<StreamInfo>>`
  - `KikoeEngine::detect_stream_language<P: AsRef<Path>>(&self, path: P, stream_index: usize) -> Result<LidResult>`
  - `KikoeEngine::calculate_sampling_timestamps(duration_secs: f64, count: usize) -> Vec<f64>`

### `src/cli/args.rs` & `src/main.rs` (Role: cli, Lines: 167)
- **Responsibility**: Command-line interface with `detect` and `inspect` subcommands.
