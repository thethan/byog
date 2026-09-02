use std::collections::HashMap;

use token_pools::TokenPool;

pub const DEFAULT_PLAYER_ID: &str = "player-1";
pub const DEFAULT_STARTING_LIFE: u32 = 40;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub token_pools: HashMap<String, TokenPool>,
}

impl Default for Player {
    fn default() -> Self {
        let life = TokenPool::configured(
            "life",
            "Life",
            "fa-solid fa-heart",
            None,
            DEFAULT_STARTING_LIFE,
            Some(0),
            None,
            true,
        )
        .expect("the default life pool is valid");

        Self {
            id: DEFAULT_PLAYER_ID.to_string(),
            name: "Player 1".to_string(),
            token_pools: [(life.id.clone(), life)].into_iter().collect(),
        }
    }
}

impl Player {
    pub fn with_token_pools(
        id: impl Into<String>,
        name: impl Into<String>,
        pools: Vec<TokenPool>,
    ) -> Result<Self, String> {
        let mut token_pools = HashMap::new();
        for pool in pools {
            pool.validate()?;
            if token_pools.insert(pool.id.clone(), pool).is_some() {
                return Err("Duplicate player token pool id".into());
            }
        }
        Ok(Self {
            id: id.into(),
            name: name.into(),
            token_pools,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_STARTING_LIFE, Player};

    #[test]
    fn player_starts_with_life() {
        let player = Player::default();
        let life = player.token_pools.get("life").expect("life pool");
        assert_eq!(life.count, DEFAULT_STARTING_LIFE);
        assert_eq!(life.token, "fa-solid fa-heart");
        assert!(life.active);
    }
}
