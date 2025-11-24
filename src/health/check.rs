//! Core health check trait and types

use std::time::Duration;

/// Status of a system check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed successfully
    Pass,
    /// Check passed with warnings
    Warn,
    /// Check failed
    Fail,
}

impl CheckStatus {
    /// Returns true if the check passed (Pass or Warn)
    pub fn is_ok(&self) -> bool {
        matches!(self, CheckStatus::Pass | CheckStatus::Warn)
    }

    /// Returns true if the check failed
    pub fn is_fail(&self) -> bool {
        matches!(self, CheckStatus::Fail)
    }

    /// Returns the status as a colored string
    pub fn as_colored_str(&self) -> String {
        use colored::Colorize;
        match self {
            CheckStatus::Pass => "PASS".green().to_string(),
            CheckStatus::Warn => "WARN".yellow().to_string(),
            CheckStatus::Fail => "FAIL".red().to_string(),
        }
    }
}

/// Result of a system check
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The status of the check
    pub status: CheckStatus,
    /// Brief message describing the result
    pub message: String,
    /// Optional detailed information
    pub details: Option<String>,
    /// How long the check took
    pub duration: Duration,
}

impl CheckResult {
    /// Creates a passing check result
    pub fn pass(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Pass,
            message: message.into(),
            details: None,
            duration: Duration::ZERO,
        }
    }

    /// Creates a warning check result
    pub fn warn(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Warn,
            message: message.into(),
            details: None,
            duration: Duration::ZERO,
        }
    }

    /// Creates a failing check result
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Fail,
            message: message.into(),
            details: None,
            duration: Duration::ZERO,
        }
    }

    /// Adds optional details to the result
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Sets the duration for this check
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

/// Trait for system health checks
pub trait SystemCheck {
    /// Name of the system being checked
    fn name(&self) -> &'static str;

    /// Perform the health check
    fn check(&self) -> CheckResult;

    /// Optional description of what this check validates
    fn description(&self) -> Option<&'static str> {
        None
    }
}
