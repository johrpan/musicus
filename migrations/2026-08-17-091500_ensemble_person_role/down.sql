DROP INDEX ensemble_persons_role_id;

ALTER TABLE ensemble_persons DROP COLUMN role_id;

UPDATE meta SET schema_version = 6, updated_at = DATETIME('now') WHERE id = 1;
