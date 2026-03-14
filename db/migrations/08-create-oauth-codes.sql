CREATE TABLE oauth_codes (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    state TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT NOT NULL,
    app_id TEXT NOT NULL,
    org_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    FOREIGN KEY (app_id) REFERENCES apps(id),
    FOREIGN KEY (org_id) REFERENCES orgs(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) STRICT;

CREATE INDEX idx_oauth_codes_expires_at ON oauth_codes(expires_at);
