use anyhow::Result;
use std::path::Path;

use anchor_version_detector::{
    clean_version, detect_current_environment, detect_versions_recursive, DetectionReport,
    ScanOptions,
};

const EXPECTED_ARGS_COUNT: usize = 2;
const UNKNOWN_VERSION: &str = "Unknown";
const UNKNOWN_ANCHOR_VERSION: &str = "Unknown (may not be an Anchor project)";

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != EXPECTED_ARGS_COUNT {
        println!("Usage: {} <project_directory>", args[0]);
        return Ok(());
    }

    let report = detect_versions_recursive(Path::new(&args[1]), &ScanOptions::default())?;
    let current_env = detect_current_environment();

    println!("Detected/Inferred Versions:");
    print_detected_versions(&report);

    if !report.warnings.is_empty() {
        println!();
        for warning in &report.warnings {
            println!("Warning: {warning}");
        }
    }

    println!();
    print_current_environment(&current_env);

    println!("\nTo work with this project, configure your environment as follows:");
    println!("```");
    if let Some(rust_version) = &report.resolved.rust_version {
        println!("rustup default {}", clean_version(rust_version));
        println!("rustup component add rust-analyzer");
    }
    if let Some(solana_version) = &report.resolved.solana_version {
        println!("agave-install init {}", clean_version(solana_version));
    }
    if let Some(anchor_version) = &report.resolved.anchor_version {
        println!("avm use {}", clean_version(anchor_version));
    }
    println!("```");

    Ok(())
}

fn print_detected_versions(report: &DetectionReport) {
    let rust_source = report
        .sources
        .iter()
        .find(|source| matches!(source.field, anchor_version_detector::VersionField::Rust))
        .map(|source| format!("(from {})", source.path.display()))
        .unwrap_or_default();

    println!(
        "Rust: {} {}",
        report
            .resolved
            .rust_version
            .as_deref()
            .unwrap_or(UNKNOWN_VERSION),
        rust_source
    );
    println!(
        "Solana: {}",
        report
            .resolved
            .solana_version
            .as_deref()
            .unwrap_or(UNKNOWN_VERSION)
    );
    println!(
        "Anchor: {}",
        report
            .resolved
            .anchor_version
            .as_deref()
            .unwrap_or(UNKNOWN_ANCHOR_VERSION)
    );
}
fn print_current_environment(env: &anchor_version_detector::CurrentEnvironment) {
    println!("Current Environment:");
    println!(
        "Rust: {}",
        env.rust_version
            .as_deref()
            .unwrap_or("Not installed/not in PATH")
    );
    println!(
        "Solana: {}",
        env.solana_version
            .as_deref()
            .unwrap_or("Not installed/not in PATH")
    );
    println!(
        "Anchor: {}",
        env.anchor_version
            .as_deref()
            .unwrap_or("Not installed/not in PATH")
    );
}
