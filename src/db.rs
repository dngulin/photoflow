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

        Ok(client)
    }

    pub fn create_index_if_not_exists(&self) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS media (
                    id TEXT UNIQUE,
                    path TEXT UNIQUE,
                    timestamp INTEGER,
                    orientation INTEGER,
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

    pub fn set_valid_with_path_if_exists(&self, id: &str, path: &str) -> rusqlite::Result<bool> {
        self.conn
            .execute(
                "UPDATE media SET (path, is_valid) = (?2, 1) WHERE id = ?1",
                (id, path),
            )
            .map(|count| count == 1)
    }

    pub fn insert_entry(&self, e: &InsertionEntry) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "INSERT INTO media (id, path, timestamp, orientation, is_valid, thumbnail) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
                (e.id, e.path, e.timestamp, e.orientation, e.thumbnail),
            )
            .map(|_| ())
    }

    pub fn get_thumbnail(&self, index: usize) -> rusqlite::Result<Vec<u8>> {
        self.conn.query_row(
            "SELECT thumbnail FROM media WHERE rowid=(SELECT id FROM media_order WHERE rowid=?1)",
            [index + 1],
            |row| row.get(0),
        )
    }

    pub fn get_path(&self, index: usize) -> rusqlite::Result<String> {
        self.conn.query_row(
            "SELECT path FROM media WHERE rowid=(SELECT id FROM media_order WHERE rowid=?1)",
            [index + 1],
            |row| row.get(0),
        )
    }

    pub fn get_item_count(&self) -> rusqlite::Result<usize> {
        self.conn
            .query_row("SELECT COUNT(id) FROM media_order", (), |row| row.get(0))
    }
}

pub struct InsertionEntry<'a> {
    pub id: &'a str,
    pub path: &'a str,
    pub timestamp: i64,
    pub orientation: u16,
    pub thumbnail: &'a [u8],
}
