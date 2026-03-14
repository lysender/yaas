CREATE TABLE org_apps (
    id VARCHAR(36) PRIMARY KEY,
    org_id VARCHAR(36) NOT NULL,
    app_id VARCHAR(36) NOT NULL,
    created_at BIGINT NOT NULL,
    FOREIGN KEY (org_id) REFERENCES orgs(id),
    FOREIGN KEY (app_id) REFERENCES apps(id)
) STRICT;

CREATE INDEX idx_org_apps_org_id ON org_apps(org_id);
CREATE INDEX idx_org_apps_app_id ON org_apps(app_id);
CREATE UNIQUE INDEX idx_org_apps_org_id_app_id ON org_apps(org_id, app_id);
