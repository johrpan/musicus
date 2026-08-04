CREATE TABLE persons_new (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE roles_new (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime'))
);

CREATE TABLE instruments_new (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id),
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE ensembles_new (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE recordings_new (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    year INTEGER,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE mediums_new (
    medium_id TEXT NOT NULL PRIMARY KEY,
    discid TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE albums_new (
    album_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
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

INSERT INTO mediums_new (
        medium_id,
        discid,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT medium_id,
    discid,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM mediums;
DROP TABLE mediums;
ALTER TABLE mediums_new
    RENAME TO mediums;

INSERT INTO albums_new (
        album_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at
    )
SELECT album_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at
FROM albums;
DROP TABLE albums;
ALTER TABLE albums_new
    RENAME TO albums;