//! Main application handler for the game

use std::sync::Arc;
use std::time::Instant;

use tracing::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

#[cfg(debug_assertions)]
use winit::keyboard::{KeyCode, PhysicalKey};

use super::debug_ui::DebugUIState;
use super::renderer::Renderer;
use super::window::window_attributes_from_config;
use super::{config::AppConfig, geometry};
use crate::sim::World;
use winit::event::{ElementState, MouseButton};

/// Main game application
pub struct App {
    config: AppConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    world: World,
    debug_ui: DebugUIState,
    last_update: Option<Instant>,
    sim_viewport_rect: Option<egui::Rect>,
    cursor_position: Option<winit::dpi::PhysicalPosition<f64>>,
    last_click_info: Option<String>,
    pending_click: Option<(f32, f32)>,
}

impl App {
    /// Creates a new game application with the provided configuration
    pub fn new(config: AppConfig) -> Self {
        info!(profile = %config.profile, "Starting game");
        info!(?config.window, "Window configuration");

        Self {
            config,
            window: None,
            renderer: None,
            world: World::new(),
            debug_ui: DebugUIState::default(),
            last_update: None,
            sim_viewport_rect: None,
            cursor_position: None,
            last_click_info: None,
            pending_click: None,
        }
    }

    /// Creates a new game application with configuration loaded from environment
    pub fn from_env() -> Self {
        let config = AppConfig::load_from_env().unwrap_or_else(|e| {
            warn!(error = %e, "Failed to load config, using default configuration");
            AppConfig::default()
        });
        Self::new(config)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::from_env()
    }
}

impl App {
    /// Toggles debug window (debug builds only)
    #[cfg(debug_assertions)]
    fn toggle_debug_window(&mut self) {
        self.debug_ui.toggle_window();
    }

    /// Processes a click at the given sim texture coordinates
    fn process_click(&mut self, sim_x: f32, sim_y: f32, config: &wgpu::SurfaceConfiguration) {
        // Create board layout matching the one used in rendering
        let layout = geometry::BoardLayout::centered(config.width as f32, config.height as f32);

        // Convert to board cell
        if let Some((row, col)) = layout.screen_to_cell(sim_x, sim_y) {
            // Try to make the move
            if let Some(tictactoe) = self.world.tictactoe_mut()
                && tictactoe.make_move(row, col)
            {
                self.last_click_info = Some(format!("Placed piece at ({}, {})", row, col));
                info!("Placed piece at ({}, {})", row, col);

                // Check if game is over and reset
                use crate::sim::tictactoe::GameState;
                match tictactoe.game_state() {
                    GameState::Won(player) => {
                        info!("Player {:?} won!", player);
                        tictactoe.reset();
                    }
                    GameState::Draw => {
                        info!("Game is a draw!");
                        tictactoe.reset();
                    }
                    GameState::Playing => {}
                }
            } else {
                self.last_click_info = Some(format!("Cell ({}, {}) already occupied", row, col));
            }
        } else {
            self.last_click_info = Some(format!(
                "Click outside board: sim_pos=({:.1}, {:.1})",
                sim_x, sim_y
            ));
        }
    }

    /// Handles mouse clicks on the tic-tac-toe board (legacy winit event handler)
    #[allow(dead_code)]
    fn handle_mouse_click(&mut self) {
        // Get cursor position
        let cursor_pos = match self.cursor_position {
            Some(pos) => pos,
            None => {
                self.last_click_info = Some("No cursor position".to_string());
                return;
            }
        };

        let window = match &self.window {
            Some(w) => w,
            None => {
                self.last_click_info = Some("No window".to_string());
                return;
            }
        };

        // Check if we have the sim viewport rect
        let viewport_rect = match self.sim_viewport_rect {
            Some(rect) => rect,
            None => {
                self.last_click_info = Some("No viewport rect".to_string());
                return;
            }
        };

        // Convert window coordinates to egui coordinates
        let scale_factor = window.scale_factor() as f32;
        let egui_pos = egui::pos2(
            cursor_pos.x as f32 / scale_factor,
            cursor_pos.y as f32 / scale_factor,
        );

        // Check if click is within the sim viewport
        if !viewport_rect.contains(egui_pos) {
            self.last_click_info = Some(format!(
                "Click outside viewport: egui_pos=({:.1}, {:.1}), viewport={:?}",
                egui_pos.x, egui_pos.y, viewport_rect
            ));
            return;
        }

        // Convert to sim texture coordinates (relative to viewport)
        let sim_x = egui_pos.x - viewport_rect.left();
        let sim_y = egui_pos.y - viewport_rect.top();

        // Get renderer config to know sim texture size
        let renderer = match &self.renderer {
            Some(r) => r,
            None => {
                self.last_click_info = Some("No renderer".to_string());
                return;
            }
        };
        let config = renderer.config();

        // Create board layout matching the one used in rendering
        let layout = geometry::BoardLayout::centered(config.width as f32, config.height as f32);

        // Convert to board cell
        if let Some((row, col)) = layout.screen_to_cell(sim_x, sim_y) {
            // Try to make the move
            if let Some(tictactoe) = self.world.tictactoe_mut()
                && tictactoe.make_move(row, col)
            {
                self.last_click_info = Some(format!("Placed piece at ({}, {})", row, col));
                info!("Placed piece at ({}, {})", row, col);

                // Check if game is over and reset after a short delay
                use crate::sim::tictactoe::GameState;
                match tictactoe.game_state() {
                    GameState::Won(player) => {
                        info!("Player {:?} won!", player);
                        // Reset for next game
                        tictactoe.reset();
                    }
                    GameState::Draw => {
                        info!("Game is a draw!");
                        // Reset for next game
                        tictactoe.reset();
                    }
                    GameState::Playing => {}
                }
            } else {
                self.last_click_info = Some(format!(
                    "Cell ({}, {}) already occupied or invalid",
                    row, col
                ));
            }

            // Request a redraw to show the new state
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            self.last_click_info = Some(format!(
                "Click outside board: sim_pos=({:.1}, {:.1}), texture_size={}x{}",
                sim_x, sim_y, config.width, config.height
            ));
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = window_attributes_from_config(&self.config.window);

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let size = window.inner_size();
                    info!(
                        window.width = size.width,
                        window.height = size.height,
                        "Window created successfully"
                    );

                    let window = Arc::new(window);

                    // Initialize renderer using tokio runtime
                    // We create a runtime here because winit's event loop is synchronous
                    let renderer = tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime")
                        .block_on(async { Renderer::new(window.clone()).await });

                    match renderer {
                        Ok(renderer) => {
                            info!("Renderer initialized successfully");
                            self.renderer = Some(renderer);
                            self.window = Some(window);
                            self.last_update = Some(Instant::now());
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to initialize renderer");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to create window");
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Update simulation
        if let Some(last_update) = self.last_update {
            let now = Instant::now();
            let delta_time = (now - last_update).as_secs_f32();
            self.last_update = Some(now);

            self.world.tick(delta_time);

            // Request redraw after simulation update
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Let egui handle the event first
        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window)
            && renderer.handle_event(window, &event)
        {
            // Event was consumed by egui
            return;
        }

        // Handle debug hotkeys (debug builds only)
        #[cfg(debug_assertions)]
        if let WindowEvent::KeyboardInput { event, .. } = &event
            && event.state.is_pressed()
            && let PhysicalKey::Code(KeyCode::Backquote) = event.physical_key
        {
            self.toggle_debug_window();
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested, exiting");
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_mouse_click();
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                    let debug_ui = &mut self.debug_ui;
                    let world = &self.world;
                    let config = renderer.config().clone();
                    let sim_viewport_rect = &mut self.sim_viewport_rect;

                    // Local variable to capture clicks from egui closure
                    let mut clicked_pos: Option<(f32, f32)> = None;

                    match renderer.draw(window, world, |ctx, texture_id| {
                        // Editor Layout
                        egui::SidePanel::left("tools_panel")
                            .resizable(true)
                            .default_width(200.0)
                            .show(ctx, |ui| {
                                ui.heading("Tic-Tac-Toe");
                                ui.separator();
                                ui.label("Click the board to play!");
                            });

                        egui::CentralPanel::default().show(ctx, |ui| {
                            // Game View
                            // Display the simulation texture filling the available space
                            // We use ui.available_size() to fill the central panel
                            let size = ui.available_size();
                            let response =
                                ui.image(egui::load::SizedTexture::new(texture_id, size));
                            *sim_viewport_rect = Some(response.rect);

                            // Handle clicks on the game viewport using egui
                            if response.clicked()
                                && let Some(click_pos) = response.interact_pointer_pos()
                            {
                                // Convert egui position to sim texture coordinates
                                let sim_x = click_pos.x - response.rect.left();
                                let sim_y = click_pos.y - response.rect.top();

                                // Store click info for processing after render
                                clicked_pos = Some((sim_x, sim_y));
                            }
                        });

                        // Debug UI overlay
                        debug_ui.render(
                            ctx,
                            world,
                            &config,
                            self.cursor_position,
                            *sim_viewport_rect,
                            &self.last_click_info,
                        );
                    }) {
                        Ok(_) => {
                            // Process any click that occurred during rendering
                            if let Some((sim_x, sim_y)) = clicked_pos {
                                self.process_click(sim_x, sim_y, &config);
                                self.pending_click = Some((sim_x, sim_y));
                            }
                        }
                        Err(wgpu::SurfaceError::Lost) => {
                            warn!("Surface lost, reconfiguring");
                            let size = window.inner_size();
                            renderer.resize(size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            error!("Out of memory, exiting");
                            event_loop.exit();
                        }
                        Err(e) => {
                            error!(error = %e, "Render error");
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
