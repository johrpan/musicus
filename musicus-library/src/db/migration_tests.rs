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
    assert_eq!(conn.applied_migrations().unwrap().len(), MIGRATION_COUNT);

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
    assert_eq!(conn.applied_migrations().unwrap().len(), MIGRATION_COUNT);
}

#[derive(QueryableByName)]
struct TextRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    value: String,
}

#[derive(QueryableByName)]
struct IntRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    value: i32,
}

/// Apply every migration except the last one, so that data can be seeded in the
/// shape the last migration expects to convert.
fn conn_before_normalize_schema() -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();

    for _ in 0..MIGRATION_COUNT - 1 {
        conn.run_next_migration(MIGRATIONS).unwrap();
    }

    conn
}

/// Timestamps written as local time must come out as the equivalent UTC
/// instant, not be reinterpreted as if they had always been UTC.
#[test]
fn normalize_schema_converts_timestamps_to_utc() {
    let mut conn = conn_before_normalize_schema();

    sql_query(
        "INSERT INTO persons (person_id, name, created_at, edited_at, last_used_at, last_played_at) \
         VALUES ('person-1', '{\"generic\":\"Test\"}', \
                 '2025-06-01 12:00:00', '2025-06-01 12:00:00', \
                 '2025-06-01 12:00:00', '2025-06-01 12:00:00')",
    )
    .execute(&mut conn)
    .unwrap();

    // What the old local-time value corresponds to in UTC, on this machine.
    let expected: TextRow = sql_query("SELECT DATETIME('2025-06-01 12:00:00', 'utc') AS value")
        .get_result(&mut conn)
        .unwrap();

    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let actual: TextRow =
        sql_query("SELECT last_played_at AS value FROM persons WHERE person_id = 'person-1'")
            .get_result(&mut conn)
            .unwrap();

    assert_eq!(actual.value, expected.value);
}

/// Mediums sharing a discid are merged into the lowest medium_id, and
/// everything referencing the losers is repointed at the survivor.
#[test]
fn normalize_schema_merges_duplicate_discids() {
    let mut conn = conn_before_normalize_schema();

    sql_query(
        "INSERT INTO mediums (medium_id, discid) VALUES ('medium-a', 'dup'), ('medium-b', 'dup')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query(
        "INSERT INTO works (work_id, name) VALUES ('work-1', '{\"generic\":\"W\"}'); \
         INSERT INTO recordings (recording_id, work_id) VALUES ('recording-1', 'work-1'); \
         INSERT INTO albums (album_id, name) VALUES ('album-1', '{\"generic\":\"A\"}');",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query(
        "INSERT INTO tracks (track_id, recording_id, recording_index, medium_id, path) \
         VALUES ('track-1', 'recording-1', 0, 'medium-b', '[\"t.mp3\"]')",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query(
        "INSERT INTO album_mediums (album_id, medium_id, sequence_number) \
         VALUES ('album-1', 'medium-a', 0), ('album-1', 'medium-b', 1)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let mediums: CountRow = sql_query("SELECT COUNT(*) AS count FROM mediums")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(mediums.count, 1, "duplicate discids should be merged");

    let track: TextRow =
        sql_query("SELECT medium_id AS value FROM tracks WHERE track_id = 'track-1'")
            .get_result(&mut conn)
            .unwrap();
    assert_eq!(
        track.value, "medium-a",
        "track should point at the survivor"
    );

    // Both album entries collapsed onto the same medium, so only one remains.
    let album_mediums: CountRow = sql_query("SELECT COUNT(*) AS count FROM album_mediums")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(album_mediums.count, 1);
}

/// Sequence numbers are renumbered from zero per owner, repairing gaps left by
/// earlier migrations while preserving the existing order.
#[test]
fn normalize_schema_renumbers_sequence_numbers() {
    let mut conn = conn_before_normalize_schema();

    sql_query(
        "INSERT INTO works (work_id, name) VALUES ('work-1', '{\"generic\":\"W\"}'); \
         INSERT INTO persons (person_id, name) VALUES \
             ('person-a', '{\"generic\":\"A\"}'), ('person-b', '{\"generic\":\"B\"}');",
    )
    .execute(&mut conn)
    .unwrap();
    sql_query(
        "INSERT INTO work_persons (work_id, person_id, sequence_number) \
         VALUES ('work-1', 'person-b', 7), ('work-1', 'person-a', 3)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.run_pending_migrations(MIGRATIONS).unwrap();

    for (person, expected) in [("person-a", 0), ("person-b", 1)] {
        let row: IntRow = sql_query(format!(
            "SELECT sequence_number AS value FROM work_persons \
             WHERE work_id = 'work-1' AND person_id = '{person}'"
        ))
        .get_result(&mut conn)
        .unwrap();
        assert_eq!(
            row.value, expected,
            "{person} should keep its relative order"
        );
    }
}
