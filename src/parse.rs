use anyhow::{Result, anyhow};
use serde::Deserialize;

use crate::types::ProjectVersions;

#[derive(Deserialize)]
struct RustToolchain {
    toolchain: RustToolchainSpec,
}

#[derive(Deserialize)]
struct RustToolchainSpec {
    channel: String,
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    dependencies: Option<Dependencies>,
    workspace: Option<Workspace>,
}

#[derive(Debug, Deserialize)]
struct Workspace {
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
    #[serde(flatten)]
    _other: std::collections::HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize)]
struct Dependencies {
    #[serde(rename = "solana-program")]
    solana_program: Option<DependencySpec>,
    #[serde(rename = "anchor-lang")]
    anchor_lang: Option<DependencySpec>,
    #[serde(rename = "anchor-spl")]
    anchor_spl: Option<DependencySpec>,
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

/// Parse a `rust-toolchain` file in TOML or plain-text form.
///
/// # Errors
///
/// Returns an error when the input is neither a valid `rust-toolchain.toml`
/// payload nor a plain-text version string.
pub fn parse_rust_toolchain(content: &str) -> Result<String> {
    if let Ok(toolchain) = toml::from_str::<RustToolchain>(content) {
        return Ok(toolchain.toolchain.channel);
    }

    let version = content.trim();
    if version.chars().any(char::is_numeric) {
        Ok(version.to_string())
    } else {
        Err(anyhow!("Invalid rust-toolchain format"))
    }
}

#[must_use]
pub fn parse_anchor_toml(content: &str) -> ProjectVersions {
    match toml::from_str::<AnchorToml>(content) {
        Ok(config) => {
            let mut versions = ProjectVersions::default();
            if let Some(toolchain) = config.toolchain {
                versions.solana_version = toolchain.solana;
                versions.anchor_version = toolchain.anchor;
            }
            versions
        }
        Err(_) => parse_anchor_toml_fallback(content),
    }
}

#[must_use]
pub fn parse_cargo_toml(content: &str) -> ProjectVersions {
    match toml::from_str::<CargoToml>(content) {
        Ok(config) => {
            let mut versions = ProjectVersions::default();
            if let Some(deps) = &config.dependencies {
                update_versions_from_dependencies(&mut versions, deps);
            }
            if let Some(workspace) = &config.workspace
                && let Some(workspace_deps) = &workspace.dependencies
            {
                update_versions_from_dependencies(&mut versions, workspace_deps);
            }
            versions
        }
        Err(_) => parse_cargo_toml_fallback(content),
    }
}

#[must_use]
pub fn parse_semver_range(version_str: &str) -> String {
    let version_str = version_str.trim();

    if let Some(comma_pos) = version_str.find(',') {
        let first_part = &version_str[..comma_pos];
        return first_part
            .trim_start_matches(">=")
            .trim_start_matches('>')
            .trim_start_matches('=')
            .trim()
            .to_string();
    }

    version_str
        .trim_start_matches(">=")
        .trim_start_matches('>')
        .trim_start_matches("<=")
        .trim_start_matches('<')
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim()
        .to_string()
}

#[must_use]
pub fn clean_version(version: &str) -> String {
    version
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('=')
        .trim_start_matches('v')
        .to_string()
}

fn parse_anchor_toml_fallback(content: &str) -> ProjectVersions {
    let mut versions = ProjectVersions::default();
    if let Ok(value) = toml::from_str::<toml::Value>(content)
        && let Some(toolchain) = value.get("toolchain").and_then(|entry| entry.as_table())
    {
        versions.solana_version = toolchain
            .get("solana_version")
            .and_then(|entry| entry.as_str())
            .map(std::string::ToString::to_string);
        versions.anchor_version = toolchain
            .get("anchor_version")
            .and_then(|entry| entry.as_str())
            .map(std::string::ToString::to_string);
    }
    versions
}

fn parse_cargo_toml_fallback(content: &str) -> ProjectVersions {
    let mut versions = ProjectVersions::default();
    if let Ok(value) = toml::from_str::<toml::Value>(content) {
        if let Some(deps) = value.get("dependencies").and_then(|entry| entry.as_table()) {
            update_versions_from_toml_table(&mut versions, deps);
        }
        if let Some(workspace) = value.get("workspace").and_then(|entry| entry.as_table())
            && let Some(workspace_deps) = workspace
                .get("dependencies")
                .and_then(|entry| entry.as_table())
        {
            update_versions_from_toml_table(&mut versions, workspace_deps);
        }
    }
    versions
}

fn get_version_from_spec(spec: &DependencySpec) -> Option<String> {
    match spec {
        DependencySpec::Simple(version) => Some(parse_semver_range(version)),
        DependencySpec::Detailed(details) => details
            .version
            .as_ref()
            .map(|version| parse_semver_range(version)),
    }
}

fn update_versions_from_dependencies(versions: &mut ProjectVersions, deps: &Dependencies) {
    if versions.solana_version.is_none()
        && let Some(solana_spec) = &deps.solana_program
    {
        versions.solana_version = get_version_from_spec(solana_spec);
    }

    if versions.anchor_version.is_none()
        && let Some(anchor_spec) = &deps.anchor_lang
    {
        versions.anchor_version = get_version_from_spec(anchor_spec);
    }

    if versions.anchor_version.is_none()
        && let Some(anchor_spl_spec) = &deps.anchor_spl
    {
        versions.anchor_version = get_version_from_spec(anchor_spl_spec);
    }
}

fn update_versions_from_toml_table(versions: &mut ProjectVersions, deps: &toml::value::Table) {
    if versions.solana_version.is_none() {
        versions.solana_version = extract_version_from_toml_value(deps.get("solana-program"));
    }

    if versions.anchor_version.is_none() {
        versions.anchor_version = extract_version_from_toml_value(deps.get("anchor-lang"));
    }

    if versions.anchor_version.is_none() {
        versions.anchor_version = extract_version_from_toml_value(deps.get("anchor-spl"));
    }
}

fn extract_version_from_toml_value(value: Option<&toml::Value>) -> Option<String> {
    value.and_then(|entry| match entry {
        toml::Value::String(version) => Some(parse_semver_range(version)),
        toml::Value::Table(table) => table
            .get("version")
            .and_then(|version| version.as_str())
            .map(parse_semver_range),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_semver_range_with_comma_range() {
        assert_eq!(parse_semver_range(">=1.18,<=2"), "1.18");
    }

    #[test]
    fn test_parse_semver_range_with_full_version_range() {
        assert_eq!(parse_semver_range(">=1.18.0,<=2.0.0"), "1.18.0");
    }

    #[test]
    fn test_parse_semver_range_with_greater_than_or_equal() {
        assert_eq!(parse_semver_range(">=1.18"), "1.18");
        assert_eq!(parse_semver_range(">=1.18.17"), "1.18.17");
    }

    #[test]
    fn test_parse_semver_range_with_caret() {
        assert_eq!(parse_semver_range("^1.18"), "1.18");
        assert_eq!(parse_semver_range("^1.18.17"), "1.18.17");
    }

    #[test]
    fn test_parse_semver_range_with_tilde() {
        assert_eq!(parse_semver_range("~1.18"), "1.18");
        assert_eq!(parse_semver_range("~1.18.17"), "1.18.17");
    }

    #[test]
    fn test_parse_semver_range_exact_version() {
        assert_eq!(parse_semver_range("1.18.17"), "1.18.17");
        assert_eq!(parse_semver_range("0.30.1"), "0.30.1");
    }

    #[test]
    fn test_parse_semver_range_with_equals() {
        assert_eq!(parse_semver_range("=1.18.17"), "1.18.17");
    }

    #[test]
    fn test_parse_semver_range_with_whitespace() {
        assert_eq!(parse_semver_range("  >=1.18  "), "1.18");
        assert_eq!(parse_semver_range("  >=1.18 , <=2  "), "1.18");
    }

    #[test]
    fn test_parse_semver_range_complex_range() {
        assert_eq!(parse_semver_range(">1.17,<2.0"), "1.17");
        assert_eq!(parse_semver_range(">=1.18.0,<2.0.0"), "1.18.0");
    }

    #[test]
    fn test_parse_cargo_toml_extracts_workspace_dependencies() {
        let versions = parse_cargo_toml(
            r#"
            [workspace.dependencies]
            solana-program = ">=1.18,<=2"
            anchor-lang = { version = "0.30.1" }
            "#,
        );

        assert_eq!(versions.solana_version.as_deref(), Some("1.18"));
        assert_eq!(versions.anchor_version.as_deref(), Some("0.30.1"));
    }

    #[test]
    fn test_parse_anchor_toml_extracts_toolchain_versions() {
        let versions = parse_anchor_toml(
            r#"
            [toolchain]
            anchor_version = "0.30.1"
            solana_version = "1.18.17"
            "#,
        );

        assert_eq!(versions.solana_version.as_deref(), Some("1.18.17"));
        assert_eq!(versions.anchor_version.as_deref(), Some("0.30.1"));
    }

    #[test]
    fn test_parse_rust_toolchain_plain_text() {
        assert_eq!(
            parse_rust_toolchain("nightly-2023-10-29\n").unwrap(),
            "nightly-2023-10-29"
        );
    }

    #[test]
    fn test_detailed_dependency_other_fields_are_ignored() {
        let details =
            toml::from_str::<DetailedDependency>("version = \">=1.18,<=2\"\nfeatures = [\"foo\"]")
                .unwrap();

        assert_eq!(details.version.as_deref(), Some(">=1.18,<=2"));
        assert!(details._other.contains_key("features"));
    }
}
