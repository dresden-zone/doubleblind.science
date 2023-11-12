
CREATE TABLE oauth (
  id UUID PRIMARY KEY,
  platform INT NOT NULL
);

CREATE TABLE project (
  id UUID PRIMARY KEY, 
  owner UUID REFERENCES TABLE oauth(id),
  domain TEXT NOT NULL, 
  commit VARCHAR(40) NOT NULL,
);

CREATE TABLE github_source (
  id UUID PRIMARY KEY,
  project_id UUID REFERENCES TABLE project(id) NOT NULL,
  github_id BITINT NOT NULL
);

