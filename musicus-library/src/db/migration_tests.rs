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

/// How many migrations precede `normalize_schema` and `tags` respectively.
///
/// Pinned rather than derived from [`MIGRATION_COUNT`], so that adding a later
/// migration cannot silently move these helpers past the migration under test
/// and seed data in the wrong schema.
const MIGRATIONS_BEFORE_NORMALIZE_SCHEMA: usize = 7;
const MIGRATIONS_BEFORE_TAGS: usize = 8;
const MIGRATIONS_BEFORE_TAG_PRIVATE: usize = 9;

fn conn_after_migrations(count: usize) -> SqliteConnection {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();

    for _ in 0..count {
        conn.run_next_migration(MIGRATIONS).unwrap();
    }

    conn
}

/// Apply every migration before `normalize_schema`, so that data can be seeded
/// in the shape that migration expects to convert.
fn conn_before_normalize_schema() -> SqliteConnection {
    conn_after_migrations(MIGRATIONS_BEFORE_NORMALIZE_SCHEMA)
}

/// Apply every migration before `tags`, while `recordings.year` still exists.
fn conn_before_tags() -> SqliteConnection {
    conn_after_migrations(MIGRATIONS_BEFORE_TAGS)
}

/// Existing tags become ordinary, non-private tags, and reverting the migration
/// leaves them intact without the column.
#[test]
fn tag_private_migration_keeps_existing_tags() {
    let mut conn = conn_after_migrations(MIGRATIONS_BEFORE_TAG_PRIVATE);

    sql_query("INSERT INTO tags (tag_id, name) VALUES ('tag-1', '{\"generic\":\"Baroque\"}')")
        .execute(&mut conn)
        .unwrap();

    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let private: IntRow = sql_query("SELECT private AS value FROM tags WHERE tag_id = 'tag-1'")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(private.value, 0, "an existing tag must not become private");

    conn.revert_last_migration(MIGRATIONS).unwrap();

    let name: TextRow = sql_query("SELECT name AS value FROM tags WHERE tag_id = 'tag-1'")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(name.value, "{\"generic\":\"Baroque\"}");
}

/// The recording year moves into the built-in Year tag, and comes back out of
/// it when the migration is reverted.
#[test]
fn tags_migration_moves_the_recording_year() {
    let mut conn = conn_before_tags();

    sql_query("INSERT INTO works (work_id, name) VALUES ('work-1', '{\"generic\":\"Test\"}')")
        .execute(&mut conn)
        .unwrap();
    sql_query(
        "INSERT INTO recordings (recording_id, work_id, year) \
         VALUES ('recording-1', 'work-1', 1963), ('recording-2', 'work-1', NULL)",
    )
    .execute(&mut conn)
    .unwrap();

    conn.run_pending_migrations(MIGRATIONS).unwrap();

    let value: TextRow = sql_query(format!(
        "SELECT value FROM recording_tags \
         WHERE recording_id = 'recording-1' AND tag_id = '{TAG_YEAR}'"
    ))
    .get_result(&mut conn)
    .unwrap();
    assert_eq!(value.value, "1963");

    // A recording without a year gets no assignment at all.
    let count: CountRow = sql_query(
        "SELECT COUNT(*) AS count FROM recording_tags WHERE recording_id = 'recording-2'",
    )
    .get_result(&mut conn)
    .unwrap();
    assert_eq!(count.count, 0);

    // The Year tag is seeded as a valued tag, so the UI offers a value field.
    let takes_value: IntRow = sql_query(format!(
        "SELECT takes_value AS value FROM tags WHERE tag_id = '{TAG_YEAR}'"
    ))
    .get_result(&mut conn)
    .unwrap();
    assert_eq!(takes_value.value, 1);

    // Back out everything applied after `tags`, so that reverting `tags` itself
    // is what restores the year column.
    for _ in MIGRATIONS_BEFORE_TAGS + 1..=MIGRATION_COUNT {
        conn.revert_last_migration(MIGRATIONS).unwrap();
    }

    let year: IntRow =
        sql_query("SELECT year AS value FROM recordings WHERE recording_id = 'recording-1'")
            .get_result(&mut conn)
            .unwrap();
    assert_eq!(year.value, 1963);
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
