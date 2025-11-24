//! Build information health check

use crate::build_info;
use crate::health::check::{CheckResult, SystemCheck};

/// Checks that build information is accessible and valid
pub struct BuildInfoCheck;

impl BuildInfoCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BuildInfoCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for BuildInfoCheck {
    fn name(&self) -> &'static str {
        "Build Info"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates build metadata (git, rustc, timestamps)")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();

        // Git information
        details.push(format!(
            "  Git: {}@{} (dirty: {})",
            build_info::GIT_BRANCH,
            build_info::git_sha_short(),
            build_info::is_git_dirty()
        ));

        // Build timestamp
        details.push(format!("  Build time: {}", build_info::BUILD_TIMESTAMP));

        // Rust compiler info
        details.push(format!(
            "  Rustc: {} ({})",
            build_info::RUSTC_SEMVER,
            build_info::RUSTC_CHANNEL
        ));

        // Target triple
        details.push(format!("  Target: {}", build_info::CARGO_TARGET_TRIPLE));

        // Optimization level
        details.push(format!("  Opt level: {}", build_info::CARGO_OPT_LEVEL));

        CheckResult::pass("Build metadata accessible").with_details(details.join("\n"))
    }
}
