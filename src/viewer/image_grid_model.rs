use crate::db::IndexDb;
use crate::viewer::ImageGridItem;
use anyhow::anyhow;
use image::codecs::jpeg::JpegDecoder;
use image::ImageDecoder;
use slint::{Image, Model, ModelNotify, ModelTracker, Rgb8Pixel, SharedPixelBuffer};
use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

pub struct ImageGridModel {
    inner: RefCell<ViewModelInner>,
    notify: ModelNotify,
}

impl ImageGridModel {
    pub fn new(db: Arc<Mutex<IndexDb>>) -> ImageGridModel {
        ImageGridModel {
            inner: RefCell::new(ViewModelInner::new(db)),
            notify: Default::default(),
        }
    }

    pub fn set_range(&self, offset: usize, len: usize) {
        self.inner.borrow_mut().set_range(offset, len, &self.notify)
    }
}

impl Model for ImageGridModel {
    type Data = ImageGridItem;

    fn row_count(&self) -> usize {
        self.inner.borrow().range.length
    }

    fn row_data(&self, row: usize) -> Option<Self::Data> {
        let inner = self.inner.borrow();

        if row >= inner.range.length {
            return None;
        }

        let db_index = inner.range.offset + row;

        let index = db_index as i32;
        let image = inner
            .image_buffers
            .get(&db_index)
            .map(|buffer| Image::from_rgb8(buffer.clone()))
            .unwrap_or_default();

        Some(Self::Data { index, image })
    }

    fn model_tracker(&self) -> &dyn ModelTracker {
        &self.notify
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ViewModelInner {
    db: Arc<Mutex<IndexDb>>,
    range: Range,

    image_buffers: HashMap<usize, SharedPixelBuffer<Rgb8Pixel>>,
    decoding_buf: Vec<u8>,
}

#[derive(Default, Eq, PartialEq)]
struct Range {
    pub offset: usize,
    pub length: usize,
}

impl ViewModelInner {
    pub fn new(db: Arc<Mutex<IndexDb>>) -> ViewModelInner {
        ViewModelInner {
            db,
            range: Default::default(),
            image_buffers: Default::default(),
            decoding_buf: Default::default(),
        }
    }

    pub fn set_range(&mut self, offset: usize, length: usize, notify: &ModelNotify) {
        let new_range = Range { offset, length };

        if self.range != new_range {
            self.rebuild(&new_range, notify);
        }
    }

    fn rebuild(&mut self, new_range: &Range, notify: &ModelNotify) {
        if new_range.is_empty() {
            self.clear(notify);
            return;
        }

        if self.range.is_empty() {
            self.range.offset = new_range.offset;
            self.add_front(new_range.length, notify);
            return;
        }

        let (old_min, old_max) = (self.range.min(), self.range.max());
        let (new_min, new_max) = (new_range.min(), new_range.max());

        if old_max < new_min || old_min > new_max {
            self.clear(notify);
            self.range.offset = new_min;
            self.add_front(new_range.length, notify);
            return;
        }

        match old_max.cmp(&new_max) {
            Ordering::Less => self.add_front(new_max - old_max, notify),
            Ordering::Equal => {}
            Ordering::Greater => self.remove_front(old_max - new_max, notify),
        };

        match old_min.cmp(&new_min) {
            Ordering::Less => self.remove_back(new_min - old_min, notify),
            Ordering::Equal => {}
            Ordering::Greater => self.add_back(old_min - new_min, notify),
        };
    }

    fn clear(&mut self, notify: &ModelNotify) {
        let remove_count = self.range.length;

        self.range = Range::default();
        self.image_buffers.clear();

        notify.row_removed(0, remove_count);
    }

    fn add_front(&mut self, count: usize, notify: &ModelNotify) {
        let start_db_idx = self.range.offset + self.range.length;

        for i in 0..count {
            self.load_thumbnail(start_db_idx + i);
        }

        self.range.length += count;
        notify.row_added(self.range.length - count, count);
    }

    fn remove_front(&mut self, count: usize, notify: &ModelNotify) {
        let start_db_idx = self.range.offset + self.range.length - count;

        for i in 0..count {
            self.image_buffers.remove(&(start_db_idx + i));
        }

        self.range.length -= count;
        notify.row_removed(self.range.length, count);
    }

    fn add_back(&mut self, count: usize, notify: &ModelNotify) {
        let start_db_idx = self.range.offset - count;

        for i in 0..count {
            self.load_thumbnail(start_db_idx + i);
        }

        self.range.offset -= count;
        self.range.length += count;
        notify.row_added(0, count);
    }

    fn remove_back(&mut self, count: usize, notify: &ModelNotify) {
        let start_db_idx = self.range.offset;

        for i in 0..count {
            self.image_buffers.remove(&(start_db_idx + i));
        }

        self.range.offset += count;
        self.range.length -= count;
        notify.row_removed(0, count);
    }

    fn load_thumbnail(&mut self, db_idx: usize) {
        match self.get_thumbnail(db_idx) {
            Ok(t) => {
                self.image_buffers.insert(db_idx, t);
            }
            Err(e) => {
                println!("Failed to get thumbnail for {}: {}", db_idx, e);
            }
        };
    }

    fn get_thumbnail(&mut self, db_idx: usize) -> anyhow::Result<SharedPixelBuffer<Rgb8Pixel>> {
        let encoded = {
            let db = self.db.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
            db.get_thumbnail(db_idx)?
        };

        let decoder = JpegDecoder::new(Cursor::new(encoded))?;
        let (w, h) = decoder.dimensions();
        let required_len = decoder.total_bytes() as usize;

        self.decoding_buf.reserve_exact(required_len);
        self.decoding_buf.resize(required_len, 0);

        decoder.read_image(&mut self.decoding_buf)?;

        let buf = SharedPixelBuffer::<Rgb8Pixel>::clone_from_slice(&self.decoding_buf, w, h);
        Ok(buf)
    }
}

impl Range {
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn min(&self) -> usize {
        self.offset
    }

    pub fn max(&self) -> usize {
        self.offset + self.length - 1
    }
}
