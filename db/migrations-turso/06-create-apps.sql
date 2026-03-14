CREATE TABLE apps (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    client_id VARCHAR(36) NOT NULL UNIQUE,
    client_secret VARCHAR(200) NOT NULL,
    redirect_uri VARCHAR(250) NOT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    deleted_at BIGINT NULL DEFAULT NULL
) STRICT;

CREATE INDEX idx_apps_deleted_at ON apps(deleted_at);
