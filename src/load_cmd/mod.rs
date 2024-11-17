mod image_grid_model;

use crate::db::IndexDb;
use image_grid_model::ImageGridModel;

use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

pub fn execute<P: AsRef<Path>>(db_path: P) -> anyhow::Result<()> {
    let db = IndexDb::open(&db_path)?;
    let item_count = db.get_item_count()?;

    let app = PhotoFlowApp::new()?;
    app.window().set_fullscreen(true);

    let image_grid_model = Rc::new(ImageGridModel::new(db));
    app.set_grid_model(image_grid_model.clone().into());
    app.on_set_grid_visible_range(move |offset, len| {
        image_grid_model.set_range(offset as usize, len as usize)
    });

    app.set_item_count(item_count as i32);

    app.run()?;

    Ok(())
}
