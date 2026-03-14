CREATE TABLE oauth_codes (
    id VARCHAR(36) PRIMARY KEY,
    code VARCHAR(36) NOT NULL UNIQUE,
    state VARCHAR(250) NOT NULL,
    redirect_uri VARCHAR(250) NOT NULL,
    scope VARCHAR(250) NOT NULL,
    app_id VARCHAR(36) NOT NULL,
    org_id VARCHAR(36) NOT NULL,
    user_id VARCHAR(36) NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    FOREIGN KEY (app_id) REFERENCES apps(id),
    FOREIGN KEY (org_id) REFERENCES orgs(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) STRICT;

CREATE INDEX idx_oauth_codes_expires_at ON oauth_codes(expires_at);
