CREATE TABLE IF NOT EXISTS account_flags (
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    flag       TEXT NOT NULL,
    PRIMARY KEY (account_id, flag)
);
