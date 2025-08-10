CREATE TABLE orgs (
    id CHAR(36) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    status VARCHAR(10) NOT NULL,
    owner_id VARCHAR(36) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
    CONSTRAINT fk_orgs_owner FOREIGN KEY (owner_id) REFERENCES users(id)
);

CREATE INDEX idx_orgs_owner_id ON orgs(owner_id);

CREATE TRIGGER trigger_update_updated_at
BEFORE UPDATE ON orgs
FOR EACH ROW
EXECUTE PROCEDURE update_updated_at_column();
