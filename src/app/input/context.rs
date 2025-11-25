//! Input event routing and distribution

use std::collections::HashMap;

use super::events::{InputEvent, MouseButton, ViewportId};
use super::handler::InputHandler;
use super::state::InputState;

/// Rectangular area for viewport hit testing
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is inside this rectangle
    pub fn contains(&self, pos: [f32; 2]) -> bool {
        pos[0] >= self.x
            && pos[0] <= self.x + self.width
            && pos[1] >= self.y
            && pos[1] <= self.y + self.height
    }

    /// Get the center point of the rectangle
    pub fn center(&self) -> [f32; 2] {
        [self.x + self.width / 2.0, self.y + self.height / 2.0]
    }
}

/// Information about a registered viewport
#[derive(Debug, Clone)]
struct ViewportInfo {
    rect: Rect,
    name: String,
}

/// Tracks drag state for each button
#[derive(Debug, Clone)]
struct DragState {
    active: bool,
    start_pos: [f32; 2],
    last_pos: [f32; 2],
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            active: false,
            start_pos: [0.0, 0.0],
            last_pos: [0.0, 0.0],
        }
    }
}

/// Central input routing and distribution system
pub struct InputContext {
    /// Registered input handlers, sorted by priority (highest first)
    handlers: Vec<Box<dyn InputHandler>>,
    /// Current input state
    state: InputState,
    /// Previous frame's input state
    prev_state: InputState,
    /// Registered viewports for hit testing
    viewports: HashMap<ViewportId, ViewportInfo>,
    /// Drag state tracking per button
    drag_states: HashMap<MouseButton, DragState>,
    /// Debug: Events generated last frame
    last_events: Vec<String>,
}

impl InputContext {
    /// Creates a new input context
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            state: InputState::new(),
            prev_state: InputState::new(),
            viewports: HashMap::new(),
            drag_states: HashMap::new(),
            last_events: Vec::new(),
        }
    }

    /// Register an input handler
    ///
    /// Handlers are automatically sorted by priority (highest first).
    pub fn register_handler(&mut self, handler: Box<dyn InputHandler>) {
        self.handlers.push(handler);
        self.handlers
            .sort_by_key(|h| std::cmp::Reverse(h.priority()));
    }

    /// Update input state from collector
    pub fn update_state(&mut self, state: InputState) {
        self.prev_state = std::mem::replace(&mut self.state, state);
    }

    /// Register a viewport for hit testing
    ///
    /// This should be called during rendering after layout is determined.
    /// Viewports are cleared each frame.
    pub fn register_viewport(&mut self, id: ViewportId, rect: Rect, name: impl Into<String>) {
        self.viewports.insert(
            id,
            ViewportInfo {
                rect,
                name: name.into(),
            },
        );
    }

    /// Clear all registered viewports
    ///
    /// Should be called at the start of each frame before rendering.
    pub fn clear_viewports(&mut self) {
        self.viewports.clear();
    }

    /// Process input and dispatch events to handlers
    ///
    /// This is the main entry point for input processing.
    /// Call this once per frame after updating state.
    pub fn process(&mut self) {
        // Clear last frame's events
        self.last_events.clear();

        // Notify handlers of frame start
        for handler in &mut self.handlers {
            handler.begin_frame();
        }

        // Generate events from state changes
        let events = self.generate_events();

        // Log events for debugging
        for event in &events {
            self.last_events.push(format!("{:?}", event));
        }

        // Dispatch events to handlers in priority order
        for event in events {
            for handler in &mut self.handlers {
                if handler.handle_event(&event, &self.state) {
                    // Event consumed, stop propagation
                    break;
                }
            }
        }

        // Update all handlers with current state
        for handler in &mut self.handlers {
            handler.update(&self.state);
        }
    }

    /// Generate semantic events from state changes
    fn generate_events(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();

        if let Some(pos) = self.state.mouse.screen_pos {
            // Click events (button just pressed)
            if self.state.mouse.buttons.left.is_just_pressed() {
                let viewport = self.find_viewport_at(pos);
                events.push(InputEvent::Click {
                    button: MouseButton::Left,
                    pos,
                    viewport,
                });

                self.drag_states.insert(
                    MouseButton::Left,
                    DragState {
                        active: true,
                        start_pos: pos,
                        last_pos: pos,
                    },
                );
            }

            if self.state.mouse.buttons.right.is_just_pressed() {
                let viewport = self.find_viewport_at(pos);
                events.push(InputEvent::Click {
                    button: MouseButton::Right,
                    pos,
                    viewport,
                });

                self.drag_states.insert(
                    MouseButton::Right,
                    DragState {
                        active: true,
                        start_pos: pos,
                        last_pos: pos,
                    },
                );
            }

            if self.state.mouse.buttons.middle.is_just_pressed() {
                let viewport = self.find_viewport_at(pos);
                events.push(InputEvent::Click {
                    button: MouseButton::Middle,
                    pos,
                    viewport,
                });

                self.drag_states.insert(
                    MouseButton::Middle,
                    DragState {
                        active: true,
                        start_pos: pos,
                        last_pos: pos,
                    },
                );
            }

            // Drag events (button held and mouse moved)
            for (button, drag_state) in &mut self.drag_states {
                if drag_state.active {
                    let button_down = match button {
                        MouseButton::Left => self.state.mouse.buttons.left.is_down(),
                        MouseButton::Right => self.state.mouse.buttons.right.is_down(),
                        MouseButton::Middle => self.state.mouse.buttons.middle.is_down(),
                    };

                    if button_down {
                        // Check if mouse moved
                        if let Some(prev_pos) = self.prev_state.mouse.screen_pos
                            && pos != prev_pos
                        {
                            let delta = [pos[0] - prev_pos[0], pos[1] - prev_pos[1]];

                            events.push(InputEvent::Drag {
                                button: *button,
                                start: drag_state.start_pos,
                                current: pos,
                                delta,
                            });

                            drag_state.last_pos = pos;
                        }
                    } else {
                        // Button released, stop drag
                        drag_state.active = false;
                    }
                }
            }

            // Hover events (mouse position changed)
            if let Some(prev_pos) = self.prev_state.mouse.screen_pos {
                if pos != prev_pos {
                    let viewport = self.find_viewport_at(pos);
                    events.push(InputEvent::Hover { pos, viewport });
                }
            } else {
                // First hover (mouse entered window)
                let viewport = self.find_viewport_at(pos);
                events.push(InputEvent::Hover { pos, viewport });
            }

            // Scroll events
            let scroll_delta = self.state.mouse.scroll_delta;
            if scroll_delta != [0.0, 0.0] {
                events.push(InputEvent::Scroll {
                    delta: scroll_delta,
                    pos,
                });
            }
        }

        events
    }

    /// Find which viewport contains the given position
    fn find_viewport_at(&self, pos: [f32; 2]) -> Option<ViewportId> {
        // Check all viewports, return first match
        // In future, could handle overlapping viewports by priority/z-order
        for (id, info) in &self.viewports {
            if info.rect.contains(pos) {
                return Some(*id);
            }
        }
        None
    }

    /// Get the rectangle of a viewport
    pub fn viewport_rect(&self, id: ViewportId) -> Option<Rect> {
        self.viewports.get(&id).map(|info| info.rect)
    }

    /// Get current input state
    pub fn state(&self) -> &InputState {
        &self.state
    }

    /// Get number of registered handlers
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }

    /// Get number of registered viewports
    pub fn viewport_count(&self) -> usize {
        self.viewports.len()
    }

    /// Get a mutable reference to a handler by name
    ///
    /// This allows access to handler-specific state or methods.
    pub fn get_handler_mut(&mut self, name: &str) -> Option<&mut (dyn InputHandler + '_)> {
        if let Some(boxed) = self.handlers.iter_mut().find(|h| h.name() == name) {
            Some(boxed.as_mut())
        } else {
            None
        }
    }

    /// Get a reference to a handler by name
    pub fn get_handler(&self, name: &str) -> Option<&dyn InputHandler> {
        self.handlers
            .iter()
            .find(|h| h.name() == name)
            .map(|b| b.as_ref())
    }

    /// Get debug information about all viewports
    pub fn debug_viewports(&self) -> Vec<(ViewportId, Rect, String)> {
        self.viewports
            .iter()
            .map(|(id, info)| (*id, info.rect, info.name.clone()))
            .collect()
    }

    /// Get debug information about all handlers
    pub fn debug_handlers(&self) -> Vec<(String, u32)> {
        self.handlers
            .iter()
            .map(|h| (h.name().to_string(), h.priority()))
            .collect()
    }

    /// Get events generated last frame (for debugging)
    pub fn debug_last_events(&self) -> &[String] {
        &self.last_events
    }
}

impl Default for InputContext {
    fn default() -> Self {
        Self::new()
    }
}
