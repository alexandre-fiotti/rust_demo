-- Your SQL goes here
CREATE TABLE stars (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    stargazer TEXT NOT NULL,
    email TEXT,
    starred_at TIMESTAMP NOT NULL,
    fetched_at TIMESTAMP NOT NULL,
    UNIQUE (repository_id, stargazer, starred_at)
);
