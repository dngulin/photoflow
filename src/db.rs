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

    pub fn create_index(&self) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "CREATE TABLE media (
                    path TEXT UNIQUE,
                    timestamp INTEGER,
                    thumbnail BLOB
                )",
                (),
            )
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

    pub fn insert_entry(&self, entry: &InsertionEntry) -> rusqlite::Result<()> {
        self.conn
            .execute(
                "INSERT INTO media (path, timestamp, thumbnail) VALUES (?1, ?2, ?3)",
                (entry.path, entry.timestamp, entry.thumbnail),
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
    pub path: &'a str,
    pub timestamp: i64,
    pub thumbnail: &'a [u8],
}
