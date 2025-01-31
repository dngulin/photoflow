use arrayvec::ArrayString;
use blake3::Hash;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub fn calculate<P: AsRef<Path>>(path: P) -> std::io::Result<ArrayString<64>> {
    let input = File::open(path)?;
    let reader = BufReader::new(input);
    let hash = blake3(reader)?;

    Ok(hash.to_hex())
}

fn blake3<R: Read>(mut reader: R) -> std::io::Result<Hash> {
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 1024 * 8];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize())
}
