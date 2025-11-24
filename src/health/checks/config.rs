//! Configuration system health check

use crate::app::AppConfig;
use crate::health::check::{CheckResult, SystemCheck};

/// Checks that configuration can be loaded for all profiles
pub struct ConfigCheck {
    profiles: Vec<&'static str>,
}

impl ConfigCheck {
    /// Creates a new config check with default profiles
    pub fn new() -> Self {
        Self {
            profiles: vec!["debug", "release"],
        }
    }

    /// Creates a config check with custom profiles
    pub fn with_profiles(profiles: Vec<&'static str>) -> Self {
        Self { profiles }
    }
}

impl Default for ConfigCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for ConfigCheck {
    fn name(&self) -> &'static str {
        "Configuration"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates configuration loading from files and environment")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();
        let mut all_success = true;
        let mut has_warnings = false;

        // Test loading each profile
        for profile in &self.profiles {
            match AppConfig::load(profile) {
                Ok(config) => {
                    details.push(format!(
                        "  ✓ Profile '{}': loaded successfully (window: {}x{})",
                        profile, config.window.width, config.window.height
                    ));
                }
                Err(e) => {
                    details.push(format!("  ✗ Profile '{}': failed to load - {}", profile, e));
                    all_success = false;
                }
            }
        }

        // Test loading from environment
        match AppConfig::load_from_env() {
            Ok(config) => {
                details.push(format!(
                    "  ✓ Environment config: profile '{}' loaded",
                    config.profile
                ));
            }
            Err(e) => {
                details.push(format!("  ⚠ Environment config: {}", e));
                has_warnings = true;
            }
        }

        let details_str = details.join("\n");

        if !all_success {
            CheckResult::fail("Failed to load one or more config profiles")
                .with_details(details_str)
        } else if has_warnings {
            CheckResult::warn("Config loaded with warnings").with_details(details_str)
        } else {
            CheckResult::pass(format!("{} profiles validated", self.profiles.len()))
                .with_details(details_str)
        }
    }
}
