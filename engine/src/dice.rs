use crate::engine::EngineError;

fn validate_roll(sides: u32) -> Result<(), EngineError> {
    if sides == 0 {
        return Err(EngineError::Validation(
            "Dice must have at least 1 side".to_string(),
        ));
    }

    Ok(())
}

fn validate_rolls(count: usize, sides: u32) -> Result<(), EngineError> {
    if count == 0 {
        return Err(EngineError::Validation(
            "Must roll at least 1 die".to_string(),
        ));
    }

    validate_roll(sides)
}

/// Validates that `icon` is a valid Font Awesome class name.
///
/// Accepted forms:
/// - Single token:  `fa-[a-z0-9-]+`  (e.g. `fa-skull`, `fa-dice-d20`)
/// - Style + icon:  `fa-(solid|regular|brands) fa-[a-z0-9-]+`  (e.g. `fa-solid fa-skull`)
///
/// Rejects empty strings, uppercase letters, underscores, and non-FA icon systems.
pub fn validate_fa_icon(icon: &str) -> Result<(), EngineError> {
    let invalid = || {
        EngineError::Validation(format!(
            "invalid Font Awesome icon: {:?}. Expected format: \"fa-<name>\" or \
             \"fa-(solid|regular|brands) fa-<name>\"",
            icon
        ))
    };

    if icon.is_empty() {
        return Err(invalid());
    }

    let parts: Vec<&str> = icon.splitn(2, ' ').collect();
    match parts.as_slice() {
        [single] => {
            if is_fa_token(single) && !is_fa_style_prefix(single) {
                Ok(())
            } else {
                Err(invalid())
            }
        }
        [style, name] => {
            let valid_style = matches!(*style, "fa-solid" | "fa-regular" | "fa-brands");
            if valid_style && is_fa_token(name) {
                Ok(())
            } else {
                Err(invalid())
            }
        }
        _ => Err(invalid()),
    }
}

/// Returns `true` if `s` matches `fa-[a-z0-9-]+`.
fn is_fa_token(s: &str) -> bool {
    s.starts_with("fa-")
        && s.len() > 3
        && s[3..]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Returns `true` if `s` is a FA style prefix (not a standalone renderable icon).
fn is_fa_style_prefix(s: &str) -> bool {
    matches!(s, "fa-solid" | "fa-regular" | "fa-brands")
}

pub fn roll_die(sides: u32) -> Result<u32, EngineError> {
    validate_roll(sides)?;
    let result = rand::random_range(1..=sides);

    Ok(result)
}

pub fn roll_dice(count: usize, sides: u32) -> Result<Vec<u32>, EngineError> {
    validate_rolls(count, sides)?;

    Ok((0..count).map(|_| rand::random_range(1..=sides)).collect())
}

pub fn roll_dice_total(count: usize, sides: u32) -> Result<u32, EngineError> {
    Ok(roll_dice(count, sides)?.into_iter().sum())
}

#[cfg(test)]
mod tests {
    use super::{roll_dice, roll_dice_total, roll_die, validate_fa_icon};

    #[test]
    fn roll_die_stays_within_range() {
        for _ in 0..100 {
            let value = roll_die(6).expect("roll d6");
            assert!((1..=6).contains(&value));
        }
    }

    #[test]
    fn roll_dice_returns_requested_count_within_range() {
        let values = roll_dice(5, 20).expect("roll 5d20");
        assert_eq!(values.len(), 5);
        assert!(values.iter().all(|value| (1..=20).contains(value)));
    }

    #[test]
    fn roll_dice_total_stays_in_expected_range() {
        for _ in 0..100 {
            let total = roll_dice_total(3, 6).expect("roll 3d6");
            assert!((3..=18).contains(&total));
        }
    }

    #[test]
    fn rejects_invalid_die_shapes() {
        assert!(roll_die(0).is_err());
        assert!(roll_dice(0, 6).is_err());
        assert!(roll_dice(1, 0).is_err());
    }

    // --- FA icon validation ---

    #[test]
    fn valid_fa_icons() {
        assert!(validate_fa_icon("fa-skull").is_ok());
        assert!(validate_fa_icon("fa-dice-d20").is_ok());
        assert!(validate_fa_icon("fa-solid fa-skull").is_ok());
        assert!(validate_fa_icon("fa-regular fa-star").is_ok());
        assert!(validate_fa_icon("fa-brands fa-github").is_ok());
    }

    #[test]
    fn rejects_empty_icon() {
        assert!(validate_fa_icon("").is_err());
    }

    #[test]
    fn rejects_uppercase_icon() {
        assert!(validate_fa_icon("Fa-Skull").is_err());
    }

    #[test]
    fn rejects_non_fa_icon_system() {
        assert!(validate_fa_icon("mdi-sword-cross").is_err());
    }

    #[test]
    fn rejects_underscore_separator() {
        assert!(validate_fa_icon("fa_heart").is_err());
    }

    #[test]
    fn rejects_malformed_style_prefix() {
        assert!(validate_fa_icon("fa-duotone fa-skull").is_err());
    }

    #[test]
    fn rejects_style_without_icon() {
        // bare style prefix is not a renderable icon
        assert!(validate_fa_icon("fa-solid").is_err());
        assert!(validate_fa_icon("fa-regular").is_err());
        assert!(validate_fa_icon("fa-brands").is_err());
        // style + empty name should also fail
        assert!(validate_fa_icon("fa-solid ").is_err());
    }
}
