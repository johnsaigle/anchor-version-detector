use anchor_version_detector::compatibility_rules;

fn main() {
    for rule in compatibility_rules() {
        println!(
            "Anchor {} | Solana {} | Rust {}",
            rule.anchor, rule.solana, rule.rust
        );
        println!("  notes: {}", rule.notes);
        println!("  source: {}", rule.source);
    }
}
