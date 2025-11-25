//! Raw input state

/// Raw input state snapshot for a single frame
#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub mouse: MouseState,
    pub keyboard: KeyboardState,
    pub time: f64,
}

/// Mouse input state
#[derive(Debug, Clone, Default)]
pub struct MouseState {
    /// Window coordinates (physical pixels)
    pub window_pos: Option<[f32; 2]>,
    /// DPI-scaled logical coordinates (screen space)
    pub screen_pos: Option<[f32; 2]>,
    /// Mouse button states
    pub buttons: MouseButtons,
    /// Scroll delta this frame
    pub scroll_delta: [f32; 2],
}

/// State of all mouse buttons
#[derive(Debug, Clone, Default)]
pub struct MouseButtons {
    pub left: ButtonState,
    pub right: ButtonState,
    pub middle: ButtonState,
}

/// Button press state with edge detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonState {
    #[default]
    Released,
    /// Pressed this frame (edge)
    JustPressed,
    /// Held down (multiple frames)
    Pressed,
    /// Released this frame (edge)
    JustReleased,
}

impl ButtonState {
    /// Advance state for next frame (transitions edges to steady states)
    pub fn advance(self) -> Self {
        match self {
            Self::JustPressed => Self::Pressed,
            Self::JustReleased => Self::Released,
            state => state,
        }
    }

    /// Returns true if button is currently down (just pressed or held)
    pub fn is_down(self) -> bool {
        matches!(self, Self::JustPressed | Self::Pressed)
    }

    /// Returns true if button was just pressed this frame
    pub fn is_just_pressed(self) -> bool {
        matches!(self, Self::JustPressed)
    }

    /// Returns true if button was just released this frame
    pub fn is_just_released(self) -> bool {
        matches!(self, Self::JustReleased)
    }
}

/// Keyboard input state
#[derive(Debug, Clone, Default)]
pub struct KeyboardState {
    // For now, keep simple - can expand with key tracking later
    pub modifiers: Modifiers,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl InputState {
    /// Creates a new empty input state
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance all button states for next frame
    pub fn advance_frame(&mut self) {
        self.mouse.buttons.left = self.mouse.buttons.left.advance();
        self.mouse.buttons.right = self.mouse.buttons.right.advance();
        self.mouse.buttons.middle = self.mouse.buttons.middle.advance();

        // Clear per-frame state
        self.mouse.scroll_delta = [0.0, 0.0];
    }
}
