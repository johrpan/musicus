CREATE TABLE persons_new (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE roles_new (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE instruments_new (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id),
    sequence_number INTEGER,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE ensembles_new (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE recordings_new (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    year INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO persons_new (
        person_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT person_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM persons;
DROP TABLE persons;
ALTER TABLE persons_new
    RENAME TO persons;

INSERT INTO roles_new (
        role_id,
        name,
        created_at,
        edited_at,
        last_used_at
    )
SELECT role_id,
    name,
    created_at,
    edited_at,
    last_used_at
FROM roles;
DROP TABLE roles;
ALTER TABLE roles_new
    RENAME TO roles;

INSERT INTO instruments_new (
        instrument_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT instrument_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM instruments;
DROP TABLE instruments;
ALTER TABLE instruments_new
    RENAME TO instruments;

INSERT INTO works_new (
        work_id,
        parent_work_id,
        sequence_number,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT work_id,
    parent_work_id,
    sequence_number,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM works;
DROP TABLE works;
ALTER TABLE works_new
    RENAME TO works;

INSERT INTO ensembles_new (
        ensemble_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT ensemble_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM ensembles;
DROP TABLE ensembles;
ALTER TABLE ensembles_new
    RENAME TO ensembles;

INSERT INTO recordings_new (
        recording_id,
        work_id,
        year,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT recording_id,
    work_id,
    year,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM recordings;
DROP TABLE recordings;
ALTER TABLE recordings_new
    RENAME TO recordings;