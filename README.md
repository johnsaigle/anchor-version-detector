## Solana Project Version Detector

A command-line tool that analyzes Solana/Anchor projects to detect or infer required versions of Rust, Solana, and Anchor.

_Disclaimer: This code is neither tested nor secure. It's increasingly vibe-coded so don't expect any safety or reliability._

## Features

- Detects versions from multiple configuration files:
  - `rust-toolchain` for Rust version
  - `Anchor.toml` for Solana and Anchor versions
  - `Cargo.toml` for Solana and Anchor dependencies
- Prints commands to make local environment compatible (agave-install, avm, rustup)
- Recursively searches subdirectories if versions aren't found in the root
- Intelligently infers missing versions based on compatibility rules
- Skips irrelevant directories (node_modules, target, etc.)
- Handles various version specification formats
- Falls back to compatibility-based version inference when needed

## Usage

Run the tool by providing a path to your Solana/Anchor project:

```bash
cargo run -- /path/to/your/project
```

Example output:
```
Detected/Inferred Versions:
Rust: nightly-2023-10-29 (from /path/to/project/rust-toolchain)
Solana: 1.18.10
Anchor: 0.29.0

To work with this project, configure your environment as follows:
rustup default nightly-2023-10-29
rustup component add rust-analyzer
agave-install init 1.18.10
avm use 0.29.0
```

## How It Works

1. **Top-level Check**: First checks the root directory for version information in configuration files.

2. **Recursive Search**: If versions are missing, recursively searches subdirectories while skipping common irrelevant directories like `node_modules` and `target`.

3. **Version Detection**:
   - Reads `rust-toolchain` for Rust version (supports both plain text and TOML formats)
   - Parses `Anchor.toml` for Solana and Anchor versions
   - Checks `Cargo.toml` for dependency versions

4. **Version Inference**: If versions are still missing after searching, uses compatibility rules to infer appropriate versions based on known working combinations.

## Supported Version Formats

### rust-toolchain
```
1.69.0
```
or
```toml
[toolchain]
channel = "1.69.0"
```

### Anchor.toml
```toml
[toolchain]
anchor_version = "0.29.0"
solana_version = "1.18.10"
```

### Cargo.toml
```toml
[dependencies]
solana-program = "1.17.0"
anchor-lang = { version = "0.29.0" }
```

## TODO

