use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectVersions {
    pub rust_version: Option<String>,
    pub solana_version: Option<String>,
    pub anchor_version: Option<String>,
}

impl ProjectVersions {
    #[must_use]
    pub const fn needs_more_info(&self) -> bool {
        self.rust_version.is_none()
            || self.solana_version.is_none()
            || self.anchor_version.is_none()
    }

    #[must_use]
    pub const fn is_solana_project(&self) -> bool {
        self.solana_version.is_some() || self.anchor_version.is_some()
    }

    pub fn merge_missing_from(&mut self, other: &Self) {
        if self.rust_version.is_none() {
            self.rust_version.clone_from(&other.rust_version);
        }

        if self.solana_version.is_none()
            && other
                .solana_version
                .as_ref()
                .is_some_and(|version| version != "*")
        {
            self.solana_version.clone_from(&other.solana_version);
        }

        if self.anchor_version.is_none() {
            self.anchor_version.clone_from(&other.anchor_version);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionField {
    Rust,
    Solana,
    Anchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionSourceKind {
    RustToolchain,
    AnchorToml,
    CargoToml,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionSource {
    pub field: VersionField,
    pub kind: VersionSourceKind,
    pub path: PathBuf,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityReason {
    ExactAnchorMatch,
    ExactSolanaMatch,
    FallbackLatestKnown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InferredFields {
    pub rust_version: bool,
    pub solana_version: bool,
    pub anchor_version: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatibilityRule {
    pub solana: &'static str,
    pub anchor: &'static str,
    pub rust: &'static str,
    pub notes: &'static str,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityAssessment {
    pub matched_rule: Option<&'static CompatibilityRule>,
    pub latest_rule: &'static CompatibilityRule,
    pub reason: CompatibilityReason,
    pub confidence: Confidence,
    pub inferred_fields: InferredFields,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionReport {
    pub detected: ProjectVersions,
    pub resolved: ProjectVersions,
    pub compatibility: CompatibilityAssessment,
    pub sources: Vec<VersionSource>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentEnvironment {
    pub rust_version: Option<String>,
    pub solana_version: Option<String>,
    pub anchor_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub recursive: bool,
    pub skip_directories: &'static [&'static str],
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            recursive: true,
            skip_directories: &[
                "node_modules",
                "target",
                ".git",
                "dist",
                "build",
                ".idea",
                ".vscode",
                "coverage",
            ],
        }
    }
}
