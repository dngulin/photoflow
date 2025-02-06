mod filter_axis_to_dpad_buttons;
mod filter_dpad_button_events;
mod keymap;

use crate::winit::WinitWindow;
use filter_axis_to_dpad_buttons::left_axis_to_dpad_btn;
use filter_dpad_button_events::filter_wrong_dpad_events;
use gilrs::ev::filter::{axis_dpad_to_button, deadzone, Jitter, Repeat};
use gilrs::{EventType, Filter, Gilrs, GilrsBuilder};
use slint::platform::WindowEvent;
use slint::Window;
use std::time::Duration;

pub struct GamepadInputListener {
    gilrs: Gilrs,
}

impl GamepadInputListener {
    pub fn new() -> Result<Self, String> {
        let gilrs = GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
            .map_err(|error| format!("Failed to init gamepad input backend: {}", error))?;

        Ok(Self { gilrs })
    }

    pub fn poll(&mut self, window: &Window) {
        let has_focus = window.has_focus();

        let gilrs = &mut self.gilrs;
        let jitter = Jitter::new();
        let repeat_filter = Repeat {
            after: Duration::from_millis(600),
            every: Duration::from_millis(50),
        };

        while let Some(event) = gilrs
            .next_event()
            .filter_ev(&axis_dpad_to_button, gilrs)
            .filter_ev(&deadzone, gilrs)
            .filter_ev(&jitter, gilrs)
            .filter_ev(&left_axis_to_dpad_btn, gilrs)
            .filter_ev(&filter_wrong_dpad_events, gilrs)
            .filter_ev(&repeat_filter, gilrs)
        {
            gilrs.update(&event);

            match event.event {
                EventType::ButtonPressed(btn, _) if has_focus => {
                    if let Some(key) = keymap::btn_to_key(btn) {
                        window.dispatch_event(WindowEvent::KeyPressed { text: key.into() });
                    }
                }
                EventType::ButtonRepeated(btn, _) if has_focus => {
                    if let Some(key) = keymap::btn_to_key(btn) {
                        window.dispatch_event(WindowEvent::KeyPressRepeated { text: key.into() });
                    }
                }
                EventType::ButtonReleased(btn, _) if has_focus => {
                    if let Some(key) = keymap::btn_to_key(btn) {
                        window.dispatch_event(WindowEvent::KeyReleased { text: key.into() });
                    }
                }
                _ => continue,
            }
        }
    }
}
