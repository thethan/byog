use std::path::Path;

use crate::card::Card;
use crate::move_logger::{MoveLogEntry, MoveLogger};
use crate::token_pool::TokenPool;
use crate::zones::{GameState, Zone};

#[derive(Debug)]
pub enum EngineError {
    Io(std::io::Error),
    Csv(csv::Error),
    Validation(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::Io(err) => write!(f, "IO error: {err}"),
            EngineError::Csv(err) => write!(f, "CSV error: {err}"),
            EngineError::Validation(message) => write!(f, "Validation error: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}

pub struct CardEngine {
    pub state: GameState,
    pub logger: MoveLogger,
}

impl CardEngine {
    pub fn new(cards: Vec<Card>, move_log_path: Option<&Path>) -> Self {
        Self {
            state: GameState::new(cards),
            logger: MoveLogger::new(move_log_path),
        }
    }

    pub fn move_card(
        &mut self,
        from: Zone,
        to: Zone,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        self.state.move_card(from, to, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        self.logger.append_move("move_card", card, from, to, None)
    }

    pub fn draw(&mut self) -> Result<MoveLogEntry, EngineError> {
        let (from_zone, card_id, note) =
            if let Some(card_id) = self.state.draw_top_from_main_stack() {
                (Zone::MainStack, card_id, None)
            } else if let Some(card_id) = self.state.draw_top_from_deck() {
                (Zone::Deck, card_id, Some("MainStack empty, drew from Deck"))
            } else {
                return Err(EngineError::Validation(
                    "Cannot draw: both MainStack and Deck are empty".to_string(),
                ));
            };

        self.state
            .zones
            .get_mut(&Zone::Hand)
            .ok_or_else(|| EngineError::Validation("Unknown zone Hand".to_string()))?
            .push(card_id.clone());

        let card = self.state.card_by_id(&card_id)?;
        self.logger
            .append_move("draw", card, from_zone, Zone::Hand, note)
    }

    pub fn play_land(&mut self, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        let card = self.state.card_by_id(card_id)?.clone();
        self.state.move_card(Zone::Hand, Zone::Lands, card_id)?;
        self.logger
            .append_move("play_land", &card, Zone::Hand, Zone::Lands, None)
    }

    pub fn discard(&mut self, from: Zone, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        self.state.move_card(from, Zone::Discard, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        self.logger
            .append_move("discard", card, from, Zone::Discard, None)
    }

    pub fn exile(&mut self, from: Zone, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        self.state.move_card(from, Zone::Exile, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        self.logger
            .append_move("exile", card, from, Zone::Exile, None)
    }

    pub fn cast_to_battlefield(&mut self, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        let card = self.state.card_by_id(card_id)?.clone();
        self.state.move_card(Zone::Hand, Zone::Battlefield, card_id)?;
        self.logger
            .append_move("cast_to_battlefield", &card, Zone::Hand, Zone::Battlefield, None)
    }

    pub fn peek_main_stack(&self, count: usize) -> Vec<String> {
        self.state.peek_main_stack(count)
    }

    pub fn set_zone_token_pools(
        &mut self,
        zone: Zone,
        pools: Vec<TokenPool>,
    ) -> Result<(), EngineError> {
        self.state.set_zone_token_pools(zone, pools)
    }

    pub fn add_zone_token_pool(&mut self, zone: Zone, pool: TokenPool) -> Result<(), EngineError> {
        self.state.add_zone_token_pool(zone, pool)
    }

    pub fn add_card_token_pool(
        &mut self,
        card_id: &str,
        pool: TokenPool,
    ) -> Result<(), EngineError> {
        self.state.add_card_token_pool(card_id, pool)
    }

    pub fn activate_zone_token_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        active: bool,
    ) -> Result<(), EngineError> {
        self.state.activate_zone_token_pool(zone, pool_id, active)
    }

    pub fn activate_card_token_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        active: bool,
    ) -> Result<(), EngineError> {
        self.state
            .activate_card_token_pool(card_id, pool_id, active)
    }

    pub fn add_tokens_to_zone_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state.add_tokens_to_zone_pool(zone, pool_id, amount)
    }

    pub fn add_tokens_to_card_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state.add_tokens_to_card_pool(card_id, pool_id, amount)
    }

    pub fn remove_tokens_from_zone_pool(
        &mut self,
        zone: Zone,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state
            .remove_tokens_from_zone_pool(zone, pool_id, amount)
    }

    pub fn remove_tokens_from_card_pool(
        &mut self,
        card_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state
            .remove_tokens_from_card_pool(card_id, pool_id, amount)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::card::{Card, CardType};
    use crate::token_pool::TokenPool;
    use crate::zones::Zone;

    use super::CardEngine;

    fn test_card(
        id: &str,
        name: &str,
        card_type: CardType,
    ) -> Card {
        Card {
            id: id.to_string(),
            name: name.to_string(),
            card_type,
            mana_cost: None,
            colors: None,
            oracle_text: None,
            power: None,
            toughness: None,
            token_pools: Vec::new(),
            starting_pile: None,
        }
    }

    fn test_log_path(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{unique}.csv"))
    }

    #[test]
    fn valid_and_invalid_zone_moves() {
        let log_path = test_log_path("moves");
        let mut engine = CardEngine::new(
            vec![
                test_card("land", "Island", CardType::Land),
                test_card(
                    "creature",
                    "Grizzly Bears",
                    CardType::Creature,
                ),
            ],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards(
                Zone::MainStack,
                vec!["land".to_string(), "creature".to_string()],
            )
            .expect("seed main stack");

        engine.draw().expect("draw 1");
        engine.draw().expect("draw 2");

        engine.play_land("land").expect("play land");
        engine.play_land("creature").expect("play non-land to lands");

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn commander_partner_rule_is_enforced() {
        let log_path = test_log_path("commander");
        let mut engine = CardEngine::new(
            vec![
                test_card("p1", "Partner One", CardType::Creature),
                test_card("p2", "Partner Two", CardType::Creature),
                test_card("solo", "Solo Commander", CardType::Creature),
            ],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards(
                Zone::Deck,
                vec!["p1".to_string(), "p2".to_string(), "solo".to_string()],
            )
            .expect("seed deck");

        engine
            .move_card(Zone::Deck, Zone::CommanderPile, "p1")
            .expect("first commander");
        engine
            .move_card(Zone::Deck, Zone::CommanderPile, "p2")
            .expect("second partner commander");

        let result = engine.move_card(Zone::Deck, Zone::CommanderPile, "solo");
        assert!(result.is_err());

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn move_log_appends_rows_with_header_once() {
        let log_path = test_log_path("log");
        let mut engine = CardEngine::new(
            vec![test_card(
                "c1",
                "Card One",
                CardType::Creature,
            )],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards(Zone::Hand, vec!["c1".to_string()])
            .expect("seed hand");

        engine.discard(Zone::Hand, "c1").expect("discard from hand");
        engine
            .exile(Zone::Discard, "c1")
            .expect("exile from discard");

        let logged = fs::read_to_string(&log_path).expect("read log");
        let lines = logged.lines().collect::<Vec<_>>();
        assert_eq!(
            lines[0],
            "timestamp,action,card_id,card_name,from_zone,to_zone,notes"
        );
        assert_eq!(lines.len(), 3);

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn manages_zone_and_card_token_pools() {
        let log_path = test_log_path("tokens");
        let mut engine = CardEngine::new(
            vec![Card {
                id: "ring".to_string(),
                name: "Sol Ring".to_string(),
                card_type: CardType::Artifact,
                mana_cost: None,
                colors: None,
                oracle_text: None,
                power: None,
                toughness: None,
                token_pools: vec![
                    TokenPool::configured(
                        "charge",
                        "Charge",
                        "fa-bolt",
                        Some("amber".to_string()),
                        1,
                        None,
                        Some(3),
                        true,
                    )
                    .expect("card token pool"),
                ],
                starting_pile: None,
            }],
            Some(&log_path),
        );

        engine
            .set_zone_token_pools(
                Zone::Hand,
                vec![
                    TokenPool::configured(
                        "energy",
                        "Energy",
                        "fa-fire",
                        Some("slate".to_string()),
                        0,
                        Some(0),
                        Some(2),
                        false,
                    )
                    .expect("zone token pool"),
                ],
            )
            .expect("seed zone token pools");

        engine
            .activate_zone_token_pool(Zone::Hand, "energy", true)
            .expect("activate zone pool");
        engine
            .add_tokens_to_zone_pool(Zone::Hand, "energy", 2)
            .expect("add zone tokens");
        engine
            .add_tokens_to_card_pool("ring", "charge", 1)
            .expect("add card tokens");

        assert_eq!(
            engine
                .state
                .zone_token_pool_icon(Zone::Hand, "energy")
                .expect("zone token icon"),
            "fa-fire"
        );
        assert_eq!(
            engine
                .state
                .zone_token_pool_background(Zone::Hand, "energy")
                .expect("zone background"),
            Some("slate")
        );
        assert_eq!(
            engine
                .state
                .card_token_pool_icon("ring", "charge")
                .expect("card token icon"),
            "fa-bolt"
        );
        assert_eq!(
            engine
                .state
                .card_token_pool_background("ring", "charge")
                .expect("card background"),
            Some("amber")
        );
        assert_eq!(
            engine
                .state
                .get_zone_token_pools(Zone::Hand)
                .expect("zone token pools")
                .get("energy")
                .expect("energy pool")
                .count,
            2
        );
        assert_eq!(
            engine
                .state
                .get_card_token_pools("ring")
                .expect("card pools")
                .and_then(|pools| pools.get("charge"))
                .expect("charge pool")
                .count,
            2
        );
        assert!(
            engine
                .remove_tokens_from_zone_pool(Zone::Hand, "energy", 2)
                .is_ok()
        );
        assert!(
            engine
                .remove_tokens_from_card_pool("ring", "charge", 3)
                .is_err()
        );
        assert!(
            engine
                .add_tokens_to_zone_pool(Zone::Hand, "energy", 3)
                .is_err()
        );

        fs::remove_file(log_path).ok();
    }
}
