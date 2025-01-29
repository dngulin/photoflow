use crate::db::IndexDb;
use crate::ui::{Mode, PhotoFlowApp};

use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

mod db;
mod exif_orientation;
mod indexer;
mod viewer;

pub mod ui {
    slint::include_modules!();
}

fn main() -> anyhow::Result<()> {
    let app = PhotoFlowApp::new()?;
    setup_app_window(&app);

    let db = IndexDb::open("photoflow.db")?;
    let db = Arc::new(Mutex::new(db));

    app.set_mode(Mode::Loading);
    indexer::update_index(&app, db.clone(), move |app: &PhotoFlowApp| {
        viewer::bind_models(app, db);
        app.set_mode(Mode::Gallery);
    })?;

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
