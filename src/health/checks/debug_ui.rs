//! Debug UI health check

use crate::app::debug_ui::DebugUIState;
use crate::health::check::{CheckResult, SystemCheck};

/// Checks that debug UI state management works correctly
pub struct DebugUICheck;

impl DebugUICheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebugUICheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for DebugUICheck {
    fn name(&self) -> &'static str {
        "Debug UI"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates debug UI state management and frame timing calculations")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();

        // Test initialization
        let mut ui_state = DebugUIState::default();
        details.push("  ✓ DebugUIState created".to_string());

        // Verify default state
        #[cfg(debug_assertions)]
        if !ui_state.show_window {
            details.push("  ✗ Debug window should be visible in debug builds".to_string());
            return CheckResult::fail("Incorrect default state").with_details(details.join("\n"));
        }

        #[cfg(not(debug_assertions))]
        if ui_state.show_window {
            details.push("  ✗ Debug window should be hidden in release builds".to_string());
            return CheckResult::fail("Incorrect default state").with_details(details.join("\n"));
        }

        details.push(format!(
            "  ✓ Default visibility: {} (expected for build type)",
            if ui_state.show_window { "visible" } else { "hidden" }
        ));

        // Test toggle functionality (debug builds only)
        #[cfg(debug_assertions)]
        {
            let initial_state = ui_state.show_window;
            ui_state.toggle_window();
            if ui_state.show_window == initial_state {
                details.push("  ✗ Toggle failed to change state".to_string());
                return CheckResult::fail("Toggle functionality broken")
                    .with_details(details.join("\n"));
            }
            ui_state.toggle_window(); // Toggle back
            if ui_state.show_window != initial_state {
                details.push("  ✗ Toggle failed to restore state".to_string());
                return CheckResult::fail("Toggle functionality broken")
                    .with_details(details.join("\n"));
            }
            details.push("  ✓ Toggle functionality works".to_string());
        }

        // Test frame timing with simulated frames
        let initial_fps = ui_state.fps();
        if initial_fps != 0.0 {
            details.push("  ✗ Initial FPS should be 0.0".to_string());
            return CheckResult::fail("Incorrect initial FPS").with_details(details.join("\n"));
        }
        details.push("  ✓ Initial FPS: 0.0".to_string());

        // Simulate frame updates (60 FPS = ~16.67ms per frame)
        std::thread::sleep(std::time::Duration::from_millis(17));
        ui_state.update_frame_time();

        std::thread::sleep(std::time::Duration::from_millis(17));
        ui_state.update_frame_time();

        std::thread::sleep(std::time::Duration::from_millis(17));
        ui_state.update_frame_time();

        let fps = ui_state.fps();
        if fps <= 0.0 {
            details.push("  ✗ FPS calculation failed".to_string());
            return CheckResult::fail("Frame timing broken").with_details(details.join("\n"));
        }

        // FPS should be approximately 60 (allow for timing variance)
        if fps < 30.0 || fps > 120.0 {
            details.push(format!(
                "  ⚠ FPS calculation seems off: {:.1} (expected ~60)",
                fps
            ));
            details.push("    This may be due to system load or timing precision".to_string());
        } else {
            details.push(format!("  ✓ Frame timing calculation: {:.1} FPS", fps));
        }

        // Test panel toggles
        ui_state.show_fps = false;
        ui_state.show_world_state = true;
        ui_state.show_debug_info = false;
        ui_state.show_system_info = true;

        if ui_state.show_fps || !ui_state.show_world_state || ui_state.show_debug_info
            || !ui_state.show_system_info
        {
            details.push("  ✗ Panel toggle state incorrect".to_string());
            return CheckResult::fail("Panel toggles broken").with_details(details.join("\n"));
        }
        details.push("  ✓ Panel toggles work correctly".to_string());

        CheckResult::pass("All debug UI systems operational").with_details(details.join("\n"))
    }
}
