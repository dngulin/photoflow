use crate::img_decoder;
use crate::img_decoder::DecodedImage;
use anyhow::anyhow;
use gstreamer::prelude::{Cast, ElementExt, ElementExtManual, GstBinExt, IsA, ObjectExt};
use gstreamer_app::{AppSink, AppSinkCallbacks};
use gstreamer_video::{VideoCapsBuilder, VideoFormat, VideoFrameExt, VideoFrameRef, VideoInfo};
use image::flat::NormalForm;
use image::{DynamicImage, FlatSamples, RgbImage};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub fn is_extension_supported(path: &Path) -> bool {
    is_video(path) || img_decoder::is_extension_supported(path)
}

fn is_video(path: &Path) -> bool {
    let supported = ["mp4", "mov"];
    path.extension()
        .map(move |ext| supported.iter().any(move |s| ext.eq_ignore_ascii_case(s)))
        .unwrap_or(false)
}

pub fn open(path: &Path) -> anyhow::Result<DecodedImage> {
    if is_video(path) {
        return get_video_preview(path);
    }

    img_decoder::open(path)
}

fn get_video_preview(path: &Path) -> anyhow::Result<DecodedImage> {
    const GST_PIPELINE: &str =
        "filesrc name=src ! decodebin ! videoflip video-direction=auto ! videoconvert ! appsink name=sink";
    let pipeline = gstreamer::parse::launch(GST_PIPELINE)?
        .downcast::<gstreamer::Pipeline>()
        .map_err(|_| anyhow!("Failed to downcast a pipeline"))?;

    let src = pipeline.get::<gstreamer::Element>("src")?;
    src.set_property("location", path);

    let sink = pipeline.get::<AppSink>("sink")?;
    sink.set_property("sync", false);
    sink.set_property("wait-on-eos", false);

    let caps = VideoCapsBuilder::new().format(VideoFormat::Rgb).build();
    sink.set_caps(Some(&caps));

    let result = Arc::new(Mutex::new(Option::<DynamicImage>::None));

    {
        let result = result.clone();
        let mut sample_taken = false;

        let callbacks = AppSinkCallbacks::builder()
            .new_sample(move |app_sink| {
                if !sample_taken {
                    *result.lock().unwrap() = decode_sample(app_sink);
                    sample_taken = true;
                }
                Err(gstreamer::FlowError::Eos)
            })
            .build();

        sink.set_callbacks(callbacks);
    }

    let bus = pipeline
        .bus()
        .ok_or_else(|| anyhow!("Failed to get bus from pipeline"))?;

    pipeline.set_state(gstreamer::State::Paused)?;
    let mut play_mode = false;

    for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
        match msg.view() {
            gstreamer::MessageView::AsyncDone(..) => {
                if !play_mode {
                    play_mode = true;
                    pipeline.set_state(gstreamer::State::Playing)?;
                }
            }
            gstreamer::MessageView::Eos(..) => {
                break;
            }
            gstreamer::MessageView::Error(e) => {
                pipeline.set_state(gstreamer::State::Null)?;
                return Err(anyhow!("GStreamer error: {}", e));
            }
            _ => {}
        }
    }

    let d = pipeline
        .query_duration::<gstreamer::format::Time>()
        .unwrap_or_default();
    println!("{:?} d={}", path, d);

    pipeline.set_state(gstreamer::State::Null)?;

    let mut result = result.lock().unwrap();
    result
        .take()
        .map(|img| DecodedImage::WithTransformations(img))
        .ok_or(anyhow!("Failed to get video thumbnail"))
}

fn decode_sample(sink: &AppSink) -> Option<DynamicImage> {
    let sample = sink.pull_sample().ok()?;

    let caps = sample.caps()?;
    let buffer = sample.buffer()?;

    let vinfo = VideoInfo::from_caps(caps).ok()?;
    let frame = VideoFrameRef::from_buffer_ref_readable(buffer, &vinfo).ok()?;

    let raw = frame.plane_data(0).ok()?;
    let stride = frame.plane_stride()[0] as usize;

    let flat = FlatSamples::<&[u8]> {
        samples: raw,
        layout: image::flat::SampleLayout {
            channels: 3,
            channel_stride: 1, // 1 byte from component to component
            width: frame.width(),
            width_stride: 3, // 4 byte from pixel to pixel
            height: frame.height(),
            height_stride: stride, // stride from line to line
        },
        color_hint: Some(image::ColorType::Rgb8),
    };

    if flat.is_normal(NormalForm::RowMajorPacked) {
        let rgb_image = RgbImage::from_raw(flat.layout.width, flat.layout.height, raw.into())?;
        return Some(DynamicImage::ImageRgb8(rgb_image));
    }

    None
}

trait PiplineExtensions {
    fn get<E: IsA<gstreamer::Element>>(&self, name: &str) -> anyhow::Result<E>;
}

impl PiplineExtensions for gstreamer::Pipeline {
    fn get<E: IsA<gstreamer::Element>>(&self, name: &str) -> anyhow::Result<E> {
        self.by_name(name)
            .ok_or_else(|| anyhow!("Failed to find a `{name}` node"))?
            .downcast::<E>()
            .map_err(|_| anyhow!("Filed to cast the `{name}` node to target type"))
    }
}
