mod filehash;
mod metadata;
mod thumbnail;

use crate::db::{IndexDb, InsertionEntry};
use crate::ui::PhotoFlowApp;
use anyhow::anyhow;
use nom_exif::MediaParser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use slint::Weak;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::{DirEntry, WalkDir};

pub fn update_index_bg(
    sources: Vec<String>,
    db: Arc<Mutex<IndexDb>>,
    weak_app: Weak<PhotoFlowApp>,
    on_done: impl FnOnce(PhotoFlowApp) + Send + 'static,
) {
    rayon::spawn(move || {
        if let Err(e) = update_index(sources, db, weak_app.clone(), on_done) {
            let _ = weak_app.upgrade_in_event_loop(move |app| {
                app.set_indexing_error(format!("{:#}", e).into());
            });
        }
    });
}

fn update_index(
    sources: Vec<String>,
    db: Arc<Mutex<IndexDb>>,
    weak_app: Weak<PhotoFlowApp>,
    on_done: impl FnOnce(PhotoFlowApp) + Send + 'static,
) -> anyhow::Result<()> {
    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.create_index_if_not_exists()?;
        db.invalidate_index()?;
    }

    let mut paths = Vec::new();
    for source in sources {
        collect_paths(source, &mut paths);
    }

    let len = paths.len() as i32;
    weak_app.upgrade_in_event_loop(move |app| {
        app.set_indexing_total(len);
    })?;

    index_parallel(&db, &paths, weak_app.clone());

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.cleanup_index()?;
        db.rebuild_order_table()?;
    }

    weak_app.upgrade_in_event_loop(on_done)?;

    Ok(())
}

fn collect_paths<P: AsRef<Path>>(source: P, target: &mut Vec<PathBuf>) {
    let it = WalkDir::new(source)
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(is_jpeg)
        .map(|e| e.path().to_path_buf());
    target.extend(it);
}

fn is_jpeg(entry: &DirEntry) -> bool {
    entry
        .path()
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("jpg"))
        .unwrap_or(false)
}

fn index_parallel(db: &Mutex<IndexDb>, paths: &[PathBuf], weak_app: Weak<PhotoFlowApp>) {
    let media_parser = Mutex::new(MediaParser::new());
    let weak_app = Mutex::new(weak_app);

    paths.par_iter().for_each(move |path| {
        let _ = index_file(path, db, &media_parser);

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

    let hash = filehash::calculate(&path)?;

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        if db.set_valid_with_path_if_exists(&hash, path_str)? {
            return Ok(());
        }
    }

    let metadata = metadata::parse_exif_metadata(&path, mp)?;
    let datetime = match metadata.datetime {
        None => metadata::get_fs_datetime(&path)?,
        Some(value) => value,
    };

    let image = image::open(&path)?;
    let thumbnail = thumbnail::get_squared_jpeg(&image, 470, metadata.orientation)?;

    let entry = InsertionEntry {
        id: &hash,
        path: path_str,
        timestamp: datetime.timestamp(),
        orientation: metadata.orientation.into(),
        thumbnail: &thumbnail,
    };

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.insert_entry(&entry)?;
    }

    Ok(())
}
