use std::path::Path;

use crate::card::Card;
use crate::move_logger::{MoveLogEntry, MoveLogger};
use crate::zones::GameState;
use token_pools::TokenPool;

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
    pub move_history: Vec<MoveLogEntry>,
    pub audit_log: Vec<MoveLogEntry>,
}

impl CardEngine {
    pub fn new(
        cards: Vec<Card>,
        zone_ids: impl IntoIterator<Item = String>,
        move_log_path: Option<&Path>,
    ) -> Self {
        Self {
            state: GameState::new(cards, zone_ids),
            logger: MoveLogger::new(move_log_path),
            move_history: Vec::new(),
            audit_log: Vec::new(),
        }
    }

    pub fn move_card(
        &mut self,
        from: &str,
        to: &str,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        if from == to {
            return Err(EngineError::Validation(
                "Source and destination piles must differ".into(),
            ));
        }
        let card = self.state.card_by_id(card_id)?.clone();
        self.state.move_card(from, to, card_id)?;
        match self.logger.append_move("move_card", &card, from, to, None) {
            Ok(entry) => {
                self.move_history.push(entry.clone());
                self.audit_log.push(entry.clone());
                Ok(entry)
            }
            Err(err) => {
                let _ = self.state.move_card(to, from, card_id);
                Err(err)
            }
        }
    }

    pub fn move_card_between_piles(
        &mut self,
        from_pile: &str,
        to_pile: &str,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        self.move_card(from_pile, to_pile, card_id)
    }

    pub fn search_pile(&self, pile_id: &str, query: &str) -> Result<Vec<Card>, EngineError> {
        Ok(self
            .state
            .search_cards(pile_id, query)
            .into_iter()
            .cloned()
            .collect())
    }

    pub fn move_card_to_bottom(&mut self, pile_id: &str, card_id: &str) -> Result<(), EngineError> {
        self.state.move_card_to_bottom(pile_id, card_id)
    }

    pub fn undo_last_move(&mut self) -> Result<MoveLogEntry, EngineError> {
        let previous = self
            .move_history
            .pop()
            .ok_or_else(|| EngineError::Validation("There are no moves to undo".into()))?;
        let from = previous.to_zone.as_str();
        let to = previous.from_zone.as_str();
        let card = self.state.card_by_id(&previous.card_id)?.clone();
        self.state.move_card(from, to, &previous.card_id)?;
        let note = format!("reverts {} at {}", previous.action, previous.timestamp);
        match self
            .logger
            .append_move("undo", &card, from, to, Some(&note))
        {
            Ok(entry) => {
                self.audit_log.push(entry.clone());
                Ok(entry)
            }
            Err(err) => {
                let _ = self.state.move_card(to, from, &previous.card_id);
                self.move_history.push(previous);
                Err(err)
            }
        }
    }

    pub fn move_history_csv(&self) -> Result<String, EngineError> {
        MoveLogger::entries_to_csv(&self.audit_log)
    }

    pub fn draw(&mut self, from_zone: &str, hand_zone: &str) -> Result<MoveLogEntry, EngineError> {
        let card_id = self.state.draw_top_from(from_zone).ok_or_else(|| {
            EngineError::Validation(format!("Cannot draw: pile '{from_zone}' is empty"))
        })?;

        self.state
            .zones
            .get_mut(hand_zone)
            .ok_or_else(|| EngineError::Validation(format!("Unknown zone {hand_zone}")))?
            .push(card_id.clone());

        let card = self.state.card_by_id(&card_id)?;
        let entry = self
            .logger
            .append_move("draw", card, from_zone, hand_zone, None)?;
        self.move_history.push(entry.clone());
        self.audit_log.push(entry.clone());
        Ok(entry)
    }

    pub fn play_card(
        &mut self,
        from: &str,
        to: &str,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        let card = self.state.card_by_id(card_id)?.clone();
        self.state.move_card(from, to, card_id)?;
        let entry = self
            .logger
            .append_move("play_card", &card, from, to, None)?;
        self.move_history.push(entry.clone());
        self.audit_log.push(entry.clone());
        Ok(entry)
    }

    pub fn discard(
        &mut self,
        from: &str,
        discard_zone: &str,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        self.state.move_card(from, discard_zone, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        let entry = self
            .logger
            .append_move("discard", card, from, discard_zone, None)?;
        self.move_history.push(entry.clone());
        self.audit_log.push(entry.clone());
        Ok(entry)
    }

    pub fn exile(
        &mut self,
        from: &str,
        exile_zone: &str,
        card_id: &str,
    ) -> Result<MoveLogEntry, EngineError> {
        self.state.move_card(from, exile_zone, card_id)?;
        let card = self.state.card_by_id(card_id)?;
        let entry = self
            .logger
            .append_move("exile", card, from, exile_zone, None)?;
        self.move_history.push(entry.clone());
        self.audit_log.push(entry.clone());
        Ok(entry)
    }

    pub fn peek_pile(&self, pile_id: &str, count: usize) -> Vec<String> {
        self.state.peek_zone(pile_id, count)
    }

    pub fn set_zone_token_pools(
        &mut self,
        zone: &str,
        pools: Vec<TokenPool>,
    ) -> Result<(), EngineError> {
        self.state.set_zone_token_pools(zone, pools)
    }

    pub fn add_zone_token_pool(&mut self, zone: &str, pool: TokenPool) -> Result<(), EngineError> {
        self.state.add_zone_token_pool(zone, pool)
    }

    pub fn add_card_token_pool(
        &mut self,
        card_id: &str,
        pool: TokenPool,
    ) -> Result<(), EngineError> {
        self.state.add_card_token_pool(card_id, pool)
    }

    pub fn add_tokens_to_player_pool(
        &mut self,
        player_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state
            .add_tokens_to_player_pool(player_id, pool_id, amount)
    }

    pub fn set_player_name(&mut self, player_id: &str, name: &str) -> Result<(), EngineError> {
        self.state.set_player_name(player_id, name)
    }

    pub fn remove_tokens_from_player_pool(
        &mut self,
        player_id: &str,
        pool_id: &str,
        amount: u32,
    ) -> Result<(), EngineError> {
        self.state
            .remove_tokens_from_player_pool(player_id, pool_id, amount)
    }

    pub fn activate_zone_token_pool(
        &mut self,
        zone: &str,
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
        zone: &str,
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
        zone: &str,
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
    use cards::cards::CardVisual;
    use token_pools::TokenPool;

    use super::CardEngine;

    fn test_card(id: &str, name: &str, card_type: CardType) -> Card {
        Card {
            id: id.to_string(),
            game_id: String::new(),
            name: name.to_string(),
            card_type_id: card_type.id,
            description: None,
            cost: None,
            visual: CardVisual::Generated {
                image: None,
                background_image: None,
                background_color: None,
                icon: None,
            },
            back_logo: None,
            back_image: None,
            mana: None,
            colors: None,
            oracle_text: None,
            power: None,
            toughness: None,
            is_commander: false,
            is_partner: false,
            token_pools: Vec::new(),
            starting_pile: None,
        }
    }

    fn card_type(name: &str) -> CardType {
        CardType {
            id: name.to_ascii_lowercase(),
            game_id: String::new(),
            name: name.to_string(),
            description: None,
            background_image: None,
            background_color: None,
            icon: None,
            back_logo: None,
        }
    }

    fn test_log_path(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{unique}.csv"))
    }

    fn test_engine(cards: Vec<Card>, path: Option<&std::path::Path>) -> CardEngine {
        CardEngine::new(
            cards,
            [
                "draw", "deck", "hand", "lands", "command", "discard", "exile",
            ]
            .into_iter()
            .map(str::to_string),
            path,
        )
    }

    #[test]
    fn valid_and_invalid_zone_moves() {
        let log_path = test_log_path("moves");
        let mut engine = test_engine(
            vec![
                test_card("land", "Island", card_type("Land")),
                test_card("creature", "Grizzly Bears", card_type("Creature")),
            ],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards("draw", vec!["land".to_string(), "creature".to_string()])
            .expect("seed main stack");

        engine.draw("draw", "hand").expect("draw 1");
        engine.draw("draw", "hand").expect("draw 2");

        engine
            .play_card("hand", "lands", "land")
            .expect("play land");
        engine
            .play_card("hand", "lands", "creature")
            .expect("play non-land to lands");

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn engine_accepts_csv_defined_zone_ids_without_embedded_rules() {
        let log_path = test_log_path("commander");
        let mut engine = test_engine(
            vec![
                test_card("p1", "Partner One", card_type("Creature")),
                test_card("p2", "Partner Two", card_type("Creature")),
                test_card("solo", "Solo Commander", card_type("Creature")),
            ],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards(
                "deck",
                vec!["p1".to_string(), "p2".to_string(), "solo".to_string()],
            )
            .expect("seed deck");

        engine
            .move_card("deck", "command", "p1")
            .expect("first commander");
        engine
            .move_card("deck", "command", "p2")
            .expect("second partner commander");

        engine
            .move_card("deck", "command", "solo")
            .expect("engine leaves capacity rules to zone configuration");
        assert_eq!(engine.state.zone_cards("command").len(), 3);

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn move_log_appends_rows_with_header_once() {
        let log_path = test_log_path("log");
        let mut engine = test_engine(
            vec![test_card("c1", "Card One", card_type("Creature"))],
            Some(&log_path),
        );

        engine
            .state
            .set_zone_cards("hand", vec!["c1".to_string()])
            .expect("seed hand");

        engine
            .discard("hand", "discard", "c1")
            .expect("discard from hand");
        engine
            .exile("discard", "exile", "c1")
            .expect("exile from discard");

        let logged = fs::read_to_string(&log_path).expect("read log");
        let lines = logged.lines().collect::<Vec<_>>();
        assert_eq!(
            lines[0],
            "timestamp,action,card_id,card_name,from_pile,to_pile,notes"
        );
        assert_eq!(lines.len(), 3);

        fs::remove_file(log_path).ok();
    }

    #[test]
    fn searches_moves_and_undoes_a_card_between_piles() {
        let log_path = test_log_path("pile_move");
        let mut engine = test_engine(
            vec![test_card("c1", "Silvercoat Lion", card_type("Creature"))],
            Some(&log_path),
        );
        engine
            .state
            .set_zone_cards("deck", vec!["c1".into()])
            .expect("seed deck");

        let found = engine.search_pile("deck", "lion").expect("search pile");
        assert_eq!(found[0].id, "c1");
        engine
            .move_card_between_piles("deck", "hand", "c1")
            .expect("move card");
        assert_eq!(engine.state.zone_cards("hand"), &["c1"]);
        engine.undo_last_move().expect("undo move");
        assert_eq!(engine.state.zone_cards("deck"), &["c1"]);

        let csv = fs::read_to_string(&log_path).expect("read log");
        assert!(csv.contains(",move_card,c1,Silvercoat Lion,deck,hand,"));
        assert!(csv.contains(",undo,c1,Silvercoat Lion,hand,deck,"));
        fs::remove_file(log_path).ok();
    }

    #[test]
    fn moves_a_selected_deck_card_to_the_bottom() {
        let mut engine = test_engine(
            vec![
                test_card("bottom", "Bottom", card_type("Creature")),
                test_card("middle", "Middle", card_type("Creature")),
                test_card("top", "Top", card_type("Creature")),
            ],
            None,
        );
        engine
            .state
            .set_zone_cards("deck", vec!["bottom".into(), "middle".into(), "top".into()])
            .expect("seed deck");

        engine
            .move_card_to_bottom("deck", "top")
            .expect("move card to bottom");

        assert_eq!(
            engine.state.zone_cards("deck"),
            &["top", "bottom", "middle"]
        );
        assert_eq!(
            engine.state.draw_top_from("deck").as_deref(),
            Some("middle")
        );
    }

    #[test]
    fn manages_zone_and_card_token_pools() {
        let log_path = test_log_path("tokens");
        let mut engine = test_engine(
            vec![Card {
                id: "ring".to_string(),
                game_id: String::new(),
                name: "Sol Ring".to_string(),
                card_type_id: "artifact".to_string(),
                description: None,
                cost: None,
                visual: CardVisual::Generated {
                    image: None,
                    background_image: None,
                    background_color: None,
                    icon: None,
                },
                back_logo: None,
                back_image: None,
                mana: None,
                colors: None,
                oracle_text: None,
                power: None,
                toughness: None,
                is_commander: false,
                is_partner: false,
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
                "hand",
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
            .activate_zone_token_pool("hand", "energy", true)
            .expect("activate zone pool");
        engine
            .add_tokens_to_zone_pool("hand", "energy", 2)
            .expect("add zone tokens");
        engine
            .add_tokens_to_card_pool("ring", "charge", 1)
            .expect("add card tokens");

        assert_eq!(
            engine
                .state
                .zone_token_pool_icon("hand", "energy")
                .expect("zone token icon"),
            "fa-fire"
        );
        assert_eq!(
            engine
                .state
                .zone_token_pool_background("hand", "energy")
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
                .get_zone_token_pools("hand")
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
                .remove_tokens_from_zone_pool("hand", "energy", 2)
                .is_ok()
        );
        assert!(
            engine
                .remove_tokens_from_card_pool("ring", "charge", 3)
                .is_err()
        );
        assert!(engine.add_tokens_to_zone_pool("hand", "energy", 3).is_err());

        fs::remove_file(log_path).ok();
    }
}
