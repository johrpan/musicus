CREATE TABLE ensemble_persons_new (
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    instrument_id TEXT REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (ensemble_id, person_id, sequence_number)
);

INSERT INTO ensemble_persons_new SELECT * FROM ensemble_persons;
DROP TABLE ensemble_persons;
ALTER TABLE ensemble_persons_new RENAME TO ensemble_persons;
