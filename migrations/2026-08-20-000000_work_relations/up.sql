-- Every migration that changes the schema must bump schema_version.

-- A work may be derived from several works, so the single `relates_to`
-- column becomes a many-to-many table. Deleting one side of a relation
-- removes the relation row (CASCADE): a derived work that lost its source
-- simply has no source left, exactly like the old ON DELETE SET NULL. The
-- check constraint keeps a work from relating to itself.

CREATE TABLE work_relations (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    related_work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    PRIMARY KEY (work_id, related_work_id),
    CHECK (work_id <> related_work_id)
);

CREATE INDEX work_relations_related_work_id ON work_relations (related_work_id);

-- Preserve every existing outgoing relation.
INSERT INTO work_relations (work_id, related_work_id)
SELECT work_id, relates_to
FROM works
WHERE relates_to IS NOT NULL;

-- SQLite cannot drop a column in place, so rebuild `works` without
-- `relates_to`.
CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id) ON DELETE CASCADE,
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user' CHECK (source IN ('user', 'metadata', 'import')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO works_new (work_id, parent_work_id, sequence_number, name, source, enable_updates,
    created_at, edited_at, last_used_at)
SELECT work_id, parent_work_id, sequence_number, name, source, enable_updates,
    created_at, edited_at, last_used_at
FROM works;

DROP TABLE works;

ALTER TABLE works_new RENAME TO works;

CREATE INDEX works_parent_work_id ON works (parent_work_id);

UPDATE meta SET schema_version = 2, updated_at = DATETIME('now') WHERE id = 1;