use std::collections::HashMap;

use crate::card::Card;
use crate::engine::EngineError;
use crate::player::{DEFAULT_PLAYER_ID, Player};
use token_pools::TokenPool;

// Todo define these by csv in games. For instance one game may have a "Main Stack" and a "Sideboard" zone, while another game may have a "Main Stack" and a "Command Zone" zone. Another may have library and graveyard zones, while another may have a "Deck" and a "Discard" zone. create more generic zones that then get by ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    MainStack,
    CommanderPile,
    Hand,
    Lands,
    Deck,
    Discard,
    Exile,
    Artifacts,
    Enchantments,
    Creatures,
    Battlefield,
}

#[cfg(test)]
mod token_pool_parent_tests {
    use super::GameState;
    use crate::player::{DEFAULT_PLAYER_ID, Player};
    use token_pools::TokenPool;

    #[test]
    fn child_changes_propagate_to_parent_atomically() {
        let life = TokenPool::configured(
            "life",
            "Life",
            "fa-solid fa-heart",
            None,
            40,
            Some(0),
            None,
            true,
        )
        .unwrap();
        let mut commander = TokenPool::configured(
            "commander",
            "Commander",
            "fa-solid fa-helmet-battle",
            None,
            21,
            Some(0),
            None,
            true,
        )
        .unwrap();
        commander.parent_id = Some("life".into());
        let mut state = GameState::new(Vec::new());
        state.players.insert(
            DEFAULT_PLAYER_ID.into(),
            Player::with_token_pools(DEFAULT_PLAYER_ID, "Player 1", vec![life, commander]).unwrap(),
        );

        state
            .remove_tokens_from_player_pool(DEFAULT_PLAYER_ID, "commander", 1)
            .unwrap();
        let pools = state.get_player_token_pools(DEFAULT_PLAYER_ID).unwrap();
        assert_eq!(pools["commander"].count, 20);
        assert_eq!(pools["life"].count, 39);
    }
}

impl Zone {
    pub const ALL: &'static [Zone] = &[
        Zone::MainStack,
        Zone::CommanderPile,
        Zone::Hand,
        Zone::Lands,
        Zone::Deck,
        Zone::Discard,
        Zone::Exile,
        Zone::Artifacts,
        Zone::Enchantments,
        Zone::Creatures,
        Zone::Battlefield,
    ];

    pub fn from_pile_id(value: &str) -> Option<Self> {
        match value {
            "main_stack" => Some(Self::MainStack),
            "commander_pile" => Some(Self::CommanderPile),
            "hand" => Some(Self::Hand),
            "lands" => Some(Self::Lands),
            "deck" => Some(Self::Deck),
            "discard" => Some(Self::Discard),
            "exile" => Some(Self::Exile),
            "artifacts" => Some(Self::Artifacts),
            "enchantments" => Some(Self::Enchantments),
            "creatures" => Some(Self::Creatures),
            "battlefield" => Some(Self::Battlefield),
            _ => None,
        }
    }

    pub fn is_battlefield(&self) -> bool {
        matches!(
            self,
            Self::Lands
                | Self::Artifacts
                | Self::Enchantments
                | Self::Creatures
                | Self::Battlefield
        )
    }
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::MainStack => write!(f, "MainStack"),
            Zone::CommanderPile => write!(f, "CommanderPile"),
            Zone::Hand => write!(f, "Hand"),
            Zone::Lands => write!(f, "Lands"),
            Zone::Deck => write!(f, "Deck"),
            Zone::Discard => write!(f, "Discard"),
            Zone::Exile => write!(f, "Exile"),
            Zone::Artifacts => write!(f, "Artifacts"),
            Zone::Enchantments => write!(f, "Enchantments"),
            Zone::Creatures => write!(f, "Creatures"),
            Zone::Battlefield => write!(f, "Battlefield"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub cards: HashMap<String, Card>,
    pub zones: HashMap<Zone, Vec<String>>,
    pub players: HashMap<String, Player>,
    pub zone_token_pools: HashMap<Zone, HashMap<String, TokenPool>>,
    pub card_token_pools: HashMap<String, HashMap<String, TokenPool>>,
}

impl GameState {
    pub fn new(cards: Vec<Card>) -> Self {
        let cards = cards
            .into_iter()
            .map(|card| (card.id.clone(), card))
            .collect::<HashMap<_, _>>();

        let card_token_pools = cards
            .values()
            .filter(|card| !card.token_pools.is_empty())
            .map(|card| {
                (
                    card.id.clone(),
                    card.token_pools
                        .iter()
                        .cloned()
                        .map(|pool| (pool.id.clone(), pool))
                        .collect::<HashMap<_, _>>(),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut zones = HashMap::new();
        let mut zone_token_pools = HashMap::new();
        for zone in Zone::ALL {
            zones.insert(*zone, Vec::new());
            zone_token_pools.insert(*zone, HashMap::new());
        }

        Self {
            cards,
            zones,
            players: [(DEFAULT_PLAYER_ID.to_string(), Player::default())]
                .into_iter()
                .collect(),
            zone_token_pools,
            card_token_pools,
        }
    }

    pub fn player(&self, player_id: &str) -> Result<&Player, EngineError> {
        self.players
            .get(player_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown player id '{player_id}'")))
    }

    pub fn set_player_name(&mut self, player_id: &str, name: &str) -> Result<(), EngineError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(EngineError::Validation(
                "Player name must not be empty".into(),
            ));
        }
        self.players
            .get_mut(player_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown player id '{player_id}'")))?
            .name = name.to_string();
        Ok(())
    }

    pub fn get_player_token_pools(
        &self,
        player_id: &str,
    ) -> Result<&HashMap<String, TokenPool>, EngineError> {
        Ok(&self.player(player_id)?.token_pools)
    }

    pub fn add_tokens_to_player_pool(
        &mut self,
        player_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.adjust_player_pool(player_id, pool_id, amount, true)
    }

    pub fn remove_tokens_from_player_pool(
        &mut self,
        player_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.adjust_player_pool(player_id, pool_id, amount, false)
    }

    fn adjust_player_pool(
        &mut self,
        player_id: &str,
        pool_id: &str,
        amount: u32,
        add: bool,
    ) -> Result<(), EngineError> {
        let player = self
            .players
            .get_mut(player_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown player id '{player_id}'")))?;
        let mut updated = player.token_pools.clone();
        let mut current = Some(pool_id.to_string());
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id.clone()) {
                return Err(EngineError::Validation(format!(
                    "Token pool parent cycle at '{id}'"
                )));
            }
            let pool = updated
                .get_mut(&id)
                .ok_or_else(|| EngineError::Validation(format!("Unknown token pool '{id}'")))?;
            if add {
                pool.add_tokens(amount)
            } else {
                pool.remove_tokens(amount)
            }
            .map_err(EngineError::Validation)?;
            current = pool.parent_id.clone();
        }
        player.token_pools = updated;
        Ok(())
    }

    pub fn set_zone_cards(&mut self, zone: Zone, card_ids: Vec<String>) -> Result<(), EngineError> {
        self.ensure_cards_exist(&card_ids)?;

        self.zones.insert(zone, card_ids);
        Ok(())
    }

    pub fn zone_cards(&self, zone: Zone) -> &[String] {
        self.zones.get(&zone).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn shuffle_zone(&mut self, zone: Zone) -> Result<(), EngineError> {
        let cards = self
            .zones
            .get_mut(&zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {zone}")))?;
        for index in (1..cards.len()).rev() {
            let swap_with = rand::random_range(0..=index);
            cards.swap(index, swap_with);
        }
        Ok(())
    }

    pub fn move_card_to_bottom(&mut self, zone: Zone, card_id: &str) -> Result<(), EngineError> {
        let cards = self
            .zones
            .get_mut(&zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {zone}")))?;
        let position = cards.iter().position(|id| id == card_id).ok_or_else(|| {
            EngineError::Validation(format!("Card '{card_id}' is not in zone {zone}"))
        })?;
        let card_id = cards.remove(position);
        cards.insert(0, card_id);
        Ok(())
    }

    pub fn search_cards(&self, zone: Zone, query: &str) -> Vec<&Card> {
        let query = query.trim().to_ascii_lowercase();
        self.zone_cards(zone)
            .iter()
            .filter_map(|id| self.cards.get(id))
            .filter(|card| {
                query.is_empty()
                    || card.id.to_ascii_lowercase().contains(&query)
                    || card.name.to_ascii_lowercase().contains(&query)
                    || card.card_type_id.to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn draw_top_from_main_stack(&mut self) -> Option<String> {
        self.zones.get_mut(&Zone::MainStack).and_then(Vec::pop)
    }

    pub fn draw_top_from_deck(&mut self) -> Option<String> {
        self.zones.get_mut(&Zone::Deck).and_then(Vec::pop)
    }

    pub fn peek_main_stack(&self, count: usize) -> Vec<String> {
        let Some(stack) = self.zones.get(&Zone::MainStack) else {
            return Vec::new();
        };

        stack.iter().rev().take(count).cloned().collect()
    }

    pub fn move_card(&mut self, from: Zone, to: Zone, card_id: &str) -> Result<(), EngineError> {
        let source = self
            .zones
            .get(&from)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {from}")))?;
        if !source.iter().any(|id| id == card_id) {
            return Err(EngineError::Validation(format!(
                "Card '{card_id}' is not in source zone {from}"
            )));
        }
        if from == to {
            return Ok(());
        }

        if to == Zone::CommanderPile {
            let mut commander_cards = self.zone_cards(Zone::CommanderPile).to_vec();
            if from == Zone::CommanderPile {
                commander_cards.retain(|id| id != card_id);
            }
            commander_cards.push(card_id.to_string());
            if commander_cards.len() > 2 {
                return Err(EngineError::Validation(
                    "Commander pile cannot contain more than two cards".to_string(),
                ));
            }
        }

        {
            let source = self
                .zones
                .get_mut(&from)
                .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {from}")))?;
            let Some(pos) = source.iter().position(|id| id == card_id) else {
                return Err(EngineError::Validation(format!(
                    "Card '{card_id}' is not in source zone {from}"
                )));
            };
            source.remove(pos);
        }

        self.zones
            .get_mut(&to)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {to}")))?
            .push(card_id.to_string());

        Ok(())
    }

    pub fn set_zone_token_pools(
        &mut self,
        zone: Zone,
        pools: Vec<TokenPool>,
    ) -> Result<(), EngineError> {
        let pools = self.validate_token_pools(pools)?;
        self.zone_token_pools.insert(zone, pools);
        Ok(())
    }

    pub fn add_zone_token_pool(&mut self, zone: Zone, pool: TokenPool) -> Result<(), EngineError> {
        let pool = self.validate_token_pool(pool)?;
        self.zone_token_pools
            .get_mut(&zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {zone}")))?
            .insert(pool.id.clone(), pool);
        Ok(())
    }

    pub fn add_card_token_pool(
        &mut self,
        card_id: &str,
        pool: TokenPool,
    ) -> Result<(), EngineError> {
        self.ensure_card_exists(card_id)?;
        let pool = self.validate_token_pool(pool)?;
        self.card_token_pools
            .entry(card_id.to_string())
            .or_default()
            .insert(pool.id.clone(), pool);
        Ok(())
    }

    pub fn get_zone_token_pools(
        &self,
        zone: Zone,
    ) -> Result<&HashMap<String, TokenPool>, EngineError> {
        self.zone_token_pools
            .get(&zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {zone}")))
    }

    pub fn get_card_token_pools(
        &self,
        card_id: &str,
    ) -> Result<Option<&HashMap<String, TokenPool>>, EngineError> {
        self.ensure_card_exists(card_id)?;
        Ok(self.card_token_pools.get(card_id))
    }

    pub fn activate_zone_token_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        active: bool,
    ) -> Result<(), EngineError> {
        self.zone_token_pool_mut(zone, pool_id)?.set_active(active);
        Ok(())
    }

    pub fn activate_card_token_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        active: bool,
    ) -> Result<(), EngineError> {
        self.card_token_pool_mut(card_id, pool_id)?
            .set_active(active);
        Ok(())
    }

    pub fn add_tokens_to_zone_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.zone_token_pool_mut(zone, pool_id)?
            .add_tokens(amount)
            .map_err(EngineError::Validation)
    }

    pub fn add_tokens_to_card_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.card_token_pool_mut(card_id, pool_id)?
            .add_tokens(amount)
            .map_err(EngineError::Validation)
    }

    pub fn remove_tokens_from_zone_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.zone_token_pool_mut(zone, pool_id)?
            .remove_tokens(amount)
            .map_err(EngineError::Validation)
    }

    pub fn remove_tokens_from_card_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.card_token_pool_mut(card_id, pool_id)?
            .remove_tokens(amount)
            .map_err(EngineError::Validation)
    }

    pub fn zone_token_pool_icon(&self, zone: Zone, pool_id: &str) -> Result<&str, EngineError> {
        Ok(self.zone_token_pool(zone, pool_id)?.token())
    }

    pub fn card_token_pool_icon(&self, card_id: &str, pool_id: &str) -> Result<&str, EngineError> {
        Ok(self.card_token_pool(card_id, pool_id)?.token())
    }

    pub fn zone_token_pool_background(
        &self,
        zone: Zone,
        pool_id: &str,
    ) -> Result<Option<&str>, EngineError> {
        Ok(self.zone_token_pool(zone, pool_id)?.background())
    }

    pub fn card_token_pool_background(
        &self,
        card_id: &str,
        pool_id: &str,
    ) -> Result<Option<&str>, EngineError> {
        Ok(self.card_token_pool(card_id, pool_id)?.background())
    }

    fn ensure_cards_exist(&self, card_ids: &[String]) -> Result<(), EngineError> {
        for card_id in card_ids {
            self.ensure_card_exists(card_id)?;
        }
        Ok(())
    }

    fn ensure_card_exists(&self, card_id: &str) -> Result<(), EngineError> {
        if !self.cards.contains_key(card_id) {
            return Err(EngineError::Validation(format!(
                "Unknown card id '{card_id}'"
            )));
        }
        Ok(())
    }

    pub fn card_by_id(&self, card_id: &str) -> Result<&Card, EngineError> {
        self.cards
            .get(card_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown card id '{card_id}'")))
    }

    fn validate_token_pools(
        &self,
        pools: Vec<TokenPool>,
    ) -> Result<HashMap<String, TokenPool>, EngineError> {
        use std::collections::hash_map::Entry;

        let mut validated = HashMap::new();
        for pool in pools {
            let pool = self.validate_token_pool(pool)?;
            match validated.entry(pool.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(pool);
                }
                Entry::Occupied(entry) => {
                    return Err(EngineError::Validation(format!(
                        "Duplicate token pool id '{}' in the same owner",
                        entry.key()
                    )));
                }
            }
        }
        Ok(validated)
    }

    fn validate_token_pool(&self, pool: TokenPool) -> Result<TokenPool, EngineError> {
        pool.validate().map_err(EngineError::Validation)?;
        Ok(pool)
    }

    fn zone_token_pool(&self, zone: Zone, pool_id: &str) -> Result<&TokenPool, EngineError> {
        self.get_zone_token_pools(zone)?
            .get(pool_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown token pool '{pool_id}'")))
    }

    fn zone_token_pool_mut(
        &mut self,
        zone: Zone,
        pool_id: &str,
    ) -> Result<&mut TokenPool, EngineError> {
        self.zone_token_pools
            .get_mut(&zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone: {zone}")))?
            .get_mut(pool_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown token pool '{pool_id}'")))
    }

    fn card_token_pool(&self, card_id: &str, pool_id: &str) -> Result<&TokenPool, EngineError> {
        self.ensure_card_exists(card_id)?;
        self.card_token_pools
            .get(card_id)
            .and_then(|pools| pools.get(pool_id))
            .ok_or_else(|| EngineError::Validation(format!("Unknown token pool '{pool_id}'")))
    }

    fn card_token_pool_mut(
        &mut self,
        card_id: &str,
        pool_id: &str,
    ) -> Result<&mut TokenPool, EngineError> {
        self.ensure_card_exists(card_id)?;
        self.card_token_pools
            .get_mut(card_id)
            .and_then(|pools| pools.get_mut(pool_id))
            .ok_or_else(|| EngineError::Validation(format!("Unknown token pool '{pool_id}'")))
    }
}
