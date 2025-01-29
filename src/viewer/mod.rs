mod image_grid_model;

use self::image_grid_model::ImageGridModel;

use crate::db::IndexDb;
use crate::ui::{MediaViewerBridge, MediaViewerModel, PhotoFlowApp};

use image::codecs::jpeg::JpegDecoder;
use image::ImageDecoder;
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer, Weak};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn execute<P: AsRef<Path>>(db_path: P) -> anyhow::Result<()> {
    let db = IndexDb::open(&db_path)?;
    let item_count = db.get_item_count()?;
    let db = Arc::new(Mutex::new(db));

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

    bind_media_loader(&app, db.clone());

    app.run()?;

    Ok(())
}

#[derive(Clone)]
struct MediaLoader {
    db: Arc<Mutex<IndexDb>>,
    requested_idx: Arc<Mutex<Option<usize>>>,
}

impl MediaLoader {
    pub fn new(db: Arc<Mutex<IndexDb>>) -> Self {
        Self {
            db,
            requested_idx: Default::default(),
        }
    }

    pub fn ensure_requested(&self, idx: usize) -> Option<()> {
        let target = self.requested_idx.lock().ok()?;

        if target.eq(&Some(idx)) {
            Some(())
        } else {
            None
        }
    }
}

fn bind_media_loader(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) {
    let bridge = app.global::<MediaViewerBridge>();
    let loader = MediaLoader::new(db);
    let weak_app = app.as_weak();

    bridge.on_load(move |idx| {
        let _ = on_load_impl(weak_app.clone(), &loader, idx as usize);
    })
}

fn on_load_impl(weak_app: Weak<PhotoFlowApp>, loader: &MediaLoader, idx: usize) -> Option<()> {
    let app = weak_app.upgrade()?;

    let path = get_loading_path(loader, idx)?;
    let file_name = Path::new(&path).file_name()?.to_str()?;

    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    bridge.set_model(MediaViewerModel {
        is_loading: true,
        file_name: file_name.into(),
        image: model.image,
    });

    let weak_app = weak_app.clone();
    let loader = loader.clone();
    rayon::spawn_fifo(move || {
        let _ = loading_impl(weak_app, &loader, idx, path);
    });

    Some(())
}

fn get_loading_path(loader: &MediaLoader, idx: usize) -> Option<String> {
    {
        let mut target = loader.requested_idx.lock().ok()?;
        if target.replace(idx) == Some(idx) {
            return None;
        }
    }

    loader.db.lock().ok()?.get_path(idx).ok()
}

fn loading_impl(
    weak_app: Weak<PhotoFlowApp>,
    loader: &MediaLoader,
    idx: usize,
    path: String,
) -> Option<()> {
    loader.ensure_requested(idx)?;
    let file = File::open(&path).ok()?;

    loader.ensure_requested(idx)?;
    let decoder = JpegDecoder::new(BufReader::new(file)).ok()?;
    let (w, h) = decoder.dimensions();
    let required_len = decoder.total_bytes() as usize;

    loader.ensure_requested(idx)?;
    let mut out: Vec<u8> = vec![0; required_len];

    loader.ensure_requested(idx)?;
    decoder.read_image(&mut out).ok()?;

    loader.ensure_requested(idx)?;
    let buf = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(&out, w, h);

    let loader = loader.clone();
    weak_app
        .upgrade_in_event_loop(move |app| {
            finish_loading_impl(&app, &loader, idx, buf);
        })
        .ok()
}

fn finish_loading_impl(
    app: &PhotoFlowApp,
    loader: &MediaLoader,
    idx: usize,
    buffer: SharedPixelBuffer<Rgb8Pixel>,
) {
    {
        let requested = loader.requested_idx.lock().unwrap();
        if !requested.eq(&Some(idx)) {
            return;
        }
    }

    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    bridge.set_model(MediaViewerModel {
        is_loading: false,
        file_name: model.file_name,
        image: Image::from_rgb8(buffer),
    });
}
