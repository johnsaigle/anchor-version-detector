use std::path::{Path, PathBuf};
use std::fs;
use serde::Deserialize;
use anyhow::{Result, anyhow};

#[derive(Deserialize)]
struct RustToolchain {
    toolchain: RustToolchainSpec,
}

#[derive(Deserialize)]
struct RustToolchainSpec {
    channel: String,
}

#[derive(Debug, Clone)]
struct ProjectVersions {
    rust_version: Option<String>,
    solana_version: Option<String>,
    anchor_version: Option<String>,
    source: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    dependencies: Option<Dependencies>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum DependencySpec {
    Simple(String),
    Detailed(DetailedDependency),
}

#[derive(Debug, Deserialize)]
struct DetailedDependency {
    version: Option<String>,
    // Add other fields that might be present
    #[serde(skip)]
    _other: (),
}

#[derive(Debug, Deserialize)]
struct Dependencies {
    #[serde(rename = "solana-program")]
    solana_program: Option<DependencySpec>,
    #[serde(rename = "anchor-lang")]
    anchor_lang: Option<DependencySpec>,
}

#[derive(Deserialize)]
struct AnchorToml {
    toolchain: Option<ToolchainConfig>,
}

#[derive(Debug, Deserialize)]
struct ToolchainConfig {
    #[serde(rename = "anchor_version")]
    anchor: Option<String>,
    #[serde(rename = "solana_version")]
    solana: Option<String>,
}

#[derive(Deserialize)]
struct PackageJson { dependencies: Option<DependenciesJson>, }

#[derive(Deserialize)]
struct DependenciesJson {
    #[serde(rename = "@solana/web3.js")]
    solana_web3: Option<String>,
    #[serde(rename = "@project-serum/anchor")]
    anchor: Option<String>,
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Usage: {} <project_directory>", args[0]);
        return Ok(());
    }

    let project_path = PathBuf::from(&args[1]);
    let versions = detect_versions_recursive(&project_path)?;
    
    println!("Detected/Inferred Versions:");
    // Store the default values in variables to avoid repeated allocation
    let unknown = "Unknown".to_string();
    let unknown_anchor = "Unknown (may not be an Anchor project)".to_string();

    // Print detected versions
    println!("Rust: {} {}", 
        versions.rust_version.as_ref().unwrap_or(&unknown),
        versions.source.as_ref().map(|p| format!("(from {})", p.display())).unwrap_or_default()
    );
    println!("Solana: {}", versions.solana_version.as_ref().unwrap_or(&unknown));
    println!("Anchor: {}", versions.anchor_version.as_ref().unwrap_or(&unknown_anchor));
    
    // Print configuration instructions
    println!("\nTo work with this project, configure your environment as follows:");
    println!("```");
    if let Some(ref toolchain) = versions.rust_version {
        println!("rustup default {}", toolchain);
    }
    if let Some(ref solana) = versions.solana_version {
        println!("solana-install init {}", solana);
    }
    if let Some(ref anchor) = versions.anchor_version {
        println!("avm use {}", anchor);
    }
    println!("```");

    Ok(())
}

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
    fn needs_more_info(&self) -> bool {
        self.rust_version.is_none() || 
        self.solana_version.is_none() ||
        self.anchor_version.is_none()
    }

    fn update_from(&mut self, other: &ProjectVersions) {
        if self.rust_version.is_none() && other.rust_version.is_some() {
            self.rust_version = other.rust_version.clone();
            self.source = other.source.clone();
        }
        if self.solana_version.is_none() 
            && other.solana_version.is_some()
            && other.solana_version.as_ref().map_or(true, |v| v != "*") {
            self.solana_version = other.solana_version.clone();
        }
        if self.anchor_version.is_none() && other.anchor_version.is_some() {
            self.anchor_version = other.anchor_version.clone();
        }
    }
}

fn search_subdirectories(dir: &Path, versions: &mut ProjectVersions) -> Result<()> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                // Skip common directories that wouldn't contain relevant files
                let dir_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
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
    }
    Ok(())
}

fn should_skip_directory(dir_name: &str) -> bool {
    let skip_dirs = [
        "node_modules",
        "target",
        ".git",
        "dist",
        "build",
        ".idea",
        ".vscode",
        "coverage",
    ];
    skip_dirs.contains(&dir_name)
}

fn get_version_from_spec(spec: &DependencySpec) -> Option<String> {
    match spec {
        DependencySpec::Simple(version) => Some(version.clone()),
        DependencySpec::Detailed(details) => details.version.clone(),
    }
}

fn detect_versions(project_path: &Path) -> Result<ProjectVersions> {
    let mut versions = ProjectVersions {
        rust_version: None,
        solana_version: None,
        anchor_version: None,
        source: None,
    };


    // Check rust-toolchain file
    let rust_toolchain_path = project_path.join("rust-toolchain");
    if rust_toolchain_path.exists() {
        let content = fs::read_to_string(&rust_toolchain_path)?;
        if let Ok(version) = parse_rust_toolchain(&content) {
            versions.rust_version = Some(version);
            versions.source = Some(rust_toolchain_path);
        }
    }

    // Check Anchor.toml
    let anchor_toml_path = project_path.join("Anchor.toml");
    if anchor_toml_path.exists() {
        println!("Checking {}", anchor_toml_path.display());
        let content = fs::read_to_string(&anchor_toml_path)?;
        
        // Try parsing with our structured approach first
        match toml::from_str::<AnchorToml>(&content) {
            Ok(config) => {
                println!("Successfully parsed Anchor.toml");
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
                            if let Some(solana_ver) = toolchain.get("solana_version").and_then(|v| v.as_str()) {
                                versions.solana_version = Some(solana_ver.to_string());
                                println!("Found solana version (fallback): {}", solana_ver);
                            }
                        }
                        
                        // Try to get anchor version
                        if versions.anchor_version.is_none() {
                            if let Some(anchor_ver) = toolchain.get("anchor_version").and_then(|v| v.as_str()) {
                                versions.anchor_version = Some(anchor_ver.to_string());
                                println!("Found anchor version (fallback): {}", anchor_ver);
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
        let content = fs::read_to_string(&cargo_toml_path)?;
        
        // First try parsing with our structured approach
        match toml::from_str::<CargoToml>(&content) {
            Ok(config) => {
                if let Some(deps) = config.dependencies {
                    // Handle solana-program
                    if let Some(solana_spec) = deps.solana_program {
                        if versions.solana_version.is_none() {
                            versions.solana_version = get_version_from_spec(&solana_spec);
                        }
                    }
                    
                    // Handle anchor-lang
                    if let Some(anchor_spec) = deps.anchor_lang {
                        if versions.anchor_version.is_none() {
                            versions.anchor_version = get_version_from_spec(&anchor_spec);
                        }
                    }
                }
            }
            Err(_) => {
                // Fallback to parsing as generic TOML
                if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                    if let Some(deps) = value.get("dependencies").and_then(|d| d.as_table()) {
                        // Try to get solana-program version
                        if versions.solana_version.is_none() {
                            versions.solana_version = extract_version_from_toml_value(deps.get("solana-program"));
                        }
                        
                        // Try to get anchor-lang version
                        if versions.anchor_version.is_none() {
                            versions.anchor_version = extract_version_from_toml_value(deps.get("anchor-lang"));
                        }
                    }
                }
            }
        }
    }

    // Check package.json
    let package_json_path = project_path.join("package.json");
    if package_json_path.exists() {
        let content = fs::read_to_string(&package_json_path)?;
        if let Ok(config) = serde_json::from_str::<PackageJson>(&content) {
            if let Some(deps) = config.dependencies {
                if versions.solana_version.is_none() {
                    versions.solana_version = deps.solana_web3;
                }
                if versions.anchor_version.is_none() {
                    versions.anchor_version = deps.anchor;
                }
            }
        }
    }


    Ok(versions)
}

fn extract_version_from_toml_value(value: Option<&toml::Value>) -> Option<String> {
    match value {
        Some(v) => {
            match v {
                toml::Value::String(s) => Some(s.clone()),
                toml::Value::Table(t) => t.get("version").and_then(|v| v.as_str()).map(String::from),
                _ => None,
            }
        }
        None => None,
    }
}

// fn detect_versions(project_path: &Path) -> Result<ProjectVersions> {
//     let mut versions = ProjectVersions {
//         rust_version: None,
//         solana_version: None,
//         anchor_version: None,
//     };
//
//     // Check rust-toolchain file
//     let rust_toolchain_path = project_path.join("rust-toolchain");
//     if rust_toolchain_path.exists() {
//         let content = fs::read_to_string(&rust_toolchain_path)?;
//         versions.rust_version = match parse_rust_toolchain(&content) {
//             Ok(version) => Some(version),
//             // Return the string as-is if parsing fails
//             Err(_) => Some(content.trim().to_string())
//         };
//     }
//
//     // Check Anchor.toml
//     let anchor_toml_path = project_path.join("Anchor.toml");
//     if anchor_toml_path.exists() {
//         let content = fs::read_to_string(&anchor_toml_path)?;
//         if let Ok(config) = toml::from_str::<AnchorToml>(&content) {
//             if let Some(toolchain) = config.toolchain {
//                 versions.solana_version = toolchain.solana;
//                 versions.anchor_version = toolchain.anchor;
//             }
//         }
//     }
//
//     // Check Cargo.toml
//     let cargo_toml_path = project_path.join("Cargo.toml");
//     if cargo_toml_path.exists() {
//         let content = fs::read_to_string(&cargo_toml_path)?;
//         if let Ok(config) = toml::from_str::<CargoToml>(&content) {
//             if let Some(deps) = config.dependencies {
//                 if versions.solana_version.is_none() {
//                     versions.solana_version = deps.solana_program;
//                 }
//                 if versions.anchor_version.is_none() {
//                     versions.anchor_version = deps.anchor_lang;
//                 }
//             }
//         }
//     }
//
//     // Check package.json
//     let package_json_path = project_path.join("package.json");
//     if package_json_path.exists() {
//         let content = fs::read_to_string(&package_json_path)?;
//         if let Ok(config) = serde_json::from_str::<PackageJson>(&content) {
//             if let Some(deps) = config.dependencies {
//                 if versions.solana_version.is_none() {
//                     versions.solana_version = deps.solana_web3;
//                 }
//                 if versions.anchor_version.is_none() {
//                     versions.anchor_version = deps.anchor;
//                 }
//             }
//         }
//     }
//
//     // Infer versions if not found
//     infer_missing_versions(&mut versions)?;
//
//     Ok(versions)
// }

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

fn infer_missing_versions(versions: &mut ProjectVersions) -> Result<()> {
    // Known version compatibility matrix
    let compatibility_rules = vec![
        // TODO update
        // (Solana, Anchor, Rust) - Newest to oldest
        ("1.17.0", "0.30.1", "1.69.0"),
        ("1.17.0", "0.30.0", "1.69.0"),
        ("1.17.0", "0.29.0", "1.69.0"),
        ("1.16.0", "0.28.0", "1.68.0"),
        ("1.15.0", "0.27.0", "1.67.0"),
        ("1.14.0", "0.26.0", "1.66.0"),
    ];
    
    // If Anchor is the only one missing, maybe this isn't an anchor project
    // match versions {
    //     Some(_), Some(_), None, Some(_) => return versions,
    //     _ => continue;
    // }

    // If we have Solana version but missing others
    if let Some(solana_ref) = &versions.solana_version {
        let solana_ver = clean_version(solana_ref);
        for &(solana, anchor, rust) in &compatibility_rules {
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
        for &(solana, anchor, rust) in &compatibility_rules {
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
    || versions.solana_version.as_ref().map_or(true, |v| v == "*") {
        println!("Solana version could not be determined. Suggesting latest.");
        versions.solana_version = Some(compatibility_rules[0].0.to_string());
    }
    // if versions.anchor_version.is_none() {
    //     println!("Anchor version could not be determined. Suggesting latest.");
    //     versions.anchor_version = Some(compatibility_rules[0].1.to_string());
    // }
    if versions.rust_version.is_none() {
        println!("Rust version could not be determined. Suggesting latest.");
        versions.rust_version = Some(compatibility_rules[0].2.to_string());
    }

    Ok(())
}

fn clean_version(version: &str) -> String {
    version.trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches("v")
        .to_string()
}
