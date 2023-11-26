#[derive(Debug)]
pub enum JoypadButton {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Select,
    Start,
}

#[derive(Debug)]
pub struct JoypadEvent {
    pub is_down: bool,
    pub button: JoypadButton,
}

impl JoypadEvent {
    pub fn new_up(button: JoypadButton) -> Self {
        Self {
            is_down: false,
            button,
        }
    }

    pub fn new_down(button: JoypadButton) -> Self {
        Self {
            is_down: true,
            button,
        }
    }
}
