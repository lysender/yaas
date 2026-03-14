CREATE TABLE users (
    id VARCHAR(36) PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(10) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    deleted_at BIGINT NULL DEFAULT NULL
) STRICT;

CREATE UNIQUE INDEX idx_users_email_deleted_at ON users(email, deleted_at);
CREATE INDEX idx_users_deleted_at ON users(deleted_at);
