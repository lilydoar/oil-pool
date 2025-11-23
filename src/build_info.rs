//! Build-time information
//!
//! This module provides access to build metadata captured at compile time,
//! including build timestamps, cargo configuration, and compiler version.

/// Build timestamp (when the binary was compiled)
pub const BUILD_TIMESTAMP: &str = env!("VERGEN_BUILD_TIMESTAMP");

/// Cargo optimization level (0, 1, 2, 3, s, z)
pub const CARGO_OPT_LEVEL: &str = env!("VERGEN_CARGO_OPT_LEVEL");

/// Target triple (e.g., x86_64-unknown-linux-gnu, x86_64-apple-darwin)
pub const CARGO_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");

/// Rust compiler version (e.g., 1.75.0)
pub const RUSTC_SEMVER: &str = env!("VERGEN_RUSTC_SEMVER");

/// Rust channel (stable, beta, or nightly)
pub const RUSTC_CHANNEL: &str = env!("VERGEN_RUSTC_CHANNEL");

/// Returns a formatted build version string
///
/// Format: `{target_triple}-opt{opt_level}`
/// Example: `x86_64-apple-darwin-opt3` or `x86_64-unknown-linux-gnu-opt0`
pub fn version_string() -> String {
    format!("{}-opt{}", CARGO_TARGET_TRIPLE, CARGO_OPT_LEVEL)
}

/// Returns a detailed build info string
///
/// Includes build timestamp, target, optimization level, and compiler version
pub fn detailed_info() -> String {
    format!(
        "Built: {}\nTarget: {}\nOptimization: {}\nRustc: {} ({})",
        BUILD_TIMESTAMP, CARGO_TARGET_TRIPLE, CARGO_OPT_LEVEL, RUSTC_SEMVER, RUSTC_CHANNEL
    )
}
