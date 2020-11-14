CREATE TABLE users (
    username TEXT NOT NULL PRIMARY KEY,
    password_hash TEXT NOT NULL,
    email TEXT,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    is_editor BOOLEAN NOT NULL DEFAULT FALSE,
    is_banned BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE persons (
    id BIGINT NOT NULL PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE instruments (
    id BIGINT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE works (
    id BIGINT NOT NULL PRIMARY KEY,
    composer BIGINT NOT NULL REFERENCES persons(id),
    title TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE instrumentations (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    instrument BIGINT NOT NULL REFERENCES instruments(id) ON DELETE CASCADE
);

CREATE TABLE work_parts (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    part_index BIGINT NOT NULL,
    title TEXT NOT NULL,
    composer BIGINT REFERENCES persons(id)
);

CREATE TABLE work_sections (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    before_index BIGINT NOT NULL
);

CREATE TABLE ensembles (
    id BIGINT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE recordings (
    id BIGINT NOT NULL PRIMARY KEY,
    work BIGINT NOT NULL REFERENCES works(id),
    comment TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE performances (
    id BIGINT NOT NULL PRIMARY KEY,
    recording BIGINT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    person BIGINT REFERENCES persons(id),
    ensemble BIGINT REFERENCES ensembles(id),
    role BIGINT REFERENCES instruments(id)
);