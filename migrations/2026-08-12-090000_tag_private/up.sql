ALTER TABLE tags ADD COLUMN private BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE meta SET schema_version = 4, updated_at = DATETIME('now') WHERE id = 1;
