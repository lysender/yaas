CREATE TABLE org_members (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    roles VARCHAR(250) NOT NULL,
    status VARCHAR(10) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT fk_org_members_org FOREIGN KEY (org_id) REFERENCES orgs(id),
    CONSTRAINT fk_org_members_user FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_org_members_org_id ON org_members(org_id);
CREATE INDEX idx_org_members_user_id ON org_members(user_id);
CREATE UNIQUE INDEX idx_org_members_org_id_user_id ON org_members(org_id, user_id);
