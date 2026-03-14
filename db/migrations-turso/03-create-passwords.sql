CREATE TABLE passwords (
    id TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;
