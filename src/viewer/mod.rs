mod image_grid_model;

use self::image_grid_model::ImageGridModel;

use crate::db::IndexDb;
use crate::ui::{MediaViewerBridge, MediaViewerModel, PhotoFlowApp};

use anyhow::anyhow;
use image::codecs::jpeg::JpegDecoder;
use image::ImageDecoder;
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer, SharedString, Weak};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn bind_models(app: &PhotoFlowApp, db: Arc<Mutex<IndexDb>>) -> anyhow::Result<()> {
    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        let item_count = db.get_item_count()?;
        app.set_item_count(item_count as i32);
    }

    let image_grid_model = Rc::new(ImageGridModel::new(db.clone()));
    app.set_grid_model(image_grid_model.clone().into());
    app.on_set_grid_visible_range(move |offset, len| {
        image_grid_model.set_range(offset as usize, len as usize);
    });

    let weak_app = app.as_weak();
    app.on_close(move || {
        if let Some(app) = weak_app.upgrade() {
            let _ = app.window().hide();
        }
    });

    bind_media_loader(app, db.clone());

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

    {
        let weak_app = app.as_weak();
        let loader = loader.clone();
        bridge.on_load(move |idx| {
            let _ = load(weak_app.clone(), &loader, idx as usize);
        });
    }

    {
        let weak_app = app.as_weak();
        let loader = loader.clone();
        bridge.on_clear(move || {
            let _ = clear(weak_app.clone(), &loader);
        });
    }
}

fn load(weak_app: Weak<PhotoFlowApp>, loader: &MediaLoader, idx: usize) -> Option<()> {
    if loader.requested_idx.lock().ok()?.replace(idx) == Some(idx) {
        return None;
    }

    let app = weak_app.upgrade()?;
    let path = {
        let db = loader.db.lock().ok()?;
        db.get_path(idx).ok()?
    };
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
        if let Some(buffer) = decode_image(&loader, idx, path) {
            let _ = weak_app.upgrade_in_event_loop(move |app| {
                let mut requested = loader.requested_idx.lock().unwrap();
                if *requested != Some(idx) {
                    return;
                }
                *requested = None;

                set_image_to_model(&app, buffer);
            });
        }
    });

    Some(())
}

fn decode_image(
    loader: &MediaLoader,
    idx: usize,
    path: String,
) -> Option<SharedPixelBuffer<Rgb8Pixel>> {
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

    Some(SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(&out, w, h))
}

fn set_image_to_model(app: &PhotoFlowApp, buffer: SharedPixelBuffer<Rgb8Pixel>) {
    let bridge = app.global::<MediaViewerBridge>();
    let model = bridge.get_model();
    bridge.set_model(MediaViewerModel {
        is_loading: false,
        file_name: model.file_name,
        image: Image::from_rgb8(buffer),
    });
}

fn clear(weak_app: Weak<PhotoFlowApp>, loader: &MediaLoader) -> Option<()> {
    *loader.requested_idx.lock().ok()? = None;

    let app = weak_app.upgrade()?;
    let bridge = app.global::<MediaViewerBridge>();
    bridge.set_model(MediaViewerModel {
        is_loading: false,
        file_name: SharedString::default(),
        image: Image::default(),
    });

    Some(())
}
