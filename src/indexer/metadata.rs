use crate::exif_orientation::ExifOrientation;
use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, Utc};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

pub struct ExifMetadata {
    pub datetime: Option<DateTime<FixedOffset>>,
    pub orientation: ExifOrientation,
}

pub fn parse_exif_metadata<P: AsRef<Path>>(
    path: P,
    mp: &Mutex<MediaParser>,
) -> anyhow::Result<ExifMetadata> {
    let ms = MediaSource::file_path(path)?;

    let iter: ExifIter = {
        let mut mp = mp
            .lock()
            .map_err(|_| anyhow!("Failed to lock MediaParser"))?;
        mp.parse(ms)?
    };

    let exif: Exif = iter.into();

    let metadata = ExifMetadata {
        datetime: exif.get(ExifTag::CreateDate).and_then(|e| e.as_time()),
        orientation: exif
            .get(ExifTag::Orientation)
            .and_then(|v| v.as_u16())
            .unwrap_or_default()
            .try_into()
            .unwrap_or_default(),
    };

    Ok(metadata)
}

pub fn get_fs_datetime<P: AsRef<Path>>(path: P) -> anyhow::Result<DateTime<FixedOffset>> {
    let metadata = fs::metadata(&path)?;

    let created = metadata.created()?;

    if let Ok(modified) = metadata.modified() {
        let min = if created < modified {
            created
        } else {
            modified
        };

        let min: DateTime<Utc> = min.into();
        return Ok(min.into());
    }

    let created: DateTime<Utc> = created.into();
    Ok(created.into())
}
