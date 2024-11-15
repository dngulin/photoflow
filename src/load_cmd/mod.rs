mod view_model;

use crate::db::IndexDb;
use crate::load_cmd::view_model::ViewModel;
use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

pub fn execute<P: AsRef<Path>>(db_path: P) -> anyhow::Result<()> {
    let db = IndexDb::open(&db_path)?;
    let item_count = db.get_item_count()?;

    let app = PhotoFlowApp::new()?;
    let view_model = Rc::new(ViewModel::new(db));

    app.set_view_model(view_model.clone().into());
    app.on_set_view_model_range(move |offset, len| {
        view_model.set_range(offset as usize, len as usize)
    });
    app.set_item_count(item_count as i32);

    app.run()?;

    Ok(())
}
