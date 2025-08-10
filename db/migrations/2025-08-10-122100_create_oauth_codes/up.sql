CREATE TABLE oauth_codes (
    id CHAR(36) PRIMARY KEY,
    code CHAR(36) NOT NULL UNIQUE,
    state VARCHAR(250) NOT NULL,
    redirect_uri VARCHAR(250) NOT NULL,
    scope VARCHAR(250) NOT NULL,
    app_id CHAR(36) NOT NULL,
    org_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    CONSTRAINT fk_oauth_codes_app FOREIGN KEY (app_id) REFERENCES apps(id),
    CONSTRAINT fk_oauth_codes_org FOREIGN KEY (org_id) REFERENCES orgs(id),
    CONSTRAINT fk_oauth_codes_user FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_oauth_codes_expires_at ON oauth_codes(expires_at);
