use std::path::Path;

use crate::card::{Card, CardType};
use crate::move_logger::{MoveLogEntry, MoveLogger};
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
        if !matches!(card.card_type, CardType::Land) {
            return Err(EngineError::Validation(format!(
                "Card '{card_id}' is not a land and cannot be played to LandPile"
            )));
        }

        self.state.move_card(Zone::Hand, Zone::LandPile, card_id)?;
        self.logger
            .append_move("play_land", &card, Zone::Hand, Zone::LandPile, None)
    }

    pub fn discard(&mut self, from: Zone, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        if !matches!(
            from,
            Zone::Hand | Zone::ArtifactList | Zone::EnchantmentList | Zone::CreatureList
        ) {
            return Err(EngineError::Validation(format!(
                "Discard is only allowed from Hand or in-play piles, got {from}"
            )));
        }

        self.state.move_card(from, Zone::Discard, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        self.logger
            .append_move("discard", card, from, Zone::Discard, None)
    }

    pub fn exile(&mut self, from: Zone, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        if matches!(from, Zone::MainStack) {
            return Err(EngineError::Validation(
                "Exile from hidden MainStack is not allowed directly".to_string(),
            ));
        }

        self.state.move_card(from, Zone::Exile, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        self.logger
            .append_move("exile", card, from, Zone::Exile, None)
    }

    pub fn cast_to_battlefield(&mut self, card_id: &str) -> Result<MoveLogEntry, EngineError> {
        let card = self.state.card_by_id(card_id)?.clone();
        let to_zone = match card.card_type {
            CardType::Artifact => Zone::ArtifactList,
            CardType::Enchantment => Zone::EnchantmentList,
            CardType::Creature => Zone::CreatureList,
            _ => {
                return Err(EngineError::Validation(format!(
                    "Card '{card_id}' cannot be cast to battlefield typed piles"
                )));
            }
        };

        self.state.move_card(Zone::Hand, to_zone, card_id)?;
        self.logger
            .append_move("cast_to_battlefield", &card, Zone::Hand, to_zone, None)
    }

    pub fn peek_main_stack(&self, count: usize) -> Vec<String> {
        self.state.peek_main_stack(count)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::card::{Card, CardType};
    use crate::zones::Zone;

    use super::CardEngine;

    fn test_card(
        id: &str,
        name: &str,
        card_type: CardType,
        is_commander: bool,
        is_partner: bool,
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
            is_commander,
            is_partner,
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
                test_card("land", "Island", CardType::Land, false, false),
                test_card(
                    "creature",
                    "Grizzly Bears",
                    CardType::Creature,
                    false,
                    false,
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
        let invalid = engine.play_land("creature");
        assert!(invalid.is_err());

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn commander_partner_rule_is_enforced() {
        let log_path = test_log_path("commander");
        let mut engine = CardEngine::new(
            vec![
                test_card("p1", "Partner One", CardType::Creature, true, true),
                test_card("p2", "Partner Two", CardType::Creature, true, true),
                test_card("solo", "Solo Commander", CardType::Creature, true, false),
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
                false,
                false,
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
}
