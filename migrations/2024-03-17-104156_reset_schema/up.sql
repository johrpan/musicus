CREATE TABLE persons_new (
    person_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE roles (
    role_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE instruments_new (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE works_new (
    work_id TEXT NOT NULL PRIMARY KEY,
    parent_work_id TEXT REFERENCES works(work_id),
    sequence_number INTEGER,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE work_persons (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, person_id, role_id)
);

CREATE TABLE work_instruments (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    instrument_id TEXT NOT NULL REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, instrument_id)
);

CREATE TABLE ensembles_new (
    ensemble_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE ensemble_persons (
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    instrument_id TEXT NOT NULL REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (ensemble_id, person_id, instrument_id)
);

CREATE TABLE recordings_new (
    recording_id TEXT NOT NULL PRIMARY KEY,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    year INTEGER,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE recording_persons (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    instrument_id TEXT REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, person_id, role_id, instrument_id)
);

CREATE TABLE recording_ensembles (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, ensemble_id, role_id)
);

CREATE TABLE tracks_new (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id),
    sequence_number INTEGER NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE track_works (
    track_id TEXT NOT NULL REFERENCES tracks(track_id) ON DELETE CASCADE,
    work_id TEXT NOT NULL REFERENCES works(work_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (track_id, work_id)
);

INSERT INTO persons_new (person_id, name)
SELECT id, json_set('{}', '$.generic', first_name || ' ' || last_name)
FROM persons;

INSERT INTO roles (role_id, name)
VALUES ('380d7e09eb2f49c1a90db2ba4acb6ffd', json_set('{}', '$.generic', 'Composer'));

INSERT INTO roles (role_id, name)
VALUES ('28ff0aeb11c041a6916d93e9b4884eef', json_set('{}', '$.generic', 'Performer'));

INSERT INTO instruments_new (instrument_id, name)
SELECT id, json_set('{}', '$.generic', name)
FROM instruments;

INSERT INTO works_new (work_id, name)
SELECT id, json_set('{}', '$.generic', title)
FROM works;

INSERT INTO works_new (work_id, parent_work_id, sequence_number, name)
SELECT id, work, part_index, json_set('{}', '$.generic', title)
FROM work_parts;

INSERT INTO work_persons (work_id, person_id, role_id, sequence_number)
SELECT id, composer, '380d7e09eb2f49c1a90db2ba4acb6ffd', 0
FROM works;

INSERT INTO work_instruments (work_id, instrument_id, sequence_number)
SELECT work, instrument, 0
FROM instrumentations;

INSERT INTO ensembles_new (ensemble_id, name)
SELECT id, json_set('{}', '$.generic', name)
FROM ensembles;

INSERT INTO recordings_new (recording_id, work_id, year)
SELECT id, work, CAST(comment as INTEGER)
FROM recordings;

UPDATE recordings_new
SET year = NULL
WHERE year <= 0;

INSERT INTO recording_persons (recording_id, person_id, role_id, instrument_id, sequence_number)
SELECT recording, person, '28ff0aeb11c041a6916d93e9b4884eef', role, 0
FROM performances
WHERE person IS NOT NULL;

INSERT INTO recording_ensembles (recording_id, ensemble_id, role_id, sequence_number)
SELECT recording, ensemble, '28ff0aeb11c041a6916d93e9b4884eef', 0
FROM performances
WHERE ensemble IS NOT NULL;

INSERT INTO tracks_new (track_id, recording_id, sequence_number, path)
SELECT id, recording, "index", path
FROM tracks;

INSERT INTO track_works (track_id, work_id, sequence_number)
SELECT tracks.id, work_parts.id, 0
FROM tracks
    JOIN recordings ON tracks.recording = recordings.id
    JOIN work_parts ON recordings.work = work_parts.work
        AND tracks.work_parts = work_parts.part_index;

DROP TABLE persons;
DROP TABLE instruments;
DROP TABLE works;
DROP TABLE instrumentations;
DROP TABLE work_parts;
DROP TABLE ensembles;
DROP TABLE recordings;
DROP TABLE performances;
DROP TABLE mediums;
DROP TABLE tracks;

ALTER TABLE persons_new RENAME TO persons;
ALTER TABLE instruments_new RENAME TO instruments;
ALTER TABLE works_new RENAME TO works;
ALTER TABLE recordings_new RENAME TO recordings;
ALTER TABLE tracks_new RENAME TO tracks;
ALTER TABLE ensembles_new RENAME TO ensembles;