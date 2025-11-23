//! Build-time information
//!
//! This module provides access to build metadata captured at compile time,
//! including build timestamps, cargo configuration, compiler version, and git information.

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

/// Git commit SHA (full hash)
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");

/// Git branch name
pub const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");

/// Git commit timestamp
pub const GIT_COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");

/// Whether the working tree was dirty (had uncommitted changes) at build time
pub const GIT_DIRTY: &str = env!("VERGEN_GIT_DIRTY");

/// Returns true if the working tree was dirty at build time
pub fn is_git_dirty() -> bool {
    GIT_DIRTY == "true"
}

/// Returns a short version of the git SHA (first 8 characters)
pub fn git_sha_short() -> &'static str {
    &GIT_SHA[..8.min(GIT_SHA.len())]
}

/// Returns a formatted build version string
///
/// Format: `{branch}@{short_sha}{dirty_marker}`
/// Example: `main@a1b2c3d4` or `feat-123@a1b2c3d4*` (asterisk indicates dirty)
pub fn version_string() -> String {
    let dirty_marker = if is_git_dirty() { "*" } else { "" };
    format!("{}@{}{}", GIT_BRANCH, git_sha_short(), dirty_marker)
}

/// Returns a detailed build info string
///
/// Includes all build metadata: git info, build time, compiler, target, and optimization
pub fn detailed_info() -> String {
    format!(
        "Git: {} @ {}{}\nCommit Time: {}\nBuilt: {}\nTarget: {}\nOptimization: {}\nRustc: {} ({})",
        GIT_BRANCH,
        git_sha_short(),
        if is_git_dirty() { " (dirty)" } else { "" },
        GIT_COMMIT_TIMESTAMP,
        BUILD_TIMESTAMP,
        CARGO_TARGET_TRIPLE,
        CARGO_OPT_LEVEL,
        RUSTC_SEMVER,
        RUSTC_CHANNEL
    )
}
