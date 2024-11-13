use image::{imageops, DynamicImage};

pub enum ExifOrientation {
    Unchanged,
    MirroredHorizontally,
    Rotated180,
    MirroredVertically,
    Rotated90AndMirroredHorizontally,
    Rotated90,
    Rotated90AndMirroredVertically,
    Rotated270, // Rotated90AndRotated180
}

impl Default for ExifOrientation {
    fn default() -> Self {
        Self::Unchanged
    }
}

impl TryFrom<u16> for ExifOrientation {
    type Error = ();

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Unchanged),
            2 => Ok(Self::MirroredHorizontally),
            3 => Ok(Self::Rotated180),
            4 => Ok(Self::MirroredVertically),
            5 => Ok(Self::Rotated90AndMirroredHorizontally),
            6 => Ok(Self::Rotated90),
            7 => Ok(Self::Rotated90AndMirroredVertically),
            8 => Ok(Self::Rotated270),
            _ => Err(()),
        }
    }
}

impl ExifOrientation {
    pub fn apply(&self, mut image: DynamicImage) -> DynamicImage {
        match self {
            ExifOrientation::Unchanged => image,
            ExifOrientation::MirroredHorizontally => {
                imageops::flip_horizontal_in_place(&mut image);
                image
            }
            ExifOrientation::Rotated180 => {
                imageops::rotate180_in_place(&mut image);
                image
            }
            ExifOrientation::MirroredVertically => {
                imageops::flip_vertical_in_place(&mut image);
                image
            }
            ExifOrientation::Rotated90AndMirroredHorizontally => {
                image = image.rotate90();
                imageops::flip_horizontal_in_place(&mut image);
                image
            }
            ExifOrientation::Rotated90 => image.rotate90(),
            ExifOrientation::Rotated90AndMirroredVertically => {
                image = image.rotate90();
                imageops::flip_vertical_in_place(&mut image);
                image
            }
            ExifOrientation::Rotated270 => image.rotate270(),
        }
    }
}
