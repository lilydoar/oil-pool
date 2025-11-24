//! System information health check

use sysinfo::System;

use crate::health::check::{CheckResult, SystemCheck};

/// Checks that system information can be gathered
pub struct SystemInfoCheck;

impl SystemInfoCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemInfoCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for SystemInfoCheck {
    fn name(&self) -> &'static str {
        "System Info"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates OS, CPU, and memory information gathering")
    }

    fn check(&self) -> CheckResult {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut details = Vec::new();

        // OS information
        let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());

        details.push(format!("  OS: {} {}", os_name, os_version));
        details.push(format!("  Kernel: {}", kernel_version));

        // CPU information
        let physical_cores = System::physical_core_count().unwrap_or(0);
        let logical_cores = sys.cpus().len();

        if physical_cores == 0 || logical_cores == 0 {
            return CheckResult::warn("Unable to detect CPU cores")
                .with_details(details.join("\n"));
        }

        details.push(format!(
            "  CPU cores: {} physical, {} logical",
            physical_cores, logical_cores
        ));

        // Memory information
        let total_memory_gb = sys.total_memory() as f64 / 1_073_741_824.0;

        if total_memory_gb < 1.0 {
            return CheckResult::warn("Low memory detected").with_details(details.join("\n"));
        }

        details.push(format!("  Memory: {:.1} GB total", total_memory_gb));

        // Host information
        if let Some(hostname) = System::host_name() {
            details.push(format!("  Hostname: {}", hostname));
        }

        CheckResult::pass("System info gathered successfully").with_details(details.join("\n"))
    }
}
