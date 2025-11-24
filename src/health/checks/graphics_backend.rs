//! Graphics backend health check

use crate::health::check::{CheckResult, SystemCheck};

/// Checks that wgpu graphics backend can initialize
pub struct GraphicsBackendCheck;

impl GraphicsBackendCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GraphicsBackendCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemCheck for GraphicsBackendCheck {
    fn name(&self) -> &'static str {
        "Graphics Backend"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Validates wgpu instance creation and adapter availability")
    }

    fn check(&self) -> CheckResult {
        let mut details = Vec::new();

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        details.push("  ✓ wgpu instance created".to_string());

        // Enumerate available adapters
        let adapters: Vec<_> = instance.enumerate_adapters(wgpu::Backends::all()).collect();

        if adapters.is_empty() {
            details.push("  ✗ No graphics adapters found".to_string());
            return CheckResult::fail("No compatible graphics adapters available")
                .with_details(details.join("\n"));
        }

        details.push(format!("  ✓ Found {} adapter(s)", adapters.len()));

        // Analyze each adapter
        let mut has_discrete = false;
        let mut has_integrated = false;
        let mut backend_types = std::collections::HashSet::new();

        for (i, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            backend_types.insert(format!("{:?}", info.backend));

            let device_type = match info.device_type {
                wgpu::DeviceType::DiscreteGpu => {
                    has_discrete = true;
                    "Discrete GPU"
                }
                wgpu::DeviceType::IntegratedGpu => {
                    has_integrated = true;
                    "Integrated GPU"
                }
                wgpu::DeviceType::VirtualGpu => "Virtual GPU",
                wgpu::DeviceType::Cpu => "CPU",
                wgpu::DeviceType::Other => "Other",
            };

            details.push(format!(
                "    [{i}] {} - {} ({:?})",
                info.name, device_type, info.backend
            ));
        }

        // Summary
        details.push(format!(
            "  Backends available: {}",
            backend_types.into_iter().collect::<Vec<_>>().join(", ")
        ));

        // Determine status
        if has_discrete {
            CheckResult::pass(format!("{} adapters found (discrete GPU available)", adapters.len()))
                .with_details(details.join("\n"))
        } else if has_integrated {
            CheckResult::warn(format!(
                "{} adapters found (integrated GPU only)",
                adapters.len()
            ))
            .with_details(details.join("\n"))
        } else {
            CheckResult::warn(format!(
                "{} adapters found (no hardware GPU detected)",
                adapters.len()
            ))
            .with_details(details.join("\n"))
        }
    }
}
