//! Keyboard input types.

/// Platform-agnostic keyboard key codes.
///
/// This enum represents all common keyboard keys in a platform-independent way.
/// Platform-specific implementations should map their native key codes to these.
#[allow(missing_docs)] // Individual key variants are self-explanatory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
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

    // Arrow keys
    Left,
    Right,
    Up,
    Down,

    // Special keys
    Space,
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,

    // Modifiers
    LeftShift,
    RightShift,
    LeftControl,
    RightControl,
    LeftAlt,
    RightAlt,
    LeftSuper, // Windows/Command key
    RightSuper,

    // Lock keys
    CapsLock,
    NumLock,
    ScrollLock,

    // Numpad
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSubtract,
    NumpadMultiply,
    NumpadDivide,
    NumpadDecimal,
    NumpadEnter,

    // Special characters
    Semicolon,
    Comma,
    Period,
    Slash,
    Backslash,
    LeftBracket,
    RightBracket,
    Quote,
    Grave,
    Minus,
    Equals,

    // Media keys
    /// Mute audio
    Mute,
    /// Increase volume
    VolumeUp,
    /// Decrease volume
    VolumeDown,
    /// Play/Pause media
    PlayPause,
    /// Stop media playback
    Stop,
    /// Previous track
    PreviousTrack,
    /// Next track
    NextTrack,

    // Other
    /// Print Screen key
    PrintScreen,
    /// Pause/Break key
    Pause,
    /// Menu/Application key
    Menu,

    /// Unknown or unsupported key
    Unknown,
}

impl KeyCode {
    /// Returns true if this is a modifier key (Shift, Ctrl, Alt, Super).
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            KeyCode::LeftShift
                | KeyCode::RightShift
                | KeyCode::LeftControl
                | KeyCode::RightControl
                | KeyCode::LeftAlt
                | KeyCode::RightAlt
                | KeyCode::LeftSuper
                | KeyCode::RightSuper
        )
    }

    /// Returns true if this is a function key (F1-F12).
    pub fn is_function_key(&self) -> bool {
        matches!(
            self,
            KeyCode::F1
                | KeyCode::F2
                | KeyCode::F3
                | KeyCode::F4
                | KeyCode::F5
                | KeyCode::F6
                | KeyCode::F7
                | KeyCode::F8
                | KeyCode::F9
                | KeyCode::F10
                | KeyCode::F11
                | KeyCode::F12
        )
    }

    /// Returns true if this is a number key (0-9, not numpad).
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            KeyCode::Num0
                | KeyCode::Num1
                | KeyCode::Num2
                | KeyCode::Num3
                | KeyCode::Num4
                | KeyCode::Num5
                | KeyCode::Num6
                | KeyCode::Num7
                | KeyCode::Num8
                | KeyCode::Num9
        )
    }

    /// Returns true if this is a letter key (A-Z).
    pub fn is_letter(&self) -> bool {
        matches!(
            self,
            KeyCode::A
                | KeyCode::B
                | KeyCode::C
                | KeyCode::D
                | KeyCode::E
                | KeyCode::F
                | KeyCode::G
                | KeyCode::H
                | KeyCode::I
                | KeyCode::J
                | KeyCode::K
                | KeyCode::L
                | KeyCode::M
                | KeyCode::N
                | KeyCode::O
                | KeyCode::P
                | KeyCode::Q
                | KeyCode::R
                | KeyCode::S
                | KeyCode::T
                | KeyCode::U
                | KeyCode::V
                | KeyCode::W
                | KeyCode::X
                | KeyCode::Y
                | KeyCode::Z
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_modifiers() {
        assert!(KeyCode::LeftShift.is_modifier());
        assert!(KeyCode::RightControl.is_modifier());
        assert!(!KeyCode::A.is_modifier());
    }

    #[test]
    fn test_function_keys() {
        assert!(KeyCode::F1.is_function_key());
        assert!(KeyCode::F12.is_function_key());
        assert!(!KeyCode::A.is_function_key());
    }

    #[test]
    fn test_number_keys() {
        assert!(KeyCode::Num0.is_number());
        assert!(KeyCode::Num9.is_number());
        assert!(!KeyCode::Numpad0.is_number());
        assert!(!KeyCode::A.is_number());
    }

    #[test]
    fn test_letter_keys() {
        assert!(KeyCode::A.is_letter());
        assert!(KeyCode::Z.is_letter());
        assert!(!KeyCode::Num0.is_letter());
    }
}
