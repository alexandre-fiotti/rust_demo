-- Your SQL goes here
CREATE TABLE repositories (
    id UUID PRIMARY KEY,
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    UNIQUE (owner, name)
);
