-- Playback statistics become an append-only event log.
--
-- Every entity used to carry a nullable `last_played_at` column, written by an
-- eight-statement fan-out on every track start. That shape could only ever
-- answer "when was this last played", never "how often" or "what did I listen
-- to last week", and it conflated two different questions on `persons`, whose
-- column was written both for a work's composers and for a recording's
-- performers but only ever read as "composer".
--
-- One row per play answers all of those, and lets an export drop the listening
-- history with a single DELETE instead of clearing eight columns.
--
-- `track_id` is nullable and does not cascade: reorganizing or replacing a file
-- should not erase the fact that it was listened to. `recording_id` is
-- denormalized onto the row so that the statistics survive that, and so that
-- the query that generates recordings needs no join to find them.

CREATE TABLE plays (
    play_id TEXT NOT NULL PRIMARY KEY,
    track_id TEXT REFERENCES tracks(track_id) ON DELETE SET NULL,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    played_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE INDEX plays_track_id ON plays (track_id);
CREATE INDEX plays_recording_id ON plays (recording_id, played_at);
CREATE INDEX plays_played_at ON plays (played_at);

-- Carry the existing history over. Tracks are the only table whose
-- `last_played_at` was written per playback rather than derived from one, so
-- they are the faithful source; every other column was a denormalization of
-- exactly this.
INSERT INTO plays (play_id, track_id, recording_id, played_at)
SELECT LOWER(HEX(RANDOMBLOB(16))), track_id, recording_id, last_played_at
FROM tracks
WHERE last_played_at IS NOT NULL;

-- Search orders its facets by how recently they were played. These views keep
-- that possible now that the columns are gone. They are also what the
-- statistics of an entity are read from elsewhere, so that "last played" has
-- exactly one definition per entity kind.
--
-- The two readings the old `persons.last_played_at` mixed together are separate
-- views here: `person_last_played` is about works someone composed, and
-- `performer_last_played` about recordings they played on. Search uses each
-- where it means something, which the single column could not express.
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

CREATE VIEW medium_last_played AS
SELECT tracks.medium_id AS medium_id,
    MAX(plays.played_at) AS last_played_at,
    COUNT(*) AS play_count
FROM plays
    JOIN tracks ON tracks.track_id = plays.track_id
WHERE tracks.medium_id IS NOT NULL
GROUP BY tracks.medium_id;

ALTER TABLE persons DROP COLUMN last_played_at;
ALTER TABLE instruments DROP COLUMN last_played_at;
ALTER TABLE works DROP COLUMN last_played_at;
ALTER TABLE ensembles DROP COLUMN last_played_at;
ALTER TABLE recordings DROP COLUMN last_played_at;
ALTER TABLE tracks DROP COLUMN last_played_at;
ALTER TABLE mediums DROP COLUMN last_played_at;
ALTER TABLE albums DROP COLUMN last_played_at;

UPDATE meta SET schema_version = 5, updated_at = DATETIME('now') WHERE id = 1;
