CREATE TABLE tags (
    tag_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    takes_value BOOLEAN NOT NULL DEFAULT FALSE,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

CREATE TABLE work_tags (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, sequence_number)
);

CREATE TABLE recording_tags (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(tag_id) ON DELETE RESTRICT,
    value TEXT,
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, sequence_number)
);

CREATE INDEX work_tags_tag_id ON work_tags (tag_id);
CREATE INDEX recording_tags_tag_id ON recording_tags (tag_id);

INSERT INTO tags (tag_id, name, takes_value)
VALUES ('c18e9585a9a5433fbc2b4e5848c96d4d', '{"generic":"Year"}', TRUE);

INSERT INTO recording_tags (recording_id, tag_id, value, sequence_number)
SELECT recording_id, 'c18e9585a9a5433fbc2b4e5848c96d4d', CAST(year AS TEXT), 0
FROM recordings
WHERE year IS NOT NULL;

ALTER TABLE recordings DROP COLUMN year;

UPDATE meta SET schema_version = 3, updated_at = DATETIME('now') WHERE id = 1;
