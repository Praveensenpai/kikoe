# ⛩️ 聴こえ (kikoe)

> **Ultra-fast temporal multi-slice spoken language identification (LID) in pure Rust.**

[![Latest Release](https://img.shields.io/github/v/release/Praveensenpai/kikoe?style=for-the-badge&color=cba6f7)](https://github.com/Praveensenpai/kikoe/releases)
[![Rust Edition](https://img.shields.io/badge/Rust-2024%20Edition-DEA584?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![ONNX Runtime](https://img.shields.io/badge/Engine-ONNX%20Runtime-blue?style=for-the-badge&logo=onnx&logoColor=white)](https://onnxruntime.ai/)
[![Platform](https://img.shields.io/badge/Platform-Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black)](https://github.com/Praveensenpai/kikoe)
[![License](https://img.shields.io/badge/License-MIT-a6e3a1?style=for-the-badge)](LICENSE)

[⚡ Quick Install](#-quick-start) • [🎯 Why kikoe?](#-why-kikoe-the-purpose) • [✨ Key Features](#-key-features) • [🔄 Architecture](#-architecture--sampling-workflow) • [💻 CLI Usage](#-cli-usage--workflows) • [📜 License](#-license)

> [!TIP]
> **Zero Tag Reliance · 100% Raw Acoustic Waveform Classification**  
> Container metadata tags in multi-audio releases (e.g., `und`, generic titles) are frequently untrustworthy, missing, or stripped. `kikoe` ignores container text tags completely and determines the true spoken language directly from raw audio waveforms across 107 languages using deep neural acoustic embeddings.

`kikoe` (聴こえ — *“audible / ability to hear”*) is a high-performance terminal utility and Rust library designed to provide **ground-truth spoken language verification** for media containers (`.mkv`, `.mp4`, `.webm`) and audio files.

---

## 🎯 Why kikoe? (The Purpose)

### 1. The Untrusted Metadata Dilemma
Multi-audio releases across tracker platforms (e.g., 1TamilMV, DUST, scene encoders) regularly bundle 4 to 8 audio streams in diverse regional languages (Tamil, Telugu, Hindi, Malayalam, Kannada, English, Japanese). However, container metadata tags are notoriously broken:
- **Missing or Undetermined Tags**: Audio streams are frequently labeled as `und` (undetermined) or have generic names like `Audio Track 1`.
- **Spam & Watermarked Titles**: Track titles are routinely overwritten with release-group spam (e.g., `www.1TamilMV.meme - [AAC2.0 - 64Kbps]`) with zero language hints.
- **Mislabeled Containers**: Audio streams are often swapped or incorrectly mapped during encoding (e.g., a Telugu dub marked with the `tam` tag).

### 2. Acoustic Truth Over Text Tags
Automated media pipelines and dub-stripping utilities cannot blindly rely on metadata tags without risking catastrophic data loss (e.g., stripping the original native theatrical track by mistake).

`kikoe` provides an objective, empirical ground truth:
- **It listens rather than reads**: Instead of trusting text tags, `kikoe` feeds raw audio sound waves through deep neural acoustic embeddings (`voxlingua107`).
- **Synchronized Dialogue Sampling**: Non-contiguously samples strategic runtime slices avoiding logos, intro music, and credits.
- **Cross-Track BGM Cancellation**: Correlates streams across the container. When background explosions or soundtracks trigger false positives across all tracks (e.g. `jw` or `la`), `kikoe` discards the music slice and isolates authentic spoken dialogue.

### 3. Purpose & Ecosystem Role
`kikoe` was built to serve as the core acoustic verification engine across the media automation stack:
- **Engine for [`dubstrip`](https://github.com/Praveensenpai/dubstrip)**: Enables lossless pruning of secondary dub tracks by identifying the true native theatrical stream, even when all audio tags are `und`.
- **Engine for [`jpsan`](https://github.com/Praveensenpai/jpsan)**: Accurately identifies authentic Japanese audio streams in raw anime containers for subtitle alignment and sentence mining.
- **Media Server Curation (Jellyfin / Plex / Radarr)**: Batch-inspects and tags missing language codes in entire media libraries without manual listening.
- **Standalone Terminal Tool**: Enables videophiles and remuxers to inspect and verify unknown audio tracks in seconds.

---

## 🔄 Architecture & Sampling Workflow

```text
┌─────────────────────────────────────────────────────────────┐
│                    🎬 Media Container Input                 │
│               (.mkv / .mp4 / .webm multi-track)             │
└──────────────────────────────┬──────────────────────────────┘
                               │  1. Fast ffprobe stream inspection (< 0.05s)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│               🎯 Strategic Multi-Slice Sampler               │
│        · 6 temporal slices sampled across 15% - 85% runtime │
│        · Skips intro logos, theme music, and end credits    │
└──────────────────────────────┬──────────────────────────────┘
                               │  2. Extract 16kHz mono f32 PCM in-memory
                               ▼
┌─────────────────────────────────────────────────────────────┐
│               🎙️ RMS Energy Voice Activity Check            │
│         · Computes RMS energy to verify active audio        │
│         · Discards low-energy dead zones and silence        │
└──────────────────────────────┬──────────────────────────────┘
                               │  3. Raw waveform tensor [1, samples]
                               ▼
┌─────────────────────────────────────────────────────────────┐
│            🧠 VoxLingua107 ONNX Acoustic Engine             │
│        · 107-class neural spoken language identification    │
│        · Numerically stable softmax probability mapping     │
└──────────────────────────────┬──────────────────────────────┘
                               │  4. Tiered confidence-weighted aggregation
                               ▼
┌─────────────────────────────────────────────────────────────┐
│        🏆 Definite Winner: Tamil (ta) — 99.3% Conf          │
│            · Secondary Candidates & Per-Slice Stats         │
└─────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

- **⚡ Sub-Second Acoustic Inference**: Driven by ONNX Runtime (`ort`) on 16kHz raw PCM buffers—no slow Python runtimes or heavy ML frameworks required.
- **🎯 107 Spoken Languages Supported**: Out-of-the-box acoustic identification across world languages including English, Japanese, Tamil, Telugu, Hindi, Malayalam, Kannada, Korean, Chinese, Spanish, German, French, and more.
- **🛡️ Battle-Tested Heuristic Sampling**: Samples 6 strategically distributed clips (avoiding intros and outros) with tiered confidence weighting to eliminate background music, explosions, and soundtrack bias.
- **🔎 Instant Container Inspection**: Non-destructively lists audio tracks, codec details, channel layouts (2.0 stereo / 5.1 surround), and language metadata via `kikoe inspect`.
- **📦 Clean Integration Ready**: Built as both a standalone CLI binary and an embeddable Rust library (`KikoeEngine`) for tools like [`dubstrip`](https://github.com/Praveensenpai/dubstrip) and [`jpsan`](https://github.com/Praveensenpai/jpsan).
- **📊 Machine-Readable Output**: Full `--json` support for seamless piping into shell scripts, daemons, and media server automation pipelines.

---

## 🚀 Quick Start

### 🪄 One-Liner Magic (Recommended)

Paste this into your terminal to install `kikoe` automatically:

```bash
curl -fsSL https://raw.githubusercontent.com/Praveensenpai/kikoe/main/install.sh | bash
```

<br>

### 🛠️ Building From Source

```bash
git clone https://github.com/Praveensenpai/kikoe.git
cd kikoe
cargo build --release
install -Dm 755 target/release/kikoe ~/.local/bin/kikoe
```

---

## 💻 CLI Usage & Workflows

### 1. Unified Multi-Track Analysis (All Streams)

Analyze all audio streams simultaneously using synchronized temporal slices with cross-track BGM cancellation:

```bash
kikoe detect "Reacher.S04E08.720p.mkv"
```

```text
🎧 聴こえ (kikoe) — Unified Multi-Track Analysis (6 audio streams)...
📁 Target: Reacher.S04E08.720p.mkv

=======================================================================================
Stream Codec    Channels       Tag    Identified Language      Confidence   Status
───────────────────────────────────────────────────────────────────────────────────────
#0     aac      2.0 Stereo     tam    Tamil (ta)               97.6%        MATCH
#1     aac      2.0 Stereo     tel    Telugu (te)              99.1%        MATCH
#2     aac      2.0 Stereo     hin    Hindi (hi)               87.7%        MATCH
#3     aac      2.0 Stereo     mal    Malayalam (ml)           98.8%        MATCH
#4     aac      2.0 Stereo     kan    Kannada (kn)             98.9%        MATCH
#5     aac      2.0 Stereo     eng    English (en)             99.1%        MATCH
=======================================================================================
```

### 2. Single Stream Detailed Drill-Down

Inspect per-slice predictions and energy metrics for an individual stream:

```bash
kikoe detect "Reacher.S04E08.720p.mkv" --track 0
```

```text
🎧 聴こえ (kikoe) — Analyzing audio stream #0...
📁 Target: Reacher.S04E08.720p.mkv

=======================================================
🎯 Language: Tamil (ta) — Confidence: 99.3%
⏱️  Media Duration: 3463.5s
📊 Slices Evaluated: 10 (Voiced: 100%)
───────────────────────────────────────────────────────
  [1] Offset:  519.5s | Top: te  ( 95.6%) | RMS: 0.0511
  [2] Offset:  788.9s | Top: ta  ( 95.5%) | RMS: 0.0828
  ...
  [9] Offset: 2674.6s | Top: ta  ( 99.6%) | RMS: 0.0324
=======================================================
```

### 2. Inspect Container Audio Streams

List all available audio streams without running inference:

```bash
kikoe inspect "Reacher.S04E08.720p.mkv"
```

```text
🎬 Container Audio Streams: Reacher.S04E08.720p.mkv
───────────────────────────────────────────────────────────────────────
#    Codec    Channels   SampleRate Lang   Title
───────────────────────────────────────────────────────────────────────
0    aac      2.0 (Stereo) 48000      tam    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
1    aac      2.0 (Stereo) 48000      tel    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
2    aac      2.0 (Stereo) 48000      hin    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
3    aac      2.0 (Stereo) 48000      mal    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
4    aac      2.0 (Stereo) 48000      kan    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
5    aac      2.0 (Stereo) 48000      eng    www.1TamilMV.meme - [AAC2.0 - 64Kbps]
───────────────────────────────────────────────────────────────────────
```

### 3. JSON Output (Scripting & Automation)

```bash
kikoe detect "movie.mkv" --track 0 --json
```

```json
{
  "stream_index": 0,
  "winner_code": "ta",
  "winner_name": "Tamil",
  "overall_confidence": 0.993,
  "duration_secs": 3463.5,
  "voiced_ratio": 0.833,
  "slices": [ ... ]
}
```

---

## 🦀 Rust Crate Integration

`kikoe` can be included as a Rust library dependency:

```toml
[dependencies]
kikoe = { git = "https://github.com/Praveensenpai/kikoe.git" }
```

```rust
use kikoe::{KikoeEngine, domain::LidConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = KikoeEngine::new()?;
    let result = engine.detect_stream_language("movie.mkv", 0)?;

    println!("Detected: {} ({:.1}%)", result.winner_name, result.overall_confidence * 100.0);
    Ok(())
}
```

---

## 📜 License

Licensed under the [MIT License](LICENSE).  
Crafted with 🦀 by Praveen Senpai ([@Praveensenpai](https://github.com/Praveensenpai)).
