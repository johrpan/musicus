ALTER TABLE ensemble_persons ADD COLUMN role_id TEXT REFERENCES roles(role_id) ON DELETE RESTRICT;

CREATE INDEX ensemble_persons_role_id ON ensemble_persons (role_id);

UPDATE meta SET schema_version = 7, updated_at = DATETIME('now') WHERE id = 1;
