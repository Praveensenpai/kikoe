//! Command-line argument specifications for the kikoe CLI.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// ⛩️ 聴こえ (kikoe) — Temporal multi-slice audio language identification.
#[derive(Debug, Parser)]
#[command(name = "kikoe", version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands for kikoe.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run multi-slice language identification on an audio stream.
    Detect(DetectArgs),
    /// Inspect all audio streams present in a media container.
    Inspect(InspectArgs),
}

/// Arguments for the `detect` subcommand.
#[derive(Debug, Args)]
pub struct DetectArgs {
    /// Path to media container file (mkv, mp4, webm, etc.).
    pub path: PathBuf,

    /// Zero-based audio stream index to analyze.
    #[arg(short = 't', long = "track", default_value_t = 0)]
    pub track: usize,

    /// Number of temporal slices to sample across the file.
    #[arg(short = 's', long = "samples", default_value_t = 6)]
    pub samples: usize,

    /// Duration of each temporal slice in seconds.
    #[arg(short = 'd', long = "duration", default_value_t = 6.0)]
    pub duration: f64,

    /// Output results formatted as JSON.
    #[arg(long = "json")]
    pub json: bool,
}

/// Arguments for the `inspect` subcommand.
#[derive(Debug, Args)]
pub struct InspectArgs {
    /// Path to media container file.
    pub path: PathBuf,

    /// Output stream metadata formatted as JSON.
    #[arg(long = "json")]
    pub json: bool,
}
