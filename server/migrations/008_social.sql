-- Add visible description to characters (what other players see on "look at").
ALTER TABLE characters ADD COLUMN description TEXT NOT NULL DEFAULT '';

-- Channel toggle preferences per character.
CREATE TABLE channel_toggles (
    character_id TEXT NOT NULL REFERENCES characters(id) ON DELETE CASCADE,
    channel      TEXT NOT NULL,
    enabled      INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (character_id, channel)
);
