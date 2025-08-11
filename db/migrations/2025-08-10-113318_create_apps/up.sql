CREATE TABLE apps (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    secret VARCHAR(200) NOT NULL,
    redirect_uri VARCHAR(250) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    deleted_at TIMESTAMP WITH TIME ZONE NULL DEFAULT NULL
);

CREATE INDEX idx_apps_deleted_at ON apps(deleted_at);
