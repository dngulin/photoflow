use crate::config::Config;
use crate::db::IndexDb;
use crate::gamepad_input::{GamepadInputListener, KeyMap};
use crate::ui::{GamepadKey, GamepadKeyMap, Mode, PhotoFlowApp};
use crate::winit::WinitWindow;
use slint::{ComponentHandle, Timer, TimerMode};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod config;
mod db;
mod exif_orientation;
mod gamepad_input;
mod image_loader;
mod indexer;
mod media;
mod video;
mod viewer;
mod winit;

pub mod ui {
    slint::include_modules!();
}

fn main() -> anyhow::Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .ok_or(anyhow::anyhow!("Usage: photoflow <DB_CONFIG_PATH>"))?;

    let config = fs::read_to_string(&config_path)?;
    let config = toml::from_str::<Config>(&config)?;

    let db = IndexDb::open(config.db_path)?;
    let db = Arc::new(Mutex::new(db));

    gstreamer::init()?;

    let app = PhotoFlowApp::new()?;
    setup_app_window(&app);
    let _gamepad_poll_timer = setup_gamepad_input(&app);

    viewer::bind_media_viewer(&app, db.clone());

    app.set_mode(Mode::PreIndexing);
    indexer::update_index_bg(
        config.sources,
        db.clone(),
        app.as_weak(),
        move |app, count| {
            app.set_indexing_total(count);
            app.set_mode(Mode::Indexing);
        },
        move |app| {
            let _ = viewer::bind_gallery_models(&app, db);
            app.set_mode(Mode::Gallery);
        },
    );

    app.run()?;

    Ok(())
}

fn setup_app_window(app: &PhotoFlowApp) {
    let window = app.window();
    window.set_fullscreen(true);
    window.hide_cursor();

    let weak_app = app.as_weak();
    app.on_close(move || {
        if let Some(app) = weak_app.upgrade() {
            let _ = app.window().hide();
        }
    });
}

fn setup_gamepad_input(app: &PhotoFlowApp) -> Timer {
    let keymap = KeyMap::default();
    let mut gamepad_manager = GamepadInputListener::new(keymap.clone()).unwrap();

    let gamepad_key = app.global::<GamepadKey>();
    gamepad_key.invoke_setup(GamepadKeyMap {
        act_down: keymap.act_down,
        act_left: keymap.act_left,
        act_right: keymap.act_right,
        act_up: keymap.act_up,

        dpad_down: keymap.dpad_down,
        dpad_left: keymap.dpad_left,
        dpad_right: keymap.dpad_right,
        dpad_up: keymap.dpad_up,

        menu_left: keymap.menu_left,
        menu_main: keymap.menu_main,
        menu_right: keymap.menu_right,

        trigger_l1: keymap.trigger_l1,
        trigger_l2: keymap.trigger_l2,
        trigger_r1: keymap.trigger_r1,
        trigger_r2: keymap.trigger_r2,
    });

    let app_weak = app.as_weak();
    let gamepad_poll_timer = Timer::default();

    gamepad_poll_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(app) = app_weak.upgrade() {
            gamepad_manager.poll(app.window());
        }
    });

    gamepad_poll_timer
}
