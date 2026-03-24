CREATE TABLE IF NOT EXISTS accounts (
    id          TEXT PRIMARY KEY NOT NULL,
    username    TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE TABLE IF NOT EXISTS sessions (
    token       TEXT PRIMARY KEY NOT NULL,
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at  INTEGER NOT NULL
);

CREATE INDEX idx_sessions_account_id ON sessions(account_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
