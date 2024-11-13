mod metadata;
mod thumbnail;

use crate::db::{IndexDb, InsertionEntry};

use anyhow::anyhow;
use nom_exif::MediaParser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use walkdir::{DirEntry, WalkDir};

pub fn execute<P: AsRef<Path>>(db: P, source: P) -> anyhow::Result<()> {
    let index_db = create_index_db(&db)?;
    let paths = collect_paths(&source);

    let index_db = index_parallel(index_db, &paths)?;
    index_db.rebuild_order_table()?;

    Ok(())
}

fn create_index_db<P: AsRef<Path>>(path: P) -> anyhow::Result<IndexDb> {
    std::fs::remove_file(&path)?;

    let index_db = IndexDb::open(&path)?;
    index_db.create_index()?;

    Ok(index_db)
}

fn collect_paths<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|r| r.ok())
        .filter(is_jpeg)
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn is_jpeg(entry: &DirEntry) -> bool {
    entry
        .path()
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("jpg"))
        .unwrap_or(false)
}

fn index_parallel(index_db: IndexDb, paths: &[PathBuf]) -> anyhow::Result<IndexDb> {
    let index_db = Mutex::new(index_db);
    let media_parser = Mutex::new(MediaParser::new());

    paths.par_iter().for_each(|path| {
        if let Err(e) = index_file(path, &index_db, &media_parser) {
            println!("E {} - {}", path.as_path().display(), e);
        } else {
            println!("I {} - OK", path.as_path().display());
        }
    });

    let index_db = index_db
        .into_inner()
        .map_err(|_| anyhow!("Failed to release IndexDB"))?;
    Ok(index_db)
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

    let metadata = metadata::parse_exif_metadata(&path, mp)?;
    let datetime = match metadata.datetime {
        None => metadata::get_fs_datetime(&path)?,
        Some(value) => value,
    };

    let image = image::open(&path)?;
    let thumbnail = thumbnail::get_squared_jpeg(&image, 470, metadata.orientation)?;

    let entry = InsertionEntry {
        path: path_str,
        timestamp: datetime.timestamp(),
        thumbnail: &thumbnail,
    };

    {
        let db = db.lock().map_err(|_| anyhow!("Failed to lock IndexDB"))?;
        db.insert_entry(&entry)?;
    }

    Ok(())
}
