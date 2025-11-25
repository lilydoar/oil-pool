//! Raw input collection from winit events

use super::state::{ButtonState, InputState, Modifiers};
use winit::event::{ElementState, WindowEvent};

/// Collects raw input from winit events and maintains InputState
pub struct InputCollector {
    state: InputState,
    scale_factor: f32,
}

impl InputCollector {
    /// Creates a new input collector
    pub fn new() -> Self {
        Self {
            state: InputState::new(),
            scale_factor: 1.0,
        }
    }

    /// Update scale factor (DPI scaling)
    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        self.scale_factor = scale_factor;
    }

    /// Handle a winit window event
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let window_pos = [position.x as f32, position.y as f32];
                let screen_pos = [
                    position.x as f32 / self.scale_factor,
                    position.y as f32 / self.scale_factor,
                ];

                self.state.mouse.window_pos = Some(window_pos);
                self.state.mouse.screen_pos = Some(screen_pos);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let button_state = match state {
                    ElementState::Pressed => ButtonState::JustPressed,
                    ElementState::Released => ButtonState::JustReleased,
                };

                match button {
                    winit::event::MouseButton::Left => {
                        self.state.mouse.buttons.left = button_state;
                    }
                    winit::event::MouseButton::Right => {
                        self.state.mouse.buttons.right = button_state;
                    }
                    winit::event::MouseButton::Middle => {
                        self.state.mouse.buttons.middle = button_state;
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                // Convert MouseScrollDelta to consistent pixel units
                let pixel_delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        // Line delta: convert to pixels (approximate)
                        [*x * 20.0, *y * 20.0]
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => [pos.x as f32, pos.y as f32],
                };

                self.state.mouse.scroll_delta = pixel_delta;
            }

            WindowEvent::ModifiersChanged(modifiers_state) => {
                self.state.keyboard.modifiers = Modifiers {
                    shift: modifiers_state.state().shift_key(),
                    ctrl: modifiers_state.state().control_key(),
                    alt: modifiers_state.state().alt_key(),
                    meta: modifiers_state.state().super_key(),
                };
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // For now, just update modifiers
                // Full key tracking can be added later when needed
                #[allow(clippy::single_match)]
                match event.state {
                    ElementState::Pressed => {
                        // Could track key presses here
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    }

    /// Advance to next frame (transitions edge states to steady states)
    pub fn advance_frame(&mut self) {
        self.state.advance_frame();
    }

    /// Get current input state
    pub fn state(&self) -> &InputState {
        &self.state
    }

    /// Clone current state for processing
    ///
    /// Note: We clone instead of take to preserve continuous state like mouse position
    pub fn clone_state(&self) -> InputState {
        self.state.clone()
    }

    /// Take ownership of current state (useful for moving into context)
    #[allow(dead_code)]
    pub fn take_state(&mut self) -> InputState {
        std::mem::take(&mut self.state)
    }

    /// Borrow state mutably
    pub fn state_mut(&mut self) -> &mut InputState {
        &mut self.state
    }
}

impl Default for InputCollector {
    fn default() -> Self {
        Self::new()
    }
}
