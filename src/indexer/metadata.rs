use crate::exif_orientation::ExifOrientation;
use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, Utc};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource, TrackInfo, TrackInfoTag};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

#[derive(Default)]
pub struct MediaMetadata {
    pub datetime: Option<DateTime<FixedOffset>>,
    pub exif_orientation: Option<ExifOrientation>,
}

pub fn parse_metadata<P: AsRef<Path>>(
    path: P,
    mp: &Mutex<MediaParser>,
) -> anyhow::Result<MediaMetadata> {
    let ms = MediaSource::file_path(path)?;

    if ms.has_exif() {
        let iter: ExifIter = mp.lock().unwrap().parse(ms)?;
        let exif: Exif = iter.into();

        return Ok(MediaMetadata {
            datetime: exif.get(ExifTag::CreateDate).and_then(|e| e.as_time()),
            exif_orientation: exif
                .get(ExifTag::Orientation)
                .and_then(|v| v.as_u16())
                .and_then(|i| i.try_into().ok()),
        });
    } else if ms.has_track() {
        let info: TrackInfo = mp.lock().unwrap().parse(ms)?;
        return Ok(MediaMetadata {
            datetime: info.get(TrackInfoTag::CreateDate).and_then(|e| e.as_time()),
            exif_orientation: None,
        });
    }

    Err(anyhow!("No metadata found"))
}

pub fn get_fs_datetime(metadata: &fs::Metadata) -> anyhow::Result<DateTime<FixedOffset>> {
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
