//! Anchor Version Detector
//!
//! A tool for detecting and inferring Rust, Solana, and Anchor versions from project files.
//! This utility recursively scans project directories to find version information from:
//! - `rust-toolchain` and `rust-toolchain.toml` files for Rust versions
//! - `Anchor.toml` files for Anchor and Solana versions
//! - `Cargo.toml` files for dependency versions
//!
//! When versions cannot be directly detected, the tool uses a compatibility matrix
//! to infer missing versions based on known working combinations.

use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Default message when version cannot be determined
const UNKNOWN_VERSION: &str = "Unknown";
/// Default message when Anchor version cannot be determined
const UNKNOWN_ANCHOR_VERSION: &str = "Unknown (may not be an Anchor project)";
/// Possible rust-toolchain file names to check
const RUST_TOOLCHAIN_FILES: &[&str] = &["rust-toolchain", "rust-toolchain.toml"];
/// Directories to skip during recursive search for performance
const SKIP_DIRECTORIES: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    ".idea",
    ".vscode",
    "coverage",
];

/// Expected number of command line arguments (program name + project directory)
const EXPECTED_ARGS_COUNT: usize = 2;
/// Maximum file size for rust-toolchain files (10KB)
const MAX_RUST_TOOLCHAIN_FILE_SIZE: usize = 10_000;
/// Maximum file size for TOML configuration files (100KB)
const MAX_TOML_FILE_SIZE: usize = 100_000;
/// Index of the latest compatibility rule (first entry in the array)
const LATEST_COMPATIBILITY_INDEX: usize = 0;

/// Represents the structure of a `rust-toolchain.toml` file
#[derive(Deserialize)]
struct RustToolchain {
    toolchain: RustToolchainSpec,
}

/// Toolchain specification within a rust-toolchain file
#[derive(Deserialize)]
struct RustToolchainSpec {
    channel: String,
}

/// Contains detected or inferred version information for a project
#[derive(Debug, Clone)]
struct ProjectVersions {
    /// Rust toolchain version (e.g., "1.76.0", "nightly-2023-04-01")
    rust_version: Option<String>,
    /// Solana/Agave version (e.g., "1.18.17")
    solana_version: Option<String>,
    /// Anchor framework version (e.g., "0.30.1")
    anchor_version: Option<String>,
    /// Path to the file where version information was found
    source: Option<PathBuf>,
}

/// Represents the structure of a `Cargo.toml` file
#[derive(Debug, Deserialize)]
struct CargoToml {
    dependencies: Option<Dependencies>,
    workspace: Option<Workspace>,
}

/// Workspace configuration in Cargo.toml
#[derive(Debug, Deserialize)]
struct Workspace {
    dependencies: Option<Dependencies>,
}

/// Represents different ways dependencies can be specified in Cargo.toml
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependencySpec {
    /// Simple version string (e.g., "1.0.0")
    Simple(String),
    /// Detailed dependency specification with additional fields
    Detailed(DetailedDependency),
}

/// Detailed dependency specification with version and other optional fields
#[derive(Debug, Deserialize)]
struct DetailedDependency {
    version: Option<String>,
    #[serde(flatten)]
    _other: std::collections::HashMap<String, toml::Value>,
}

/// Relevant dependencies we track for version detection
#[derive(Debug, Deserialize)]
struct Dependencies {
    #[serde(rename = "solana-program")]
    solana_program: Option<DependencySpec>,
    #[serde(rename = "anchor-lang")]
    anchor_lang: Option<DependencySpec>,
    #[serde(rename = "anchor-spl")]
    anchor_spl: Option<DependencySpec>,
}

/// Represents the structure of an `Anchor.toml` file
#[derive(Deserialize)]
struct AnchorToml {
    toolchain: Option<ToolchainConfig>,
}

/// Toolchain configuration within Anchor.toml
#[derive(Debug, Deserialize)]
struct ToolchainConfig {
    #[serde(rename = "anchor_version")]
    anchor: Option<String>,
    #[serde(rename = "solana_version")]
    solana: Option<String>,
}

/// Main entry point for the anchor version detector
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != EXPECTED_ARGS_COUNT {
        println!("Usage: {} <project_directory>", args[0]);
        return Ok(());
    }

    let project_path = PathBuf::from(&args[1]);

    // [SECURITY INTENT]: Validate that the provided path exists and is a directory
    // [SECURITY REASONING]: Prevents path traversal attacks and ensures we only process valid directories
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

    // [SECURITY INTENT]: Canonicalize path to prevent directory traversal attacks
    // [SECURITY REASONING]: Resolves symlinks and relative paths to absolute canonical form
    let project_path = project_path.canonicalize().map_err(|e| {
        anyhow!(
            "Failed to canonicalize path {}: {}",
            project_path.display(),
            e
        )
    })?;

    let versions = detect_versions_recursive(&project_path)?;

    println!("Detected/Inferred Versions:");
    print_detected_versions(&versions);

    // Print configuration instructions
    println!("\nTo work with this project, configure your environment as follows:");
    println!("```");
    if let Some(ref toolchain) = versions.rust_version {
        println!("rustup default {}", toolchain);
    }
    if let Some(ref solana) = versions.solana_version {
        println!("agave-install init {}", solana);
    }
    if let Some(ref anchor) = versions.anchor_version {
        println!("avm use {}", anchor);
    }
    println!("```");

    Ok(())
}

/// Recursively detect versions from project files, starting with the given directory
/// and searching subdirectories if needed, then inferring missing versions
fn detect_versions_recursive(project_path: &Path) -> Result<ProjectVersions> {
    // First try to detect versions in the current directory
    let mut versions = detect_versions(project_path)?;

    // If we couldn't determine all versions, search subdirectories recursively
    if versions.needs_more_info() {
        search_subdirectories(project_path, &mut versions)?;
    }

    // If we still don't have all versions, try to infer them
    infer_missing_versions(&mut versions)?;

    Ok(versions)
}

impl ProjectVersions {
    /// Returns true if any version information is missing
    fn needs_more_info(&self) -> bool {
        self.rust_version.is_none()
            || self.solana_version.is_none()
            || self.anchor_version.is_none()
    }

    /// Updates this instance with version information from another ProjectVersions
    /// Only updates fields that are currently None
    fn update_from(&mut self, other: &ProjectVersions) {
        if self.rust_version.is_none() && other.rust_version.is_some() {
            self.rust_version = other.rust_version.clone();
            self.source = other.source.clone();
        }
        if self.solana_version.is_none()
            && other.solana_version.is_some()
            && other.solana_version.as_ref().is_none_or(|v| v != "*")
        {
            self.solana_version = other.solana_version.clone();
        }
        if self.anchor_version.is_none() && other.anchor_version.is_some() {
            self.anchor_version = other.anchor_version.clone();
        }
    }
}

/// Recursively searches subdirectories for version information
/// Skips common build/cache directories to improve performance
fn search_subdirectories(dir: &Path, versions: &mut ProjectVersions) -> Result<()> {
    // [SECURITY INTENT]: Safely handle directory reading with proper error handling
    let entries = fs::read_dir(dir)
        .map_err(|e| anyhow!("Failed to read directory {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            // Skip common directories that wouldn't contain relevant files
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if should_skip_directory(dir_name) {
                continue;
            }

            // Check this directory
            let sub_versions = detect_versions(&path)?;
            versions.update_from(&sub_versions);

            // Recurse into subdirectory if we still need more info
            if versions.needs_more_info() {
                search_subdirectories(&path, versions)?;
            }

            // If we have all versions, we can stop searching
            if !versions.needs_more_info() {
                break;
            }
        }
    }
    Ok(())
}

/// Determines if a directory should be skipped during recursive search
/// Returns true for common build/cache directories that won't contain relevant files
fn should_skip_directory(dir_name: &str) -> bool {
    SKIP_DIRECTORIES.contains(&dir_name)
}

/// Prints the detected version information in a formatted way
fn print_detected_versions(versions: &ProjectVersions) {
    println!(
        "Rust: {} {}",
        versions.rust_version.as_deref().unwrap_or(UNKNOWN_VERSION),
        versions
            .source
            .as_ref()
            .map(|p| format!("(from {})", p.display()))
            .unwrap_or_default()
    );
    println!(
        "Solana: {}",
        versions
            .solana_version
            .as_deref()
            .unwrap_or(UNKNOWN_VERSION)
    );
    println!(
        "Anchor: {}",
        versions
            .anchor_version
            .as_deref()
            .unwrap_or(UNKNOWN_ANCHOR_VERSION)
    );
}

/// Extracts version string from a dependency specification
fn get_version_from_spec(spec: &DependencySpec) -> Option<String> {
    match spec {
        DependencySpec::Simple(version) => Some(version.clone()),
        DependencySpec::Detailed(details) => details.version.clone(),
    }
}

/// Update version information from structured Dependencies
fn update_versions_from_dependencies(versions: &mut ProjectVersions, deps: &Dependencies) {
    if versions.solana_version.is_none() {
        if let Some(solana_spec) = &deps.solana_program {
            versions.solana_version = get_version_from_spec(solana_spec);
        }
    }

    if versions.anchor_version.is_none() {
        if let Some(anchor_spec) = &deps.anchor_lang {
            versions.anchor_version = get_version_from_spec(anchor_spec);
        }
    }

    // Use anchor-spl as fallback for anchor version
    if versions.anchor_version.is_none() {
        if let Some(anchor_spl_spec) = &deps.anchor_spl {
            versions.anchor_version = get_version_from_spec(anchor_spl_spec);
        }
    }
}

/// Update version information from generic TOML table (fallback parsing)
fn update_versions_from_toml_table(versions: &mut ProjectVersions, deps: &toml::value::Table) {
    if versions.solana_version.is_none() {
        versions.solana_version = extract_version_from_toml_value(deps.get("solana-program"));
    }

    if versions.anchor_version.is_none() {
        versions.anchor_version = extract_version_from_toml_value(deps.get("anchor-lang"));
    }

    // Use anchor-spl as fallback for anchor version
    if versions.anchor_version.is_none() {
        versions.anchor_version = extract_version_from_toml_value(deps.get("anchor-spl"));
    }
}

/// Detects version information from files in a single directory
/// Checks rust-toolchain files, Anchor.toml, and Cargo.toml for version information
fn detect_versions(project_path: &Path) -> Result<ProjectVersions> {
    let mut versions = ProjectVersions {
        rust_version: None,
        solana_version: None,
        anchor_version: None,
        source: None,
    };

    // Check for a rust-toolchain file.
    for filename in RUST_TOOLCHAIN_FILES {
        let path = project_path.join(filename);
        if path.exists() {
            // [SECURITY INTENT]: Safely read file contents with size limits
            let content = fs::read_to_string(&path)
                .map_err(|e| anyhow!("Failed to read {}: {}", path.display(), e))?;

            // [SECURITY REASONING]: Validate file size to prevent memory exhaustion
            if content.len() > MAX_RUST_TOOLCHAIN_FILE_SIZE {
                return Err(anyhow!("File {} is too large (>10KB)", path.display()));
            }

            if let Ok(version) = parse_rust_toolchain(&content) {
                versions.rust_version = Some(version);
                versions.source = Some(path);
            }
            break;
        }
    }

    // Check Anchor.toml
    let anchor_toml_path = project_path.join("Anchor.toml");
    if anchor_toml_path.exists() {
        let content = fs::read_to_string(&anchor_toml_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", anchor_toml_path.display(), e))?;

        // [SECURITY REASONING]: Validate file size to prevent memory exhaustion
        if content.len() > MAX_TOML_FILE_SIZE {
            return Err(anyhow!(
                "File {} is too large (>100KB)",
                anchor_toml_path.display()
            ));
        }

        // Try parsing with our structured approach first
        match toml::from_str::<AnchorToml>(&content) {
            Ok(config) => {
                if let Some(toolchain) = config.toolchain {
                    // Handle solana version
                    if let Some(solana_ver) = toolchain.solana {
                        versions.solana_version = Some(solana_ver);
                    }

                    // Handle anchor version
                    if let Some(anchor_ver) = toolchain.anchor {
                        versions.anchor_version = Some(anchor_ver);
                    }
                }
            }
            Err(_) => {
                // Fallback to parsing as generic TOML
                if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                    if let Some(toolchain) = value.get("toolchain").and_then(|t| t.as_table()) {
                        // Try to get solana version
                        if versions.solana_version.is_none() {
                            if let Some(solana_ver) =
                                toolchain.get("solana_version").and_then(|v| v.as_str())
                            {
                                versions.solana_version = Some(solana_ver.to_string());
                            }
                        }

                        // Try to get anchor version
                        if versions.anchor_version.is_none() {
                            if let Some(anchor_ver) =
                                toolchain.get("anchor_version").and_then(|v| v.as_str())
                            {
                                versions.anchor_version = Some(anchor_ver.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    // Check Cargo.toml
    let cargo_toml_path = project_path.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| anyhow!("Failed to read {}: {}", cargo_toml_path.display(), e))?;

        // [SECURITY REASONING]: Validate file size to prevent memory exhaustion
        if content.len() > MAX_TOML_FILE_SIZE {
            return Err(anyhow!(
                "File {} is too large (>100KB)",
                cargo_toml_path.display()
            ));
        }

        // First try parsing with our structured approach
        match toml::from_str::<CargoToml>(&content) {
            Ok(config) => {
                // Check regular dependencies first
                if let Some(deps) = &config.dependencies {
                    update_versions_from_dependencies(&mut versions, deps);
                }

                // Check workspace dependencies if versions not found in regular dependencies
                if let Some(workspace) = &config.workspace {
                    if let Some(workspace_deps) = &workspace.dependencies {
                        update_versions_from_dependencies(&mut versions, workspace_deps);
                    }
                }
            }
            Err(_) => {
                // Fallback to parsing as generic TOML
                if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                    // Check regular dependencies first
                    if let Some(deps) = value.get("dependencies").and_then(|d| d.as_table()) {
                        update_versions_from_toml_table(&mut versions, deps);
                    }

                    // Check workspace dependencies if versions not found
                    if let Some(workspace) = value.get("workspace").and_then(|w| w.as_table()) {
                        if let Some(workspace_deps) =
                            workspace.get("dependencies").and_then(|d| d.as_table())
                        {
                            update_versions_from_toml_table(&mut versions, workspace_deps);
                        }
                    }
                }
            }
        }
    }

    Ok(versions)
}

/// Extracts version string from a TOML value (fallback parsing)
fn extract_version_from_toml_value(value: Option<&toml::Value>) -> Option<String> {
    match value {
        Some(v) => match v {
            toml::Value::String(s) => Some(s.clone()),
            toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).map(String::from),
            _ => None,
        },
        None => None,
    }
}

/// Parses rust-toolchain file content to extract version information
/// Supports both TOML format and plain text format
fn parse_rust_toolchain(content: &str) -> Result<String> {
    // First try parsing as TOML
    if let Ok(toolchain) = toml::from_str::<RustToolchain>(content) {
        return Ok(toolchain.toolchain.channel);
    }

    // If TOML parsing fails, try parsing as plain version string
    let version = content.trim();

    // Basic validation - check if it looks like a version number
    // This covers formats like "1.69.0" and "nightly-2023-04-01"
    if version.chars().any(|c| c.is_numeric()) {
        Ok(version.to_string())
    } else {
        Err(anyhow!("Invalid rust-toolchain format"))
    }
}

/// Compatibility matrix for Solana/Agave, Anchor, and Rust versions
/// Each tuple contains (Solana version, Anchor version, Rust version)
/// Ordered from newest to oldest for proper version inference
const COMPATIBILITY_RULES: &[(&str, &str, &str)] = &[
    // (Solana, Anchor, Rust) - Newest to oldest
    ("2.1.0", "0.31.0", "1.84.1"), // based on rust-toolchain in main agave repo and https://www.anchor-lang.com/release-notes/0.31.0
    ("1.18.17", "0.30.1", "1.76.0"), // based on rust-toolchain in main solana repo and https://www.anchor-lang.com/release-notes/0.30.1
    ("1.18.8", "0.30.0", "1.76.0"), // based on rust-toolchain in main solana repo and https://www.anchor-lang.com/release-notes/0.30.0
    ("1.17.0", "0.29.0", "1.69.0"), // listed in https://www.anchor-lang.com/release-notes/0.29.0
    ("1.16.0", "0.28.0", "1.68.0"), // https://www.anchor-lang.com/release-notes/changelog#0-28-0-2023-06-09
    ("1.15.0", "0.27.0", "1.67.0"),
    ("1.14.0", "0.26.0", "1.66.0"),
];

/// Checks if the directory appears to be a Solana project by looking for Solana-related indicators
/// Returns true if any Solana or Anchor version information was found
fn is_solana_project(versions: &ProjectVersions) -> bool {
    versions.solana_version.is_some() || versions.anchor_version.is_some()
}

/// Infers missing version information using the compatibility matrix
/// Uses known working combinations to fill in missing versions
/// Returns an error if no Solana project indicators are found
fn infer_missing_versions(versions: &mut ProjectVersions) -> Result<()> {
    // [SECURITY INTENT]: Validate that this is actually a Solana project before proceeding
    // [SECURITY REASONING]: Prevents the tool from providing misleading version information for non-Solana projects
    if !is_solana_project(versions) {
        return Err(anyhow!(
            "This directory does not appear to be a Solana project. No Solana or Anchor version information found.\n\
            Expected to find one of:\n\
            - Anchor.toml with toolchain configuration\n\
            - Cargo.toml with solana-program, anchor-lang, or anchor-spl dependencies"
        ));
    }

    // If we have Solana version but missing others
    if let Some(solana_ref) = &versions.solana_version {
        let solana_ver = clean_version(solana_ref);
        for &(solana, anchor, rust) in COMPATIBILITY_RULES {
            if solana_ver.starts_with(solana) {
                if versions.anchor_version.is_none() {
                    versions.anchor_version = Some(anchor.to_string());
                }
                if versions.rust_version.is_none() {
                    versions.rust_version = Some(rust.to_string());
                }
                break;
            }
        }
    }
    // If we have Anchor version but missing others
    else if let Some(anchor_ref) = &versions.anchor_version {
        let anchor_ver = clean_version(anchor_ref);
        for &(solana, anchor, rust) in COMPATIBILITY_RULES {
            if anchor_ver.starts_with(anchor) {
                if versions.solana_version.is_none() {
                    versions.solana_version = Some(solana.to_string());
                }
                if versions.rust_version.is_none() {
                    versions.rust_version = Some(rust.to_string());
                }
                break;
            }
        }
    }

    // If still missing versions, use latest known compatible versions
    if versions.solana_version.is_none()
        || versions.solana_version.as_ref().is_none_or(|v| v == "*")
    {
        println!("Solana version could not be determined. Suggesting latest.");
        versions.solana_version = Some(
            COMPATIBILITY_RULES[LATEST_COMPATIBILITY_INDEX]
                .0
                .to_string(),
        );
    }
    if versions.rust_version.is_none() {
        println!("Rust version could not be determined. Suggesting latest.");
        versions.rust_version = Some(
            COMPATIBILITY_RULES[LATEST_COMPATIBILITY_INDEX]
                .2
                .to_string(),
        );
    }

    Ok(())
}

/// Cleans version strings by removing common prefixes (^, ~, =, v)
fn clean_version(version: &str) -> String {
    version
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches('v')
        .to_string()
}
