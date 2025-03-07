use crate::exif_orientation::ExifOrientation;
use image::DynamicImage;
use libheif_rs::{ColorSpace, HeifContext, LibHeif, Plane, RgbChroma};
use std::cmp::Ordering;
use std::path::Path;

pub fn is_extension_supported(path: &Path) -> bool {
    let supported = ["jpg", "jpeg", "heic"];
    path.extension()
        .map(move |ext| supported.iter().any(move |s| ext.eq_ignore_ascii_case(s)))
        .unwrap_or(false)
}

pub fn open(path: &Path) -> anyhow::Result<DecodedImage> {
    if !is_extension_supported(path) {
        anyhow::bail!("Unsupported image format")
    }

    if is_heic(path) {
        let image = decode_heic(path)?;
        return Ok(DecodedImage::WithTransformations(image));
    }

    let image = image::open(path)?;
    Ok(DecodedImage::WithoutTransformations(image))
}

fn is_heic(path: &Path) -> bool {
    path.extension()
        .map(move |ext| ext.eq_ignore_ascii_case("heic"))
        .unwrap_or(false)
}

fn decode_heic(path: &Path) -> anyhow::Result<DynamicImage> {
    let read_ctx = HeifContext::read_from_file(path.as_os_str().to_str().unwrap())?;
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
