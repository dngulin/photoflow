use rusqlite::Connection;
use std::path::Path;

pub struct IndexDb {
    conn: Connection,
}

impl IndexDb {
    pub fn open<P: AsRef<Path>>(path: P) -> rusqlite::Result<IndexDb> {
        let client = IndexDb {
            conn: Connection::open(path)?,
        };

        client.conn.pragma_update(None, "synchronous", "OFF")?;
        client.conn.pragma_update(None, "journal_mode", "OFF")?;

        Ok(client)
    }

    pub fn create_index_if_not_exists(&self) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS media (
                    path TEXT PRIMARY KEY,
                    finfo TEXT, -- file size and mtime (used for changes detection)
                    timestamp INTEGER,
                    metadata INTEGER, -- exif_orientation for images / duration_ms for videos
                    is_valid INTEGER,
                    thumbnail BLOB
                )",
                (),
            )
            .map(|_| ())
    }

    pub fn invalidate_index(&self) -> rusqlite::Result<()> {
        self.conn
            .execute("UPDATE media SET is_valid = 0", ())
            .map(|_| ())
    }

    pub fn cleanup_index(&self) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM media WHERE is_valid = 0", ())
            .map(|_| ())
    }

    pub fn rebuild_order_table(&self) -> rusqlite::Result<()> {
        self.conn.execute("DROP TABLE IF EXISTS media_order", ())?;
        self.conn
            .execute("CREATE TABLE media_order (id INTEGER UNIQUE)", ())?;
        self.conn
            .execute(
                "INSERT INTO media_order (id) SELECT rowid FROM media ORDER BY timestamp",
                (),
            )
            .map(|_| ())
    }

    pub fn set_valid_if_unchanged(&self, path: &str, finfo: &str) -> rusqlite::Result<bool> {
        self.conn
            .execute(
                "UPDATE media SET is_valid = 1 WHERE path = ?1 AND finfo = ?2",
                (path, finfo),
            )
            .map(|count| count == 1)
    }

    pub fn upsert_entry(&self, e: &InsertionEntry) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "INSERT INTO media
                    (path, finfo, timestamp, metadata, is_valid, thumbnail)
                VALUES
                    (?1, ?2, ?3, ?4, 1, ?5)
                ON CONFLICT(path) DO UPDATE SET
                    finfo = excluded.finfo,
                    timestamp = excluded.timestamp,
                    metadata = excluded.metadata,
                    is_valid = excluded.is_valid,
                    thumbnail = excluded.thumbnail",
                (e.path, e.finfo, e.timestamp, e.metadata, e.thumbnail),
            )
            .map(|_| ())
    }

    pub fn get_path_metadata_and_thumbnail(
        &self,
        index: usize,
    ) -> rusqlite::Result<(String, u64, Vec<u8>)> {
        self.conn.query_row(
            "SELECT path, metadata, thumbnail FROM media WHERE rowid=(SELECT id FROM media_order WHERE rowid=?1)",
            [index + 1],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
    }

    pub fn get_path_and_metadata(&self, index: usize) -> rusqlite::Result<(String, u64)> {
        self.conn.query_row(
            "SELECT path, metadata FROM media WHERE rowid=(SELECT id FROM media_order WHERE rowid=?1)",
            [index + 1],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }

    pub fn get_item_count(&self) -> rusqlite::Result<usize> {
        self.conn
            .query_row("SELECT COUNT(id) FROM media_order", (), |row| row.get(0))
    }
}

pub struct InsertionEntry<'a> {
    pub path: &'a str,
    pub finfo: &'a str,
    pub timestamp: i64,
    pub metadata: u64,
    pub thumbnail: &'a [u8],
}
