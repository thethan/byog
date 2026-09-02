use std::io::Cursor;

use prost::Message;
use wasm_bindgen::prelude::*;

use cards::cards::CardVisual;
use decks::{Pile, ZoneLayout, parse_piles_csv, parse_zones_csv};
use engine::{Card, CardEngine, DEFAULT_PLAYER_ID, EngineError};
use token_pools::{TokenPool, TokenPoolOwner, ingest_token_pools_csv, ingest_token_types_csv};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = error)]
    fn console_error(message: &str);
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    let message = error.to_string();
    console_error(&format!("[WASM ERROR] {message}"));
    JsValue::from_str(&message)
}

fn install_panic_logger() {
    static INSTALLED: std::sync::Once = std::sync::Once::new();
    INSTALLED.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            console_error(&format!("[WASM PANIC] {info}"));
        }));
    });
}

const DEFAULT_CARDS_CSV: &str = include_str!("../data/players/player-1/cards.csv");
const DEFAULT_PILES_CSV: &str = include_str!("../data/piles.csv");
const DEFAULT_ZONES_CSV: &str = include_str!("../data/zones.csv");
const DEFAULT_TOKEN_TYPES_CSV: &str = include_str!("../data/token_types.csv");
const DEFAULT_TOKEN_POOLS_CSV: &str = include_str!("../data/token_pools.csv");

#[wasm_bindgen]
pub struct WasmGame {
    engine: CardEngine,
    piles: Vec<Pile>,
    zone_layouts: Vec<ZoneLayout>,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmGame, JsValue> {
        Self::from_cards_csv(DEFAULT_CARDS_CSV)
    }

    /// Creates one player's isolated game state from that player's cards.csv.
    #[wasm_bindgen(js_name = fromCardsCsv)]
    pub fn from_cards_csv(raw_csv: &str) -> Result<WasmGame, JsValue> {
        install_panic_logger();
        let mut cards = load_cards_from_embedded_csv(raw_csv).map_err(js_error)?;
        if cards.is_empty() {
            return Err(js_error("No cards found in embedded CSV"));
        }

        for card in &mut cards {
            if !card.token_pools.iter().any(|pool| pool.id == "counters") {
                card.token_pools.push(
                    TokenPool::configured(
                        "counters",
                        "Counters",
                        "fa-solid fa-plus",
                        None,
                        0,
                        Some(0),
                        None,
                        true,
                    )
                    .map_err(js_error)?,
                );
            }
            if !card.token_pools.iter().any(|pool| pool.id == "tapped") {
                card.token_pools.push(
                    TokenPool::configured(
                        "tapped",
                        "Tapped",
                        "fa-solid fa-rotate",
                        None,
                        0,
                        Some(0),
                        Some(0),
                        false,
                    )
                    .map_err(js_error)?,
                );
            }
            if card.back_image.is_some()
                && !card.token_pools.iter().any(|pool| pool.id == "flipped")
            {
                card.token_pools.push(
                    TokenPool::configured(
                        "flipped",
                        "Back face",
                        "fa-solid fa-repeat",
                        None,
                        0,
                        Some(0),
                        Some(0),
                        false,
                    )
                    .map_err(js_error)?,
                );
            }
        }

        let piles = parse_piles_csv(DEFAULT_PILES_CSV).map_err(js_error)?;

        let zone_layouts = parse_zones_csv(DEFAULT_ZONES_CSV).map_err(js_error)?;

        let pile_ids = piles.iter().map(|pile| pile.id.clone()).collect::<Vec<_>>();
        let commander_pile = piles
            .iter()
            .find(|pile| pile.role.as_deref() == Some("commander"))
            .map(|pile| pile.id.as_str());
        let draw_pile = piles
            .iter()
            .find(|pile| pile.role.as_deref() == Some("draw"))
            .map(|pile| pile.id.as_str())
            .ok_or_else(|| js_error("piles CSV must define a pile with role 'draw'"))?;
        let mut zone_cards: std::collections::HashMap<String, Vec<String>> = pile_ids
            .iter()
            .cloned()
            .map(|id| (id, Vec::new()))
            .collect();

        for card in &cards {
            let target_zone = card
                .starting_pile
                .as_deref()
                .filter(|id| zone_cards.contains_key(*id))
                .or(if card.is_commander {
                    commander_pile
                } else {
                    None
                })
                .unwrap_or(draw_pile)
                .to_string();
            zone_cards
                .entry(target_zone)
                .or_default()
                .push(card.id.clone());
        }

        let mut engine = CardEngine::new(cards, pile_ids, None);
        let token_types = ingest_token_types_csv(DEFAULT_TOKEN_TYPES_CSV).map_err(js_error)?;
        let definitions =
            ingest_token_pools_csv(DEFAULT_TOKEN_POOLS_CSV, &token_types).map_err(js_error)?;
        let player_pools = definitions
            .iter()
            .filter(|definition| definition.owner == TokenPoolOwner::Player)
            .map(|definition| definition.pool.clone())
            .collect::<Vec<_>>();
        engine.state.players.insert(
            DEFAULT_PLAYER_ID.to_string(),
            engine::Player::with_token_pools(DEFAULT_PLAYER_ID, "Player 1", player_pools)
                .map_err(js_error)?,
        );
        for definition in definitions {
            match definition.owner {
                TokenPoolOwner::Card | TokenPoolOwner::Creature => {
                    let card_id = definition.owner_id.as_deref().ok_or_else(|| {
                        js_error(format!("Pool '{}' requires owner_id", definition.pool.id))
                    })?;
                    engine
                        .add_card_token_pool(card_id, definition.pool)
                        .map_err(js_error)?;
                }
                TokenPoolOwner::Zone | TokenPoolOwner::Battlefield => {
                    let zone = definition
                        .owner_id
                        .as_deref()
                        .or_else(|| {
                            piles
                                .iter()
                                .find(|pile| pile.role.as_deref() == Some("play_default"))
                                .map(|pile| pile.id.as_str())
                        })
                        .ok_or_else(|| {
                            js_error(format!(
                                "Pool '{}' needs owner_id or a play_default pile",
                                definition.pool.id
                            ))
                        })?;
                    engine
                        .add_zone_token_pool(zone, definition.pool)
                        .map_err(js_error)?;
                }
                TokenPoolOwner::Player => {}
            }
        }
        for layout in &zone_layouts {
            if !layout.token_pools.is_empty() {
                for pile in piles.iter().filter(|pile| pile.zone_id == layout.id) {
                    engine
                        .set_zone_token_pools(&pile.id, layout.token_pools.clone())
                        .map_err(js_error)?;
                }
            }
        }

        for (zone, ids) in zone_cards {
            if !ids.is_empty() {
                engine.state.set_zone_cards(&zone, ids).map_err(js_error)?;
            }
        }

        Ok(Self {
            engine,
            piles,
            zone_layouts,
        })
    }

    pub fn state_proto(&self) -> Result<Vec<u8>, JsValue> {
        self.snapshot_proto().map_err(js_error)
    }

    /// Restores mutable game state from a previously saved snapshot.
    /// Card definitions still come from the current deck CSV, so stale or corrupt
    /// snapshots cannot silently add, remove, or duplicate cards.
    pub fn restore_state_proto(&mut self, bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
        let snapshot = GameStateSnapshotProto::decode(bytes)
            .map_err(|err| js_error(format!("Failed to decode saved game state: {err}")))?;
        let known_piles = self
            .piles
            .iter()
            .map(|pile| pile.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let known_cards = self
            .engine
            .state
            .cards
            .keys()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>();
        let mut restored_state = self.engine.state.clone();
        let mut restored_zones = std::collections::HashMap::new();
        let mut restored_cards = std::collections::HashSet::new();

        for zone in snapshot.zones {
            if !known_piles.contains(zone.id.as_str())
                || restored_zones.contains_key(zone.id.as_str())
            {
                return Err(js_error(format!(
                    "Saved game state contains an unknown or duplicate pile '{}'",
                    zone.id
                )));
            }
            restore_token_pool_views(
                restored_state
                    .zone_token_pools
                    .get_mut(&zone.id)
                    .ok_or_else(|| js_error(format!("Unknown saved pile '{}'", zone.id)))?,
                zone.token_pools,
                &format!("pile '{}'", zone.id),
            )
            .map_err(js_error)?;
            let mut card_ids = Vec::with_capacity(zone.cards.len());
            for card in zone.cards {
                if !known_cards.contains(card.id.as_str())
                    || !restored_cards.insert(card.id.clone())
                {
                    return Err(js_error(format!(
                        "Saved game state contains an unknown or duplicate card '{}'",
                        card.id
                    )));
                }
                restore_token_pool_views(
                    restored_state
                        .card_token_pools
                        .entry(card.id.clone())
                        .or_default(),
                    card.token_pools,
                    &format!("card '{}'", card.id),
                )
                .map_err(js_error)?;
                card_ids.push(card.id);
            }
            restored_zones.insert(zone.id, card_ids);
        }

        if restored_zones.len() != known_piles.len() || restored_cards.len() != known_cards.len() {
            return Err(js_error(
                "Saved game state does not match the current deck and pile configuration",
            ));
        }

        let mut restored_players = std::collections::HashSet::new();
        for player in snapshot.players {
            if !restored_players.insert(player.id.clone()) {
                return Err(js_error(format!(
                    "Saved game state contains duplicate player '{}'",
                    player.id
                )));
            }
            let restored_player = restored_state.players.get_mut(&player.id).ok_or_else(|| {
                js_error(format!(
                    "Saved game state contains unknown player '{}'",
                    player.id
                ))
            })?;
            restore_token_pool_views(
                &mut restored_player.token_pools,
                player.token_pools,
                &format!("player '{}'", player.id),
            )
            .map_err(js_error)?;
            restored_player.name = player.name;
        }
        if restored_players.len() != restored_state.players.len() {
            return Err(js_error("Saved game state does not contain every player"));
        }

        // Validation above is complete, so applying the replacement is atomic.
        restored_state.zones = restored_zones;
        self.engine.state = restored_state;
        self.snapshot_proto().map_err(js_error)
    }

    pub fn board_layout_proto(&self) -> Result<Vec<u8>, JsValue> {
        let zones = self
            .zone_layouts
            .iter()
            .map(|z| ZoneLayoutProto {
                id: z.id.clone(),
                name: z.name.clone(),
                color: z.color.clone(),
                x: z.x as u32,
                y: z.y as u32,
                width: z.width as u32,
                height: z.height as u32,
                scope: z.scope.as_str().to_string(),
                parent_zone: z.parent_zone.clone(),
                allowed_card_types: z.allowed_card_types.clone(),
                max_cards: z.max_cards.map(|value| value as u32),
            })
            .collect();

        let piles = self
            .piles
            .iter()
            .map(|p| PileViewProto {
                id: p.id.clone(),
                name: p.name.clone(),
                zone_id: p.zone_id.clone(),
                x: p.x as u32,
                y: p.y as u32,
                associated_piles: p.associated_piles.clone(),
                visible: p.visible,
                role: p.role.clone(),
            })
            .collect();

        let layout = BoardLayoutProto { zones, piles };
        let mut bytes = Vec::new();
        layout
            .encode(&mut bytes)
            .map_err(|err| js_error(format!("Failed to encode board layout protobuf: {err}")))?;
        Ok(bytes)
    }

    pub fn draw(&mut self) -> Result<Vec<u8>, JsValue> {
        let draw = self.pile_for_role("draw").map_err(js_error)?.to_string();
        let hand = self.pile_for_role("hand").map_err(js_error)?.to_string();
        self.engine.draw(&draw, &hand).map_err(js_error)?;
        self.state_proto()
    }

    pub fn auto_play_first_hand_card(&mut self) -> Result<Vec<u8>, JsValue> {
        let hand = self.pile_for_role("hand").map_err(js_error)?.to_string();
        let Some(card_id) = self.engine.state.zone_cards(&hand).first().cloned() else {
            return self.state_proto();
        };
        let destination = self
            .destination_for_card(&card_id)
            .map_err(js_error)?
            .to_string();
        self.engine
            .play_card(&hand, &destination, &card_id)
            .map_err(js_error)?;

        self.state_proto()
    }

    pub fn discard_first_hand_card(&mut self) -> Result<Vec<u8>, JsValue> {
        let hand = self.pile_for_role("hand").map_err(js_error)?.to_string();
        let discard = self.pile_for_role("discard").map_err(js_error)?.to_string();
        let Some(card_id) = self.engine.state.zone_cards(&hand).first().cloned() else {
            return self.state_proto();
        };
        self.engine
            .discard(&hand, &discard, &card_id)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn add_hand_energy(&mut self, amount: u32) -> Result<Vec<u8>, JsValue> {
        let hand = self.pile_for_role("hand").map_err(js_error)?.to_string();
        self.engine
            .add_tokens_to_zone_pool(&hand, "energy", amount)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn move_card(
        &mut self,
        card_id: &str,
        from_pile: &str,
        to_pile: &str,
    ) -> Result<Vec<u8>, JsValue> {
        self.validate_card_destination(card_id, to_pile)
            .map_err(js_error)?;
        self.engine
            .move_card_between_piles(from_pile, to_pile, card_id)
            .map_err(js_error)?;
        self.state_proto()
    }

    /// Creates a card during play and places it directly into a configured pile.
    #[wasm_bindgen(js_name = createCard)]
    pub fn create_card(
        &mut self,
        id: &str,
        name: &str,
        card_type: &str,
        text: &str,
        offense: &str,
        defense: &str,
        to_pile: &str,
    ) -> Result<Vec<u8>, JsValue> {
        let id = id.trim();
        let name = name.trim();
        let card_type = card_type.trim().to_ascii_lowercase().replace(' ', "-");
        if id.is_empty() || name.is_empty() || card_type.is_empty() {
            return Err(js_error("Card ID, name, and type are required"));
        }
        if self.engine.state.cards.contains_key(id) {
            return Err(js_error(format!("Card '{id}' already exists")));
        }
        let pile = self
            .piles
            .iter()
            .find(|pile| pile.id == to_pile)
            .ok_or_else(|| js_error(format!("Unknown pile '{to_pile}'")))?;
        let zone = self
            .zone_layouts
            .iter()
            .find(|zone| zone.id == pile.zone_id)
            .ok_or_else(|| js_error(format!("Pile '{to_pile}' has no zone layout")))?;
        if zone
            .max_cards
            .is_some_and(|max| self.engine.state.zone_cards(to_pile).len() >= max)
        {
            return Err(js_error(format!("{} is full", zone.name)));
        }
        if !zone.allowed_card_types.is_empty()
            && !zone
                .allowed_card_types
                .iter()
                .any(|allowed| allowed == &card_type)
        {
            return Err(js_error(format!(
                "{} does not accept {} cards",
                zone.name, card_type
            )));
        }

        let counters = TokenPool::configured(
            "counters",
            "Counters",
            "fa-solid fa-plus",
            None,
            0,
            Some(0),
            None,
            true,
        )
        .map_err(js_error)?;
        let tapped = TokenPool::configured(
            "tapped",
            "Tapped",
            "fa-solid fa-rotate",
            None,
            0,
            Some(0),
            Some(0),
            false,
        )
        .map_err(js_error)?;
        let card = Card {
            id: id.to_string(),
            game_id: String::new(),
            name: name.to_string(),
            card_type_id: card_type,
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
            oracle_text: (!text.trim().is_empty()).then(|| text.trim().to_string()),
            power: (!offense.trim().is_empty()).then(|| offense.trim().to_string()),
            toughness: (!defense.trim().is_empty()).then(|| defense.trim().to_string()),
            is_commander: false,
            is_partner: false,
            token_pools: vec![counters.clone(), tapped.clone()],
            starting_pile: Some(to_pile.to_string()),
        };
        self.engine.state.cards.insert(id.to_string(), card);
        self.engine.state.card_token_pools.insert(
            id.to_string(),
            [(counters.id.clone(), counters), (tapped.id.clone(), tapped)]
                .into_iter()
                .collect(),
        );
        self.engine
            .state
            .zones
            .get_mut(to_pile)
            .expect("validated pile must exist")
            .push(id.to_string());
        self.state_proto()
    }

    fn validate_card_destination(&self, card_id: &str, to_pile: &str) -> Result<(), EngineError> {
        let pile = self
            .piles
            .iter()
            .find(|pile| pile.id == to_pile)
            .ok_or_else(|| EngineError::Validation(format!("Unknown pile '{to_pile}'")))?;
        let zone = self
            .zone_layouts
            .iter()
            .find(|zone| zone.id == pile.zone_id)
            .ok_or_else(|| {
                EngineError::Validation(format!(
                    "Pile '{to_pile}' references unknown zone '{}'",
                    pile.zone_id
                ))
            })?;
        if let Some(max) = zone.max_cards {
            if self.engine.state.zone_cards(to_pile).len() >= max {
                return Err(EngineError::Validation(format!(
                    "{} cannot contain more than {max} cards",
                    zone.name
                )));
            }
        }
        if zone.allowed_card_types.is_empty() {
            return Ok(());
        }
        let card = self.engine.state.card_by_id(card_id)?;
        let card_type = card
            .card_type_id
            .trim()
            .to_ascii_lowercase()
            .replace(' ', "-");
        if zone
            .allowed_card_types
            .iter()
            .any(|allowed| allowed == &card_type)
        {
            Ok(())
        } else {
            Err(EngineError::Validation(format!(
                "{} cards cannot be moved to {} (accepts: {})",
                card.card_type_id,
                zone.name,
                zone.allowed_card_types.join(", ")
            )))
        }
    }

    fn pile_for_role(&self, role: &str) -> Result<&str, EngineError> {
        self.piles
            .iter()
            .find(|pile| pile.role.as_deref() == Some(role))
            .map(|pile| pile.id.as_str())
            .ok_or_else(|| {
                EngineError::Validation(format!("piles CSV must define a pile with role '{role}'"))
            })
    }

    fn destination_for_card(&self, card_id: &str) -> Result<&str, EngineError> {
        let card = self.engine.state.card_by_id(card_id)?;
        let card_type = card
            .card_type_id
            .trim()
            .to_ascii_lowercase()
            .replace(' ', "-");
        self.piles
            .iter()
            .find(|pile| {
                self.zone_layouts
                    .iter()
                    .find(|zone| zone.id == pile.zone_id)
                    .is_some_and(|zone| {
                        zone.allowed_card_types
                            .iter()
                            .any(|kind| kind == &card_type)
                    })
            })
            .map(|pile| pile.id.as_str())
            .or_else(|| self.pile_for_role("play_default").ok())
            .ok_or_else(|| {
                EngineError::Validation(format!(
                    "No zone accepts card type '{}' and no play_default pile is configured",
                    card.card_type_id
                ))
            })
    }

    pub fn take_top(&mut self, from_pile: &str, to_pile: &str) -> Result<Vec<u8>, JsValue> {
        self.move_cards(from_pile, to_pile, 1, false)
    }

    pub fn move_cards(
        &mut self,
        from_pile: &str,
        to_pile: &str,
        count: usize,
        random: bool,
    ) -> Result<Vec<u8>, JsValue> {
        if from_pile == to_pile {
            return Err(js_error("Source and destination piles must differ"));
        }
        if count == 0 {
            return Err(js_error("Card count must be at least one"));
        }
        let cards = self.engine.state.zone_cards(from_pile);
        if cards.is_empty() {
            return Err(js_error(format!("Pile '{from_pile}' is empty")));
        }

        let move_count = count.min(cards.len());
        let card_ids = if random {
            let mut available = cards.to_vec();
            (0..move_count)
                .map(|_| {
                    let index = rand::random_range(0..available.len());
                    available.remove(index)
                })
                .collect::<Vec<_>>()
        } else {
            cards.iter().rev().take(move_count).cloned().collect()
        };

        let target_pile = self
            .piles
            .iter()
            .find(|pile| pile.id == to_pile)
            .ok_or_else(|| js_error(format!("Unknown pile '{to_pile}'")))?;
        let target_zone = self
            .zone_layouts
            .iter()
            .find(|zone| zone.id == target_pile.zone_id)
            .ok_or_else(|| js_error(format!("Unknown zone '{}'", target_pile.zone_id)))?;
        if target_zone
            .max_cards
            .is_some_and(|max| self.engine.state.zone_cards(to_pile).len() + card_ids.len() > max)
        {
            return Err(js_error(format!(
                "{} does not have room for {} more cards",
                target_zone.name,
                card_ids.len()
            )));
        }

        // Validate the entire batch before moving anything so a mixed group cannot
        // leave the source pile partially moved when a restricted zone rejects it.
        for card_id in &card_ids {
            self.validate_card_destination(card_id, to_pile)
                .map_err(js_error)?;
        }
        for card_id in card_ids {
            self.engine
                .move_card_between_piles(from_pile, to_pile, &card_id)
                .map_err(js_error)?;
        }
        self.state_proto()
    }

    pub fn shuffle_pile(&mut self, pile_id: &str) -> Result<Vec<u8>, JsValue> {
        self.engine.state.shuffle_zone(pile_id).map_err(js_error)?;
        self.state_proto()
    }

    pub fn move_card_to_bottom(
        &mut self,
        pile_id: &str,
        card_id: &str,
    ) -> Result<Vec<u8>, JsValue> {
        self.engine
            .move_card_to_bottom(pile_id, card_id)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn set_card_tapped(&mut self, card_id: &str, tapped: bool) -> Result<Vec<u8>, JsValue> {
        self.engine
            .activate_card_token_pool(card_id, "tapped", tapped)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn set_card_flipped(&mut self, card_id: &str, flipped: bool) -> Result<Vec<u8>, JsValue> {
        let card = self.engine.state.card_by_id(card_id).map_err(js_error)?;
        if card.back_image.is_none() {
            return Err(js_error("This card does not define a back_image"));
        }
        self.engine
            .activate_card_token_pool(card_id, "flipped", flipped)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn add_card_counter(&mut self, card_id: &str) -> Result<Vec<u8>, JsValue> {
        self.engine
            .add_tokens_to_card_pool(card_id, "counters", 1)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn remove_card_counter(&mut self, card_id: &str) -> Result<Vec<u8>, JsValue> {
        self.engine
            .remove_tokens_from_card_pool(card_id, "counters", 1)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn add_life(&mut self, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.add_player_tokens("life", amount)
    }

    pub fn add_player_tokens(&mut self, pool_id: &str, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.engine
            .add_tokens_to_player_pool(DEFAULT_PLAYER_ID, pool_id, amount)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn remove_life(&mut self, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.remove_player_tokens("life", amount)
    }

    pub fn remove_player_tokens(&mut self, pool_id: &str, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.engine
            .remove_tokens_from_player_pool(DEFAULT_PLAYER_ID, pool_id, amount)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn set_player_name(&mut self, name: &str) -> Result<Vec<u8>, JsValue> {
        self.engine
            .set_player_name(DEFAULT_PLAYER_ID, name)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn undo_last_move(&mut self) -> Result<Vec<u8>, JsValue> {
        self.engine.undo_last_move().map_err(js_error)?;
        self.state_proto()
    }

    pub fn move_history_csv(&self) -> Result<String, JsValue> {
        self.engine.move_history_csv().map_err(js_error)
    }
}

impl WasmGame {
    fn snapshot_proto(&self) -> Result<Vec<u8>, EngineError> {
        let mut zones = Vec::new();
        for pile in &self.piles {
            let mut cards = Vec::new();
            for card_id in self.engine.state.zone_cards(&pile.id) {
                let card = self.engine.state.card_by_id(card_id)?;
                cards.push(CardViewProto {
                    id: card.id.clone(),
                    name: card.name.clone(),
                    card_type: card.card_type_id.clone(),
                    mana: card.mana.clone(),
                    oracle_text: card.oracle_text.clone(),
                    image: match &card.visual {
                        CardVisual::Generated { image, .. } => image.clone(),
                        CardVisual::FullImage { image } => Some(image.clone()),
                    },
                    background_image: match &card.visual {
                        CardVisual::Generated {
                            background_image, ..
                        } => background_image.clone(),
                        CardVisual::FullImage { .. } => None,
                    },
                    colors: card.colors.clone(),
                    power: card.power.clone(),
                    toughness: card.toughness.clone(),
                    back_image: card.back_image.clone(),
                    token_pools: self
                        .engine
                        .state
                        .get_card_token_pools(&card.id)?
                        .into_iter()
                        .flat_map(|pools| pools.values())
                        .map(token_pool_view)
                        .collect(),
                });
            }
            let mut token_pools = Vec::new();
            for pool in self.engine.state.get_zone_token_pools(&pile.id)?.values() {
                token_pools.push(token_pool_view(pool));
            }
            let layout = self
                .zone_layouts
                .iter()
                .find(|zone| zone.id == pile.zone_id);
            zones.push(ZoneViewProto {
                id: pile.id.clone(),
                battlefield: pile.role.as_deref() == Some("play_default")
                    || layout.is_some_and(|zone| zone.parent_zone.is_some()),
                cards,
                token_pools,
            });
        }

        let players = self
            .engine
            .state
            .players
            .values()
            .map(|player| PlayerViewProto {
                id: player.id.clone(),
                name: player.name.clone(),
                token_pools: player.token_pools.values().map(token_pool_view).collect(),
            })
            .collect();
        let snapshot = GameStateSnapshotProto { zones, players };
        let mut bytes = Vec::new();
        snapshot.encode(&mut bytes).map_err(|err| {
            EngineError::Validation(format!("Failed to encode state protobuf: {err}"))
        })?;
        Ok(bytes)
    }
}

// ── Protobuf message definitions ─────────────────────────────────────────────

#[derive(Clone, PartialEq, Message)]
struct GameStateSnapshotProto {
    #[prost(message, repeated, tag = "1")]
    zones: Vec<ZoneViewProto>,
    #[prost(message, repeated, tag = "2")]
    players: Vec<PlayerViewProto>,
}

#[derive(Clone, PartialEq, Message)]
struct PlayerViewProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(message, repeated, tag = "3")]
    token_pools: Vec<TokenPoolViewProto>,
}

#[derive(Clone, PartialEq, Message)]
struct ZoneViewProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(bool, tag = "2")]
    battlefield: bool,
    #[prost(message, repeated, tag = "3")]
    cards: Vec<CardViewProto>,
    #[prost(message, repeated, tag = "4")]
    token_pools: Vec<TokenPoolViewProto>,
}

#[derive(Clone, PartialEq, Message)]
struct CardViewProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(string, tag = "3")]
    card_type: String,
    #[prost(message, repeated, tag = "4")]
    token_pools: Vec<TokenPoolViewProto>,
    #[prost(string, optional, tag = "5")]
    mana: Option<String>,
    #[prost(string, optional, tag = "6")]
    oracle_text: Option<String>,
    #[prost(string, optional, tag = "7")]
    image: Option<String>,
    #[prost(string, optional, tag = "8")]
    background_image: Option<String>,
    #[prost(string, optional, tag = "9")]
    colors: Option<String>,
    #[prost(string, optional, tag = "10")]
    power: Option<String>,
    #[prost(string, optional, tag = "11")]
    toughness: Option<String>,
    #[prost(string, optional, tag = "12")]
    back_image: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
struct TokenPoolViewProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    label: String,
    #[prost(string, tag = "3")]
    token: String,
    #[prost(string, optional, tag = "4")]
    background: Option<String>,
    #[prost(uint32, tag = "5")]
    count: u32,
    #[prost(bool, tag = "6")]
    active: bool,
    #[prost(uint32, optional, tag = "7")]
    min: Option<u32>,
    #[prost(uint32, optional, tag = "8")]
    max: Option<u32>,
    #[prost(uint32, tag = "9")]
    plus: u32,
    #[prost(uint32, tag = "10")]
    minus: u32,
    #[prost(uint32, tag = "11")]
    starting: u32,
    #[prost(string, optional, tag = "12")]
    parent_id: Option<String>,
    #[prost(string, optional, tag = "13")]
    icon_color: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
struct ZoneLayoutProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(string, tag = "3")]
    color: String,
    #[prost(uint32, tag = "4")]
    x: u32,
    #[prost(uint32, tag = "5")]
    y: u32,
    #[prost(uint32, tag = "6")]
    width: u32,
    #[prost(uint32, tag = "7")]
    height: u32,
    #[prost(string, tag = "8")]
    scope: String,
    #[prost(string, optional, tag = "9")]
    parent_zone: Option<String>,
    #[prost(string, repeated, tag = "10")]
    allowed_card_types: Vec<String>,
    #[prost(uint32, optional, tag = "11")]
    max_cards: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
struct PileViewProto {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(string, tag = "3")]
    zone_id: String,
    #[prost(uint32, tag = "4")]
    x: u32,
    #[prost(uint32, tag = "5")]
    y: u32,
    #[prost(string, repeated, tag = "6")]
    associated_piles: Vec<String>,
    #[prost(bool, tag = "7")]
    visible: bool,
    #[prost(string, optional, tag = "8")]
    role: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
struct BoardLayoutProto {
    #[prost(message, repeated, tag = "1")]
    zones: Vec<ZoneLayoutProto>,
    #[prost(message, repeated, tag = "2")]
    piles: Vec<PileViewProto>,
}

fn restore_token_pool_views(
    target: &mut std::collections::HashMap<String, TokenPool>,
    views: Vec<TokenPoolViewProto>,
    owner: &str,
) -> Result<(), EngineError> {
    if views.len() != target.len() {
        return Err(EngineError::Validation(format!(
            "Saved {owner} token pools do not match the current configuration"
        )));
    }
    let mut restored = std::collections::HashSet::new();
    for view in views {
        if !restored.insert(view.id.clone()) {
            return Err(EngineError::Validation(format!(
                "Saved {owner} contains duplicate token pool '{}'",
                view.id
            )));
        }
        let pool = target.get_mut(&view.id).ok_or_else(|| {
            EngineError::Validation(format!(
                "Saved {owner} contains unknown token pool '{}'",
                view.id
            ))
        })?;
        pool.count = view.count;
        pool.active = view.active;
        pool.validate().map_err(EngineError::Validation)?;
    }
    Ok(())
}

fn token_pool_view(pool: &TokenPool) -> TokenPoolViewProto {
    TokenPoolViewProto {
        id: pool.id.clone(),
        label: pool.label.clone(),
        token: pool.token.clone(),
        background: pool.background.clone(),
        count: pool.count,
        active: pool.active,
        min: pool.min,
        max: pool.max,
        plus: pool.plus,
        minus: pool.minus,
        starting: pool.starting,
        parent_id: pool.parent_id.clone(),
        icon_color: pool.icon_color.clone(),
    }
}

// ── Embedded CSV helpers ──────────────────────────────────────────────────────

fn load_cards_from_embedded_csv(raw_csv: &str) -> Result<Vec<Card>, EngineError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(Cursor::new(raw_csv.as_bytes()));
    let headers = reader
        .headers()
        .map_err(EngineError::Csv)?
        .iter()
        .enumerate()
        .map(|(idx, h)| (h.trim().to_ascii_lowercase(), idx))
        .collect::<std::collections::HashMap<_, _>>();

    let id_idx = headers
        .get("id")
        .copied()
        .ok_or_else(|| EngineError::Validation("Missing required 'id' CSV header".to_string()))?;
    let name_idx = headers
        .get("name")
        .copied()
        .ok_or_else(|| EngineError::Validation("Missing required 'name' CSV header".to_string()))?;
    let card_type_idx = headers
        .get("card_type")
        .copied()
        .or_else(|| headers.get("type").copied())
        .ok_or_else(|| {
            EngineError::Validation("Missing required 'card_type' CSV header".to_string())
        })?;

    let mut cards = Vec::new();
    for (line_idx, record) in reader.records().enumerate() {
        let record = record.map_err(EngineError::Csv)?;
        let id = required_value(&record, id_idx, "id", line_idx + 2)?;
        let name = required_value(&record, name_idx, "name", line_idx + 2)?;
        let card_type_raw = required_value(&record, card_type_idx, "card_type", line_idx + 2)?;

        cards.push(Card {
            id,
            game_id: String::new(),
            name,
            card_type_id: card_type_raw.trim().to_ascii_lowercase().replace(' ', "-"),
            description: None,
            cost: None,
            visual: CardVisual::Generated {
                image: optional_value(&record, headers.get("image").copied()),
                background_image: optional_value(&record, headers.get("background_image").copied()),
                background_color: optional_value(&record, headers.get("background_color").copied()),
                icon: None,
            },
            back_logo: None,
            back_image: optional_value(&record, headers.get("back_image").copied()),
            mana: optional_value(&record, headers.get("mana").copied()),
            colors: optional_value(&record, headers.get("colors").copied()),
            oracle_text: optional_value(&record, headers.get("oracle_text").copied()),
            power: optional_value(&record, headers.get("power").copied()),
            toughness: optional_value(&record, headers.get("toughness").copied()),
            is_commander: optional_bool(&record, headers.get("is_commander").copied()),
            is_partner: optional_bool(&record, headers.get("is_partner").copied()),
            token_pools: optional_token_pools(
                &record,
                headers.get("token_pools").copied(),
                line_idx + 2,
            )?,
            starting_pile: optional_value(&record, headers.get("starting_pile").copied()),
        });
    }

    Ok(cards)
}

fn required_value(
    record: &csv::StringRecord,
    idx: usize,
    field: &str,
    line: usize,
) -> Result<String, EngineError> {
    let Some(value) = record.get(idx).map(str::trim).filter(|v| !v.is_empty()) else {
        return Err(EngineError::Validation(format!(
            "Missing required value for '{field}' at CSV line {line}"
        )));
    };
    Ok(value.to_string())
}

fn optional_value(record: &csv::StringRecord, idx: Option<usize>) -> Option<String> {
    idx.and_then(|column| record.get(column))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn optional_bool(record: &csv::StringRecord, idx: Option<usize>) -> bool {
    let Some(value) = optional_value(record, idx) else {
        return false;
    };

    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "y"
    )
}

fn optional_token_pools(
    record: &csv::StringRecord,
    idx: Option<usize>,
    line: usize,
) -> Result<Vec<TokenPool>, EngineError> {
    let Some(value) = optional_value(record, idx) else {
        return Ok(Vec::new());
    };

    TokenPool::parse_list(&value).map_err(|message| {
        EngineError::Validation(format!(
            "Invalid token_pools value at CSV line {line}: {message}"
        ))
    })
}
