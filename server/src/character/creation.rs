//! Character creation logic: point-buy validation and initial stat calculation.

use super::types::{ability_modifier, Class, Race, CON, INT};

/// Results of initial stat calculation for a new level-1 character.
#[derive(Debug, Clone)]
pub struct CharacterStats {
    /// Final ability scores after racial bonuses.
    pub final_scores: [u8; 6],
    pub hp: i32,
    pub max_hp: i32,
    pub mana: i32,
    pub max_mana: i32,
    pub stamina: i32,
    pub max_stamina: i32,
}

/// Validate a point-buy ability score allocation.
///
/// Rules:
/// - All 6 scores must be in range 8-15 (before racial bonuses).
/// - Point cost: 8→9 = 1pt, 9→10 = 1pt, ... 12→13 = 1pt, 13→14 = 2pt, 14→15 = 2pt.
/// - Total cost must equal exactly 27 points.
pub fn validate_point_buy(scores: &[u8; 6]) -> Result<(), String> {
    let mut total_cost: u32 = 0;

    for (i, &score) in scores.iter().enumerate() {
        if score < 8 || score > 15 {
            return Err(format!(
                "Ability score {} is {} — must be between 8 and 15",
                crate::character::types::ABILITY_NAMES[i],
                score
            ));
        }

        // Cost from 8 to target score
        let cost = point_buy_cost(score);
        total_cost += cost;
    }

    if total_cost != 27 {
        return Err(format!(
            "Point buy total is {} — must be exactly 27",
            total_cost
        ));
    }

    Ok(())
}

/// Calculate the point-buy cost for a single ability score.
///
/// Scores 8-13 cost 1 point per increment from 8.
/// Scores 14-15 cost 2 points per increment above 13.
fn point_buy_cost(score: u8) -> u32 {
    match score {
        8 => 0,
        9 => 1,
        10 => 2,
        11 => 3,
        12 => 4,
        13 => 5,
        14 => 7, // 5 + 2
        15 => 9, // 5 + 2 + 2
        _ => 0,  // Should not happen after range check
    }
}

/// Validate the character name.
///
/// Rules:
/// - 2-24 characters long
/// - Only letters, spaces, hyphens, apostrophes
/// - Must start with a letter
/// - No consecutive spaces/hyphens/apostrophes
pub fn validate_name(name: &str) -> Result<(), String> {
    let name = name.trim();

    if name.len() < 2 {
        return Err("Character name must be at least 2 characters".to_string());
    }
    if name.len() > 24 {
        return Err("Character name must be 24 characters or fewer".to_string());
    }
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err("Character name must start with a letter".to_string());
    }

    let mut prev_special = false;
    for c in name.chars() {
        if c.is_ascii_alphabetic() {
            prev_special = false;
        } else if c == ' ' || c == '-' || c == '\'' {
            if prev_special {
                return Err("Character name cannot have consecutive special characters".to_string());
            }
            prev_special = true;
        } else {
            return Err(format!(
                "Character name contains invalid character: '{}'",
                c
            ));
        }
    }

    Ok(())
}

/// Calculate initial stats for a new level-1 character.
///
/// Formulas:
/// - Final scores = base scores + racial bonuses
/// - HP = class hit die (max) + CON modifier + racial HP bonus
/// - Mana = 10 + INT modifier × 2 (min 0)
/// - Stamina = 10 + CON modifier × 2 (min 0)
pub fn calculate_initial_stats(
    race: &Race,
    class: &Class,
    base_scores: &[u8; 6],
    racial_choices: &[u8],
) -> CharacterStats {
    let bonuses = race.stat_bonuses(racial_choices);

    let mut final_scores = [0u8; 6];
    for i in 0..6 {
        // Clamp to prevent underflow (shouldn't happen with valid point buy + positive bonuses)
        final_scores[i] = (base_scores[i] as i32 + bonuses[i] as i32).clamp(1, 30) as u8;
    }

    let con_mod = ability_modifier(final_scores[CON]);
    let int_mod = ability_modifier(final_scores[INT]);

    let hp = class.hit_die() + con_mod + race.bonus_hp_per_level();
    let hp = hp.max(1); // Minimum 1 HP

    let mana = (10 + int_mod * 2).max(0);
    let stamina = (10 + con_mod * 2).max(0);

    CharacterStats {
        final_scores,
        hp,
        max_hp: hp,
        mana,
        max_mana: mana,
        stamina,
        max_stamina: stamina,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::types::{Class, Race};

    #[test]
    fn test_validate_point_buy_standard() {
        // Classic "15, 14, 13, 12, 10, 8" = 9 + 7 + 5 + 4 + 2 + 0 = 27
        assert!(validate_point_buy(&[15, 14, 13, 12, 10, 8]).is_ok());
    }

    #[test]
    fn test_validate_point_buy_all_tens() {
        // 6 × 10 = 6 × 2 = 12 points, not 27
        assert!(validate_point_buy(&[10, 10, 10, 10, 10, 10]).is_err());
    }

    #[test]
    fn test_validate_point_buy_too_high() {
        // Score 16 is above 15
        assert!(validate_point_buy(&[16, 14, 13, 12, 10, 8]).is_err());
    }

    #[test]
    fn test_validate_point_buy_too_low() {
        // Score 7 is below 8
        assert!(validate_point_buy(&[7, 14, 13, 12, 10, 8]).is_err());
    }

    #[test]
    fn test_validate_point_buy_over_budget() {
        // 15, 15, 15, 12, 10, 8 = 9 + 9 + 9 + 4 + 2 + 0 = 33
        assert!(validate_point_buy(&[15, 15, 15, 12, 10, 8]).is_err());
    }

    #[test]
    fn test_validate_point_buy_even_spread() {
        // 13, 13, 13, 12, 12, 8 = 5+5+5+4+4+0 = 23, not 27
        assert!(validate_point_buy(&[13, 13, 13, 12, 12, 8]).is_err());
    }

    #[test]
    fn test_validate_point_buy_another_valid() {
        // 15, 15, 12, 10, 8, 8 = 9+9+4+2+0+0 = 24... not 27
        // 15, 14, 14, 10, 10, 8 = 9+7+7+2+2+0 = 27 ✓
        assert!(validate_point_buy(&[15, 14, 14, 10, 10, 8]).is_ok());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(validate_name("Grok").is_ok());
        assert!(validate_name("Sir Lancelot").is_ok());
        assert!(validate_name("O'Brien").is_ok());
        assert!(validate_name("Half-Pint").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(validate_name("G").is_err()); // too short
        assert!(validate_name("").is_err());
        assert!(validate_name("123abc").is_err()); // starts with number
        assert!(validate_name("Grok!!").is_err()); // invalid char
        assert!(validate_name("A  B").is_err()); // consecutive spaces
    }

    #[test]
    fn test_calculate_stats_dwarf_warrior() {
        // Dwarf Warrior: base STR 15, DEX 10, CON 15, INT 8, WIS 10, CHA 8
        // Dwarf bonus: +2 CON → CON becomes 17, mod = +3
        // Warrior hit die: 10
        // HP = 10 + 3 (CON mod) + 1 (Dwarf HP bonus) = 14
        // Mana = 10 + (-1) * 2 = 8 (INT 8 → mod -1)
        // Stamina = 10 + 3 * 2 = 16
        let stats =
            calculate_initial_stats(&Race::Dwarf, &Class::Warrior, &[15, 10, 15, 8, 10, 8], &[]);
        assert_eq!(stats.final_scores, [15, 10, 17, 8, 10, 8]);
        assert_eq!(stats.hp, 14);
        assert_eq!(stats.max_hp, 14);
        assert_eq!(stats.mana, 8);
        assert_eq!(stats.stamina, 16);
    }

    #[test]
    fn test_calculate_stats_elf_mage() {
        // Elf Mage: base STR 8, DEX 14, CON 12, INT 15, WIS 10, CHA 10
        // Elf bonus: +2 DEX → DEX becomes 16
        // Mage hit die: 6
        // INT 15 → mod +2
        // CON 12 → mod +1
        // HP = 6 + 1 = 7
        // Mana = 10 + 2 * 2 = 14
        // Stamina = 10 + 1 * 2 = 12
        let stats =
            calculate_initial_stats(&Race::Elf, &Class::Mage, &[8, 14, 12, 15, 10, 10], &[]);
        assert_eq!(stats.final_scores, [8, 16, 12, 15, 10, 10]);
        assert_eq!(stats.hp, 7);
        assert_eq!(stats.mana, 14);
        assert_eq!(stats.stamina, 12);
    }

    #[test]
    fn test_calculate_stats_human_rogue() {
        // Human Rogue: base STR 10, DEX 15, CON 14, INT 10, WIS 8, CHA 12
        // Human bonus: +1 DEX (idx 1), +1 CON (idx 2) → DEX 16, CON 15
        // Rogue hit die: 8
        // DEX 16 → mod +3
        // CON 15 → mod +2
        // INT 10 → mod 0
        // HP = 8 + 2 = 10
        // Mana = 10 + 0 = 10
        // Stamina = 10 + 2 * 2 = 14
        let stats = calculate_initial_stats(
            &Race::Human,
            &Class::Rogue,
            &[10, 15, 14, 10, 8, 12],
            &[1, 2], // +1 DEX, +1 CON
        );
        assert_eq!(stats.final_scores, [10, 16, 15, 10, 8, 12]);
        assert_eq!(stats.hp, 10);
        assert_eq!(stats.mana, 10);
        assert_eq!(stats.stamina, 14);
    }
}
