mod media_type;
mod metadata;

pub use self::media_type::*;
pub use self::metadata::*;
use crate::video::Video;
use slint::Image;

pub enum Media {
    Image(Image),
    Video(Video),
}
