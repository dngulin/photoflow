mod db;
mod exif_orientation;
mod indexer;
mod viewer;

fn main() {
    viewer::execute("photoflow.db").unwrap();
}
