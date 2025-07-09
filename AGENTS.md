# Agent Guidelines for anchor-version-detector

## Build/Test Commands
- Build: `cargo build` or `cargo build --release`
- Run: `cargo run <project_directory>`
- Test: `cargo test` (no specific tests exist yet)
- Single test: `cargo test <test_name>` (when tests are added)
- Lint: `cargo clippy` (includes security lints from Cargo.toml)
- Format: `cargo fmt`

## Code Style & Conventions
- **Language**: Rust (edition 2024)
- **Error Handling**: Use `anyhow::Result<T>` for functions that can fail
- **Imports**: Group std imports first, then external crates, then local modules (see main.rs:12-15)
- **Naming**: snake_case for functions/variables, PascalCase for types/structs, SCREAMING_SNAKE_CASE for constants
- **Types**: Use explicit types for public APIs, prefer `Option<T>` over nullable patterns
- **Structs**: Use `#[derive(Debug, Deserialize)]` for config structs, add `Clone` when needed
- **Constants**: Define at module level with descriptive names and documentation

## Security Requirements (from .cursorrules.md)
- Never hardcode secrets, tokens, passwords, or API keys
- Use standard library over dependencies when possible
- All user inputs must be sanitized and validated (see path validation in main.rs:136-159)
- Add security reasoning comments: `// [SECURITY REASONING]: This approach is safe because...`
- Include security intent markers: `// [SECURITY INTENT]: What this protects`
- File size limits enforced (10KB for rust-toolchain, 100KB for TOML files)
- Path canonicalization required to prevent directory traversal attacks

## Project-Specific Notes
- Tool detects Rust/Solana/Anchor versions from project files with recursive directory scanning
- Main logic in `src/main.rs` with structured TOML parsing and fallback to generic parsing
- Uses compatibility matrix (COMPATIBILITY_RULES) for version inference when direct detection fails
- Skips common build/cache directories (node_modules, target, .git, etc.) for performance