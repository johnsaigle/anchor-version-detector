use anyhow::{Result, anyhow};

use crate::parse::clean_version;
use crate::types::{
    CompatibilityAssessment, CompatibilityReason, CompatibilityRule, Confidence, InferredFields,
    ProjectVersions,
};

const COMPATIBILITY_RULES: [CompatibilityRule; 10] = [
    // Anchor > v1
    CompatibilityRule {
        solana: "3.1.10",
        anchor: "1.0.2",
        rust: "1.89.0",
        notes: "Anchor 1.0.0 compatibility from the v1.0.0 release notes and Rust template.",
        source: "https://www.anchor-lang.com/docs/updates/release-notes/1-0-0",
    },
    CompatibilityRule {
        solana: "3.1.10",
        anchor: "1.0.1",
        rust: "1.89.0",
        notes: "Anchor 1.0.0 compatibility from the v1.0.0 release notes and Rust template.",
        source: "https://www.anchor-lang.com/docs/updates/release-notes/1-0-0",
    },
    CompatibilityRule {
        solana: "3.1.10",
        anchor: "1.0.0",
        rust: "1.89.0",
        notes: "Anchor 1.0.0 compatibility from the v1.0.0 release notes and Rust template.",
        source: "https://www.anchor-lang.com/docs/updates/release-notes/1-0-0",
    },
    // Anchor < v1
    CompatibilityRule {
        solana: "3.0.6",
        anchor: "0.32.1",
        rust: "1.89.0",
        notes: "Minor bug-fix release aligned with Anchor 0.32.0 toolchain expectations.",
        source: "https://github.com/johnsaigle/anchor-version-detector/issues/2",
    },
    CompatibilityRule {
        solana: "3.0.6",
        anchor: "0.32.0",
        rust: "1.89.0",
        notes: "Anchor 0.32.0 compatibility inferred from release notes and Agave toolchain state.",
        source: "https://github.com/johnsaigle/anchor-version-detector/issues/2",
    },
    CompatibilityRule {
        solana: "2.1.0",
        anchor: "0.31.0",
        rust: "1.84.1",
        notes: "Based on the Agave rust-toolchain and Anchor 0.31.0 release notes.",
        source: "https://www.anchor-lang.com/release-notes/0.31.0",
    },
    CompatibilityRule {
        solana: "1.18.17",
        anchor: "0.30.1",
        rust: "1.76.0",
        notes: "Based on the Solana rust-toolchain and Anchor 0.30.1 release notes.",
        source: "https://www.anchor-lang.com/release-notes/0.30.1",
    },
    CompatibilityRule {
        solana: "1.18.8",
        anchor: "0.30.0",
        rust: "1.76.0",
        notes: "Based on the Solana rust-toolchain and Anchor 0.30.0 release notes.",
        source: "https://www.anchor-lang.com/release-notes/0.30.0",
    },
    CompatibilityRule {
        solana: "1.17.0",
        anchor: "0.29.0",
        rust: "1.69.0",
        notes: "Listed directly in the Anchor 0.29.0 release notes.",
        source: "https://www.anchor-lang.com/release-notes/0.29.0",
    },
    CompatibilityRule {
        solana: "1.16.0",
        anchor: "0.28.0",
        rust: "1.68.0",
        notes: "Listed in the Anchor changelog for 0.28.0.",
        source: "https://www.anchor-lang.com/release-notes/changelog#0-28-0-2023-06-09",
    },
    CompatibilityRule {
        solana: "1.15.0",
        anchor: "0.27.0",
        rust: "1.67.0",
        notes: "Historical compatibility entry retained from the original detector matrix.",
        source: "project compatibility matrix",
    },
    CompatibilityRule {
        solana: "1.14.0",
        anchor: "0.26.0",
        rust: "1.66.0",
        notes: "Historical compatibility entry retained from the original detector matrix.",
        source: "project compatibility matrix",
    },
];

#[must_use]
pub const fn compatibility_rules() -> &'static [CompatibilityRule] {
    &COMPATIBILITY_RULES
}

#[must_use]
pub const fn latest_compatible_rule() -> &'static CompatibilityRule {
    &COMPATIBILITY_RULES[0]
}

#[must_use]
pub fn find_rule_by_solana(version: &str) -> Option<&'static CompatibilityRule> {
    let cleaned = clean_version(version);
    compatibility_rules()
        .iter()
        .find(|rule| cleaned.starts_with(rule.solana))
}

#[must_use]
pub fn find_rule_by_anchor(version: &str) -> Option<&'static CompatibilityRule> {
    let cleaned = clean_version(version);
    compatibility_rules()
        .iter()
        .find(|rule| cleaned.starts_with(rule.anchor))
}

/// Build compatibility metadata for the detected project versions.
///
/// # Errors
///
/// Returns an error when the input does not look like a Solana or Anchor project.
pub fn assess_versions(detected: &ProjectVersions) -> Result<CompatibilityAssessment> {
    if !detected.is_solana_project() {
        return Err(anyhow!(
            "This directory does not appear to be a Solana project. No Solana or Anchor version information found.\n\
            Expected to find one of:\n\
            - Anchor.toml with toolchain configuration\n\
            - Cargo.toml with solana-program, anchor-lang, or anchor-spl dependencies"
        ));
    }

    if let Some(solana_version) = &detected.solana_version
        && solana_version != "*"
        && let Some(rule) = find_rule_by_solana(solana_version)
    {
        return Ok(build_assessment(
            detected,
            Some(rule),
            CompatibilityReason::ExactSolanaMatch,
        ));
    }

    if let Some(anchor_version) = &detected.anchor_version
        && let Some(rule) = find_rule_by_anchor(anchor_version)
    {
        return Ok(build_assessment(
            detected,
            Some(rule),
            CompatibilityReason::ExactAnchorMatch,
        ));
    }

    Ok(build_assessment(
        detected,
        None,
        CompatibilityReason::FallbackLatestKnown,
    ))
}

/// Resolve missing versions using the compatibility matrix.
///
/// # Errors
///
/// Returns an error when the input does not look like a Solana or Anchor project.
pub fn resolve_versions(
    detected: &ProjectVersions,
) -> Result<(ProjectVersions, CompatibilityAssessment, Vec<String>)> {
    let assessment = assess_versions(detected)?;
    let mut resolved = detected.clone();
    let mut warnings = Vec::new();

    if let Some(rule) = assessment.matched_rule {
        if assessment.inferred_fields.anchor_version {
            resolved.anchor_version = Some(rule.anchor.to_string());
        }
        if assessment.inferred_fields.solana_version {
            resolved.solana_version = Some(rule.solana.to_string());
        }
        if assessment.inferred_fields.rust_version {
            resolved.rust_version = Some(rule.rust.to_string());
        }
    }

    if resolved
        .solana_version
        .as_ref()
        .is_none_or(|version| version == "*")
    {
        warnings.push("Solana version could not be determined exactly. Suggesting latest known compatible version.".to_string());
        resolved.solana_version = Some(assessment.latest_rule.solana.to_string());
    }

    if resolved.rust_version.is_none() {
        warnings.push("Rust version could not be determined exactly. Suggesting latest known compatible version.".to_string());
        resolved.rust_version = Some(assessment.latest_rule.rust.to_string());
    }

    Ok((resolved, assessment, warnings))
}

fn build_assessment(
    detected: &ProjectVersions,
    matched_rule: Option<&'static CompatibilityRule>,
    reason: CompatibilityReason,
) -> CompatibilityAssessment {
    let latest_rule = latest_compatible_rule();

    CompatibilityAssessment {
        matched_rule,
        latest_rule,
        reason,
        confidence: match reason {
            CompatibilityReason::ExactAnchorMatch | CompatibilityReason::ExactSolanaMatch => {
                Confidence::High
            }
            CompatibilityReason::FallbackLatestKnown => Confidence::Low,
        },
        inferred_fields: InferredFields {
            rust_version: detected.rust_version.is_none(),
            solana_version: detected
                .solana_version
                .as_ref()
                .is_none_or(|version| version == "*")
                && matched_rule.is_some(),
            anchor_version: detected.anchor_version.is_none() && matched_rule.is_some(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_rule_by_anchor() {
        let rule = find_rule_by_anchor("^0.30.1").unwrap();
        assert_eq!(rule.solana, "1.18.17");
        assert_eq!(rule.rust, "1.76.0");
    }

    #[test]
    fn test_resolve_versions_from_anchor() {
        let detected = ProjectVersions {
            rust_version: None,
            solana_version: None,
            anchor_version: Some("0.30.1".to_string()),
        };

        let (resolved, assessment, warnings) = resolve_versions(&detected).unwrap();
        assert_eq!(resolved.solana_version.as_deref(), Some("1.18.17"));
        assert_eq!(resolved.rust_version.as_deref(), Some("1.76.0"));
        assert_eq!(assessment.reason, CompatibilityReason::ExactAnchorMatch);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_fallback_for_unknown_solana_version() {
        let detected = ProjectVersions {
            rust_version: None,
            solana_version: Some("*".to_string()),
            anchor_version: Some("9.9.9".to_string()),
        };

        let (resolved, assessment, warnings) = resolve_versions(&detected).unwrap();
        assert_eq!(
            resolved.solana_version.as_deref(),
            Some(latest_compatible_rule().solana)
        );
        assert_eq!(
            resolved.rust_version.as_deref(),
            Some(latest_compatible_rule().rust)
        );
        assert_eq!(assessment.reason, CompatibilityReason::FallbackLatestKnown);
        assert_eq!(warnings.len(), 2);
    }
}
