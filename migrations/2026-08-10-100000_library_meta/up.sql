-- Every migration that changes the schema must bump schema_version.

CREATE TABLE meta (
    -- Constrained to a single row.
    id INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now'))
);

INSERT INTO meta (id, schema_version) VALUES (1, 1);
