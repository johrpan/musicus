CREATE TABLE persons (
    id TEXT NOT NULL PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL
);

CREATE TABLE instruments (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE works (
    id TEXT NOT NULL PRIMARY KEY,
    composer TEXT NOT NULL REFERENCES persons(id),
    title TEXT NOT NULL
);

CREATE TABLE instrumentations (
    id BIGINT NOT NULL PRIMARY KEY,
    work TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    instrument TEXT NOT NULL REFERENCES instruments(id) ON DELETE CASCADE
);

CREATE TABLE work_parts (
    id BIGINT NOT NULL PRIMARY KEY,
    work TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    part_index BIGINT NOT NULL,
    title TEXT NOT NULL,
    composer TEXT REFERENCES persons(id)
);

CREATE TABLE work_sections (
    id BIGINT NOT NULL PRIMARY KEY,
    work TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    before_index BIGINT NOT NULL
);

CREATE TABLE ensembles (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE recordings (
    id TEXT NOT NULL PRIMARY KEY,
    work TEXT NOT NULL REFERENCES works(id),
    comment TEXT NOT NULL
);

CREATE TABLE performances (
    id BIGINT NOT NULL PRIMARY KEY,
    recording TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    person TEXT REFERENCES persons(id),
    ensemble TEXT REFERENCES ensembles(id),
    role TEXT REFERENCES instruments(id)
);

CREATE TABLE tracks (
    id BIGINT NOT NULL PRIMARY KEY,
    file_name TEXT NOT NULL,
    recording TEXT NOT NULL REFERENCES recordings(id),
    track_index INTEGER NOT NULL,
    work_parts TEXT NOT NULL
);