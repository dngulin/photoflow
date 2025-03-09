use crate::exif_orientation::ExifOrientation;
use crate::media::ImageType;
use image::DynamicImage;
use libheif_rs::{ColorSpace, HeifContext, LibHeif, Plane, RgbChroma};
use std::cmp::Ordering;
use std::path::Path;

pub fn open<P: AsRef<Path>>(path: P, image_type: ImageType) -> anyhow::Result<DecodedImage> {
    match image_type {
        ImageType::Jpeg => {
            let image = image::open(path)?;
            Ok(DecodedImage::WithoutTransformations(image))
        }
        ImageType::Heic => {
            let image = decode_heic(path)?;
            Ok(DecodedImage::WithTransformations(image))
        }
    }
}

fn decode_heic<P: AsRef<Path>>(path: P) -> anyhow::Result<DynamicImage> {
    let path = path
        .as_ref()
        .as_os_str()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert path to str"))?;

    let read_ctx = HeifContext::read_from_file(path)?;
    let handle = read_ctx.primary_image_handle()?;

    let lib_heif = LibHeif::new();
    let heif_image = lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)?;

    let plane = heif_image
        .planes()
        .interleaved
        .ok_or_else(|| anyhow::anyhow!("Missing interleaved HEIF plane"))?;

    let rgb_buffer = allocate_rgb_buffer(&plane)
        .ok_or_else(|| anyhow::anyhow!("Failed to allocate RGB buffer"))?;

    let rgb_image = image::RgbImage::from_raw(plane.width, plane.height, rgb_buffer)
        .ok_or_else(|| anyhow::anyhow!("Invalid RGB buffer"))?;

    Ok(DynamicImage::ImageRgb8(rgb_image))
}

fn allocate_rgb_buffer(p: &Plane<&[u8]>) -> Option<Vec<u8>> {
    let line_len = (p.width * 3) as usize;

    match p.stride.cmp(&line_len) {
        Ordering::Less => None,
        Ordering::Equal => Some(p.data.to_vec()),
        Ordering::Greater => {
            let mut buf = Vec::with_capacity(line_len * (p.height as usize));
            for line in p.data.chunks(p.stride) {
                buf.extend(&line[..line_len]);
            }
            Some(buf)
        }
    }
}

pub enum DecodedImage {
    WithTransformations(DynamicImage),
    WithoutTransformations(DynamicImage),
}

impl DecodedImage {
    pub fn oriented(self, orientation: ExifOrientation) -> DynamicImage {
        match self {
            DecodedImage::WithTransformations(image) => image,
            DecodedImage::WithoutTransformations(image) => orientation.apply(image),
        }
    }

    pub fn map<F: FnOnce(DynamicImage) -> DynamicImage>(self, f: F) -> Self {
        match self {
            DecodedImage::WithTransformations(i) => DecodedImage::WithTransformations(f(i)),
            DecodedImage::WithoutTransformations(i) => DecodedImage::WithoutTransformations(f(i)),
        }
    }
}
