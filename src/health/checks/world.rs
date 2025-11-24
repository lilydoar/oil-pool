//! Game world/simulation health check

use crate::health::check::{CheckResult, SystemCheck};
use crate::sim::World;

/// Checks that the game world can be initialized and ticked
pub struct WorldCheck;

impl WorldCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WorldCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for WorldCheck {
    fn name(&self) -> &'static str {
        "World/Simulation"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates game world initialization and simulation tick")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();

        // Test basic initialization
        let mut world = World::new();
        details.push("  ✓ World initialized successfully".to_string());

        // Verify initial state
        if world.tick_count() != 0 {
            details.push("  ✗ Initial tick count should be 0".to_string());
            return CheckResult::fail("World initialization failed")
                .with_details(details.join("\n"));
        }
        details.push(format!("  ✓ Initial tick count: {}", world.tick_count()));

        if world.sim_time() != 0.0 {
            details.push("  ✗ Initial sim time should be 0.0".to_string());
            return CheckResult::fail("World initialization failed")
                .with_details(details.join("\n"));
        }
        details.push(format!("  ✓ Initial sim time: {:.2}s", world.sim_time()));

        // Test tick functionality
        world.tick(0.016); // ~60 FPS
        if world.tick_count() != 1 {
            details.push("  ✗ Tick count should increment".to_string());
            return CheckResult::fail("World tick failed").with_details(details.join("\n"));
        }
        details.push(format!("  ✓ After tick: count={}", world.tick_count()));

        // Test pause functionality
        world.pause();
        if !world.is_paused() {
            details.push("  ✗ Pause failed".to_string());
            return CheckResult::fail("World pause failed").with_details(details.join("\n"));
        }
        details.push("  ✓ Pause state: working".to_string());

        // Test time scale
        world.resume();
        world.set_time_scale(2.0);
        if world.time_scale() != 2.0 {
            details.push("  ✗ Time scale failed".to_string());
            return CheckResult::fail("World time scale failed").with_details(details.join("\n"));
        }
        details.push("  ✓ Time scale: working".to_string());

        CheckResult::pass("All world systems operational").with_details(details.join("\n"))
    }
}
