use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

use crate::compatibility::resolve_versions;
use crate::parse::{parse_anchor_toml, parse_cargo_toml, parse_rust_toolchain};
use crate::types::{
    DetectionReport, ProjectVersions, ScanOptions, VersionField, VersionSource, VersionSourceKind,
};

const RUST_TOOLCHAIN_FILES: &[&str] = &["rust-toolchain", "rust-toolchain.toml"];
const MAX_RUST_TOOLCHAIN_FILE_SIZE: usize = 10_000;
const MAX_TOML_FILE_SIZE: usize = 100_000;

/// Detect version signals from files in a single directory.
///
/// # Errors
///
/// Returns an error when project files cannot be read or exceed enforced size limits.
pub fn detect_versions_in_dir(
    project_path: &Path,
) -> Result<(ProjectVersions, Vec<VersionSource>)> {
    let mut versions = ProjectVersions::default();
    let mut sources = Vec::new();

    check_rust_toolchain_files(project_path, &mut versions, &mut sources)?;
    check_anchor_toml(project_path, &mut versions, &mut sources)?;
    check_cargo_toml(project_path, &mut versions, &mut sources)?;

    Ok((versions, sources))
}

/// Detect versions for a project path, optionally recursing into subdirectories.
///
/// # Errors
///
/// Returns an error when the path is invalid, project files cannot be read,
/// or the directory does not appear to be a Solana project.
pub fn detect_versions_recursive(
    project_path: &Path,
    options: &ScanOptions,
) -> Result<DetectionReport> {
    let project_path = validate_project_path(project_path)?;
    let (mut detected, mut sources) = detect_versions_in_dir(&project_path)?;

    if options.recursive && detected.needs_more_info() {
        search_subdirectories(&project_path, options, &mut detected, &mut sources)?;
    }

    let (resolved, compatibility, warnings) = resolve_versions(&detected)?;

    Ok(DetectionReport {
        detected,
        resolved,
        compatibility,
        sources,
        warnings,
    })
}

fn validate_project_path(project_path: &Path) -> Result<PathBuf> {
    if !project_path.exists() {
        return Err(anyhow!(
            "Project directory does not exist: {}",
            project_path.display()
        ));
    }

    if !project_path.is_dir() {
        return Err(anyhow!(
            "Path is not a directory: {}",
            project_path.display()
        ));
    }

    project_path.canonicalize().map_err(|error| {
        anyhow!(
            "Failed to canonicalize path {}: {}",
            project_path.display(),
            error
        )
    })
}

fn search_subdirectories(
    dir: &Path,
    options: &ScanOptions,
    versions: &mut ProjectVersions,
    sources: &mut Vec<VersionSource>,
) -> Result<()> {
    let entries = fs::read_dir(dir)
        .map_err(|error| anyhow!("Failed to read directory {}: {}", dir.display(), error))?;

    for entry in entries {
        let entry = entry.map_err(|error| anyhow!("Failed to read directory entry: {}", error))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if options.skip_directories.contains(&dir_name) {
            continue;
        }

        let (sub_versions, sub_sources) = detect_versions_in_dir(&path)?;
        versions.merge_missing_from(&sub_versions);
        sources.extend(sub_sources);

        if versions.needs_more_info() {
            search_subdirectories(&path, options, versions, sources)?;
        }

        if !versions.needs_more_info() {
            break;
        }
    }

    Ok(())
}

fn check_rust_toolchain_files(
    project_path: &Path,
    versions: &mut ProjectVersions,
    sources: &mut Vec<VersionSource>,
) -> Result<()> {
    for filename in RUST_TOOLCHAIN_FILES {
        let path = project_path.join(filename);
        if !path.exists() {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|error| anyhow!("Failed to read {}: {}", path.display(), error))?;

        if content.len() > MAX_RUST_TOOLCHAIN_FILE_SIZE {
            return Err(anyhow!("File {} is too large (>10KB)", path.display()));
        }

        if let Ok(version) = parse_rust_toolchain(&content) {
            set_version(
                versions,
                sources,
                VersionField::Rust,
                VersionSourceKind::RustToolchain,
                &path,
                version,
            );
        }
        break;
    }

    Ok(())
}

fn check_anchor_toml(
    project_path: &Path,
    versions: &mut ProjectVersions,
    sources: &mut Vec<VersionSource>,
) -> Result<()> {
    let path = project_path.join("Anchor.toml");
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| anyhow!("Failed to read {}: {}", path.display(), error))?;

    if content.len() > MAX_TOML_FILE_SIZE {
        return Err(anyhow!("File {} is too large (>100KB)", path.display()));
    }

    let parsed = parse_anchor_toml(&content);
    if let Some(solana_version) = parsed.solana_version {
        set_version(
            versions,
            sources,
            VersionField::Solana,
            VersionSourceKind::AnchorToml,
            &path,
            solana_version,
        );
    }
    if let Some(anchor_version) = parsed.anchor_version {
        set_version(
            versions,
            sources,
            VersionField::Anchor,
            VersionSourceKind::AnchorToml,
            &path,
            anchor_version,
        );
    }

    Ok(())
}

fn check_cargo_toml(
    project_path: &Path,
    versions: &mut ProjectVersions,
    sources: &mut Vec<VersionSource>,
) -> Result<()> {
    let path = project_path.join("Cargo.toml");
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)
        .map_err(|error| anyhow!("Failed to read {}: {}", path.display(), error))?;

    if content.len() > MAX_TOML_FILE_SIZE {
        return Err(anyhow!("File {} is too large (>100KB)", path.display()));
    }

    let parsed = parse_cargo_toml(&content);
    if let Some(solana_version) = parsed.solana_version {
        set_version(
            versions,
            sources,
            VersionField::Solana,
            VersionSourceKind::CargoToml,
            &path,
            solana_version,
        );
    }
    if let Some(anchor_version) = parsed.anchor_version {
        set_version(
            versions,
            sources,
            VersionField::Anchor,
            VersionSourceKind::CargoToml,
            &path,
            anchor_version,
        );
    }

    Ok(())
}

fn set_version(
    versions: &mut ProjectVersions,
    sources: &mut Vec<VersionSource>,
    field: VersionField,
    kind: VersionSourceKind,
    path: &Path,
    value: String,
) {
    let target = match field {
        VersionField::Rust => &mut versions.rust_version,
        VersionField::Solana => &mut versions.solana_version,
        VersionField::Anchor => &mut versions.anchor_version,
    };

    if target.is_none() || target.as_ref().is_some_and(|current| current == "*") {
        *target = Some(value.clone());
        sources.push(VersionSource {
            field,
            kind,
            path: path.to_path_buf(),
            value,
        });
    }
}
