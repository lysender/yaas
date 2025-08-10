CREATE TABLE org_apps (
    id CHAR(36) PRIMARY KEY,
    org_id CHAR(36) NOT NULL,
    app_id CHAR(36) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT fk_org_apps_org FOREIGN KEY (org_id) REFERENCES orgs(id),
    CONSTRAINT fk_org_apps_app FOREIGN KEY (app_id) REFERENCES apps(id)
);

CREATE INDEX idx_org_apps_org_id ON org_apps(org_id);
CREATE INDEX idx_org_apps_app_id ON org_apps(app_id);
CREATE UNIQUE INDEX idx_org_apps_org_id_app_id ON org_apps(org_id, app_id);
