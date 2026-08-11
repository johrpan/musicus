-- Restore the recording year column from the well-known Year tag before the tag
-- tables go away. Values that are not plain digits were never reachable through
-- the old integer column and are dropped.

ALTER TABLE recordings ADD COLUMN year INTEGER;

UPDATE recordings
SET year = (
    SELECT CAST(recording_tags.value AS INTEGER)
    FROM recording_tags
    WHERE recording_tags.recording_id = recordings.recording_id
      AND recording_tags.tag_id = 'c18e9585a9a5433fbc2b4e5848c96d4d'
      AND recording_tags.value GLOB '[0-9]*'
    LIMIT 1
);

DROP TABLE recording_tags;
DROP TABLE work_tags;
DROP TABLE tags;

UPDATE meta SET schema_version = 2, updated_at = DATETIME('now') WHERE id = 1;
