use arrayvec::ArrayString;
use blake3::Hash;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn calculate<P: AsRef<Path>>(path: P) -> std::io::Result<ArrayString<64>> {
    let input = File::open(path)?;
    let reader = BufReader::new(input);
    let hash = blake3_beginning(reader)?;

    Ok(hash.to_hex())
}

fn blake3_beginning<R: Read>(mut reader: R) -> std::io::Result<Hash> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 1024 * 4];

    let count = reader.read(&mut buffer)?;
    hasher.update(&buffer[..count]);

    Ok(hasher.finalize())
}
