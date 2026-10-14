use rusqlite::Connection;

pub(crate) fn compact(connection: &Connection) -> Result<(), String> {
    let pages: u64 = connection.pragma_query_value(None, "page_count", |row| row.get(0)).map_err(|error| error.to_string())?;
    let free: u64 = connection.pragma_query_value(None, "freelist_count", |row| row.get(0)).map_err(|error| error.to_string())?;
    if free >= 64 && free.saturating_mul(4) >= pages {
        connection.execute_batch("VACUUM").map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclaims_unused_pages_without_changing_remaining_records() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.sqlite3");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TABLE records(id INTEGER PRIMARY KEY, content BLOB NOT NULL);
            WITH RECURSIVE numbers(id) AS (VALUES(1) UNION ALL SELECT id+1 FROM numbers WHERE id<1000)
            INSERT INTO records SELECT id,randomblob(4096) FROM numbers;").unwrap();
        let before = std::fs::metadata(&path).unwrap().len();
        connection.execute("DELETE FROM records WHERE id>50", []).unwrap();
        compact(&connection).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() < before / 4);
        assert_eq!(connection.query_row("SELECT count(*) FROM records", [], |row| row.get::<_, i64>(0)).unwrap(), 50);
    }
}
