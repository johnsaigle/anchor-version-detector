# Agent Guidelines for anchor-version-detector

## Build/Test Commands
- Build: `cargo build` or `cargo build --release`
- Run: `cargo run <project_directory>`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Code Style & Conventions
- **Language**: Rust (edition 2024)
- **Error Handling**: Use `anyhow::Result<T>` for functions that can fail
- **Imports**: Group std imports first, then external crates, then local modules
- **Naming**: snake_case for functions/variables, PascalCase for types/structs
- **Types**: Use explicit types for public APIs, prefer `Option<T>` over nullable patterns
- **Structs**: Use `#[derive(Debug, Deserialize)]` for config structs, add `Clone` when needed

## Security Requirements (from .cursorrules.md)
- Never hardcode secrets, tokens, passwords, or API keys
- Use standard library over dependencies when possible
- All user inputs must be sanitized and validated
- Add security reasoning comments for sensitive operations: `// [SECURITY REASONING]: ...`
- Include security intent markers: `// [SECURITY INTENT]: What this protects`

## Project-Specific Notes
- This tool detects Rust/Solana/Anchor versions from project files
- Main logic in `src/main.rs` with recursive directory scanning
- Uses TOML parsing for `Cargo.toml`, `Anchor.toml`, and `rust-toolchain` files
- Includes version compatibility matrix for inference when versions are missing