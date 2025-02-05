use crate::exif_orientation::ExifOrientation;
use image::DynamicImage;
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use std::path::Path;

pub fn is_extension_supported(path: &Path) -> bool {
    let supported = ["jpg", "jpeg", "heic"];
    path.extension()
        .map(move |ext| supported.iter().any(move |s| ext.eq_ignore_ascii_case(s)))
        .unwrap_or(false)
}

fn has_heic_extension(path: &Path) -> bool {
    path.extension()
        .map(move |ext| ext.eq_ignore_ascii_case("heic"))
        .unwrap_or(false)
}

pub fn open(path: &Path, orientation: ExifOrientation) -> anyhow::Result<DynamicImage> {
    if !is_extension_supported(path) {
        anyhow::bail!("Unsupported image format")
    }

    if has_heic_extension(path) {
        return decode_heic(path);
    }

    let image = image::open(path)?;
    Ok(orientation.apply(image))
}

fn decode_heic(path: &Path) -> anyhow::Result<DynamicImage> {
    let read_ctx = HeifContext::read_from_file(path.as_os_str().to_str().unwrap())?;
    let handle = read_ctx.primary_image_handle()?;

    let lib_heif = LibHeif::new();
    let heif_image = lib_heif.decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)?;

    let interleaved = heif_image
        .planes()
        .interleaved
        .ok_or_else(|| anyhow::anyhow!("Missing interleaved HEIF plane"))?;

    let rgb_image = image::RgbImage::from_raw(
        interleaved.width,
        interleaved.height,
        interleaved.data.to_vec(),
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to decode an interleaved HEIF plane"))?;

    Ok(DynamicImage::ImageRgb8(rgb_image))
}
