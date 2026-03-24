CREATE TABLE IF NOT EXISTS world_state (
    room_id     TEXT NOT NULL,
    state_key   TEXT NOT NULL,
    state_value TEXT NOT NULL,
    updated_at  INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (room_id, state_key)
);
