-- Every migration that changes the schema must bump schema_version.

CREATE TABLE meta (
    -- Constrained to a single row.
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO meta (id, schema_version) VALUES (1, 1);

CREATE TABLE persons (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE roles (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE instruments (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE ensembles (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE works (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    relates_to TEXT REFERENCES works(work_id) ON DELETE SET NULL
);

CREATE INDEX works_parent_work_id ON works (parent_work_id);
CREATE INDEX works_relates_to ON works (relates_to);

CREATE TABLE recordings (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE RESTRICT,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    comment TEXT
);

CREATE INDEX recordings_work_id ON recordings (work_id);

CREATE TABLE albums (
    album_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE tracks (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE INDEX tracks_recording_id ON tracks (recording_id);

CREATE TABLE tags (
    tag_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    takes_value BOOLEAN NOT NULL DEFAULT FALSE,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    private BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE work_persons (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

CREATE INDEX work_persons_person_id ON work_persons (person_id);
CREATE INDEX work_persons_role_id ON work_persons (role_id);

CREATE TABLE work_instruments (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    instrument_id TEXT NOT NULL REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

CREATE INDEX work_instruments_instrument_id ON work_instruments (instrument_id);

CREATE TABLE work_tags (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

CREATE INDEX work_tags_tag_id ON work_tags (tag_id);

CREATE TABLE ensemble_persons (
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    PRIMARY KEY (ensemble_id, sequence_number)
);

CREATE INDEX ensemble_persons_person_id ON ensemble_persons (person_id);
CREATE INDEX ensemble_persons_instrument_id ON ensemble_persons (instrument_id);
CREATE INDEX ensemble_persons_role_id ON ensemble_persons (role_id);

CREATE TABLE recording_persons (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    instrument_id TEXT REFERENCES instruments(instrument_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

CREATE INDEX recording_persons_person_id ON recording_persons (person_id);
CREATE INDEX recording_persons_role_id ON recording_persons (role_id);
CREATE INDEX recording_persons_instrument_id ON recording_persons (instrument_id);

CREATE TABLE recording_ensembles (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE RESTRICT,
    role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

CREATE INDEX recording_ensembles_ensemble_id ON recording_ensembles (ensemble_id);
CREATE INDEX recording_ensembles_role_id ON recording_ensembles (role_id);

CREATE TABLE recording_tags (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

CREATE INDEX recording_tags_tag_id ON recording_tags (tag_id);

CREATE TABLE track_works (
    track_id TEXT NOT NULL REFERENCES tracks(track_id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (track_id, sequence_number)
);

CREATE INDEX track_works_work_id ON track_works (work_id);

CREATE TABLE album_recordings (
    album_id TEXT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, sequence_number)
);

CREATE INDEX album_recordings_recording_id ON album_recordings (recording_id);

CREATE TABLE plays (
    play_id TEXT NOT NULL PRIMARY KEY,
    track_id TEXT REFERENCES tracks(track_id) ON DELETE SET NULL,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    played_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE INDEX plays_track_id ON plays (track_id);
CREATE INDEX plays_recording_id ON plays (recording_id, played_at);
CREATE INDEX plays_played_at ON plays (played_at);

CREATE VIEW recording_last_played AS
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
