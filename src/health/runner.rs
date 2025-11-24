//! Test runner for orchestrating health checks

use std::time::Instant;

use super::check::{CheckResult, CheckStatus, SystemCheck};

/// Results from running a health check suite
#[derive(Debug)]
pub struct HealthCheckReport {
    /// Individual check results with their system names
    pub results: Vec<(String, CheckResult)>,
    /// Total number of checks run
    pub total: usize,
    /// Number of passing checks
    pub passed: usize,
    /// Number of checks with warnings
    pub warned: usize,
    /// Number of failing checks
    pub failed: usize,
}

impl HealthCheckReport {
    /// Returns true if all checks passed (no failures)
    pub fn is_healthy(&self) -> bool {
        self.failed == 0
    }

    /// Returns true if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.warned > 0
    }

    /// Returns the appropriate exit code for this report
    /// 0 = all pass, 1 = any fail, 2 = any warn (but no fail)
    pub fn exit_code(&self) -> i32 {
        if self.failed > 0 {
            1
        } else if self.warned > 0 {
            2
        } else {
            0
        }
    }
}

/// Orchestrates running health checks and collecting results
pub struct HealthCheckRunner {
    checks: Vec<Box<dyn SystemCheck>>,
}

impl HealthCheckRunner {
    /// Creates a new runner with no checks
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Adds a check to the runner
    pub fn add_check<C: SystemCheck + 'static>(mut self, check: C) -> Self {
        self.checks.push(Box::new(check));
        self
    }

    /// Runs all registered checks and returns a report
    pub fn run(self) -> HealthCheckReport {
        let mut results = Vec::new();
        let mut passed = 0;
        let mut warned = 0;
        let mut failed = 0;

        for check in self.checks {
            let name = check.name().to_string();
            let start = Instant::now();
            let mut result = check.check();
            let duration = start.elapsed();

            // Update duration
            result = result.with_duration(duration);

            // Update counters
            match result.status {
                CheckStatus::Pass => passed += 1,
                CheckStatus::Warn => warned += 1,
                CheckStatus::Fail => failed += 1,
            }

            results.push((name, result));
        }

        let total = results.len();

        HealthCheckReport {
            results,
            total,
            passed,
            warned,
            failed,
        }
    }
}

impl Default for HealthCheckRunner {
    fn default() -> Self {
        Self::new()
    }
}
