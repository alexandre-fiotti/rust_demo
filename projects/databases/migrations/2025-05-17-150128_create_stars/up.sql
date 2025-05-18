CREATE TABLE stars (
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    stargazer TEXT NOT NULL,
    starred_at TIMESTAMP NOT NULL,
    fetched_at TIMESTAMP NOT NULL
);

ALTER TABLE stars ADD CONSTRAINT stars_pkey PRIMARY KEY (repository_id, stargazer);
CREATE INDEX idx_stars_repo_starred_at ON stars (repository_id, starred_at);
