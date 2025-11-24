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

use super::config::AppConfig;
use super::debug_ui::DebugUIState;
use super::renderer::Renderer;
use super::window::window_attributes_from_config;
use crate::sim::World;

/// Main game application
pub struct App {
    config: AppConfig,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    world: World,
    debug_ui: DebugUIState,
    last_update: Option<Instant>,
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
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                    let debug_ui = &mut self.debug_ui;
                    let world = &self.world;
                    let config = renderer.config().clone();
                    match renderer.draw(window, |ctx| {
                        debug_ui.render(ctx, world, &config);
                        // Future: Game UI will be rendered here
                    }) {
                        Ok(_) => {}
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
