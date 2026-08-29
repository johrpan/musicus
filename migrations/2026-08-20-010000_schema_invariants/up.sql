-- Every migration that changes the schema must bump schema_version.

-- Detect existing violations before rebuilding, and fail with a clear
-- message instead of an opaque constraint error. SQLite only allows RAISE()
-- inside triggers, so the checks force a CAST of the message to REAL, whose
-- failure message carries the text. The whole migration runs in one
-- transaction, so nothing is changed on failure.

-- Track paths: legacy rows store either a JSON array of host path components
-- (e.g. '["recording_00.mp3"]') or a plain flat file name next to the
-- database. Anything else is unsafe or ambiguous.
SELECT CASE WHEN EXISTS (
    SELECT 1 FROM tracks WHERE
        path = '' OR
        path LIKE '/%' OR
        instr(path, '\') > 0 OR
        path = '..' OR path LIKE '../%' OR path LIKE '%/../%' OR path LIKE '%/..' OR
        path = '.' OR path LIKE './%' OR path LIKE '%/./%' OR path LIKE '%/.' OR
        (path LIKE '[%' AND json_valid(path) = 0) OR
        (path NOT LIKE '[%' AND path NOT LIKE 'tracks/%' AND instr(path, '/') > 0)
) THEN CAST('A track path in this library is unsafe or ambiguous (absolute, ".." or "." component, empty path, non-JSON multi-component path, or malformed JSON path) and cannot be migrated automatically' AS REAL) END;

SELECT CASE WHEN EXISTS (
    SELECT 1 FROM tracks WHERE recording_index < 0
) THEN CAST('A track has a negative recording index; the library cannot be migrated automatically' AS REAL) END;

SELECT CASE WHEN EXISTS (
    SELECT 1 FROM tracks
    GROUP BY recording_id, recording_index
    HAVING COUNT(*) > 1
) THEN CAST('Two tracks of one recording share a recording index; the library cannot be migrated automatically' AS REAL) END;

-- Canonicalize every track path: portable slash-separated relative paths with
-- managed media under tracks/.
UPDATE tracks SET path = CASE
    WHEN path LIKE 'tracks/%' THEN path
    WHEN path LIKE '[%' THEN 'tracks/' || json_extract(path, '$[0]')
    ELSE 'tracks/' || path
END;

SELECT CASE WHEN EXISTS (
    SELECT 1 FROM tracks
    GROUP BY path
    HAVING COUNT(*) > 1
) THEN CAST('Two tracks share the same file path after conversion; the library cannot be migrated automatically' AS REAL) END;

-- Association sequence numbers must be nonnegative.
SELECT CASE WHEN EXISTS (
    SELECT sequence_number FROM work_persons WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM work_instruments WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM work_tags WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM ensemble_persons WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM recording_persons WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM recording_ensembles WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM recording_tags WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM track_works WHERE sequence_number < 0 UNION ALL
    SELECT sequence_number FROM album_recordings WHERE sequence_number < 0
) THEN CAST('An ordered association has a negative sequence number; the library cannot be migrated automatically' AS REAL) END;

-- Work hierarchy invariants.
SELECT CASE WHEN EXISTS (
    SELECT 1 FROM works WHERE work_id = parent_work_id
) THEN CAST('A work is its own parent; the library cannot be migrated automatically' AS REAL) END;

SELECT CASE WHEN EXISTS (
    SELECT 1 FROM works
    WHERE (parent_work_id IS NULL) != (sequence_number IS NULL)
) THEN CAST('A work has a sequence number without a parent or a parent without a sequence number; the library cannot be migrated automatically' AS REAL) END;

SELECT CASE WHEN EXISTS (
    SELECT parent_work_id, sequence_number FROM works
    WHERE parent_work_id IS NOT NULL
    GROUP BY parent_work_id, sequence_number
    HAVING COUNT(*) > 1
) THEN CAST('Two child works of one parent share a sequence number; the library cannot be migrated automatically' AS REAL) END;

-- The views reference the tables being rebuilt, and SQLite cannot rename a
-- table in the same transaction while a view still depends on it. Drop them
-- first and recreate them unchanged at the end.
DROP VIEW recording_last_played;
DROP VIEW track_last_played;
DROP VIEW work_last_played;
DROP VIEW person_last_played;
DROP VIEW performer_last_played;
DROP VIEW instrument_last_played;
DROP VIEW ensemble_last_played;
DROP VIEW album_last_played;

-- Rebuild the ordered-association tables with a nonnegative-sequence check.
-- Their primary keys already give each owner a unique, ordered index.

CREATE TABLE work_persons_new (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (work_id, sequence_number)
);

INSERT INTO work_persons_new SELECT * FROM work_persons;
DROP TABLE work_persons;
ALTER TABLE work_persons_new RENAME TO work_persons;

CREATE INDEX work_persons_person_id ON work_persons (person_id);
CREATE INDEX work_persons_role_id ON work_persons (role_id);

CREATE TABLE work_instruments_new (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    instrument_id TEXT NOT NULL REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (work_id, sequence_number)
);

INSERT INTO work_instruments_new SELECT * FROM work_instruments;
DROP TABLE work_instruments;
ALTER TABLE work_instruments_new RENAME TO work_instruments;

CREATE INDEX work_instruments_instrument_id ON work_instruments (instrument_id);

CREATE TABLE work_tags_new (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (work_id, sequence_number)
);

INSERT INTO work_tags_new SELECT * FROM work_tags;
DROP TABLE work_tags;
ALTER TABLE work_tags_new RENAME TO work_tags;

CREATE INDEX work_tags_tag_id ON work_tags (tag_id);

CREATE TABLE ensemble_persons_new (
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    PRIMARY KEY (ensemble_id, sequence_number)
);

INSERT INTO ensemble_persons_new SELECT * FROM ensemble_persons;
DROP TABLE ensemble_persons;
ALTER TABLE ensemble_persons_new RENAME TO ensemble_persons;

CREATE INDEX ensemble_persons_person_id ON ensemble_persons (person_id);
CREATE INDEX ensemble_persons_instrument_id ON ensemble_persons (instrument_id);
CREATE INDEX ensemble_persons_role_id ON ensemble_persons (role_id);

CREATE TABLE recording_persons_new (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (recording_id, sequence_number)
);

INSERT INTO recording_persons_new SELECT * FROM recording_persons;
DROP TABLE recording_persons;
ALTER TABLE recording_persons_new RENAME TO recording_persons;

CREATE INDEX recording_persons_person_id ON recording_persons (person_id);
CREATE INDEX recording_persons_role_id ON recording_persons (role_id);
CREATE INDEX recording_persons_instrument_id ON recording_persons (instrument_id);

CREATE TABLE recording_ensembles_new (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (recording_id, sequence_number)
);

INSERT INTO recording_ensembles_new SELECT * FROM recording_ensembles;
DROP TABLE recording_ensembles;
ALTER TABLE recording_ensembles_new RENAME TO recording_ensembles;

CREATE INDEX recording_ensembles_ensemble_id ON recording_ensembles (ensemble_id);
CREATE INDEX recording_ensembles_role_id ON recording_ensembles (role_id);

CREATE TABLE recording_tags_new (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (recording_id, sequence_number)
);

INSERT INTO recording_tags_new SELECT * FROM recording_tags;
DROP TABLE recording_tags;
ALTER TABLE recording_tags_new RENAME TO recording_tags;

CREATE INDEX recording_tags_tag_id ON recording_tags (tag_id);

CREATE TABLE track_works_new (
    track_id TEXT NOT NULL REFERENCES tracks(track_id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (track_id, sequence_number)
);

INSERT INTO track_works_new SELECT * FROM track_works;
DROP TABLE track_works;
ALTER TABLE track_works_new RENAME TO track_works;

CREATE INDEX track_works_work_id ON track_works (work_id);

CREATE TABLE album_recordings_new (
    album_id TEXT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL CHECK (sequence_number >= 0),
    PRIMARY KEY (album_id, sequence_number)
);

INSERT INTO album_recordings_new SELECT * FROM album_recordings;
DROP TABLE album_recordings;
ALTER TABLE album_recordings_new RENAME TO album_recordings;

CREATE INDEX album_recordings_recording_id ON album_recordings (recording_id);

-- Rebuild tracks with its invariants: nonnegative indices, one index per
-- recording, no shared files between tracks.
CREATE TABLE tracks_new (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL CHECK (recording_index >= 0),
    path TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    CONSTRAINT tracks_recording_order UNIQUE (recording_id, recording_index)
);

INSERT INTO tracks_new (track_id, recording_id, recording_index, path, created_at, edited_at, last_used_at)
SELECT track_id, recording_id, recording_index, path, created_at, edited_at, last_used_at FROM tracks;

DROP TABLE tracks;
ALTER TABLE tracks_new RENAME TO tracks;

-- Rebuild works with its hierarchy invariants. The unique parent/sequence
-- pair also covers the ordered lookup the editors and list views use.
CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    CHECK (work_id <> parent_work_id),
    CHECK ((parent_work_id IS NULL) = (sequence_number IS NULL)),
    CONSTRAINT works_child_order UNIQUE (parent_work_id, sequence_number)
);

INSERT INTO works_new (work_id, parent_work_id, sequence_number, name, source, enable_updates,
    created_at, edited_at, last_used_at)
SELECT work_id, parent_work_id, sequence_number, name, source, enable_updates,
    created_at, edited_at, last_used_at
FROM works;

DROP TABLE works;
ALTER TABLE works_new RENAME TO works;

UPDATE meta SET schema_version = 3, updated_at = DATETIME('now') WHERE id = 1;CREATE VIEW recording_last_played AS
SELECT recording_id,
    MAX(played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
GROUP BY recording_id;

CREATE VIEW track_last_played AS
SELECT track_id,
    MAX(played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
WHERE track_id IS NOT NULL
GROUP BY track_id;

CREATE VIEW work_last_played AS
SELECT recordings.work_id AS work_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN recordings ON recordings.recording_id = plays.recording_id
GROUP BY recordings.work_id;

CREATE VIEW person_last_played AS
SELECT work_persons.person_id AS person_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN recordings ON recordings.recording_id = plays.recording_id
    JOIN work_persons ON work_persons.work_id = recordings.work_id
GROUP BY work_persons.person_id;

CREATE VIEW performer_last_played AS
SELECT recording_persons.person_id AS person_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN recording_persons ON recording_persons.recording_id = plays.recording_id
GROUP BY recording_persons.person_id;

CREATE VIEW instrument_last_played AS
SELECT work_instruments.instrument_id AS instrument_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN recordings ON recordings.recording_id = plays.recording_id
    JOIN work_instruments ON work_instruments.work_id = recordings.work_id
GROUP BY work_instruments.instrument_id;

CREATE VIEW ensemble_last_played AS
SELECT recording_ensembles.ensemble_id AS ensemble_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN recording_ensembles ON recording_ensembles.recording_id = plays.recording_id
GROUP BY recording_ensembles.ensemble_id;

CREATE VIEW album_last_played AS
SELECT album_recordings.album_id AS album_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN album_recordings ON album_recordings.recording_id = plays.recording_id
GROUP BY album_recordings.album_id;
