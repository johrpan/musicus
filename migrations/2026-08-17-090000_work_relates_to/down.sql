DROP INDEX works_relates_to;

ALTER TABLE works DROP COLUMN relates_to;

UPDATE meta SET schema_version = 5, updated_at = DATETIME('now') WHERE id = 1;
