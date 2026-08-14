-- Fold the play log back into one `last_played_at` column per entity.
--
-- The columns come back at the end of their tables rather than in their
-- original position. Diesel names every column it reads and writes, so the
-- order does not matter to the code; only a positional `SELECT *` would care,
-- and the schema conventions forbid those.
--
-- This direction loses information, as it must: play counts and the individual
-- timestamps collapse into a maximum. A person's column gets the later of their
-- composer and performer recency, which is the mixture the column used to hold.

ALTER TABLE persons ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE instruments ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE works ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE ensembles ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE recordings ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE tracks ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE mediums ADD COLUMN last_played_at TIMESTAMP;
ALTER TABLE albums ADD COLUMN last_played_at TIMESTAMP;

UPDATE tracks
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
        WHERE plays.track_id = tracks.track_id
    );

UPDATE recordings
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
        WHERE plays.recording_id = recordings.recording_id
    );

UPDATE works
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
            JOIN recordings ON recordings.recording_id = plays.recording_id
        WHERE recordings.work_id = works.work_id
    );

UPDATE persons
SET last_played_at = MAX(
        IFNULL(
            (
                SELECT MAX(plays.played_at)
                FROM plays
                    JOIN recordings ON recordings.recording_id = plays.recording_id
                    JOIN work_persons ON work_persons.work_id = recordings.work_id
                WHERE work_persons.person_id = persons.person_id
            ),
            ''
        ),
        IFNULL(
            (
                SELECT MAX(plays.played_at)
                FROM plays
                    JOIN recording_persons ON recording_persons.recording_id = plays.recording_id
                WHERE recording_persons.person_id = persons.person_id
            ),
            ''
        )
    );

-- The empty strings above only existed to keep the scalar MAX() from
-- swallowing a present value next to a missing one; a person with no plays at
-- all ends up with one and has to go back to NULL.
UPDATE persons SET last_played_at = NULL WHERE last_played_at = '';

UPDATE instruments
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
            JOIN recordings ON recordings.recording_id = plays.recording_id
            JOIN work_instruments ON work_instruments.work_id = recordings.work_id
        WHERE work_instruments.instrument_id = instruments.instrument_id
    );

UPDATE ensembles
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
            JOIN recording_ensembles ON recording_ensembles.recording_id = plays.recording_id
        WHERE recording_ensembles.ensemble_id = ensembles.ensemble_id
    );

UPDATE mediums
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
            JOIN tracks ON tracks.track_id = plays.track_id
        WHERE tracks.medium_id = mediums.medium_id
    );

UPDATE albums
SET last_played_at = (
        SELECT MAX(plays.played_at)
        FROM plays
            JOIN album_recordings ON album_recordings.recording_id = plays.recording_id
        WHERE album_recordings.album_id = albums.album_id
    );

DROP VIEW medium_last_played;
DROP VIEW album_last_played;
DROP VIEW ensemble_last_played;
DROP VIEW instrument_last_played;
DROP VIEW performer_last_played;
DROP VIEW person_last_played;
DROP VIEW work_last_played;
DROP VIEW track_last_played;
DROP VIEW recording_last_played;

DROP TABLE plays;

UPDATE meta SET schema_version = 4, updated_at = DATETIME('now') WHERE id = 1;
