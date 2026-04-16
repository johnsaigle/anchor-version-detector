## Anchor Version Detector

A Rust library and CLI for detecting and resolving compatible Rust, Solana/Agave, and Anchor versions from Solana project files.

The crate is now library-first: other tools can depend on it directly and consume structured compatibility data instead of scraping CLI output.

## Features

- Detects versions from:
  - `rust-toolchain`
  - `rust-toolchain.toml`
  - `Anchor.toml`
  - `Cargo.toml`
- Recursively scans subdirectories when root-level signals are incomplete
- Exposes a public compatibility matrix with notes and source citations
- Returns structured detection reports with:
  - raw detected versions
  - resolved versions
  - inference metadata
  - source file provenance
  - warnings
- Includes a thin CLI wrapper for terminal use

## Installation

Library dependency:

```toml
[dependencies]
anchor-version-detector = "1"
```

CLI usage from this repo:

```bash
cargo run -- /path/to/solana/project
```

## Library API

Primary entry points:

- `detect_versions_recursive`
- `detect_versions_in_dir`
- `compatibility_rules`
- `find_rule_by_anchor`
- `find_rule_by_solana`
- `resolve_versions`
- `detect_current_environment`

Key result types:

- `DetectionReport`
- `ProjectVersions`
- `CompatibilityAssessment`
- `CompatibilityRule`
- `VersionSource`
- `ScanOptions`

## Examples

Detect versions for a repository and inspect the resolved output:

```rust
use std::path::Path;

use anchor_version_detector::{ScanOptions, detect_versions_recursive};

fn main() -> anyhow::Result<()> {
    let report = detect_versions_recursive(Path::new("../some-solana-repo"), &ScanOptions::default())?;

    println!("detected: {:?}", report.detected);
    println!("resolved: {:?}", report.resolved);
    println!("compatibility confidence: {:?}", report.compatibility.confidence);

    for source in &report.sources {
        println!("{:?} came from {}", source.field, source.path.display());
    }

    Ok(())
}
```

Look up a known compatibility rule directly by Anchor version:

```rust
use anchor_version_detector::find_rule_by_anchor;

fn main() {
    let rule = find_rule_by_anchor("^0.30.1").expect("known Anchor version");

    println!("Anchor {} -> Solana {} -> Rust {}", rule.anchor, rule.solana, rule.rust);
    println!("why: {}", rule.notes);
    println!("source: {}", rule.source);
}
```

Resolve partial version input inside another tool:

```rust
use anchor_version_detector::{ProjectVersions, resolve_versions};

fn main() -> anyhow::Result<()> {
    let detected = ProjectVersions {
        rust_version: None,
        solana_version: None,
        anchor_version: Some("0.31.0".to_string()),
    };

    let (resolved, assessment, warnings) = resolve_versions(&detected)?;

    assert_eq!(resolved.solana_version.as_deref(), Some("2.1.0"));
    assert_eq!(resolved.rust_version.as_deref(), Some("1.84.1"));
    assert!(warnings.is_empty());

    println!("reason: {:?}", assessment.reason);
    Ok(())
}
```

Use non-recursive scanning when the caller wants tighter control over traversal:

```rust
use std::path::Path;

use anchor_version_detector::{ScanOptions, detect_versions_recursive};

fn main() -> anyhow::Result<()> {
    let options = ScanOptions {
        recursive: false,
        ..ScanOptions::default()
    };

    let report = detect_versions_recursive(Path::new("../single-package"), &options)?;
    println!("resolved versions: {:?}", report.resolved);
    Ok(())
}
```

## CLI Output

Example:

```text
Detected/Inferred Versions:
Rust: 1.76.0 (from /path/to/project/rust-toolchain)
Solana: 1.18.17
Anchor: 0.30.1

Current Environment:
Rust: 1.76.0
Solana: 2.1.0
Anchor: 0.30.1

To work with this project, configure your environment as follows:
```

The CLI is intentionally minimal. If you need richer metadata, use the library API instead.

## Detection Flow

1. Read version signals from the current directory.
2. Recursively scan subdirectories if configured and required.
3. Build a compatibility assessment from detected Solana or Anchor versions.
4. Resolve missing fields from the compatibility matrix.
5. Return a structured report with provenance and warnings.

## Supported Files

`rust-toolchain` plain-text form:

```text
1.76.0
```

`rust-toolchain.toml` form:

```toml
[toolchain]
channel = "1.76.0"
```

`Anchor.toml` form:

```toml
[toolchain]
anchor_version = "0.30.1"
solana_version = "1.18.17"
```

`Cargo.toml` form:

```toml
[dependencies]
solana-program = ">=1.18,<=2"
anchor-lang = { version = "0.30.1" }
```

Workspace dependencies are also supported:

```toml
[workspace.dependencies]
solana-program = "1.18.17"
anchor-lang = "0.30.1"
```

## Notes

- If a project does not look like a Solana or Anchor project, the detector returns an error instead of inventing compatibility data.
- If an exact rule cannot be found, the resolver falls back to the latest known compatible versions and records warnings.
- Directory traversal skips common build and cache paths by default.
