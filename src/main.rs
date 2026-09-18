//! CLI entrypoint for kikoe binary.

use std::process::ExitCode;

use clap::Parser;
use kikoe::cli::{Cli, Commands, DetectArgs, InspectArgs};
use kikoe::domain::LidConfig;
use kikoe::{KikoeEngine, LanguageRegistry, Result};

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

    if let (Some(track_idx), false) = (args.track, args.all) {
        return handle_single_stream_detect(&engine, &args, track_idx);
    }

    handle_multi_stream_detect(&engine, &args)
}

fn handle_single_stream_detect(
    engine: &KikoeEngine,
    args: &DetectArgs,
    track: usize,
) -> Result<()> {
    if !args.json {
        println!("🎧 聴こえ (kikoe) — Analyzing audio stream #{track}...");
        println!("📁 Target: {}", args.path.display());
    }

    let result = engine.detect_stream_language(&args.path, track)?;

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

fn handle_multi_stream_detect(engine: &KikoeEngine, args: &DetectArgs) -> Result<()> {
    let streams = KikoeEngine::inspect_streams(&args.path)?;
    if !args.json {
        println!(
            "🎧 聴こえ (kikoe) — Unified Multi-Track Analysis ({} audio streams)...",
            streams.len()
        );
        println!("📁 Target: {}", args.path.display());
    }

    let results = engine.detect_all_streams(&args.path)?;

    if args.json {
        let json_str = serde_json::to_string_pretty(&results)?;
        println!("{json_str}");
        return Ok(());
    }

    println!(
        "\n======================================================================================="
    );
    println!(
        "{:<6} {:<8} {:<14} {:<6} {:<24} {:<12} Status",
        "Stream", "Codec", "Channels", "Tag", "Identified Language", "Confidence"
    );
    println!(
        "───────────────────────────────────────────────────────────────────────────────────────"
    );

    for (s, res) in streams.iter().zip(results.iter()) {
        let tag = s.metadata_lang.as_deref().unwrap_or("und");
        let ch_desc = if s.channels == 2 {
            "2.0 Stereo"
        } else if s.channels == 6 {
            "5.1 Surround"
        } else {
            "Other"
        };
        let lang_desc = format!("{} ({})", res.winner_name, res.winner_code);
        let conf_desc = format!("{:.1}%", res.overall_confidence * 100.0);
        let tag_name = LanguageRegistry::name_for_code(tag);
        let status =
            if tag_name != "Unknown" && tag_name.to_lowercase() == res.winner_name.to_lowercase() {
                "MATCH"
            } else if tag == "und" {
                "RESOLVED"
            } else {
                "MISMATCH"
            };

        println!(
            "#{:<5} {:<8} {:<14} {:<6} {:<24} {:<12} {}",
            res.stream_index, s.codec_name, ch_desc, tag, lang_desc, conf_desc, status
        );
    }
    println!(
        "======================================================================================="
    );
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
