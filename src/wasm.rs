use std::io::Cursor;

use prost::Message;
use wasm_bindgen::prelude::*;

use cards::cards::CardVisual;
use decks::{Pile, ZoneLayout, parse_piles_csv, parse_zones_csv};
use engine::{Card, CardEngine, DEFAULT_PLAYER_ID, EngineError, Zone};
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

const DEFAULT_CARDS_CSV: &str = include_str!("../data/cards.csv");
const DEFAULT_PILES_CSV: &str = include_str!("../data/piles.csv");
const DEFAULT_ZONES_CSV: &str = include_str!("../data/zones.csv");
const DEFAULT_TOKEN_TYPES_CSV: &str = include_str!("../data/token_types.csv");
const DEFAULT_TOKEN_POOLS_CSV: &str = include_str!("../data/token_pools.csv");

/// Maps a pile id string to the corresponding `Zone` variant.
fn pile_id_to_zone(pile_id: &str) -> Option<Zone> {
    match pile_id {
        "commander_pile" => Some(Zone::CommanderPile),
        "main_stack" => Some(Zone::MainStack),
        "hand" => Some(Zone::Hand),
        "lands" => Some(Zone::Lands),
        "deck" => Some(Zone::Deck),
        "discard" => Some(Zone::Discard),
        "exile" => Some(Zone::Exile),
        "artifacts" => Some(Zone::Artifacts),
        "enchantments" => Some(Zone::Enchantments),
        "creatures" => Some(Zone::Creatures),
        "battlefield" | "main_zone" => Some(Zone::Battlefield),
        _ => None,
    }
}

/// Maps board-layout zone ids to the engine zone that owns their token pools.
fn layout_id_to_zone(zone_id: &str) -> Option<Zone> {
    match zone_id {
        "command_zone" => Some(Zone::CommanderPile),
        "deck_zone" => Some(Zone::Deck),
        "stack_zone" => Some(Zone::MainStack),
        "discard_zone" => Some(Zone::Discard),
        "exile_zone" => Some(Zone::Exile),
        "battlefield" | "main_zone" => Some(Zone::Battlefield),
        "lands_zone" => Some(Zone::Lands),
        "artifacts_zone" => Some(Zone::Artifacts),
        "enchantments_zone" => Some(Zone::Enchantments),
        "creatures_zone" => Some(Zone::Creatures),
        "hand" => Some(Zone::Hand),
        id => Zone::from_pile_id(id),
    }
}

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
        install_panic_logger();
        let mut cards = load_cards_from_embedded_csv(DEFAULT_CARDS_CSV).map_err(js_error)?;
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
        }

        let piles = parse_piles_csv(DEFAULT_PILES_CSV).map_err(js_error)?;

        let zone_layouts = parse_zones_csv(DEFAULT_ZONES_CSV).map_err(js_error)?;

        // Group cards by starting_pile; fall back to main_stack for non-commanders,
        // commander_pile for commanders with no starting_pile.
        let mut zone_cards: std::collections::HashMap<Zone, Vec<String>> =
            Zone::ALL.iter().map(|z| (*z, Vec::new())).collect();

        for card in &cards {
            let target_zone = card
                .starting_pile
                .as_deref()
                .and_then(pile_id_to_zone)
                .unwrap_or_else(|| {
                    if card.is_commander {
                        Zone::CommanderPile
                    } else {
                        Zone::MainStack
                    }
                });
            zone_cards
                .entry(target_zone)
                .or_default()
                .push(card.id.clone());
        }

        let mut engine = CardEngine::new(cards, None);
        let token_types = ingest_token_types_csv(DEFAULT_TOKEN_TYPES_CSV).map_err(js_error)?;
        let definitions =
            ingest_token_pools_csv(DEFAULT_TOKEN_POOLS_CSV, &token_types).map_err(js_error)?;
        let player_pools = definitions
            .iter()
            .filter(|definition| definition.owner == TokenPoolOwner::Player)
            .map(|definition| definition.pool.clone())
            .collect();
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
                        .and_then(pile_id_to_zone)
                        .unwrap_or(Zone::Battlefield);
                    engine
                        .add_zone_token_pool(zone, definition.pool)
                        .map_err(js_error)?;
                }
                TokenPoolOwner::Player => {}
            }
        }
        for layout in &zone_layouts {
            if !layout.token_pools.is_empty() {
                let zone = layout_id_to_zone(&layout.id).ok_or_else(|| {
                    js_error(format!(
                        "Zone '{}' defines token pools but has no engine zone mapping",
                        layout.id
                    ))
                })?;
                engine
                    .set_zone_token_pools(zone, layout.token_pools.clone())
                    .map_err(js_error)?;
            }
        }

        for (zone, ids) in zone_cards {
            if !ids.is_empty() {
                engine.state.set_zone_cards(zone, ids).map_err(js_error)?;
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
        self.engine.draw().map_err(js_error)?;
        self.state_proto()
    }

    pub fn auto_play_first_hand_card(&mut self) -> Result<Vec<u8>, JsValue> {
        let Some(card_id) = self.engine.state.zone_cards(Zone::Hand).first().cloned() else {
            return self.state_proto();
        };

        let card_type = self
            .engine
            .state
            .card_by_id(&card_id)
            .map_err(js_error)?
            .card_type_id
            .clone();

        match card_type.trim().to_ascii_lowercase().as_str() {
            "land" => self.engine.play_land(&card_id).map_err(js_error)?,
            "artifact" | "enchantment" | "creature" => self
                .engine
                .cast_to_battlefield(&card_id)
                .map_err(js_error)?,
            _ => self
                .engine
                .discard(Zone::Hand, &card_id)
                .map_err(js_error)?,
        };

        self.state_proto()
    }

    pub fn discard_first_hand_card(&mut self) -> Result<Vec<u8>, JsValue> {
        let Some(card_id) = self.engine.state.zone_cards(Zone::Hand).first().cloned() else {
            return self.state_proto();
        };
        self.engine
            .discard(Zone::Hand, &card_id)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn add_hand_energy(&mut self, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.engine
            .add_tokens_to_zone_pool(Zone::Hand, "energy", amount)
            .map_err(js_error)?;
        self.state_proto()
    }

    pub fn move_card(
        &mut self,
        card_id: &str,
        from_pile: &str,
        to_pile: &str,
    ) -> Result<Vec<u8>, JsValue> {
        self.engine
            .move_card_between_piles(from_pile, to_pile, card_id)
            .map_err(js_error)?;
        self.state_proto()
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
        let from = pile_id_to_zone(from_pile)
            .ok_or_else(|| js_error(format!("Unknown pile '{from_pile}'")))?;
        let cards = self.engine.state.zone_cards(from);
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

        for card_id in card_ids {
            self.engine
                .move_card_between_piles(from_pile, to_pile, &card_id)
                .map_err(js_error)?;
        }
        self.state_proto()
    }

    pub fn shuffle_pile(&mut self, pile_id: &str) -> Result<Vec<u8>, JsValue> {
        let zone = pile_id_to_zone(pile_id)
            .ok_or_else(|| js_error(format!("Unknown pile '{pile_id}'")))?;
        self.engine.state.shuffle_zone(zone).map_err(js_error)?;
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
        for zone in Zone::ALL {
            let mut cards = Vec::new();
            for card_id in self.engine.state.zone_cards(*zone) {
                let card = self.engine.state.card_by_id(card_id)?;
                cards.push(CardViewProto {
                    id: card.id.clone(),
                    name: card.name.clone(),
                    card_type: card.card_type_id.clone(),
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
            for pool in self.engine.state.get_zone_token_pools(*zone)?.values() {
                token_pools.push(token_pool_view(pool));
            }
            zones.push(ZoneViewProto {
                id: zone.to_string(),
                battlefield: zone.is_battlefield(),
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
}

#[derive(Clone, PartialEq, Message)]
struct BoardLayoutProto {
    #[prost(message, repeated, tag = "1")]
    zones: Vec<ZoneLayoutProto>,
    #[prost(message, repeated, tag = "2")]
    piles: Vec<PileViewProto>,
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
    let mut reader = csv::Reader::from_reader(Cursor::new(raw_csv.as_bytes()));
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
                image: None,
                background_image: None,
                background_color: None,
                icon: None,
            },
            back_logo: None,
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
