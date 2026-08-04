CREATE TABLE ensemble_persons_old (
    ensemble_id TEXT NOT NULL REFERENCES ensembles(ensemble_id) ON DELETE CASCADE,
    person_id TEXT NOT NULL REFERENCES persons(person_id),
    instrument_id TEXT NOT NULL REFERENCES instruments(instrument_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (ensemble_id, person_id, instrument_id)
);

DELETE FROM ensemble_persons WHERE instrument_id IS NULL;

INSERT INTO ensemble_persons_old SELECT * FROM ensemble_persons;
DROP TABLE ensemble_persons;
ALTER TABLE ensemble_persons_old RENAME TO ensemble_persons;
