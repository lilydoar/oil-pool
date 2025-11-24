//! Health check system for validating application initialization and status
//!
//! This module provides a framework for testing system health, useful for:
//! - Validating application startup
//! - CI/CD health checks
//! - Debugging initialization issues
//! - Ensuring all systems are operational
//!
//! # Example
//!
//! ```no_run
//! use oil_pool::health::{HealthCheckRunner, checks::*};
//!
//! let report = HealthCheckRunner::new()
//!     .add_check(ConfigCheck::new())
//!     .add_check(WorldCheck::new())
//!     .add_check(BuildInfoCheck::new())
//!     .add_check(SystemInfoCheck::new())
//!     .run();
//!
//! if report.is_healthy() {
//!     println!("All systems operational!");
//! }
//! ```

pub mod check;
pub mod checks;
pub mod reporter;
pub mod runner;

pub use check::{CheckResult, CheckStatus, SystemCheck};
pub use reporter::{format_report, print_report};
pub use runner::{HealthCheckReport, HealthCheckRunner};

/// Runs all default health checks and returns a report
pub fn run_all_checks() -> HealthCheckReport {
    HealthCheckRunner::new()
        .add_check(checks::ConfigCheck::new())
        .add_check(checks::WorldCheck::new())
        .add_check(checks::BuildInfoCheck::new())
        .add_check(checks::SystemInfoCheck::new())
        .run()
}
