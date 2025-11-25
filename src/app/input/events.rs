//! Semantic input events

/// Semantic input events generated from raw state changes
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse click at screen position
    Click {
        button: MouseButton,
        /// Screen position in logical pixels
        pos: [f32; 2],
        /// Which viewport was clicked (if any)
        viewport: Option<ViewportId>,
    },

    /// Mouse drag operation
    Drag {
        button: MouseButton,
        /// Starting position when button was pressed
        start: [f32; 2],
        /// Current position
        current: [f32; 2],
        /// Delta since last frame
        delta: [f32; 2],
    },

    /// Mouse hover over position
    Hover {
        /// Current hover position
        pos: [f32; 2],
        /// Viewport being hovered (if any)
        viewport: Option<ViewportId>,
    },

    /// Mouse scroll
    Scroll {
        /// Scroll delta (x, y)
        delta: [f32; 2],
        /// Position where scroll occurred
        pos: [f32; 2],
    },

    /// Key press event
    KeyPress {
        key: KeyCode,
        modifiers: super::state::Modifiers,
    },

    /// Key release event
    KeyRelease { key: KeyCode },
}

/// Mouse button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Viewport identifier for hit testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewportId(pub u32);

/// Key code (simplified for now - can expand later)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Common keys
    Space,
    Enter,
    Escape,
    Backspace,
    Tab,

    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Arrows
    Left,
    Right,
    Up,
    Down,

    // Other
    Other,
}

/// Convert from winit key code
impl From<winit::keyboard::KeyCode> for KeyCode {
    fn from(key: winit::keyboard::KeyCode) -> Self {
        use winit::keyboard::KeyCode as WK;
        match key {
            WK::Space => Self::Space,
            WK::Enter => Self::Enter,
            WK::Escape => Self::Escape,
            WK::Backspace => Self::Backspace,
            WK::Tab => Self::Tab,

            WK::KeyA => Self::A,
            WK::KeyB => Self::B,
            WK::KeyC => Self::C,
            WK::KeyD => Self::D,
            WK::KeyE => Self::E,
            WK::KeyF => Self::F,
            WK::KeyG => Self::G,
            WK::KeyH => Self::H,
            WK::KeyI => Self::I,
            WK::KeyJ => Self::J,
            WK::KeyK => Self::K,
            WK::KeyL => Self::L,
            WK::KeyM => Self::M,
            WK::KeyN => Self::N,
            WK::KeyO => Self::O,
            WK::KeyP => Self::P,
            WK::KeyQ => Self::Q,
            WK::KeyR => Self::R,
            WK::KeyS => Self::S,
            WK::KeyT => Self::T,
            WK::KeyU => Self::U,
            WK::KeyV => Self::V,
            WK::KeyW => Self::W,
            WK::KeyX => Self::X,
            WK::KeyY => Self::Y,
            WK::KeyZ => Self::Z,

            WK::Digit0 => Self::Num0,
            WK::Digit1 => Self::Num1,
            WK::Digit2 => Self::Num2,
            WK::Digit3 => Self::Num3,
            WK::Digit4 => Self::Num4,
            WK::Digit5 => Self::Num5,
            WK::Digit6 => Self::Num6,
            WK::Digit7 => Self::Num7,
            WK::Digit8 => Self::Num8,
            WK::Digit9 => Self::Num9,

            WK::F1 => Self::F1,
            WK::F2 => Self::F2,
            WK::F3 => Self::F3,
            WK::F4 => Self::F4,
            WK::F5 => Self::F5,
            WK::F6 => Self::F6,
            WK::F7 => Self::F7,
            WK::F8 => Self::F8,
            WK::F9 => Self::F9,
            WK::F10 => Self::F10,
            WK::F11 => Self::F11,
            WK::F12 => Self::F12,

            WK::ArrowLeft => Self::Left,
            WK::ArrowRight => Self::Right,
            WK::ArrowUp => Self::Up,
            WK::ArrowDown => Self::Down,

            _ => Self::Other,
        }
    }
}
