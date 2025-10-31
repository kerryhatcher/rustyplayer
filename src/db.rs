use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub struct DB {
    conn: Connection,
}

/// Database operations for media library
impl DB {
    /// Open or create the database at the given path and run minimal migrations.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tracks (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                title TEXT,
                artist TEXT,
                album TEXT,
                duration_seconds INTEGER,
                added_at INTEGER,
                play_count INTEGER DEFAULT 0,
                last_played INTEGER
            );",
        )?;
        Ok(Self { conn })
    }

    /// Get the total number of tracks in the library
    pub fn track_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tracks",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }
}
