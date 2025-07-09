# TODO

## Parsing anchor version should not include `=`

```BUG
Rust version could not be determined. Suggesting latest.
Detected/Inferred Versions:
Rust: 1.84.1
Solana: 2
Anchor: =0.30.1

To work with this project, configure your environment as follows:
```
rustup default 1.84.1
agave-install init 2
avm use =0.30.1
```
ENDBUG```



## Enhance detection methods for Anchor version
If the anchor version isn't found in Anchor.toml, the script should look in Cargo.toml for e.g.

```toml
[workspace.dependencies]
anchor-lang = "0.29.0"
anchor-spl = "0.29.0"
```

## Resolve any errors raised by clippy
