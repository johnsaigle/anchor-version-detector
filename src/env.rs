use std::process::Command;

use crate::types::CurrentEnvironment;

#[must_use]
pub fn detect_current_environment() -> CurrentEnvironment {
    CurrentEnvironment {
        rust_version: get_rustc_version(),
        solana_version: get_agave_version(),
        anchor_version: get_avm_version(),
    }
}

pub fn get_rustc_version() -> Option<String> {
    match Command::new("rustc").arg("--version").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(1)
            .map(std::string::ToString::to_string),
        _ => None,
    }
}

pub fn get_agave_version() -> Option<String> {
    match Command::new("agave-install").arg("-V").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(1)
            .map(std::string::ToString::to_string),
        _ => None,
    }
}

pub fn get_avm_version() -> Option<String> {
    match Command::new("avm").arg("-V").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .nth(1)
            .map(std::string::ToString::to_string),
        _ => None,
    }
}
