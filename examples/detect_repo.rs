use std::path::Path;

use anchor_version_detector::{detect_versions_recursive, ScanOptions};
use anyhow::Result;

fn main() -> Result<()> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../some-solana-repo".to_string());

    let report = detect_versions_recursive(Path::new(&target), &ScanOptions::default())?;

    println!("detected: {:?}", report.detected);
    println!("resolved: {:?}", report.resolved);
    println!("compatibility: {:?}", report.compatibility);

    for source in &report.sources {
        println!(
            "{:?} from {} => {}",
            source.field,
            source.path.display(),
            source.value
        );
    }

    for warning in &report.warnings {
        println!("warning: {warning}");
    }

    Ok(())
}
