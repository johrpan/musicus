-- Recreate the medium/CD tables. Their previous contents are gone: `up.sql`
-- deleted the rows, and there is no record of what they were.

CREATE TABLE mediums (
    medium_id TEXT NOT NULL PRIMARY KEY,
    discid TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE album_mediums (
    album_id TEXT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    medium_id TEXT NOT NULL REFERENCES mediums(medium_id) ON DELETE RESTRICT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, sequence_number)
);

CREATE INDEX album_mediums_medium_id ON album_mediums (medium_id);

CREATE TABLE tracks_new (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL,
    medium_id TEXT REFERENCES mediums(medium_id) ON DELETE RESTRICT,
    medium_index INTEGER,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO tracks_new (track_id, recording_id, recording_index, path, created_at, edited_at, last_used_at)
SELECT track_id, recording_id, recording_index, path, created_at, edited_at, last_used_at
FROM tracks;

DROP TABLE tracks;
ALTER TABLE tracks_new RENAME TO tracks;

CREATE INDEX tracks_recording_id ON tracks (recording_id);
CREATE INDEX tracks_medium_id ON tracks (medium_id);

CREATE VIEW medium_last_played AS
SELECT tracks.medium_id AS medium_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN tracks ON tracks.track_id = plays.track_id
WHERE tracks.medium_id IS NOT NULL
GROUP BY tracks.medium_id;

UPDATE meta SET schema_version = 7, updated_at = DATETIME('now') WHERE id = 1;
