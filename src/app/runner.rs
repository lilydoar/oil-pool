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

use super::debug_ui::{DebugUIState, MouseDebugInfo};
use super::input::{
    GameAction, GameInputHandler, InputCollector, InputContext, MouseButton as InputMouseButton,
    Rect, ViewportId,
};
use super::renderer::Renderer;
use super::window::window_attributes_from_config;
use super::{config::AppConfig, geometry};
use crate::sim::World;

/// Main game application
pub struct App {
    config: AppConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    world: World,
    debug_ui: DebugUIState,
    last_update: Option<Instant>,
    // Input system
    input_collector: InputCollector,
    input_context: InputContext,
    // Debug info
    last_click_info: Option<String>,
}

impl App {
    /// Creates a new game application with the provided configuration
    pub fn new(config: AppConfig) -> Self {
        info!(profile = %config.profile, "Starting game");
        info!(?config.window, "Window configuration");

        // Initialize input system
        let mut input_context = InputContext::new();

        // Register game input handler
        let game_handler = Box::new(GameInputHandler::new(ViewportId(0)));
        input_context.register_handler(game_handler);

        Self {
            config,
            window: None,
            renderer: None,
            world: World::new(),
            debug_ui: DebugUIState::default(),
            last_update: None,
            input_collector: InputCollector::new(),
            input_context,
            last_click_info: None,
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

    /// Process game input actions and apply them to simulations
    fn process_game_actions(&mut self) {
        // Get renderer config for coordinate conversion
        let config = match &self.renderer {
            Some(r) => r.config().clone(),
            None => return,
        };

        // Get viewport rect first (immutable borrow)
        let viewport_rect = self.input_context.viewport_rect(ViewportId(0));

        // Then get handler and take its actions (mutable borrow)
        let actions = if let Some(handler) = self.input_context.get_handler_mut("game") {
            // Downcast to GameInputHandler to access take_actions()
            if let Some(game_handler) = handler.as_any_mut().downcast_mut::<GameInputHandler>() {
                // Update viewport rect for coordinate conversion
                if let Some(rect) = viewport_rect {
                    game_handler.set_viewport_rect(rect);
                }
                game_handler.take_actions()
            } else {
                Vec::new()
            }
        } else {
            return;
        };

        for action in actions {
            if let GameAction::ViewportClick {
                local_pos,
                button: InputMouseButton::Left,
                ..
            } = action
            {
                self.process_viewport_click(local_pos, &config, viewport_rect);
            }
        }
    }

    /// Process a click in the game viewport
    fn process_viewport_click(
        &mut self,
        local_pos: [f32; 2],
        config: &wgpu::SurfaceConfiguration,
        viewport_rect: Option<Rect>,
    ) {
        // Create board layout matching the one used in rendering
        let layout = geometry::BoardLayout::centered(config.width as f32, config.height as f32);

        // Scale viewport coordinates to texture coordinates
        // The texture is rendered at config dimensions but displayed scaled in viewport
        let texture_pos = if let Some(viewport) = viewport_rect {
            let scale_x = config.width as f32 / viewport.width;
            let scale_y = config.height as f32 / viewport.height;
            [local_pos[0] * scale_x, local_pos[1] * scale_y]
        } else {
            local_pos // Fallback if no viewport info
        };

        // Convert texture coordinates to board cell
        if let Some((row, col)) = layout.screen_to_cell(texture_pos[0], texture_pos[1]) {
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
                "Click outside board: viewport_pos=({:.1}, {:.1})",
                local_pos[0], local_pos[1]
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

            // Process input BEFORE simulation update
            // Clone state BEFORE advancing so we can detect edge transitions (JustPressed)
            let input_state = self.input_collector.clone_state();
            self.input_context.update_state(input_state);
            self.input_context.process();

            // Advance frame AFTER processing to transition edges to steady states
            self.input_collector.advance_frame();

            // Apply game actions to simulations
            self.process_game_actions();

            // Update simulation
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
        // Feed events to input collector FIRST (before egui)
        // This ensures we see all raw input
        self.input_collector.handle_window_event(&event);

        // Update scale factor if needed
        if let Some(window) = &self.window {
            let scale_factor = window.scale_factor() as f32;
            self.input_collector.set_scale_factor(scale_factor);
        }

        // Let egui handle the event for UI interactions
        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
            let _ = renderer.handle_event(window, &event);
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
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                    // Initialize leaf vines if needed (mutable borrow of world)
                    renderer.init_leaf_vines(&mut self.world);

                    let debug_ui = &mut self.debug_ui;
                    let world = &self.world;
                    let config = renderer.config().clone();
                    let input_context = &mut self.input_context;
                    let last_click_info = &self.last_click_info;

                    // Viewports are re-registered each frame (no need to clear)
                    // We need them to persist between frames for input processing

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
                            let size = ui.available_size();
                            let response =
                                ui.image(egui::load::SizedTexture::new(texture_id, size));

                            // Register viewport for input hit testing
                            let rect = response.rect;
                            input_context.register_viewport(
                                ViewportId(0),
                                Rect::new(rect.left(), rect.top(), rect.width(), rect.height()),
                                "sim_viewport",
                            );
                        });

                        // Debug UI overlay
                        let input_state = input_context.state();
                        let cursor_pos = input_state.mouse.screen_pos.map(|pos| {
                            winit::dpi::PhysicalPosition::new(pos[0] as f64, pos[1] as f64)
                        });

                        debug_ui.render(
                            ctx,
                            world,
                            &config,
                            MouseDebugInfo {
                                cursor_pos,
                                viewport_rect: input_context.viewport_rect(ViewportId(0)).map(
                                    |r| {
                                        egui::Rect::from_min_size(
                                            egui::pos2(r.x, r.y),
                                            egui::vec2(r.width, r.height),
                                        )
                                    },
                                ),
                                last_click_info,
                            },
                            input_context,
                        );
                    }) {
                        Ok(_) => {
                            // Rendering successful
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
