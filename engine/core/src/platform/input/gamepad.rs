//! Gamepad input types.

/// Gamepad identifier.
///
/// Gamepads are identified by an index (0-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GamepadId(pub u8);

impl GamepadId {
    /// Create a new gamepad ID.
    pub fn new(id: u8) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    pub fn id(&self) -> u8 {
        self.0
    }
}

/// Platform-agnostic gamepad button identifiers.
///
/// Based on the standard gamepad layout (similar to Xbox controller).
#[allow(missing_docs)] // Button names are self-explanatory with comments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadButton {
    // Face buttons (right side)
    South, // A (Xbox), Cross (PlayStation)
    East,  // B (Xbox), Circle (PlayStation)
    West,  // X (Xbox), Square (PlayStation)
    North, // Y (Xbox), Triangle (PlayStation)

    // D-pad
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,

    // Shoulder buttons
    LeftShoulder,  // LB / L1
    RightShoulder, // RB / R1
    LeftTrigger,   // LT / L2
    RightTrigger,  // RT / R2

    // Stick buttons
    LeftStick,  // L3
    RightStick, // R3

    // Center buttons
    Select, // Back / Share
    Start,  // Start / Options
    Guide,  // Xbox / PS button

    // Unknown or other buttons
    Unknown,
}

/// Gamepad axis identifiers.
///
/// Axis values range from -1.0 to 1.0, with 0.0 being center.
#[allow(missing_docs)] // Axis names have inline documentation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GamepadAxis {
    /// Left stick horizontal axis (left = -1.0, right = 1.0)
    LeftStickX,
    /// Left stick vertical axis (up = -1.0, down = 1.0)
    LeftStickY,

    /// Right stick horizontal axis (left = -1.0, right = 1.0)
    RightStickX,
    /// Right stick vertical axis (up = -1.0, down = 1.0)
    RightStickY,

    /// Left trigger pressure (not pressed = 0.0, fully pressed = 1.0)
    LeftTriggerPressure,
    /// Right trigger pressure (not pressed = 0.0, fully pressed = 1.0)
    RightTriggerPressure,

    /// Unknown or other axes
    Unknown,
}

impl GamepadAxis {
    /// Returns true if this is a stick axis.
    pub fn is_stick(&self) -> bool {
        matches!(
            self,
            GamepadAxis::LeftStickX
                | GamepadAxis::LeftStickY
                | GamepadAxis::RightStickX
                | GamepadAxis::RightStickY
        )
    }

    /// Returns true if this is a trigger axis.
    pub fn is_trigger(&self) -> bool {
        matches!(self, GamepadAxis::LeftTriggerPressure | GamepadAxis::RightTriggerPressure)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gamepad_id() {
        let id = GamepadId::new(0);
        assert_eq!(id.id(), 0);

        let id2 = GamepadId::new(3);
        assert_eq!(id2.id(), 3);
    }

    #[test]
    fn test_stick_axes() {
        assert!(GamepadAxis::LeftStickX.is_stick());
        assert!(GamepadAxis::RightStickY.is_stick());
        assert!(!GamepadAxis::LeftTriggerPressure.is_stick());
    }

    #[test]
    fn test_trigger_axes() {
        assert!(GamepadAxis::LeftTriggerPressure.is_trigger());
        assert!(GamepadAxis::RightTriggerPressure.is_trigger());
        assert!(!GamepadAxis::LeftStickX.is_trigger());
    }
}
