use super::*;

#[test]
fn current_schema_has_one_request_owner_and_no_session_replay_cursor() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize(&mut connection).unwrap();
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    assert_eq!(version, 3);
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_schema WHERE name='codex_session_sync'", [], |row| row.get(0)).unwrap(), 0);
}

#[test]
fn existing_v2_ledger_drops_only_the_abandoned_derived_cursor() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(&SCHEMA.replace("PRAGMA user_version = 3;", "CREATE TABLE codex_session_sync(path TEXT); PRAGMA user_version=2;")).unwrap();
    connection.execute("INSERT INTO codex_requests(id,at_ms,payload) VALUES('saved',1,'{}')", []).unwrap();
    initialize(&mut connection).unwrap();
    assert_eq!(connection.query_row::<String,_,_>("SELECT payload FROM codex_requests WHERE id='saved'", [], |row| row.get(0)).unwrap(), "{}");
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_schema WHERE name='codex_session_sync'", [], |row| row.get(0)).unwrap(), 0);
    initialize(&mut connection).unwrap();
}

#[test]
fn v1_upgrade_preserves_payload_facts_and_adds_the_current_origin_once() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection.execute_batch("CREATE TABLE codex_requests(id TEXT PRIMARY KEY,payload TEXT); \
        INSERT INTO codex_requests VALUES('saved','{\"id\":\"saved\",\"inputTokens\":17}'); PRAGMA user_version=1;").unwrap();
    initialize(&mut connection).unwrap();
    let raw: String = connection.query_row("SELECT payload FROM codex_requests", [], |row| row.get(0)).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(payload["inputTokens"], 17);
    assert_eq!(payload["origin"], "proxy");
    initialize(&mut connection).unwrap();
}
