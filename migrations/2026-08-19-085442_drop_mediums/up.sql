DROP VIEW medium_last_played;

CREATE TABLE tracks_new (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE RESTRICT,
    recording_index INTEGER NOT NULL,
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

DROP TABLE album_mediums;
DROP TABLE mediums;

UPDATE meta SET schema_version = 8, updated_at = DATETIME('now') WHERE id = 1;
