use crate::config::Config;
use crate::db::IndexDb;
use crate::ui::{Mode, PhotoFlowApp};
use slint::ComponentHandle;
use std::fs;
use std::sync::{Arc, Mutex};

mod config;
mod db;
mod exif_orientation;
mod img_decoder;
mod indexer;
mod viewer;

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

    let app = PhotoFlowApp::new()?;
    setup_app_window(&app);

    app.set_mode(Mode::Loading);
    indexer::update_index_bg(config.sources, db.clone(), app.as_weak(), move |app| {
        let _ = viewer::bind_models(&app, db);
        app.set_mode(Mode::Gallery);
    });

    app.run()?;

    Ok(())
}

fn setup_app_window(app: &PhotoFlowApp) {
    app.window().set_fullscreen(true);

    let weak_app = app.as_weak();
    app.on_close(move || {
        if let Some(app) = weak_app.upgrade() {
            let _ = app.window().hide();
        }
    });
}
