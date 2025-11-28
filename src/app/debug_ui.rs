//! Debug UI state and rendering

use std::time::Instant;

use sysinfo::System;

/// Mouse debug information
pub struct MouseDebugInfo<'a> {
    pub cursor_pos: Option<winit::dpi::PhysicalPosition<f64>>,
    pub viewport_rect: Option<egui::Rect>,
    pub viewport_id: Option<String>,
    pub viewport_local_pos: Option<[f32; 2]>,
    pub world_pos: Option<[f32; 2]>,
    pub last_click_info: &'a Option<String>,
}

/// Debug UI state for toggling different panels
#[derive(Debug, Clone)]
pub struct DebugUIState {
    pub show_window: bool,
    pub show_fps: bool,
    pub show_world_state: bool,
    pub show_debug_info: bool,
    pub show_system_info: bool,
    pub show_mouse_info: bool,
    pub show_input_system: bool,
    frame_times: Vec<f32>,
    last_frame_time: Instant,
}

impl Default for DebugUIState {
    fn default() -> Self {
        Self {
            show_window: cfg!(debug_assertions),
            show_fps: true,
            show_world_state: true,
            show_debug_info: true,
            show_system_info: true,
            show_mouse_info: true,
            show_input_system: true,
            frame_times: Vec::with_capacity(100),
            last_frame_time: Instant::now(),
        }
    }
}

impl DebugUIState {
    /// Toggles the debug window visibility (debug builds only)
    #[cfg(debug_assertions)]
    pub fn toggle_window(&mut self) {
        self.show_window = !self.show_window;
    }

    /// Updates frame timing information
    pub fn update_frame_time(&mut self) {
        let now = Instant::now();
        let frame_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }
    }

    /// Gets the current FPS
    pub fn fps(&self) -> f32 {
        if self.frame_times.is_empty() {
            return 0.0;
        }
        let avg_frame_time: f32 =
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        }
    }

    /// Renders the debug UI
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        world: &crate::sim::World,
        surface_config: &wgpu::SurfaceConfiguration,
        mouse_info: MouseDebugInfo,
        input_context: &crate::app::input::InputContext,
    ) {
        // Only show debug window if enabled
        if !self.show_window {
            return;
        }

        // Update frame timing
        self.update_frame_time();

        // Single debug window with toggleable sections
        egui::Window::new("Debug Info")
            .default_pos([10.0, 40.0])
            .default_width(300.0)
            .resizable(true)
            .scroll([false, true])
            .show(ctx, |ui| {
                ui.heading("Categories");
                ui.separator();

                // Toggle checkboxes
                ui.checkbox(&mut self.show_fps, "FPS");
                ui.checkbox(&mut self.show_mouse_info, "Mouse Info");
                ui.checkbox(&mut self.show_input_system, "Input System");
                ui.checkbox(&mut self.show_world_state, "World State");
                ui.checkbox(&mut self.show_debug_info, "Renderer Info");
                ui.checkbox(&mut self.show_system_info, "System Info");

                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // FPS Section
                        if self.show_fps {
                            ui.heading("FPS");
                            ui.label(format!("FPS: {:.1}", self.fps()));
                            ui.label(format!(
                                "Frame time: {:.2}ms",
                                if !self.frame_times.is_empty() {
                                    self.frame_times.last().unwrap() * 1000.0
                                } else {
                                    0.0
                                }
                            ));
                            ui.separator();
                        }

                        // Mouse Info Section
                        if self.show_mouse_info {
                            ui.heading("Mouse Info");

                            // Cursor position (window coordinates)
                            if let Some(pos) = mouse_info.cursor_pos {
                                ui.label(format!("Window pos: ({:.1}, {:.1})", pos.x, pos.y));
                            } else {
                                ui.label("Window pos: None");
                            }

                            // Egui hover position
                            if let Some(hover_pos) = ctx.pointer_hover_pos() {
                                ui.label(format!(
                                    "Egui pos: ({:.1}, {:.1})",
                                    hover_pos.x, hover_pos.y
                                ));
                            } else {
                                ui.label("Egui pos: None");
                            }

                            // Mouse button states
                            let pointer = ctx.input(|i| i.pointer.clone());
                            ui.label(format!("Primary down: {}", pointer.primary_down()));
                            ui.label(format!("Secondary down: {}", pointer.secondary_down()));
                            ui.label(format!("Middle down: {}", pointer.middle_down()));

                            // Viewport info
                            if let Some(id) = &mouse_info.viewport_id {
                                ui.label(format!("Viewport ID: {}", id));
                            } else {
                                ui.label("Viewport ID: None");
                            }

                            if let Some(rect) = mouse_info.viewport_rect {
                                ui.label(format!(
                                    "Viewport rect: ({:.0}, {:.0}) -> ({:.0}, {:.0})",
                                    rect.left(),
                                    rect.top(),
                                    rect.right(),
                                    rect.bottom()
                                ));
                                ui.label(format!(
                                    "Viewport size: {:.0}x{:.0}",
                                    rect.width(),
                                    rect.height()
                                ));
                            } else {
                                ui.label("Viewport rect: None");
                            }

                            // Viewport-local coordinates
                            if let Some(local_pos) = mouse_info.viewport_local_pos {
                                ui.label(format!(
                                    "Viewport-local: ({:.1}, {:.1})",
                                    local_pos[0], local_pos[1]
                                ));
                            } else {
                                ui.label("Viewport-local: None");
                            }

                            // World coordinates
                            if let Some(world_pos) = mouse_info.world_pos {
                                ui.label(format!(
                                    "World coords: ({:.3}, {:.3})",
                                    world_pos[0], world_pos[1]
                                ));
                            } else {
                                ui.label("World coords: None");
                            }

                            ui.separator();

                            // Last click info
                            if let Some(info) = mouse_info.last_click_info {
                                ui.label(format!("Last click: {}", info));
                            } else {
                                ui.label("Last click: None");
                            }

                            ui.separator();
                        }

                        // Input System Section
                        if self.show_input_system {
                            ui.heading("Input System");

                            // Raw input state
                            let input_state = input_context.state();
                            ui.label("=== Raw Input State ===");

                            // Mouse buttons
                            ui.label(format!("Left button: {:?}", input_state.mouse.buttons.left));
                            ui.label(format!(
                                "Right button: {:?}",
                                input_state.mouse.buttons.right
                            ));
                            ui.label(format!(
                                "Middle button: {:?}",
                                input_state.mouse.buttons.middle
                            ));

                            // Scroll
                            if input_state.mouse.scroll_delta != [0.0, 0.0] {
                                ui.label(format!(
                                    "Scroll delta: ({:.1}, {:.1})",
                                    input_state.mouse.scroll_delta[0],
                                    input_state.mouse.scroll_delta[1]
                                ));
                            }

                            ui.add_space(5.0);

                            // Events generated this frame
                            ui.label("=== Events This Frame ===");
                            let events = input_context.debug_last_events();
                            if events.is_empty() {
                                ui.label("No events");
                            } else {
                                for event in events {
                                    ui.label(event);
                                }
                            }

                            ui.add_space(5.0);

                            // Viewports
                            ui.label("=== Viewports ===");
                            let viewports = input_context.debug_viewports();
                            if viewports.is_empty() {
                                ui.label("No viewports registered");
                            } else {
                                for (id, rect, name) in viewports {
                                    ui.label(format!(
                                        "{:?} '{}': ({:.0},{:.0}) {}x{}",
                                        id, name, rect.x, rect.y, rect.width, rect.height
                                    ));
                                }
                            }

                            ui.add_space(5.0);

                            // Handlers
                            ui.label("=== Handlers ===");
                            let handlers = input_context.debug_handlers();
                            for (name, priority) in handlers {
                                ui.label(format!("'{}' priority={}", name, priority));
                            }

                            ui.separator();
                        }

                        // World State Section
                        if self.show_world_state {
                            ui.heading("World State");
                            ui.label(format!("Simulation time: {:.2}s", world.sim_time()));
                            ui.label(format!("Tick count: {}", world.tick_count()));
                            ui.label(format!("Time scale: {:.2}x", world.time_scale()));
                            ui.label(format!("Paused: {}", world.is_paused()));
                            ui.separator();
                        }

                        // Renderer Info Section
                        if self.show_debug_info {
                            ui.heading("Renderer Info");
                            ui.label(format!("wgpu backend: {:?}", wgpu::Backends::all()));
                            ui.label(format!(
                                "Surface: {}x{}",
                                surface_config.width, surface_config.height
                            ));
                            ui.label(format!("Format: {:?}", surface_config.format));
                            ui.separator();
                        }

                        // System Info Section
                        if self.show_system_info {
                            ui.heading("System Info");

                            // Gather system information
                            let mut sys = System::new_all();
                            sys.refresh_all();

                            let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
                            let os_version =
                                System::os_version().unwrap_or_else(|| "Unknown".to_string());
                            let kernel_version =
                                System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
                            let physical_cores = System::physical_core_count().unwrap_or(0);
                            let logical_cores = sys.cpus().len();
                            let total_memory_gb = sys.total_memory() as f64 / 1_073_741_824.0;

                            ui.label(format!("OS: {} {}", os_name, os_version));
                            ui.label(format!("Kernel: {}", kernel_version));
                            ui.label(format!("Physical cores: {}", physical_cores));
                            ui.label(format!("Logical cores: {}", logical_cores));
                            ui.label(format!("Memory: {:.1} GB", total_memory_gb));
                            ui.separator();
                        }
                    });
            });
    }
}
