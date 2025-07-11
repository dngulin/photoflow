mod preview_loader;
mod thumbnail;

use crate::db::{IndexDb, InsertionEntry};
use crate::media::{MediaMetadata, MediaType};
use crate::ui::PhotoFlowApp;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use image::codecs::jpeg::JpegEncoder;
use image::DynamicImage;
use nom_exif::MediaParser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use slint::Weak;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::{DirEntry, WalkDir};

pub fn update_index_bg(
    sources: Vec<String>,
    db: Arc<Mutex<IndexDb>>,
    weak_app: Weak<PhotoFlowApp>,
    on_start: impl FnOnce(PhotoFlowApp, i32) + Send + 'static,
    on_finish: impl FnOnce(PhotoFlowApp) + Send + 'static,
) {
    rayon::spawn(move || {
        if let Err(e) = update_index(sources, db, weak_app.clone(), on_start, on_finish) {
            log::error!("Update Index error: {e}");
            let _ = weak_app.upgrade_in_event_loop(move |app| {
                app.set_indexing_error(format!("{e}").into());
            });
        }
    });
}

fn update_index(
    sources: Vec<String>,
    db: Arc<Mutex<IndexDb>>,
    weak_app: Weak<PhotoFlowApp>,
    on_start: impl FnOnce(PhotoFlowApp, i32) + Send + 'static,
    on_finish: impl FnOnce(PhotoFlowApp) + Send + 'static,
) -> anyhow::Result<()> {
    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.create_index_if_not_exists()?;
        db.invalidate_index()?;
    }

    let mut paths = HashSet::new();
    for source in sources {
        collect_paths(source, &mut paths);
    }

    let len = paths.len() as i32;
    weak_app.upgrade_in_event_loop(move |app| {
        on_start(app, len);
    })?;

    index_parallel(&db, &paths, weak_app.clone());

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.cleanup_index()?;
        db.rebuild_order_table()?;
    }

    weak_app.upgrade_in_event_loop(on_finish)?;

    Ok(())
}

fn collect_paths<P: AsRef<Path>>(source: P, target: &mut HashSet<PathBuf>) {
    let it = WalkDir::new(source)
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(is_not_hidden)
        .filter(|e| MediaType::from_path(e.path()).is_some())
        .map(|e| e.path().to_path_buf());
    target.extend(it);
}

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name_str| !name_str.starts_with("."))
        .unwrap_or(false)
}

fn index_parallel(db: &Mutex<IndexDb>, paths: &HashSet<PathBuf>, weak_app: Weak<PhotoFlowApp>) {
    let media_parser = Mutex::new(MediaParser::new());
    let weak_app = Mutex::new(weak_app);

    paths.par_iter().for_each(move |path| {
        if let Err(e) = index_file(path, db, &media_parser) {
            log::error!(
                "Failed to index file `{}`: {}",
                path.to_str().unwrap_or_default(),
                e
            );
        }

        {
            let weak_app = weak_app.lock().unwrap();
            let _ = weak_app.upgrade_in_event_loop(move |app| {
                app.set_indexing_processed(app.get_indexing_processed() + 1);
            });
        }
    });
}

fn index_file<P: AsRef<Path>>(
    path: P,
    db: &Mutex<IndexDb>,
    mp: &Mutex<MediaParser>,
) -> anyhow::Result<()> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| anyhow!("Non-unicode path"))?;

    let file_meta = fs::metadata(&path)?;
    let finfo = get_finfo_str(&file_meta)?;

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        if db.set_valid_if_unchanged(path_str, &finfo)? {
            return Ok(());
        }
    }

    let media_type = MediaType::from_path(&path).ok_or_else(|| anyhow!("Invalid media type"))?;
    let media_meta = {
        let mut mp = mp.lock().unwrap();
        MediaMetadata::parse(&path, &media_type, &mut mp)?
    };

    let preview = preview_loader::open(&path, &media_type)?;
    let thumbnail = preview
        .map(|img| thumbnail::squared(&img, 470))
        .oriented(media_meta.exif_orientation().unwrap_or_default());

    let entry = InsertionEntry {
        path: path_str,
        finfo: &finfo,
        timestamp: media_meta.timestamp(),
        metadata: db_meta(&media_meta),
        thumbnail: &encode_jpeg(&thumbnail)?,
    };

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.upsert_entry(&entry)?;
    }

    Ok(())
}

fn db_meta(metadata: &MediaMetadata) -> u64 {
    match metadata {
        MediaMetadata::Image { orientation, .. } => (*orientation).into(),
        MediaMetadata::Video { duration_ms, .. } => *duration_ms,
    }
}

fn get_finfo_str(m: &fs::Metadata) -> anyhow::Result<String> {
    let modified: DateTime<Utc> = m.modified()?.into();
    let formatted = format!("{:x}:{:x}", m.len(), modified.timestamp());
    Ok(formatted)
}

fn encode_jpeg(image: &DynamicImage) -> anyhow::Result<Vec<u8>> {
    let mut result = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut result, 70);
    image.write_with_encoder(encoder)?;

    Ok(result)
}
