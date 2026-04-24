//! Pure combat resolution functions — no async, no DB, no IO.
//!
//! All randomness flows through `rand::rng()`. The functions are deterministic
//! given the same RNG state, making them testable with seeded RNG in the future.

use rand::Rng;

/// Result of an attack roll.
#[derive(Debug, Clone, PartialEq)]
pub enum AttackResult {
    /// Natural 20 — double damage dice.
    CriticalHit { roll: i32, total: i32 },
    /// Roll + bonus >= AC.
    Hit { roll: i32, total: i32 },
    /// Roll + bonus < AC.
    Miss { roll: i32, total: i32 },
    /// Natural 1 — automatic miss regardless of bonuses.
    CriticalMiss,
}

impl AttackResult {
    pub fn is_hit(&self) -> bool {
        matches!(
            self,
            AttackResult::CriticalHit { .. } | AttackResult::Hit { .. }
        )
    }

    pub fn is_crit(&self) -> bool {
        matches!(self, AttackResult::CriticalHit { .. })
    }
}

/// Roll a d20.
pub fn roll_d20() -> i32 {
    rand::rng().random_range(1..=20)
}

/// Roll `count` dice of `sides` sides and return the total.
pub fn roll_dice(count: u32, sides: u32) -> i32 {
    let mut rng = rand::rng();
    (0..count).map(|_| rng.random_range(1..=sides as i32)).sum()
}

/// Roll initiative: d20 + DEX modifier.
pub fn roll_initiative(dex_mod: i32) -> i32 {
    roll_d20() + dex_mod
}

/// Resolve an attack roll: d20 + attacker_bonus vs defender_ac.
///
/// - Natural 20: CriticalHit (always hits, double damage dice)
/// - Natural 1: CriticalMiss (always misses)
/// - Otherwise: Hit if total >= AC, Miss if total < AC
pub fn resolve_attack(attacker_bonus: i32, defender_ac: i32) -> AttackResult {
    let roll = roll_d20();
    let total = roll + attacker_bonus;

    if roll == 20 {
        AttackResult::CriticalHit { roll, total }
    } else if roll == 1 {
        AttackResult::CriticalMiss
    } else if total >= defender_ac {
        AttackResult::Hit { roll, total }
    } else {
        AttackResult::Miss { roll, total }
    }
}

/// Roll damage: `count`d`sides` + bonus. On critical hit, double the dice count.
///
/// Returns (total_damage, description_string).
/// Example: (7, "1d6+3") or (14, "2d6+3") for a crit.
pub fn roll_damage(count: u32, sides: u32, bonus: i32, is_crit: bool) -> (i32, String) {
    let dice_count = if is_crit { count * 2 } else { count };
    let dice_total = roll_dice(dice_count, sides);
    let total = (dice_total + bonus).max(0);

    let desc = if bonus > 0 {
        format!("{}d{}+{}", dice_count, sides, bonus)
    } else if bonus < 0 {
        format!("{}d{}{}", dice_count, sides, bonus)
    } else {
        format!("{}d{}", dice_count, sides)
    };

    (total, desc)
}

/// Format a combat log entry per CMBT-01.
///
/// Examples:
/// - "Grok rolled 14 + 2 STR vs AC 12 — HIT! 7 damage (1d6+3)"
/// - "Goblin Scout rolled 8 + 4 vs AC 16 — MISS!"
/// - "CRITICAL HIT! Grok rolled 20 + 2 STR vs AC 12 — 14 damage (2d6+3)"
/// - "Grok rolled 1 — CRITICAL MISS!"
pub fn format_combat_log(
    attacker_name: &str,
    result: &AttackResult,
    attacker_bonus: i32,
    defender_ac: i32,
    ability_label: &str,
    damage: Option<(i32, String)>,
) -> String {
    match result {
        AttackResult::CriticalHit { roll, .. } => {
            if let Some((dmg, desc)) = damage {
                format!(
                    "CRITICAL HIT! {} rolled {} + {} {} vs AC {} — {} damage ({})",
                    attacker_name, roll, attacker_bonus, ability_label, defender_ac, dmg, desc
                )
            } else {
                format!(
                    "CRITICAL HIT! {} rolled {} + {} {} vs AC {}",
                    attacker_name, roll, attacker_bonus, ability_label, defender_ac
                )
            }
        }
        AttackResult::Hit { roll, .. } => {
            if let Some((dmg, desc)) = damage {
                format!(
                    "{} rolled {} + {} {} vs AC {} — HIT! {} damage ({})",
                    attacker_name, roll, attacker_bonus, ability_label, defender_ac, dmg, desc
                )
            } else {
                format!(
                    "{} rolled {} + {} {} vs AC {} — HIT!",
                    attacker_name, roll, attacker_bonus, ability_label, defender_ac
                )
            }
        }
        AttackResult::Miss { roll, .. } => {
            format!(
                "{} rolled {} + {} {} vs AC {} — MISS!",
                attacker_name, roll, attacker_bonus, ability_label, defender_ac
            )
        }
        AttackResult::CriticalMiss => {
            format!("{} rolled 1 — CRITICAL MISS!", attacker_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roll_d20_in_range() {
        for _ in 0..100 {
            let r = roll_d20();
            assert!((1..=20).contains(&r), "d20 roll out of range: {}", r);
        }
    }

    #[test]
    fn test_roll_dice_in_range() {
        for _ in 0..100 {
            let r = roll_dice(2, 6);
            assert!((2..=12).contains(&r), "2d6 roll out of range: {}", r);
        }
    }

    #[test]
    fn test_format_combat_log_hit() {
        let result = AttackResult::Hit {
            roll: 14,
            total: 16,
        };
        let log = format_combat_log(
            "Grok",
            &result,
            2,
            12,
            "STR",
            Some((7, "1d6+3".to_string())),
        );
        assert_eq!(
            log,
            "Grok rolled 14 + 2 STR vs AC 12 — HIT! 7 damage (1d6+3)"
        );
    }

    #[test]
    fn test_format_combat_log_miss() {
        let result = AttackResult::Miss { roll: 8, total: 12 };
        let log = format_combat_log("Goblin Scout", &result, 4, 16, "", None);
        assert_eq!(log, "Goblin Scout rolled 8 + 4  vs AC 16 — MISS!");
    }

    #[test]
    fn test_format_combat_log_crit() {
        let result = AttackResult::CriticalHit {
            roll: 20,
            total: 22,
        };
        let log = format_combat_log(
            "Grok",
            &result,
            2,
            12,
            "STR",
            Some((14, "2d6+3".to_string())),
        );
        assert_eq!(
            log,
            "CRITICAL HIT! Grok rolled 20 + 2 STR vs AC 12 — 14 damage (2d6+3)"
        );
    }

    #[test]
    fn test_format_combat_log_crit_miss() {
        let result = AttackResult::CriticalMiss;
        let log = format_combat_log("Grok", &result, 2, 12, "STR", None);
        assert_eq!(log, "Grok rolled 1 — CRITICAL MISS!");
    }

    #[test]
    fn test_roll_damage_normal() {
        for _ in 0..50 {
            let (dmg, desc) = roll_damage(1, 6, 3, false);
            assert!((4..=9).contains(&dmg), "1d6+3 damage out of range: {}", dmg);
            assert_eq!(desc, "1d6+3");
        }
    }

    #[test]
    fn test_roll_damage_crit_doubles_dice() {
        for _ in 0..50 {
            let (dmg, desc) = roll_damage(1, 6, 3, true);
            assert!(
                (5..=15).contains(&dmg),
                "crit 2d6+3 damage out of range: {}",
                dmg
            );
            assert_eq!(desc, "2d6+3");
        }
    }
}
