CREATE TABLE persons_old (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE roles_old (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE instruments_old (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE works_old (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id),
    sequence_number INTEGER,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE ensembles_old (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE recordings_old (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    year INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

INSERT INTO persons_old (
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
ALTER TABLE persons_old
    RENAME TO persons;

INSERT INTO roles_old (
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
ALTER TABLE roles_old
    RENAME TO roles;

INSERT INTO instruments_old (
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
ALTER TABLE works_old
    RENAME TO works;

INSERT INTO ensembles_old (
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
ALTER TABLE ensembles_old
    RENAME TO ensembles;

INSERT INTO recordings_old (
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
ALTER TABLE recordings_old
    RENAME TO recordings;