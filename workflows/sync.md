# Sync Anchor Releases

LLM agent workflow for keeping `anchor-version-detector` aligned with new Anchor releases.

## Goal

Find Anchor releases that are not represented in the tool, determine their supported Solana/Agave and Rust versions from upstream evidence, update the compatibility matrix, test the detector, and bump the crate version.

## Inputs

- Local repository: `anchor-version-detector`
- Anchor repository: `https://github.com/coral-xyz/anchor`
- Anchor documentation and release notes:
  - `https://www.anchor-lang.com/docs/updates/release-notes`
  - `https://www.anchor-lang.com/release-notes/changelog`
  - GitHub releases and tags for `coral-xyz/anchor`

## Outputs

- Updated `src/compatibility.rs` entries for every newly supported Anchor release.
- Updated or added unit tests covering new compatibility rules.
- Updated documentation if behavior, latest known versions, or examples changed.
- Bumped `version` in `Cargo.toml` and regenerated `Cargo.lock`.
- A short summary of releases reviewed, releases added, releases skipped, and evidence used.

## Workflow

1. Establish current support.

   Read `src/compatibility.rs` and collect every `CompatibilityRule.anchor` value. Treat these as already supported versions.

2. Discover upstream Anchor releases.

   Inspect Anchor release notes, changelog entries, GitHub releases, and repository tags. Build a candidate list of Anchor versions newer than, or missing from, the local compatibility matrix.

3. Filter to actionable releases.

   Exclude pre-releases, release candidates, yanked tags, and versions without enough evidence to infer Solana/Agave and Rust compatibility. If a release is skipped, record the exact reason in the final summary.

4. Determine compatibility for each missing release.

   Prefer explicit upstream statements in release notes or changelog entries. If release notes are incomplete, inspect the matching Anchor tag in the Anchor repository and use these files as evidence:

   - `Cargo.toml` and workspace dependency versions for `solana-*` or `agave-*` crates.
   - `rust-toolchain` or `rust-toolchain.toml`.
   - CLI templates that generate project `rust-toolchain`, `Anchor.toml`, or dependency versions.
   - Migration notes that state required Solana, Agave, or Rust versions.

   Do not infer compatibility from local environment versions or from unrelated downstream projects.

5. Update the compatibility matrix.

   Edit `src/compatibility.rs` only after collecting evidence. Add new `CompatibilityRule` entries at the top of `COMPATIBILITY_RULES`, ordered from newest Anchor version to oldest. For each rule:

   - Set `anchor` to the exact Anchor release version.
   - Set `solana` to the exact supported Solana or Agave version where available.
   - Set `rust` to the exact Rust toolchain version where available.
   - Set `notes` to a concise explanation of the evidence.
   - Set `source` to the strongest public source URL. Prefer release notes over repository inspection when both are available.

   If a patch release has the same compatibility as the previous release, add an explicit rule for the patch release with notes explaining the inherited compatibility.

6. Update tests.

   Add or adjust unit tests in `src/compatibility.rs` for the newest supported Anchor version and any edge case introduced by the update. Keep tests focused on public behavior such as `find_rule_by_anchor`, `latest_compatible_rule`, and `resolve_versions`.

7. Update docs if needed.

   Review `README.md` examples and notes. Update them only if the latest known versions, public behavior, or documented examples would otherwise be stale or misleading.

8. Bump the crate version.

   Bump `Cargo.toml` using SemVer:

   - Patch bump for compatibility matrix updates only.
   - Minor bump for new public API, new detection behavior, or expanded file support.
   - Major bump only for breaking public API or CLI changes.

   After editing `Cargo.toml`, run `cargo check` or another Cargo command that regenerates `Cargo.lock`, then confirm the lockfile package version matches.

9. Verify.

   Run:

   ```bash
   cargo test
   cargo check
   cargo clippy --all-targets --all-features
   ```

   If `cargo clippy` fails because of pre-existing warnings unrelated to the update, report that clearly and include the exact failing lint category.

10. Final report.

   Summarize:

   - Anchor releases checked.
   - Releases added to `COMPATIBILITY_RULES`.
   - Releases skipped and why.
   - Evidence sources used for each added release.
   - Version bump applied.
   - Verification commands run and their results.

## Guardrails

- Do not add a compatibility rule without a source URL or repository tag evidence.
- Do not use guessed Solana, Agave, or Rust versions.
- Do not reorder existing historical rules except to keep newest releases first.
- Do not remove existing compatibility rules unless upstream evidence proves they are wrong.
- Do not bump `Cargo.toml` without also updating `Cargo.lock`.
