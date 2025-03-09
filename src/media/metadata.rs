use super::MediaType;
use crate::exif_orientation::ExifOrientation;
use anyhow::anyhow;
use chrono::{DateTime, FixedOffset, Utc};
use nom_exif::{Exif, ExifIter, ExifTag, MediaParser, MediaSource, TrackInfo, TrackInfoTag};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum MediaMetadata {
    Image {
        datetime: DateTime<FixedOffset>,
        orientation: ExifOrientation,
    },
    Video {
        datetime: DateTime<FixedOffset>,
        duration_ms: u64,
    },
}

impl MediaMetadata {
    pub fn parse<P: AsRef<Path>>(
        path: P,
        mt: &MediaType,
        mp: &mut MediaParser,
    ) -> anyhow::Result<MediaMetadata> {
        let ms = MediaSource::file_path(&path)?;

        match mt {
            MediaType::Image(_) if ms.has_exif() => {
                let iter: ExifIter = mp.parse(ms)?;
                let exif: Exif = iter.into();
                return Ok(MediaMetadata::Image {
                    datetime: exif
                        .get(ExifTag::CreateDate)
                        .and_then(|e| e.as_time())
                        .or_else(|| fs_datetime(&path))
                        .unwrap_or_default(),
                    orientation: exif
                        .get(ExifTag::Orientation)
                        .and_then(|e| e.as_u16())
                        .and_then(|u| (u as u64).try_into().ok())
                        .unwrap_or_default(),
                });
            }

            MediaType::Video(_) if ms.has_track() => {
                let info: TrackInfo = mp.parse(ms)?;
                return Ok(MediaMetadata::Video {
                    datetime: info
                        .get(TrackInfoTag::CreateDate)
                        .and_then(|e| e.as_time())
                        .or_else(|| fs_datetime(&path))
                        .unwrap_or_default(),
                    duration_ms: info
                        .get(TrackInfoTag::DurationMs)
                        .and_then(|e| e.as_u64())
                        .unwrap_or_default(),
                });
            }

            _ => {}
        }

        Err(anyhow!("No metadata found"))
    }

    pub fn exif_orientation(&self) -> Option<ExifOrientation> {
        match self {
            MediaMetadata::Image { orientation, .. } => Some(*orientation),
            MediaMetadata::Video { .. } => None,
        }
    }

    pub fn timestamp(&self) -> i64 {
        match self {
            MediaMetadata::Image { datetime, .. } => datetime.timestamp(),
            MediaMetadata::Video { datetime, .. } => datetime.timestamp(),
        }
    }
}

fn fs_datetime<P: AsRef<Path>>(path: P) -> Option<DateTime<FixedOffset>> {
    let metadata = fs::metadata(path.as_ref()).ok()?;
    let created = metadata.created().ok()?;

    if let Ok(modified) = metadata.modified() {
        let min = if created < modified {
            created
        } else {
            modified
        };

        let min: DateTime<Utc> = min.into();
        return Some(min.into());
    }

    let created: DateTime<Utc> = created.into();
    Some(created.into())
}
