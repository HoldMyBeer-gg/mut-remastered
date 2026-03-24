CREATE TABLE IF NOT EXISTS player_positions (
    account_id  TEXT PRIMARY KEY NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    room_id     TEXT NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
