//! Input handling system
//!
//! Provides a clean, priority-based input routing system that:
//! - Collects raw input from winit events
//! - Generates semantic input events (clicks, drags, hovers, etc.)
//! - Routes events to handlers in priority order
//! - Supports event consumption to prevent input conflicts
//! - Handles multiple viewports with automatic hit testing
//!
//! # Architecture
//!
//! ```text
//! Raw Input (winit) → InputCollector → InputState
//!                                          ↓
//!                                    InputContext
//!                                    (generates events)
//!                                          ↓
//!                                   InputHandlers
//!                                   (by priority)
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // In App::new()
//! let mut input_context = InputContext::new();
//! input_context.register_handler(Box::new(MyHandler::new()));
//!
//! // In window_event()
//! collector.handle_window_event(&event);
//!
//! // Each frame, before simulation update
//! collector.advance_frame();
//! let state = collector.take_state();
//! input_context.update_state(state);
//! input_context.process();
//! ```

mod collector;
mod context;
mod events;
mod game_handler;
mod handler;
mod state;

// Re-export public API
pub use collector::InputCollector;
pub use context::{InputContext, Rect};
pub use events::{InputEvent, KeyCode, MouseButton, ViewportId};
pub use game_handler::{GameAction, GameInputHandler};
pub use handler::InputHandler;
pub use state::{ButtonState, InputState, KeyboardState, Modifiers, MouseButtons, MouseState};
