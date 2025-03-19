use gilrs::Button;
use slint::platform::Key;

pub fn btn_to_key(value: Button) -> Option<Key> {
    match value {
        Button::South => Some(Key::Return),
        Button::East => Some(Key::Escape),
        Button::LeftTrigger => Some(Key::PageUp),
        Button::LeftTrigger2 => Some(Key::Home),
        Button::RightTrigger => Some(Key::PageDown),
        Button::RightTrigger2 => Some(Key::End),
        Button::DPadUp => Some(Key::UpArrow),
        Button::DPadDown => Some(Key::DownArrow),
        Button::DPadLeft => Some(Key::LeftArrow),
        Button::DPadRight => Some(Key::RightArrow),
        _ => None,
    }
}
