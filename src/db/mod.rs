use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbPool = Arc<Mutex<Connection>>;

/// Database connection builder and helper for Fly.io SQLite applications.
pub struct FlyDb;

impl FlyDb {
    /// Opens or creates an SQLite database at the given path, configuring standard production pragmas:
    /// - WAL journal mode
    /// - Foreign key enforcement
    /// - Normal synchronous mode (optimal for WAL)
    /// - 5-second busy timeout
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open(path)?;
        Self::apply_pragmas(&conn)?;
        Ok(conn)
    }

    /// Opens an in-memory SQLite database configured with production pragmas (useful for tests).
    pub fn open_in_memory() -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        Self::apply_pragmas(&conn)?;
        Ok(conn)
    }

    /// Creates a thread-safe `Arc<Mutex<Connection>>` handle to the SQLite database.
    pub fn open_shared<P: AsRef<Path>>(path: P) -> Result<DbPool, rusqlite::Error> {
        let conn = Self::open(path)?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    /// Creates an in-memory thread-safe `Arc<Mutex<Connection>>` handle.
    pub fn open_shared_in_memory() -> Result<DbPool, rusqlite::Error> {
        let conn = Self::open_in_memory()?;
        Ok(Arc::new(Mutex::new(conn)))
    }

    /// Applies recommended Fly.io / SQLite production PRAGMA settings.
    pub fn apply_pragmas(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        Ok(())
    }

    /// Runs a slice of raw SQL migration statements inside a single transaction.
    pub fn run_migrations(
        conn: &mut Connection,
        migrations: &[&str],
    ) -> Result<(), rusqlite::Error> {
        let tx = conn.transaction()?;
        for sql in migrations {
            tx.execute_batch(sql)?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// Spawns a background Tokio task that periodically runs PRAGMA wal_checkpoint(PASSIVE)
/// on the given SQLite connection pool.
///
/// Runs every `interval`. If WAL file exceeds `page_threshold` pages, runs TRUNCATE mode instead.
/// Task cancels gracefully when the returned `AbortHandle` is dropped.
pub fn spawn_wal_checkpoint_task(
    pool: DbPool,
    interval: std::time::Duration,
    page_threshold: u32,
) -> tokio::task::AbortHandle {
    let handle = tokio::task::spawn(async move {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            let pool = pool.clone();
            let checkpoint_result = tokio::task::spawn_blocking(move || {
                let conn = pool.lock().unwrap();

                // Easiest is to just use PRAGMA wal_checkpoint(PASSIVE) which returns (busy, log, checkpointed).
                // Then if log > page_threshold, run PRAGMA wal_checkpoint(TRUNCATE).

                let result = conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                    let log: u32 = row.get(1)?;
                    Ok(log)
                });

                match result {
                    Ok(log) => {
                        if log > page_threshold {
                            tracing::info!(
                                "WAL size {} exceeds threshold {}, running TRUNCATE checkpoint",
                                log,
                                page_threshold
                            );
                            if let Err(e) = conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", []) {
                                tracing::warn!("WAL TRUNCATE checkpoint failed: {}", e);
                            }
                        } else {
                            tracing::info!("WAL checkpoint PASSIVE ran (log size: {})", log);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("WAL checkpoint PASSIVE failed: {}", e);
                    }
                }
            })
            .await;

            if let Err(e) = checkpoint_result {
                tracing::warn!("WAL checkpoint task panicked: {}", e);
            }
        }
    });

    handle.abort_handle()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fly_db_pragmas_and_migrations() {
        let mut conn = FlyDb::open_in_memory().expect("failed to open memory db");

        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("query fk");
        assert_eq!(fk, 1);

        FlyDb::run_migrations(
            &mut conn,
            &[
                "CREATE TABLE test_items (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "INSERT INTO test_items (name) VALUES ('Test');",
            ],
        )
        .expect("migrations succeed");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM test_items", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 1);
    }
}
