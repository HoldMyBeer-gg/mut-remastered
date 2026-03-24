//! Race, class, gender, and ability score definitions for MUT characters.
//!
//! All game-rule constants live here so they can be changed without touching
//! protocol or persistence code.

use std::fmt;

/// The six D&D ability scores, in canonical order.
pub const ABILITY_NAMES: [&str; 6] = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];

/// Indices into the 6-element ability score array.
pub const STR: usize = 0;
pub const DEX: usize = 1;
pub const CON: usize = 2;
pub const INT: usize = 3;
pub const WIS: usize = 4;
pub const CHA: usize = 5;

// ── Race ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Race {
    Human,
    Elf,
    Dwarf,
    Halfling,
    Orc,
    Gnome,
    HalfElf,
    Tiefling,
}

impl Race {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").replace(' ', "_").as_str() {
            "human" => Some(Race::Human),
            "elf" => Some(Race::Elf),
            "dwarf" => Some(Race::Dwarf),
            "halfling" => Some(Race::Halfling),
            "orc" => Some(Race::Orc),
            "gnome" => Some(Race::Gnome),
            "half_elf" | "halfelf" => Some(Race::HalfElf),
            "tiefling" => Some(Race::Tiefling),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Race::Human => "human",
            Race::Elf => "elf",
            Race::Dwarf => "dwarf",
            Race::Halfling => "halfling",
            Race::Orc => "orc",
            Race::Gnome => "gnome",
            Race::HalfElf => "half_elf",
            Race::Tiefling => "tiefling",
        }
    }

    /// Display name for player-facing messages.
    pub fn display_name(&self) -> &'static str {
        match self {
            Race::Human => "Human",
            Race::Elf => "Elf",
            Race::Dwarf => "Dwarf",
            Race::Halfling => "Halfling",
            Race::Orc => "Orc",
            Race::Gnome => "Gnome",
            Race::HalfElf => "Half-Elf",
            Race::Tiefling => "Tiefling",
        }
    }

    /// Fixed racial stat bonuses as [STR, DEX, CON, INT, WIS, CHA].
    ///
    /// Human and Half-Elf have flexible bonuses handled by `choices`.
    /// - Human: `choices` should contain two distinct indices (0-5) for +1 each
    /// - Half-Elf: gets +2 CHA fixed, plus `choices` should contain one index (0-5, not CHA) for +1
    /// - All others: `choices` is ignored
    pub fn stat_bonuses(&self, choices: &[u8]) -> [i8; 6] {
        let mut bonuses = [0i8; 6];
        match self {
            Race::Human => {
                // +1 to two chosen abilities
                for &idx in choices.iter().take(2) {
                    if (idx as usize) < 6 {
                        bonuses[idx as usize] += 1;
                    }
                }
            }
            Race::Elf => {
                bonuses[DEX] += 2;
            }
            Race::Dwarf => {
                bonuses[CON] += 2;
            }
            Race::Halfling => {
                bonuses[DEX] += 2;
            }
            Race::Orc => {
                bonuses[STR] += 2;
                bonuses[CON] += 1;
            }
            Race::Gnome => {
                bonuses[INT] += 2;
            }
            Race::HalfElf => {
                bonuses[CHA] += 2;
                // +1 to one chosen ability (not CHA)
                for &idx in choices.iter().take(1) {
                    if (idx as usize) < 6 && idx as usize != CHA {
                        bonuses[idx as usize] += 1;
                    }
                }
            }
            Race::Tiefling => {
                bonuses[CHA] += 2;
                bonuses[INT] += 1;
            }
        }
        bonuses
    }

    /// Whether this race gets bonus HP per level.
    pub fn bonus_hp_per_level(&self) -> i32 {
        match self {
            Race::Dwarf => 1,
            _ => 0,
        }
    }

    /// Validate racial bonus choices. Returns an error message if invalid.
    pub fn validate_choices(&self, choices: &[u8]) -> Result<(), String> {
        match self {
            Race::Human => {
                if choices.len() != 2 {
                    return Err("Human requires exactly 2 racial bonus choices".to_string());
                }
                if choices[0] == choices[1] {
                    return Err("Human racial bonus choices must be different abilities".to_string());
                }
                for &c in choices {
                    if c as usize >= 6 {
                        return Err(format!("Invalid ability index: {}", c));
                    }
                }
                Ok(())
            }
            Race::HalfElf => {
                if choices.len() != 1 {
                    return Err("Half-Elf requires exactly 1 racial bonus choice".to_string());
                }
                if choices[0] as usize >= 6 {
                    return Err(format!("Invalid ability index: {}", choices[0]));
                }
                if choices[0] as usize == CHA {
                    return Err("Half-Elf cannot choose CHA for the flexible bonus (already +2 CHA)".to_string());
                }
                Ok(())
            }
            _ => Ok(()), // Other races ignore choices
        }
    }
}

impl fmt::Display for Race {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ── Class ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Warrior,
    Ranger,
    Cleric,
    Mage,
    Rogue,
}

impl Class {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "warrior" | "fighter" => Some(Class::Warrior),
            "ranger" | "archer" => Some(Class::Ranger),
            "cleric" | "priest" | "healer" => Some(Class::Cleric),
            "mage" | "wizard" | "sorcerer" => Some(Class::Mage),
            "rogue" | "thief" => Some(Class::Rogue),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Class::Warrior => "warrior",
            Class::Ranger => "ranger",
            Class::Cleric => "cleric",
            Class::Mage => "mage",
            Class::Rogue => "rogue",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Class::Warrior => "Warrior",
            Class::Ranger => "Ranger",
            Class::Cleric => "Cleric",
            Class::Mage => "Mage",
            Class::Rogue => "Rogue",
        }
    }

    /// Maximum HP die for this class at level 1 (characters start with max roll).
    pub fn hit_die(&self) -> i32 {
        match self {
            Class::Warrior => 10,
            Class::Ranger => 8,
            Class::Cleric => 8,
            Class::Mage => 6,
            Class::Rogue => 8,
        }
    }

    /// The primary ability for this class (affects certain abilities).
    pub fn primary_ability(&self) -> usize {
        match self {
            Class::Warrior => STR,
            Class::Ranger => DEX,
            Class::Cleric => WIS,
            Class::Mage => INT,
            Class::Rogue => DEX,
        }
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ── Gender ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    NonBinary,
}

impl Gender {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "_").replace(' ', "_").as_str() {
            "male" | "m" => Some(Gender::Male),
            "female" | "f" => Some(Gender::Female),
            "nonbinary" | "non_binary" | "nb" | "enby" => Some(Gender::NonBinary),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Gender::Male => "male",
            Gender::Female => "female",
            Gender::NonBinary => "non_binary",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Gender::Male => "Male",
            Gender::Female => "Female",
            Gender::NonBinary => "Non-Binary",
        }
    }
}

impl fmt::Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ── Ability Score Helpers ─────────────────────────────────────────

/// Calculate the ability modifier from a score.
/// Formula: floor((score - 10) / 2), using D&D floor division.
///
/// Rust's integer division truncates toward zero, but D&D floor division
/// rounds toward negative infinity. For odd ability scores below 10 this
/// matters: score 9 should give -1, not 0.
pub fn ability_modifier(score: u8) -> i32 {
    let diff = score as i32 - 10;
    // Use div_euclid for floor division toward negative infinity
    diff.div_euclid(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_modifier() {
        assert_eq!(ability_modifier(1), -5); // floor((1-10)/2) = floor(-4.5) = -5
        assert_eq!(ability_modifier(8), -1);
        assert_eq!(ability_modifier(9), -1); // floor((9-10)/2) = floor(-0.5) = -1
        assert_eq!(ability_modifier(10), 0);
        assert_eq!(ability_modifier(11), 0); // floor((11-10)/2) = floor(0.5) = 0
        assert_eq!(ability_modifier(12), 1);
        assert_eq!(ability_modifier(14), 2);
        assert_eq!(ability_modifier(16), 3);
        assert_eq!(ability_modifier(18), 4);
        assert_eq!(ability_modifier(20), 5);
    }

    #[test]
    fn test_ability_modifier_odd_scores() {
        // D&D floor division: odd scores below 10 round down
        assert_eq!(ability_modifier(7), -2); // floor(-3/2) = floor(-1.5) = -2
        assert_eq!(ability_modifier(9), -1); // floor(-1/2) = floor(-0.5) = -1
        assert_eq!(ability_modifier(11), 0); // floor(1/2) = floor(0.5) = 0
        assert_eq!(ability_modifier(13), 1); // floor(3/2) = floor(1.5) = 1
        assert_eq!(ability_modifier(15), 2);
    }

    #[test]
    fn test_race_from_str() {
        assert_eq!(Race::from_str("human"), Some(Race::Human));
        assert_eq!(Race::from_str("HALF_ELF"), Some(Race::HalfElf));
        assert_eq!(Race::from_str("half-elf"), Some(Race::HalfElf));
        assert_eq!(Race::from_str("halfelf"), Some(Race::HalfElf));
        assert_eq!(Race::from_str("tiefling"), Some(Race::Tiefling));
        assert_eq!(Race::from_str("invalid"), None);
    }

    #[test]
    fn test_class_from_str() {
        assert_eq!(Class::from_str("warrior"), Some(Class::Warrior));
        assert_eq!(Class::from_str("fighter"), Some(Class::Warrior));
        assert_eq!(Class::from_str("Mage"), Some(Class::Mage));
        assert_eq!(Class::from_str("wizard"), Some(Class::Mage));
        assert_eq!(Class::from_str("invalid"), None);
    }

    #[test]
    fn test_gender_from_str() {
        assert_eq!(Gender::from_str("male"), Some(Gender::Male));
        assert_eq!(Gender::from_str("M"), Some(Gender::Male));
        assert_eq!(Gender::from_str("non-binary"), Some(Gender::NonBinary));
        assert_eq!(Gender::from_str("nb"), Some(Gender::NonBinary));
        assert_eq!(Gender::from_str("enby"), Some(Gender::NonBinary));
        assert_eq!(Gender::from_str("invalid"), None);
    }

    #[test]
    fn test_human_racial_bonuses() {
        let bonuses = Race::Human.stat_bonuses(&[0, 2]); // +1 STR, +1 CON
        assert_eq!(bonuses, [1, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn test_elf_racial_bonuses() {
        let bonuses = Race::Elf.stat_bonuses(&[]);
        assert_eq!(bonuses, [0, 2, 0, 0, 0, 0]); // +2 DEX
    }

    #[test]
    fn test_orc_racial_bonuses() {
        let bonuses = Race::Orc.stat_bonuses(&[]);
        assert_eq!(bonuses, [2, 0, 1, 0, 0, 0]); // +2 STR, +1 CON
    }

    #[test]
    fn test_half_elf_racial_bonuses() {
        let bonuses = Race::HalfElf.stat_bonuses(&[0]); // +1 STR choice, +2 CHA fixed
        assert_eq!(bonuses, [1, 0, 0, 0, 0, 2]);
    }

    #[test]
    fn test_human_validate_choices() {
        assert!(Race::Human.validate_choices(&[0, 2]).is_ok());
        assert!(Race::Human.validate_choices(&[0]).is_err()); // need 2
        assert!(Race::Human.validate_choices(&[0, 0]).is_err()); // must be different
        assert!(Race::Human.validate_choices(&[0, 7]).is_err()); // invalid index
    }

    #[test]
    fn test_half_elf_validate_choices() {
        assert!(Race::HalfElf.validate_choices(&[0]).is_ok()); // STR
        assert!(Race::HalfElf.validate_choices(&[5]).is_err()); // CHA not allowed
        assert!(Race::HalfElf.validate_choices(&[]).is_err()); // need 1
    }
}
