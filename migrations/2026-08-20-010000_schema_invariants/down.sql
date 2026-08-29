-- Roll back to the schema as the work-relations migration left it: flat or
-- JSON-array track paths anywhere, unconstrained sequence numbers, and no
-- ordering constraints on works.

DROP VIEW recording_last_played;
DROP VIEW track_last_played;
DROP VIEW work_last_played;
DROP VIEW person_last_played;
DROP VIEW performer_last_played;
DROP VIEW instrument_last_played;
DROP VIEW ensemble_last_played;
DROP VIEW album_last_played;

UPDATE tracks SET path = CASE
    WHEN path LIKE 'tracks/%' THEN substr(path, length('tracks/') + 1)
    ELSE path
END;

CREATE TABLE works_old (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO works_old SELECT * FROM works;
DROP TABLE works;
ALTER TABLE works_old RENAME TO works;

CREATE INDEX works_parent_work_id ON works (parent_work_id);

CREATE TABLE tracks_old (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO tracks_old SELECT * FROM tracks;
DROP TABLE tracks;
ALTER TABLE tracks_old RENAME TO tracks;

CREATE INDEX tracks_recording_id ON tracks (recording_id);

UPDATE meta SET schema_version = 2, updated_at = DATETIME('now') WHERE id = 1;

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