-- Items owned by characters. slot=NULL means in inventory; non-NULL means equipped to that slot.
CREATE TABLE items (
    id           TEXT PRIMARY KEY NOT NULL,
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    template_id  TEXT NOT NULL,
    slot         TEXT,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_items_character_id ON items(character_id);

-- Items on the floor of a room (dropped by players or from loot).
CREATE TABLE room_items (
    id          TEXT PRIMARY KEY NOT NULL,
    room_id     TEXT NOT NULL,
    template_id TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
CREATE INDEX idx_room_items_room_id ON room_items(room_id);

-- Add gold to characters (default 0).
ALTER TABLE characters ADD COLUMN gold INTEGER NOT NULL DEFAULT 0;
