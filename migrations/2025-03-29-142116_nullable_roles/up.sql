CREATE TABLE work_persons_new (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, person_id, sequence_number)
);

CREATE TABLE recording_persons_new (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT REFERENCES roles(role_id),
    instrument_id TEXT REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, person_id, sequence_number)
);

CREATE TABLE recording_ensembles_new (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id),
    role_id TEXT REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, ensemble_id, sequence_number)
);

INSERT OR IGNORE INTO work_persons_new SELECT * FROM work_persons;
UPDATE work_persons_new SET role_id = NULL WHERE role_id = '380d7e09eb2f49c1a90db2ba4acb6ffd';
DROP TABLE work_persons;
ALTER TABLE work_persons_new RENAME TO work_persons;

INSERT OR IGNORE INTO recording_persons_new SELECT * FROM recording_persons;
UPDATE recording_persons_new SET role_id = NULL WHERE role_id = '28ff0aeb11c041a6916d93e9b4884eef';
DROP TABLE recording_persons;
ALTER TABLE recording_persons_new RENAME TO recording_persons;

INSERT OR IGNORE INTO recording_ensembles_new SELECT * FROM recording_ensembles;
UPDATE recording_ensembles_new SET role_id = NULL WHERE role_id = '28ff0aeb11c041a6916d93e9b4884eef';
DROP TABLE recording_ensembles;
ALTER TABLE recording_ensembles_new RENAME TO recording_ensembles;

DELETE FROM roles WHERE role_id IN ('380d7e09eb2f49c1a90db2ba4acb6ffd', '28ff0aeb11c041a6916d93e9b4884eef');