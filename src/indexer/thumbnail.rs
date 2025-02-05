use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::DynamicImage;
use image::GenericImageView;

pub fn get_squared_jpeg(image: &DynamicImage, size: u32) -> anyhow::Result<Vec<u8>> {
    let thumbnail = get_squared(image, size);

    let mut result = Vec::new();
    let encoder = JpegEncoder::new_with_quality(&mut result, 70);
    thumbnail.write_with_encoder(encoder)?;

    Ok(result)
}

fn get_squared(image: &DynamicImage, size: u32) -> DynamicImage {
    let (w, h) = image.dimensions();

    if w == h {
        return image.resize(size, size, FilterType::Lanczos3);
    }

    let image = if w > h {
        image.crop_imm((w - h) / 2, 0, h, h)
    } else {
        image.crop_imm(0, (h - w) / 2, w, w)
    };

    image.resize(size, size, FilterType::Lanczos3)
}
