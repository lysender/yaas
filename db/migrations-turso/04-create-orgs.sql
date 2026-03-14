CREATE TABLE orgs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    owner_id TEXT NULL DEFAULT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER NULL DEFAULT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
) STRICT;

CREATE INDEX idx_orgs_owner_id ON orgs(owner_id);
CREATE INDEX idx_orgs_deleted_at ON orgs(deleted_at);
