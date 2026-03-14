CREATE TABLE passwords (
    id VARCHAR(36) PRIMARY KEY,
    password VARCHAR(250) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
) STRICT;
