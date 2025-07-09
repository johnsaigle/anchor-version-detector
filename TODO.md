# TODO

## Return early and give an error if the directory does not look like a Solana project

Currently the program will output the default/latest Rust/Agave information when no info can be found.

Instead, if there is no trace of either a Solana or Anchor version, the code should tell the user 
that their target doesn't look like a Solana project, then exit

## Enhance detection methods for Anchor version
If the anchor version isn't found in Anchor.toml, the script should look in Cargo.toml for e.g.

```toml
[workspace.dependencies]
anchor-lang = "0.29.0"
anchor-spl = "0.29.0"
```

## Resolve any errors raised by clippy
