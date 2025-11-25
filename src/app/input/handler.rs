//! Input handler trait for subsystems

use super::events::InputEvent;
use super::state::InputState;

/// Trait for subsystems that handle input
///
/// Handlers are called in priority order (highest first).
/// When a handler consumes an event (returns true), propagation stops.
pub trait InputHandler {
    /// Name of this handler for debugging
    fn name(&self) -> &str;

    /// Priority for input routing (higher = earlier)
    ///
    /// Priority ranges:
    /// - 200+: Critical system handlers (debug overlays)
    /// - 100-199: UI handlers (editor panels, menus)
    /// - 50-99: Game/simulation handlers
    /// - 0-49: Global/fallback handlers (hotkeys, camera)
    fn priority(&self) -> u32;

    /// Handle an input event
    ///
    /// # Arguments
    /// * `event` - The semantic input event to handle
    /// * `state` - Current raw input state for additional context
    ///
    /// # Returns
    /// * `true` if the event was consumed (stops propagation to lower priority handlers)
    /// * `false` if the event was not handled (continues to next handler)
    fn handle_event(&mut self, event: &InputEvent, state: &InputState) -> bool;

    /// Called every frame with current input state
    ///
    /// Useful for continuous input like hover states or held buttons.
    /// This is called after all events are processed.
    fn update(&mut self, _state: &InputState) {
        // Default: no-op
    }

    /// Called at the start of each frame, before events are generated
    ///
    /// Useful for resetting per-frame state
    fn begin_frame(&mut self) {
        // Default: no-op
    }

    /// Downcast to concrete type for accessing handler-specific methods
    ///
    /// This enables type-safe access to handler-specific functionality.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
