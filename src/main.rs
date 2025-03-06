use crate::config::Config;
use crate::db::IndexDb;
use crate::gamepad_input::GamepadInputListener;
use crate::ui::{Mode, PhotoFlowApp};
use crate::viewer::bind_media_loader;
use crate::winit::WinitWindow;
use slint::{ComponentHandle, Timer, TimerMode};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod config;
mod db;
mod exif_orientation;
mod gamepad_input;
mod img_decoder;
mod indexer;
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

    bind_media_loader(&app, db.clone());

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
            let _ = viewer::bind_models(&app, db);
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
    let mut gamepad_manager = GamepadInputListener::new().unwrap();

    let app_weak = app.as_weak();
    let gamepad_poll_timer = Timer::default();

    gamepad_poll_timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(app) = app_weak.upgrade() {
            gamepad_manager.poll(app.window());
        }
    });

    gamepad_poll_timer
}
