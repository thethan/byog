use rand::Rng;

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

pub fn roll_die(sides: u32) -> Result<u32, EngineError> {
    validate_roll(sides)?;
    Ok(rand::rng().random_range(1..=sides))
}

pub fn roll_dice(count: usize, sides: u32) -> Result<Vec<u32>, EngineError> {
    validate_rolls(count, sides)?;
    let mut rng = rand::rng();

    Ok((0..count).map(|_| rng.random_range(1..=sides)).collect())
}

pub fn roll_dice_total(count: usize, sides: u32) -> Result<u32, EngineError> {
    Ok(roll_dice(count, sides)?.into_iter().sum())
}

#[cfg(test)]
mod tests {
    use super::{roll_dice, roll_dice_total, roll_die};

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
}
