CREATE TABLE persons (
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

CREATE TABLE instruments (
    instrument_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE works (
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

CREATE TABLE ensembles (
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

CREATE TABLE recordings (
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

CREATE TABLE tracks (
    track_id TEXT NOT NULL PRIMARY KEY,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id),
    recording_index INTEGER NOT NULL,
    medium_id TEXT REFERENCES mediums(medium_id),
    medium_index INTEGER,
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

CREATE TABLE mediums (
    medium_id TEXT NOT NULL PRIMARY KEY,
    discid TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE albums (
    album_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    edited_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_played_at TIMESTAMP
);

CREATE TABLE album_recordings (
    album_id TEXT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    recording_id TEXT NOT NULL REFERENCES recordings(recording_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, recording_id)
);

CREATE TABLE album_mediums (
    album_id TEXT NOT NULL REFERENCES albums(album_id) ON DELETE CASCADE,
    medium_id TEXT NOT NULL REFERENCES mediums(medium_id),
    sequence_number INTEGER NOT NULL,
    PRIMARY KEY (album_id, medium_id)
);
