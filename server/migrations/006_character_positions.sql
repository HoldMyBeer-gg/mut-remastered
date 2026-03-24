-- Position tracking by character_id (replaces account-based player_positions).
-- The old player_positions table is kept for backward compatibility during migration.
-- New code uses character_positions exclusively.
CREATE TABLE character_positions (
    character_id TEXT PRIMARY KEY NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    room_id      TEXT NOT NULL,
    updated_at   INTEGER NOT NULL DEFAULT (unixepoch())
);
