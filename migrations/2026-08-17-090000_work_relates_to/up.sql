ALTER TABLE works ADD COLUMN relates_to TEXT REFERENCES works(work_id) ON DELETE SET NULL;

CREATE INDEX works_relates_to ON works (relates_to);

UPDATE meta SET schema_version = 6, updated_at = DATETIME('now') WHERE id = 1;
