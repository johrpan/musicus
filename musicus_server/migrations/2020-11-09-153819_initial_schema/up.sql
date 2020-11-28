CREATE TABLE users (
    username TEXT NOT NULL PRIMARY KEY,
    password_hash TEXT NOT NULL,
    email TEXT,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    is_editor BOOLEAN NOT NULL DEFAULT FALSE,
    is_banned BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE persons (
    id TEXT NOT NULL PRIMARY KEY,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE instruments (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE works (
    id TEXT NOT NULL PRIMARY KEY,
    composer TEXT NOT NULL REFERENCES persons(id),
    title TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
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
    name TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE recordings (
    id TEXT NOT NULL PRIMARY KEY,
    work TEXT NOT NULL REFERENCES works(id),
    comment TEXT NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(username)
);

CREATE TABLE performances (
    id BIGINT NOT NULL PRIMARY KEY,
    recording TEXT NOT NULL REFERENCES recordings(id) ON DELETE CASCADE,
    person TEXT REFERENCES persons(id),
    ensemble TEXT REFERENCES ensembles(id),
    role TEXT REFERENCES instruments(id)
);