CREATE TABLE persons_old (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE roles_old (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE instruments_old (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE works_old (
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

CREATE TABLE ensembles_old (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE recordings_old (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    year INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE mediums_old (
    medium_id TEXT NOT NULL PRIMARY KEY REFERENCES item_state(id),
    discid TEXT NOT NULL,
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

CREATE TABLE albums_old (
    album_id TEXT NOT NULL PRIMARY KEY REFERENCES item_state(id),
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO persons_old (
        person_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at,
        enable_updates
    )
SELECT person_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at,
    enable_updates
FROM persons;
DROP TABLE persons;
ALTER TABLE persons_old
    RENAME TO persons;

INSERT INTO roles_old (
        role_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        enable_updates
    )
SELECT role_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    enable_updates
FROM roles;
DROP TABLE roles;
ALTER TABLE roles_old
    RENAME TO roles;

INSERT INTO instruments_old (
        instrument_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at,
        enable_updates
    )
SELECT instrument_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at,
    enable_updates
FROM instruments;
DROP TABLE instruments;
ALTER TABLE instruments_old
    RENAME TO instruments;

INSERT INTO works_old (
        work_id,
        parent_work_id,
        sequence_number,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at,
        enable_updates
    )
SELECT work_id,
    parent_work_id,
    sequence_number,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at,
    enable_updates
FROM works;
DROP TABLE works;
ALTER TABLE works_old
    RENAME TO works;

INSERT INTO ensembles_old (
        ensemble_id,
        name,
        created_at,
        edited_at,
        last_used_at,
        last_played_at,
        enable_updates
    )
SELECT ensemble_id,
    name,
    created_at,
    edited_at,
    last_used_at,
    last_played_at,
    enable_updates
FROM ensembles;
DROP TABLE ensembles;
ALTER TABLE ensembles_old
    RENAME TO ensembles;

INSERT INTO recordings_old (
        recording_id,
        work_id,
        year,
        created_at,
        edited_at,
        last_used_at,
        last_played_at,
        enable_updates
    )
SELECT recording_id,
    work_id,
    year,
    created_at,
    edited_at,
    last_used_at,
    last_played_at,
    enable_updates
FROM recordings;
DROP TABLE recordings;
ALTER TABLE recordings_old
    RENAME TO recordings;

INSERT INTO mediums_old (
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
ALTER TABLE mediums_old
    RENAME TO mediums;

INSERT INTO albums_old (
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
ALTER TABLE albums_old
    RENAME TO albums;