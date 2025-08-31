CREATE TABLE users (
    id CHAR(36) PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(10) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    deleted_at TIMESTAMP WITH TIME ZONE NULL DEFAULT NULL
);

CREATE UNIQUE INDEX idx_users_email_deleted_at ON users(email, deleted_at);
CREATE INDEX idx_users_deleted_at ON users(deleted_at);
