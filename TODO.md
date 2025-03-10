If the anchor version isn't found in Anchor.toml, the script should look in Cargo.toml for e.g.

```toml
[workspace.dependencies]
anchor-lang = "0.29.0"
anchor-spl = "0.29.0"
```
