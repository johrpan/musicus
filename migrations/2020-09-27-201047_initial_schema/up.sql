CREATE TABLE persons (
    id BIGINT NOT NULL PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL
);

CREATE TABLE instruments (
    id BIGINT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE works (
    id BIGINT NOT NULL PRIMARY KEY,
    composer BIGINT NOT NULL REFERENCES persons(id),
    title TEXT NOT NULL
);

CREATE TABLE instrumentations (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    instrument BIGINT NOT NULL REFERENCES instruments(id)
);

CREATE TABLE work_parts (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    part_index BIGINT NOT NULL,
    composer BIGINT REFERENCES persons(id),
    title TEXT NOT NULL
);

CREATE TABLE part_instrumentations (
    id BIGINT NOT NULL PRIMARY KEY,
    work_part BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    instrument BIGINT NOT NULL REFERENCES instruments(id)
);

CREATE TABLE work_sections (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    before_index BIGINT NOT NULL
);

CREATE TABLE ensembles (
    id BIGINT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL
);

CREATE TABLE recordings (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id),
    comment TEXT NOT NULL
);

CREATE TABLE performances (
    id BIGINT NOT NULL PRIMARY KEY,
    recording BIGINT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    person BIGINT REFERENCES persons(id) ON DELETE CASCADE,
    ensemble BIGINT REFERENCES ensembles(id) ON DELETE CASCADE,
    role BIGINT REFERENCES instruments(id)
);