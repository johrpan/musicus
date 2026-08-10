-- Normalise the schema before 1.0 freezes it.
--
-- All of the following need a table rebuild, so they are done in one pass
-- rather than in four separate migrations:
--
--  1. Timestamps are stored in UTC. They were previously naive local time
--     (Local::now().naive_local() in the code, DATETIME('now', 'localtime') as
--     the column default), which makes last_played_at non-monotonic across DST
--     transitions and machine relocations. generate_recording's scoring reads
--     these columns through UNIXEPOCH(), which assumes UTC, so the ordering it
--     produced was subtly wrong. Existing values are converted on the
--     assumption that they were written in local time, which is what the code
--     always did.
--
--  2. `source` is constrained to the values the code actually understands, so
--     an unrecognised value can no longer be written back as the literal
--     'unknown', silently destroying an item's provenance.
--
--  3. mediums.discid is unique. It is the natural key identifying a CD, and
--     duplicates were free to accumulate. Existing duplicates are merged into
--     the lowest medium_id and the references are rewritten.
--
--  4. Ordered relationship tables are keyed by (owner, sequence_number)
--     instead of carrying sequence_number as a tie-breaker inside a wider
--     primary key, so reordering cannot collide. Sequence numbers are
--     renumbered from zero per owner, which also repairs any gaps or
--     duplicates left behind by earlier INSERT OR IGNORE migrations.
--
--  5. Every foreign key has an explicit ON DELETE action, and every foreign key
--     column is indexed. There were previously no indices at all.
--
-- Column lists are written out explicitly rather than using SELECT *, so that a
-- future column reordering cannot silently shift data between columns.

-- Mediums that share a discid are merged into the one with the lowest
-- medium_id. The remapping happens while copying into the rebuilt tables rather
-- than as an UPDATE on the old ones, so no statement has to satisfy the old
-- primary keys along the way.

-- Entity tables.

CREATE TABLE persons_new (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO persons_new (person_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT person_id, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM persons;

CREATE TABLE roles_new (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO roles_new (role_id, name, source, enable_updates, created_at, edited_at, last_used_at)
SELECT role_id, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc')
FROM roles;

CREATE TABLE instruments_new (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO instruments_new (instrument_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT instrument_id, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM instruments;

CREATE TABLE ensembles_new (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO ensembles_new (ensemble_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT ensemble_id, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM ensembles;

CREATE TABLE mediums_new (
    medium_id TEXT NOT NULL PRIMARY KEY,
    discid TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

-- Only the surviving medium of each discid.
INSERT INTO mediums_new (medium_id, discid, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT medium_id, discid,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM mediums m
WHERE m.medium_id = (SELECT MIN(m2.medium_id) FROM mediums m2 WHERE m2.discid = m.discid);

-- Parts belong to their parent work, so deleting a work now deletes its parts
-- instead of failing.
CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works_new(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO works_new (work_id, parent_work_id, sequence_number, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT work_id, parent_work_id, sequence_number, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM works;

CREATE TABLE recordings_new (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works_new(work_id) ON DELETE RESTRICT,
    year INTEGER,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO recordings_new (recording_id, work_id, year, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT recording_id, work_id, year,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM recordings;

CREATE TABLE albums_new (
    album_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO albums_new (album_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT album_id, name,
       CASE WHEN source IN ('user', 'metadata', 'import') THEN source ELSE 'user' END,
       enable_updates,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM albums;

CREATE TABLE tracks_new (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings_new(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL,
    medium_id TEXT REFERENCES mediums_new(medium_id) ON DELETE RESTRICT,
    medium_index INTEGER,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_played_at TIMESTAMP
);

INSERT INTO tracks_new (track_id, recording_id, recording_index, medium_id, medium_index, path, created_at, edited_at, last_used_at, last_played_at)
SELECT track_id, recording_id, recording_index,
       (SELECT MIN(m2.medium_id) FROM mediums m2
        WHERE m2.discid = (SELECT m.discid FROM mediums m WHERE m.medium_id = t.medium_id)),
       medium_index, path,
       DATETIME(created_at, 'utc'), DATETIME(edited_at, 'utc'), DATETIME(last_used_at, 'utc'),
       DATETIME(last_played_at, 'utc')
FROM tracks t;

-- Ordered relationship tables, keyed by (owner, sequence_number).

CREATE TABLE work_persons_new (
    work_id TEXT NOT NULL REFERENCES works_new(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_new(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles_new(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

INSERT INTO work_persons_new (work_id, person_id, role_id, sequence_number)
SELECT work_id, person_id, role_id,
       ROW_NUMBER() OVER (PARTITION BY work_id ORDER BY sequence_number, person_id) - 1
FROM work_persons;

CREATE TABLE work_instruments_new (
    work_id TEXT NOT NULL REFERENCES works_new(work_id) ON DELETE CASCADE,
    instrument_id TEXT NOT NULL REFERENCES instruments_new(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

INSERT INTO work_instruments_new (work_id, instrument_id, sequence_number)
SELECT work_id, instrument_id,
       ROW_NUMBER() OVER (PARTITION BY work_id ORDER BY sequence_number, instrument_id) - 1
FROM work_instruments;

CREATE TABLE ensemble_persons_new (
    ensemble_id TEXT NOT NULL REFERENCES ensembles_new(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_new(person_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments_new(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (ensemble_id, sequence_number)
);

INSERT INTO ensemble_persons_new (ensemble_id, person_id, instrument_id, sequence_number)
SELECT ensemble_id, person_id, instrument_id,
       ROW_NUMBER() OVER (PARTITION BY ensemble_id ORDER BY sequence_number, person_id) - 1
FROM ensemble_persons;

CREATE TABLE recording_persons_new (
    recording_id TEXT NOT NULL REFERENCES recordings_new(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_new(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles_new(role_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments_new(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

INSERT INTO recording_persons_new (recording_id, person_id, role_id, instrument_id, sequence_number)
SELECT recording_id, person_id, role_id, instrument_id,
       ROW_NUMBER() OVER (PARTITION BY recording_id ORDER BY sequence_number, person_id) - 1
FROM recording_persons;

CREATE TABLE recording_ensembles_new (
    recording_id TEXT NOT NULL REFERENCES recordings_new(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles_new(ensemble_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles_new(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

INSERT INTO recording_ensembles_new (recording_id, ensemble_id, role_id, sequence_number)
SELECT recording_id, ensemble_id, role_id,
       ROW_NUMBER() OVER (PARTITION BY recording_id ORDER BY sequence_number, ensemble_id) - 1
FROM recording_ensembles;

CREATE TABLE track_works_new (
    track_id TEXT NOT NULL REFERENCES tracks_new(track_id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works_new(work_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (track_id, sequence_number)
);

INSERT INTO track_works_new (track_id, work_id, sequence_number)
SELECT track_id, work_id,
       ROW_NUMBER() OVER (PARTITION BY track_id ORDER BY sequence_number, work_id) - 1
FROM track_works;

CREATE TABLE album_recordings_new (
    album_id TEXT NOT NULL REFERENCES albums_new(album_id) ON DELETE CASCADE,
    recording_id TEXT NOT NULL REFERENCES recordings_new(recording_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, sequence_number)
);

INSERT INTO album_recordings_new (album_id, recording_id, sequence_number)
SELECT album_id, recording_id,
       ROW_NUMBER() OVER (PARTITION BY album_id ORDER BY sequence_number, recording_id) - 1
FROM album_recordings;

CREATE TABLE album_mediums_new (
    album_id TEXT NOT NULL REFERENCES albums_new(album_id) ON DELETE CASCADE,
    medium_id TEXT NOT NULL REFERENCES mediums_new(medium_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, sequence_number)
);

-- Remapping merged mediums can make an album reference the same medium twice,
-- so distinct pairs are selected before numbering them.
INSERT INTO album_mediums_new (album_id, medium_id, sequence_number)
SELECT album_id, medium_id,
       ROW_NUMBER() OVER (PARTITION BY album_id ORDER BY sequence_number, medium_id) - 1
FROM (
    SELECT album_id, medium_id, MIN(sequence_number) AS sequence_number
    FROM (
        SELECT am.album_id AS album_id,
               (SELECT MIN(m2.medium_id) FROM mediums m2
                WHERE m2.discid = (SELECT m.discid FROM mediums m WHERE m.medium_id = am.medium_id))
                   AS medium_id,
               am.sequence_number AS sequence_number
        FROM album_mediums am
    )
    GROUP BY album_id, medium_id
);

-- Swap the rebuilt tables in. Children first so that no rename resolves a
-- reference to the table it is replacing.

DROP TABLE album_mediums;
DROP TABLE album_recordings;
DROP TABLE track_works;
DROP TABLE recording_ensembles;
DROP TABLE recording_persons;
DROP TABLE ensemble_persons;
DROP TABLE work_instruments;
DROP TABLE work_persons;
DROP TABLE tracks;
DROP TABLE albums;
DROP TABLE recordings;
DROP TABLE works;
DROP TABLE mediums;
DROP TABLE ensembles;
DROP TABLE instruments;
DROP TABLE roles;
DROP TABLE persons;

ALTER TABLE persons_new RENAME TO persons;
ALTER TABLE roles_new RENAME TO roles;
ALTER TABLE instruments_new RENAME TO instruments;
ALTER TABLE ensembles_new RENAME TO ensembles;
ALTER TABLE mediums_new RENAME TO mediums;
ALTER TABLE works_new RENAME TO works;
ALTER TABLE recordings_new RENAME TO recordings;
ALTER TABLE albums_new RENAME TO albums;
ALTER TABLE tracks_new RENAME TO tracks;
ALTER TABLE work_persons_new RENAME TO work_persons;
ALTER TABLE work_instruments_new RENAME TO work_instruments;
ALTER TABLE ensemble_persons_new RENAME TO ensemble_persons;
ALTER TABLE recording_persons_new RENAME TO recording_persons;
ALTER TABLE recording_ensembles_new RENAME TO recording_ensembles;
ALTER TABLE track_works_new RENAME TO track_works;
ALTER TABLE album_recordings_new RENAME TO album_recordings;
ALTER TABLE album_mediums_new RENAME TO album_mediums;

-- Index every foreign key column that is not already the leading column of the
-- primary key. Searching and playlist generation join five to seven tables and
-- none of these were indexed.

CREATE INDEX works_parent_work_id ON works (parent_work_id);
CREATE INDEX recordings_work_id ON recordings (work_id);
CREATE INDEX tracks_recording_id ON tracks (recording_id);
CREATE INDEX tracks_medium_id ON tracks (medium_id);
CREATE INDEX work_persons_person_id ON work_persons (person_id);
CREATE INDEX work_persons_role_id ON work_persons (role_id);
CREATE INDEX work_instruments_instrument_id ON work_instruments (instrument_id);
CREATE INDEX ensemble_persons_person_id ON ensemble_persons (person_id);
CREATE INDEX ensemble_persons_instrument_id ON ensemble_persons (instrument_id);
CREATE INDEX recording_persons_person_id ON recording_persons (person_id);
CREATE INDEX recording_persons_role_id ON recording_persons (role_id);
CREATE INDEX recording_persons_instrument_id ON recording_persons (instrument_id);
CREATE INDEX recording_ensembles_ensemble_id ON recording_ensembles (ensemble_id);
CREATE INDEX recording_ensembles_role_id ON recording_ensembles (role_id);
CREATE INDEX track_works_work_id ON track_works (work_id);
CREATE INDEX album_recordings_recording_id ON album_recordings (recording_id);
CREATE INDEX album_mediums_medium_id ON album_mediums (medium_id);

UPDATE library_meta SET schema_version = 2, updated_at = DATETIME('now') WHERE id = 1;
