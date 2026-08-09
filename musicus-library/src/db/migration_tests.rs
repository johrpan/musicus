//! A single round-trip test that walks the entire migration history at once, applying all
//! of them, reverting all of them, and re-applying them again.

use diesel::{sql_query, QueryableByName};

use super::*;

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(QueryableByName)]
struct PathRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    path: String,
}

const ENTITY_ROWS: &[(&str, &str, &str)] = &[
    ("persons", "person_id", "person-1"),
    ("roles", "role_id", "role-1"),
    ("instruments", "instrument_id", "instrument-1"),
    ("works", "work_id", "work-1"),
    ("ensembles", "ensemble_id", "ensemble-1"),
    ("recordings", "recording_id", "recording-1"),
    ("tracks", "track_id", "track-1"),
    ("mediums", "medium_id", "medium-1"),
    ("albums", "album_id", "album-1"),
];

#[test]
fn full_migration_history_round_trips() {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();

    // Apply just the base schema, then insert one row per entity table.
    conn.run_next_migration(MIGRATIONS).unwrap();
    sql_query(
        "INSERT INTO persons (person_id, name) VALUES ('person-1', '{\"generic\":\"Test Person\"}')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query("INSERT INTO roles (role_id, name) VALUES ('role-1', '{\"generic\":\"Test Role\"}')")
        .execute(&mut conn)
        .unwrap();
    sql_query(
        "INSERT INTO instruments (instrument_id, name) VALUES ('instrument-1', '{\"generic\":\"Test Instrument\"}')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query("INSERT INTO works (work_id, name) VALUES ('work-1', '{\"generic\":\"Test Work\"}')")
        .execute(&mut conn)
        .unwrap();
    sql_query(
        "INSERT INTO ensembles (ensemble_id, name) VALUES ('ensemble-1', '{\"generic\":\"Test Ensemble\"}')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query("INSERT INTO recordings (recording_id, work_id) VALUES ('recording-1', 'work-1')")
        .execute(&mut conn)
        .unwrap();
    sql_query(
        "INSERT INTO tracks (track_id, recording_id, recording_index, path) \
         VALUES ('track-1', 'recording-1', 0, 'a/b/c.mp3')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query("INSERT INTO mediums (medium_id, discid) VALUES ('medium-1', 'test-discid')")
        .execute(&mut conn)
        .unwrap();
    sql_query(
        "INSERT INTO albums (album_id, name) VALUES ('album-1', '{\"generic\":\"Test Album\"}')",
    )
    .execute(&mut conn)
    .unwrap();

    // Apply the rest of the chain.
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    assert!(!conn.has_pending_migration(MIGRATIONS).unwrap());
    assert_eq!(conn.applied_migrations().unwrap().len(), 6);

    // Every row should have survived each table rebuild along the way.
    for (table, id_column, id) in ENTITY_ROWS {
        let row: CountRow = sql_query(format!(
            "SELECT COUNT(*) AS count FROM {table} WHERE {id_column} = '{id}'"
        ))
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(
            row.count, 1,
            "{table} row should survive the full migration chain"
        );
    }

    // The json_paths migration should have run: path was converted to a JSON array.
    let track_path: PathRow = sql_query("SELECT path FROM tracks WHERE track_id = 'track-1'")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(track_path.path, r#"["a","b","c.mp3"]"#);

    // Revert the entire history back to nothing...
    conn.revert_all_migrations(MIGRATIONS).unwrap();
    assert_eq!(conn.applied_migrations().unwrap().len(), 0);

    // ...and reapply, proving the whole chain is consistent in both directions.
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    assert_eq!(conn.applied_migrations().unwrap().len(), 6);
}
