//! CLI entrypoint for kikoe binary.

use clap::Parser;
use std::process::ExitCode;

use kikoe::cli::{Cli, Commands, DetectArgs, InspectArgs};
use kikoe::domain::LidConfig;
use kikoe::{KikoeEngine, Result};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(err) = run_command(cli) {
        eprintln!("❌ Error: {err}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Detect(args) => handle_detect(args),
        Commands::Inspect(args) => handle_inspect(args),
    }
}

fn handle_detect(args: DetectArgs) -> Result<()> {
    let config = LidConfig {
        sample_count: args.samples,
        clip_duration_secs: args.duration,
        ..Default::default()
    };
    let engine = KikoeEngine::with_config(config)?;

    if !args.json {
        println!(
            "🎧 聴こえ (kikoe) — Analyzing audio stream #{}...",
            args.track
        );
        println!("📁 Target: {}", args.path.display());
    }

    let result = engine.detect_stream_language(&args.path, args.track)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&result)?;
        println!("{json_str}");
        return Ok(());
    }

    println!("\n=======================================================");
    println!(
        "🎯 Language: {} ({}) — Confidence: {:.1}%",
        result.winner_name,
        result.winner_code,
        result.overall_confidence * 100.0
    );
    println!("⏱️  Media Duration: {:.1}s", result.duration_secs);
    println!(
        "📊 Slices Evaluated: {} (Voiced: {:.0}%)",
        result.slices.len(),
        result.voiced_ratio * 100.0
    );
    println!("───────────────────────────────────────────────────────");
    for (i, slice) in result.slices.iter().enumerate() {
        println!(
            "  [{}] Offset: {:>6.1}s | Top: {:<3} ({:>5.1}%) | RMS: {:.4}",
            i + 1,
            slice.timestamp_secs,
            slice.top_language,
            slice.confidence * 100.0,
            slice.rms_energy
        );
    }
    println!("=======================================================");
    Ok(())
}

fn handle_inspect(args: InspectArgs) -> Result<()> {
    let streams = KikoeEngine::inspect_streams(&args.path)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&streams)?;
        println!("{json_str}");
        return Ok(());
    }

    println!("🎬 Container Audio Streams: {}", args.path.display());
    println!("───────────────────────────────────────────────────────────────────────");
    println!(
        "{:<4} {:<8} {:<10} {:<10} {:<6} Title",
        "#", "Codec", "Channels", "SampleRate", "Lang"
    );
    println!("───────────────────────────────────────────────────────────────────────");
    for s in streams {
        let lang = s.metadata_lang.as_deref().unwrap_or("und");
        let title = s.track_title.as_deref().unwrap_or("-");
        let ch_desc = if s.channels == 2 {
            "2.0 (Stereo)"
        } else if s.channels == 6 {
            "5.1 (Surround)"
        } else {
            "Other"
        };
        println!(
            "{:<4} {:<8} {:<10} {:<10} {:<6} {}",
            s.index, s.codec_name, ch_desc, s.sample_rate, lang, title
        );
    }
    println!("───────────────────────────────────────────────────────────────────────");
    Ok(())
}
