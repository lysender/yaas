CREATE TABLE orgs (
    id VARCHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(10) NOT NULL,
    owner_id VARCHAR(36) NULL DEFAULT NULL,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    deleted_at BIGINT NULL DEFAULT NULL,
    FOREIGN KEY (owner_id) REFERENCES users(id)
) STRICT;

CREATE INDEX idx_orgs_owner_id ON orgs(owner_id);
CREATE INDEX idx_orgs_deleted_at ON orgs(deleted_at);
