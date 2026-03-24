-- Allow account_flags to store flags for character IDs too.
-- Drop and recreate without strict FK (SQLite doesn't support ALTER CONSTRAINT).
DROP TABLE IF EXISTS account_flags;
CREATE TABLE account_flags (
    account_id TEXT NOT NULL,
    flag       TEXT NOT NULL,
    PRIMARY KEY (account_id, flag)
);
