//! Integration tests for the health check system

use oil_pool::health::{self, HealthCheckRunner, SystemCheck, checks::*};

#[test]
fn test_all_health_checks() {
    // Run all health checks
    let report = health::run_all_checks();

    // Print report for debugging if tests fail
    if !report.is_healthy() {
        eprintln!("\n{}", health::format_report(&report));
    }

    // Assert that all checks passed (no failures)
    assert!(
        report.is_healthy(),
        "Health checks failed: {} failures, {} warnings",
        report.failed,
        report.warned
    );
}

#[test]
fn test_config_check() {
    let check = ConfigCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "Config check failed: {}",
        result.message
    );
}

#[test]
fn test_world_check() {
    let check = WorldCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "World check failed: {}",
        result.message
    );
}

#[test]
fn test_build_info_check() {
    let check = BuildInfoCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "Build info check failed: {}",
        result.message
    );
}

#[test]
fn test_system_info_check() {
    let check = SystemInfoCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "System info check failed: {}",
        result.message
    );
}

#[test]
fn test_runner_collects_all_checks() {
    let report = HealthCheckRunner::new()
        .add_check(ConfigCheck::new())
        .add_check(WorldCheck::new())
        .run();

    assert_eq!(report.total, 2, "Expected 2 checks in report");
    assert_eq!(report.passed + report.warned + report.failed, report.total);
}

#[test]
fn test_report_exit_codes() {
    // Test with successful checks
    let report = HealthCheckRunner::new().add_check(ConfigCheck::new()).run();

    if report.failed > 0 {
        assert_eq!(report.exit_code(), 1);
    } else if report.warned > 0 {
        assert_eq!(report.exit_code(), 2);
    } else {
        assert_eq!(report.exit_code(), 0);
    }
}

#[test]
fn test_graphics_backend_check() {
    let check = GraphicsBackendCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "Graphics backend check failed: {}",
        result.message
    );
}

#[test]
fn test_debug_ui_check() {
    let check = DebugUICheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "Debug UI check failed: {}",
        result.message
    );
}

#[test]
fn test_egui_context_check() {
    let check = EguiContextCheck::new();
    let result = check.check();

    assert!(
        result.status.is_ok(),
        "Egui context check failed: {}",
        result.message
    );
}
