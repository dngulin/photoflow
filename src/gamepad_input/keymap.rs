use gilrs::Button;
use slint::platform::Key;
use slint::SharedString;
use std::collections::HashMap;

#[derive(Clone)]
pub struct KeyMap {
    pub act_up: SharedString,
    pub act_right: SharedString,
    pub act_down: SharedString,
    pub act_left: SharedString,

    pub dpad_up: SharedString,
    pub dpad_right: SharedString,
    pub dpad_down: SharedString,
    pub dpad_left: SharedString,

    pub menu_left: SharedString,
    pub menu_right: SharedString,
    pub menu_main: SharedString,

    pub trig_l1: SharedString,
    pub trig_l2: SharedString,
    pub trig_r1: SharedString,
    pub trig_r2: SharedString,
}

impl Default for KeyMap {
    fn default() -> Self {
        KeyMap {
            act_up: Key::Space.into(),
            act_right: Key::Escape.into(),
            act_down: Key::Return.into(),
            act_left: Key::Backspace.into(),

            dpad_up: Key::UpArrow.into(),
            dpad_right: Key::RightArrow.into(),
            dpad_down: Key::DownArrow.into(),
            dpad_left: Key::LeftArrow.into(),

            menu_left: Key::Insert.into(),
            menu_right: Key::Menu.into(),
            menu_main: Key::Meta.into(),

            trig_l1: Key::PageUp.into(),
            trig_l2: Key::Home.into(),
            trig_r1: Key::PageDown.into(),
            trig_r2: Key::End.into(),
        }
    }
}

impl From<KeyMap> for HashMap<Button, SharedString> {
    fn from(value: KeyMap) -> Self {
        HashMap::from([
            (Button::North, value.act_up),
            (Button::East, value.act_right),
            (Button::South, value.act_down),
            (Button::West, value.act_left),
            (Button::DPadUp, value.dpad_up),
            (Button::DPadRight, value.dpad_right),
            (Button::DPadDown, value.dpad_down),
            (Button::DPadLeft, value.dpad_left),
            (Button::Select, value.menu_left),
            (Button::Start, value.menu_right),
            (Button::Mode, value.menu_main),
            (Button::LeftTrigger, value.trig_l1),
            (Button::LeftTrigger2, value.trig_l2),
            (Button::RightTrigger, value.trig_r1),
            (Button::RightTrigger2, value.trig_r2),
        ])
    }
}
