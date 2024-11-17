mod image_grid_model;

use crate::db::IndexDb;
use image_grid_model::ImageGridModel;

use slint::Image;
use std::path::Path;
use std::rc::Rc;

slint::include_modules!();

pub fn execute<P: AsRef<Path>>(db_path: P) -> anyhow::Result<()> {
    let db = Rc::new(IndexDb::open(&db_path)?);
    let item_count = db.get_item_count()?;

    let app = PhotoFlowApp::new()?;
    app.window().set_fullscreen(true);

    let image_grid_model = Rc::new(ImageGridModel::new(db.clone()));
    app.set_grid_model(image_grid_model.clone().into());
    app.on_set_grid_visible_range(move |offset, len| {
        image_grid_model.set_range(offset as usize, len as usize);
    });

    app.set_item_count(item_count as i32);

    let weak_app = app.as_weak();
    app.on_close(move || {
        if let Some(app) = weak_app.upgrade() {
            let _ = app.window().hide();
        }
    });

    let ms = app.global::<MediaSource>();

    let weak_app = app.as_weak();
    ms.on_clear(move || {
        if let Some(app) = weak_app.upgrade() {
            app.global::<MediaSource>().set_image(Image::default())
        }
    });

    let img_provider = Rc::new(ImageProvider::new(db.clone()));
    let weak_app = app.as_weak();
    ms.on_set_index(move |idx| {
        let idx = idx as usize;
        if Some(idx) == img_provider.index {
            return;
        }

        if let Some(app) = weak_app.upgrade() {
            let image = img_provider.load_image(idx).unwrap_or_default();
            app.global::<MediaSource>().set_image(image);
        }
    });

    app.run()?;

    Ok(())
}

pub struct ImageProvider {
    db: Rc<IndexDb>,
    pub index: Option<usize>,
}

impl ImageProvider {
    pub fn new(db: Rc<IndexDb>) -> Self {
        Self { db, index: None }
    }

    pub fn load_image(&self, index: usize) -> anyhow::Result<Image> {
        let path_str = self.db.get_path(index)?;
        let image = Image::load_from_path(Path::new(&path_str))?;
        Ok(image)
    }
}
