CREATE TABLE work_persons_old (
    work_id TEXT NOT NULL REFERENCES works(work_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (work_id, person_id, role_id)
);

CREATE TABLE recording_persons_old (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    instrument_id TEXT REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, person_id, role_id, instrument_id)
);

CREATE TABLE recording_ensembles_old (
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id) ON DELETE CASCADE,
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id),
    role_id TEXT NOT NULL REFERENCES roles(role_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (recording_id, ensemble_id, role_id)
);

INSERT INTO roles (role_id, name) VALUES ('380d7e09eb2f49c1a90db2ba4acb6ffd', '{"generic":"Composer"}');
INSERT INTO roles (role_id, name) VALUES ('28ff0aeb11c041a6916d93e9b4884eef', '{"generic":"Performer"}');

UPDATE work_persons SET role_id = '380d7e09eb2f49c1a90db2ba4acb6ffd' WHERE role_id IS NULL;
UPDATE recording_persons SET role_id = '28ff0aeb11c041a6916d93e9b4884eef' WHERE role_id IS NULL;
UPDATE recording_ensembles SET role_id = '28ff0aeb11c041a6916d93e9b4884eef' WHERE role_id IS NULL;

INSERT INTO work_persons_old SELECT * FROM work_persons;
DROP TABLE work_persons;
ALTER TABLE work_persons_old RENAME TO work_persons;

INSERT INTO recording_persons_old SELECT * FROM recording_persons;
DROP TABLE recording_persons;
ALTER TABLE recording_persons_old RENAME TO recording_persons;

INSERT INTO recording_ensembles_old SELECT * FROM recording_ensembles;
DROP TABLE recording_ensembles;
ALTER TABLE recording_ensembles_old RENAME TO recording_ensembles;