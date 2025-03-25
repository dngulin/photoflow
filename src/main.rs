use crate::config::Config;
use crate::db::IndexDb;
use crate::gamepad_input::{GamepadInputListener, KeyMap};
use crate::ui::{GamepadKey, Mode, PhotoFlowApp};
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
mod util;
mod video;
mod viewer;
mod winit;

pub mod ui {
    slint::include_modules!();
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

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

    log::info!("Evaluating media files count...");
    app.set_mode(Mode::PreIndexing);
    indexer::update_index_bg(
        config.sources,
        db.clone(),
        app.as_weak(),
        move |app, count| {
            log::info!("Media files count: {}. Start indexing...", count);
            app.set_indexing_total(count);
            app.set_mode(Mode::Indexing);
        },
        move |app| {
            log::info!("Indexing finished!");
            if let Err(e) = viewer::bind_gallery_models(&app, db) {
                log::error!("Failed to bind gallery models: {}", e);
            }
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
            if let Err(e) = app.window().hide() {
                log::error!("Failed to hide window: {}", e);
            }
        }
    });
}

fn setup_gamepad_input(app: &PhotoFlowApp) -> Timer {
    let map = KeyMap::default();
    let mut gamepad_manager = GamepadInputListener::new(map.clone()).unwrap();

    let ui_map = app.global::<GamepadKey>();
    ui_map.invoke_set_actions(map.act_up, map.act_right, map.act_down, map.act_left);
    ui_map.invoke_set_dpad(map.dpad_up, map.dpad_right, map.dpad_down, map.dpad_left);
    ui_map.invoke_set_menupad(map.menu_main, map.menu_left, map.menu_right);
    ui_map.invoke_set_triggers(map.trig_l1, map.trig_l2, map.trig_r1, map.trig_r2);

    let app_weak = app.as_weak();
    let gamepad_poll_timer = Timer::default();

    gamepad_poll_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(app) = app_weak.upgrade() {
            gamepad_manager.poll(app.window());
        }
    });

    gamepad_poll_timer
}
