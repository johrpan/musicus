ALTER TABLE tags DROP COLUMN private;

UPDATE meta SET schema_version = 3, updated_at = DATETIME('now') WHERE id = 1;
