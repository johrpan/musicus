-- Roll back to the single `relates_to` column. Multiple outgoing relations
-- cannot be represented; the first one keeps the link.

CREATE TABLE works_old (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    relates_to TEXT REFERENCES works(work_id) ON DELETE SET NULL
);

INSERT INTO works_old (work_id, parent_work_id, sequence_number, name, source, enable_updates,
    created_at, edited_at, last_used_at, relates_to)
SELECT works.work_id, works.parent_work_id, works.sequence_number, works.name, works.source,
    works.enable_updates, works.created_at, works.edited_at, works.last_used_at,
    (SELECT related_work_id FROM work_relations
     WHERE work_relations.work_id = works.work_id
     ORDER BY related_work_id LIMIT 1)
FROM works;

DROP TABLE works;

ALTER TABLE works_old RENAME TO works;

CREATE INDEX works_parent_work_id ON works (parent_work_id);
CREATE INDEX works_relates_to ON works (relates_to);

DROP TABLE work_relations;

UPDATE meta SET schema_version = 1, updated_at = DATETIME('now') WHERE id = 1;