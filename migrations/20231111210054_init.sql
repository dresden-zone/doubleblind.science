
CREATE TABLE oauth (
  id UUID PRIMARY KEY,
  platform INT NOT NULL
);

CREATE TABLE project (
  id UUID PRIMARY KEY, 
  owner UUID NOT NULL REFERENCES oauth(id),
  domain TEXT NOT NULL, 
  commit VARCHAR(40) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL,
  last_update TIMESTAMPTZ NOT NULL
);

CREATE TABLE github_source (
  id UUID PRIMARY KEY,
  project_id UUID NOT NULL REFERENCES project(id),
  refresh_token TEXT NOT NULL
);

