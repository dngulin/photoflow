use image::imageops::FilterType;
use image::DynamicImage;
use image::GenericImageView;

pub fn squared(image: &DynamicImage, size: u32) -> DynamicImage {
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
