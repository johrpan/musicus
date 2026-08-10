-- Mediums that were merged because they shared a discid cannot be unmerged;
-- that part of the migration is not reversible.

DROP INDEX works_parent_work_id;
DROP INDEX recordings_work_id;
DROP INDEX tracks_recording_id;
DROP INDEX tracks_medium_id;
DROP INDEX work_persons_person_id;
DROP INDEX work_persons_role_id;
DROP INDEX work_instruments_instrument_id;
DROP INDEX ensemble_persons_person_id;
DROP INDEX ensemble_persons_instrument_id;
DROP INDEX recording_persons_person_id;
DROP INDEX recording_persons_role_id;
DROP INDEX recording_persons_instrument_id;
DROP INDEX recording_ensembles_ensemble_id;
DROP INDEX recording_ensembles_role_id;
DROP INDEX track_works_work_id;
DROP INDEX album_recordings_recording_id;
DROP INDEX album_mediums_medium_id;

CREATE TABLE persons_old (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO persons_old (person_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT person_id, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM persons;

CREATE TABLE roles_old (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime'))
);

INSERT INTO roles_old (role_id, name, source, enable_updates, created_at, edited_at, last_used_at)
SELECT role_id, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime')
FROM roles;

CREATE TABLE instruments_old (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO instruments_old (instrument_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT instrument_id, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM instruments;

CREATE TABLE ensembles_old (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO ensembles_old (ensemble_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT ensemble_id, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM ensembles;

CREATE TABLE mediums_old (
    medium_id TEXT NOT NULL PRIMARY KEY,
    discid TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO mediums_old (medium_id, discid, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT medium_id, discid, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM mediums;

CREATE TABLE works_old (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works_old(work_id),
    sequence_number INTEGER,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO works_old (work_id, parent_work_id, sequence_number, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT work_id, parent_work_id, sequence_number, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM works;

CREATE TABLE recordings_old (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works_old(work_id),
    year INTEGER,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO recordings_old (recording_id, work_id, year, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT recording_id, work_id, year, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM recordings;

CREATE TABLE albums_old (
    album_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'user',
    enable_updates BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO albums_old (album_id, name, source, enable_updates, created_at, edited_at, last_used_at, last_played_at)
SELECT album_id, name, source, enable_updates,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM albums;

CREATE TABLE tracks_old (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings_old(recording_id),
    recording_index INTEGER NOT NULL,
    medium_id TEXT REFERENCES mediums_old(medium_id),
    medium_index INTEGER,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    edited_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_used_at TIMESTAMP NOT NULL DEFAULT (DATETIME('now', 'localtime')),
    last_played_at TIMESTAMP
);

INSERT INTO tracks_old (track_id, recording_id, recording_index, medium_id, medium_index, path, created_at, edited_at, last_used_at, last_played_at)
SELECT track_id, recording_id, recording_index, medium_id, medium_index, path,
       DATETIME(created_at, 'localtime'), DATETIME(edited_at, 'localtime'),
       DATETIME(last_used_at, 'localtime'), DATETIME(last_played_at, 'localtime')
FROM tracks;

CREATE TABLE work_persons_old (
    work_id TEXT NOT NULL REFERENCES works_old(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_old(person_id),
    role_id TEXT REFERENCES roles_old(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, person_id, sequence_number)
);

INSERT INTO work_persons_old (work_id, person_id, role_id, sequence_number)
SELECT work_id, person_id, role_id, sequence_number FROM work_persons;

CREATE TABLE work_instruments_old (
    work_id TEXT NOT NULL REFERENCES works_old(work_id) ON DELETE CASCADE,
    instrument_id TEXT NOT NULL REFERENCES instruments_old(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, instrument_id)
);

INSERT INTO work_instruments_old (work_id, instrument_id, sequence_number)
SELECT work_id, instrument_id, sequence_number FROM work_instruments;

CREATE TABLE ensemble_persons_old (
    ensemble_id TEXT NOT NULL REFERENCES ensembles_old(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_old(person_id),
    instrument_id TEXT REFERENCES instruments_old(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (ensemble_id, person_id, sequence_number)
);

INSERT INTO ensemble_persons_old (ensemble_id, person_id, instrument_id, sequence_number)
SELECT ensemble_id, person_id, instrument_id, sequence_number FROM ensemble_persons;

CREATE TABLE recording_persons_old (
    recording_id TEXT NOT NULL REFERENCES recordings_old(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons_old(person_id),
    role_id TEXT REFERENCES roles_old(role_id),
    instrument_id TEXT REFERENCES instruments_old(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, person_id, sequence_number)
);

INSERT INTO recording_persons_old (recording_id, person_id, role_id, instrument_id, sequence_number)
SELECT recording_id, person_id, role_id, instrument_id, sequence_number FROM recording_persons;

CREATE TABLE recording_ensembles_old (
    recording_id TEXT NOT NULL REFERENCES recordings_old(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles_old(ensemble_id),
    role_id TEXT REFERENCES roles_old(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, ensemble_id, sequence_number)
);

INSERT INTO recording_ensembles_old (recording_id, ensemble_id, role_id, sequence_number)
SELECT recording_id, ensemble_id, role_id, sequence_number FROM recording_ensembles;

CREATE TABLE track_works_old (
    track_id TEXT NOT NULL REFERENCES tracks_old(track_id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works_old(work_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (track_id, work_id)
);

INSERT INTO track_works_old (track_id, work_id, sequence_number)
SELECT track_id, work_id, sequence_number FROM track_works;

CREATE TABLE album_recordings_old (
    album_id TEXT NOT NULL REFERENCES albums_old(album_id) ON DELETE CASCADE,
    recording_id TEXT NOT NULL REFERENCES recordings_old(recording_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, recording_id)
);

INSERT INTO album_recordings_old (album_id, recording_id, sequence_number)
SELECT album_id, recording_id, sequence_number FROM album_recordings;

CREATE TABLE album_mediums_old (
    album_id TEXT NOT NULL REFERENCES albums_old(album_id) ON DELETE CASCADE,
    medium_id TEXT NOT NULL REFERENCES mediums_old(medium_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, medium_id)
);

INSERT INTO album_mediums_old (album_id, medium_id, sequence_number)
SELECT album_id, medium_id, sequence_number FROM album_mediums;

DROP TABLE album_mediums;
DROP TABLE album_recordings;
DROP TABLE track_works;
DROP TABLE recording_ensembles;
DROP TABLE recording_persons;
DROP TABLE ensemble_persons;
DROP TABLE work_instruments;
DROP TABLE work_persons;
DROP TABLE tracks;
DROP TABLE albums;
DROP TABLE recordings;
DROP TABLE works;
DROP TABLE mediums;
DROP TABLE ensembles;
DROP TABLE instruments;
DROP TABLE roles;
DROP TABLE persons;

ALTER TABLE persons_old RENAME TO persons;
ALTER TABLE roles_old RENAME TO roles;
ALTER TABLE instruments_old RENAME TO instruments;
ALTER TABLE ensembles_old RENAME TO ensembles;
ALTER TABLE mediums_old RENAME TO mediums;
ALTER TABLE works_old RENAME TO works;
ALTER TABLE recordings_old RENAME TO recordings;
ALTER TABLE albums_old RENAME TO albums;
ALTER TABLE tracks_old RENAME TO tracks;
ALTER TABLE work_persons_old RENAME TO work_persons;
ALTER TABLE work_instruments_old RENAME TO work_instruments;
ALTER TABLE ensemble_persons_old RENAME TO ensemble_persons;
ALTER TABLE recording_persons_old RENAME TO recording_persons;
ALTER TABLE recording_ensembles_old RENAME TO recording_ensembles;
ALTER TABLE track_works_old RENAME TO track_works;
ALTER TABLE album_recordings_old RENAME TO album_recordings;
ALTER TABLE album_mediums_old RENAME TO album_mediums;

UPDATE library_meta SET schema_version = 1, updated_at = DATETIME('now') WHERE id = 1;
