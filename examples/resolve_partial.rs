use anchor_version_detector::{resolve_versions, ProjectVersions};
use anyhow::Result;

fn main() -> Result<()> {
    let detected = ProjectVersions {
        rust_version: None,
        solana_version: None,
        anchor_version: Some("0.31.0".to_string()),
    };

    let (resolved, assessment, warnings) = resolve_versions(&detected)?;

    println!("resolved: {:?}", resolved);
    println!("assessment: {:?}", assessment);
    println!("warnings: {:?}", warnings);

    Ok(())
}
