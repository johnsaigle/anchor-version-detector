pub mod compatibility;
pub mod detect;
pub mod env;
pub mod parse;
pub mod types;

pub use compatibility::{
    assess_versions, compatibility_rules, find_rule_by_anchor, find_rule_by_solana,
    latest_compatible_rule, resolve_versions,
};
pub use detect::{detect_versions_in_dir, detect_versions_recursive};
pub use env::{detect_current_environment, get_agave_version, get_avm_version, get_rustc_version};
pub use parse::{
    clean_version, parse_anchor_toml, parse_cargo_toml, parse_rust_toolchain, parse_semver_range,
};
pub use types::{
    CompatibilityAssessment, CompatibilityReason, CompatibilityRule, Confidence,
    CurrentEnvironment, DetectionReport, InferredFields, ProjectVersions, ScanOptions,
    VersionField, VersionSource, VersionSourceKind,
};
