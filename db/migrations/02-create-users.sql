CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER NULL DEFAULT NULL
) STRICT;

CREATE UNIQUE INDEX idx_users_email_deleted_at ON users(email, deleted_at);
CREATE INDEX idx_users_deleted_at ON users(deleted_at);
