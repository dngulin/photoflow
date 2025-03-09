use std::ffi::OsStr;
use std::path::Path;

#[derive(Clone, Copy)]
pub enum ImageType {
    Jpeg,
    Heic,
}

#[derive(Clone, Copy)]
pub enum VideoType {
    Mp4,
    Mov,
}

#[derive(Clone, Copy)]
pub enum MediaType {
    Image(ImageType),
    Video(VideoType),
}

impl ImageType {
    pub fn from_ext(ext: &OsStr) -> Option<ImageType> {
        if ext.eq_ignore_ascii_case("jpeg") || ext.eq_ignore_ascii_case("jpg") {
            return Some(ImageType::Jpeg);
        }

        if ext.eq_ignore_ascii_case("heic") {
            return Some(ImageType::Heic);
        }

        None
    }
}

impl VideoType {
    pub fn from_ext(ext: &OsStr) -> Option<VideoType> {
        if ext.eq_ignore_ascii_case("mp4") {
            return Some(VideoType::Mp4);
        }

        if ext.eq_ignore_ascii_case("mov") {
            return Some(VideoType::Mov);
        }

        None
    }
}

impl MediaType {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Option<MediaType> {
        let ext = path.as_ref().extension()?;

        if let Some(image_type) = ImageType::from_ext(ext) {
            return Some(MediaType::Image(image_type));
        }

        if let Some(video_type) = VideoType::from_ext(ext) {
            return Some(MediaType::Video(video_type));
        }

        None
    }
}
