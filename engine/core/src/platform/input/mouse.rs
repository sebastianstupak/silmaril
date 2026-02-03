//! Mouse input types.

/// Platform-agnostic mouse button identifiers.
#[allow(missing_docs)] // Button names are self-explanatory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button (primary button)
    Left,
    /// Right mouse button (secondary button)
    Right,
    /// Middle mouse button (scroll wheel button)
    Middle,
    /// Additional mouse button 1 (back)
    Button4,
    /// Additional mouse button 2 (forward)
    Button5,
    /// Other mouse buttons
    Other(u8),
}

impl MouseButton {
    /// Returns true if this is a primary button (left, right, or middle).
    pub fn is_primary(&self) -> bool {
        matches!(self, MouseButton::Left | MouseButton::Right | MouseButton::Middle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_buttons() {
        assert!(MouseButton::Left.is_primary());
        assert!(MouseButton::Right.is_primary());
        assert!(MouseButton::Middle.is_primary());
        assert!(!MouseButton::Button4.is_primary());
        assert!(!MouseButton::Button5.is_primary());
    }

    #[test]
    fn test_other_buttons() {
        let button = MouseButton::Other(6);
        assert!(!button.is_primary());
    }
}
