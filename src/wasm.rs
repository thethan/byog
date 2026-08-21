use std::io::Cursor;

use prost::Message;
use wasm_bindgen::prelude::*;

use crate::card::{Card, CardType};
use crate::engine::{CardEngine, EngineError};
use crate::token_pool::TokenPool;
use crate::zones::Zone;

const DEFAULT_CARDS_CSV: &str = include_str!("../data/cards.csv");

#[wasm_bindgen]
pub struct WasmGame {
    engine: CardEngine,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmGame, JsValue> {
        let cards = load_cards_from_embedded_csv(DEFAULT_CARDS_CSV)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        if cards.is_empty() {
            return Err(JsValue::from_str("No cards found in embedded CSV"));
        }

        let mut commander_ids = Vec::new();
        let mut main_stack = Vec::new();
        for card in &cards {
            if card.is_commander && commander_ids.len() < 2 {
                commander_ids.push(card.id.clone());
            } else {
                main_stack.push(card.id.clone());
            }
        }

        let mut engine = CardEngine::new(cards, None);
        let energy_pool = TokenPool::configured(
            "energy",
            "Energy",
            "fa-bolt",
            Some("amber".to_string()),
            1,
            Some(0),
            Some(9),
            true,
        )
        .map_err(|message| JsValue::from_str(&message))?;
        engine
            .set_zone_token_pools(Zone::Hand, vec![energy_pool])
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        if !commander_ids.is_empty() {
            engine
                .state
                .set_zone_cards(Zone::CommanderPile, commander_ids)
                .map_err(|err| JsValue::from_str(&err.to_string()))?;
        }

        engine
            .state
            .set_zone_cards(Zone::MainStack, main_stack)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;

        Ok(Self { engine })
    }

    pub fn state_proto(&self) -> Result<Vec<u8>, JsValue> {
        self.snapshot_proto()
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    pub fn draw(&mut self) -> Result<Vec<u8>, JsValue> {
        self.engine
            .draw()
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
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
            .map_err(|err| JsValue::from_str(&err.to_string()))?
            .card_type
            .clone();

        match card_type {
            CardType::Land => self
                .engine
                .play_land(&card_id)
                .map_err(|err| JsValue::from_str(&err.to_string()))?,
            CardType::Artifact | CardType::Enchantment | CardType::Creature => self
                .engine
                .cast_to_battlefield(&card_id)
                .map_err(|err| JsValue::from_str(&err.to_string()))?,
            CardType::Other(_) => self
                .engine
                .discard(Zone::Hand, &card_id)
                .map_err(|err| JsValue::from_str(&err.to_string()))?,
        };

        self.state_proto()
    }

    pub fn discard_first_hand_card(&mut self) -> Result<Vec<u8>, JsValue> {
        let Some(card_id) = self.engine.state.zone_cards(Zone::Hand).first().cloned() else {
            return self.state_proto();
        };
        self.engine
            .discard(Zone::Hand, &card_id)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.state_proto()
    }

    pub fn add_hand_energy(&mut self, amount: u32) -> Result<Vec<u8>, JsValue> {
        self.engine
            .add_tokens_to_zone_pool(Zone::Hand, "energy", amount)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.state_proto()
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
                    card_type: card.card_type.to_string(),
                });
            }
            let mut token_pools = Vec::new();
            for pool in self.engine.state.get_zone_token_pools(*zone)?.values() {
                token_pools.push(TokenPoolViewProto {
                    id: pool.id.clone(),
                    label: pool.label.clone(),
                    token: pool.token.clone(),
                    background: pool.background.clone(),
                    count: pool.count,
                    active: pool.active,
                });
            }
            zones.push(ZoneViewProto {
                id: zone.to_string(),
                battlefield: zone.is_battlefield(),
                cards,
                token_pools,
            });
        }

        let snapshot = GameStateSnapshotProto { zones };
        let mut bytes = Vec::new();
        snapshot.encode(&mut bytes).map_err(|err| {
            EngineError::Validation(format!("Failed to encode state protobuf: {err}"))
        })?;
        Ok(bytes)
    }
}

#[derive(Clone, PartialEq, Message)]
struct GameStateSnapshotProto {
    #[prost(message, repeated, tag = "1")]
    zones: Vec<ZoneViewProto>,
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
}

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
            name,
            card_type: CardType::parse(&card_type_raw),
            mana_cost: optional_value(&record, headers.get("mana_cost").copied()),
            colors: optional_value(&record, headers.get("colors").copied()),
            oracle_text: optional_value(&record, headers.get("oracle_text").copied()),
            power: optional_value(&record, headers.get("power").copied()),
            toughness: optional_value(&record, headers.get("toughness").copied()),
            is_commander: optional_bool(&record, headers.get("is_commander").copied()),
            is_partner: optional_bool(&record, headers.get("is_partner").copied()),
            token_pools: optional_token_pools(&record, headers.get("token_pools").copied(), line_idx + 2)?,
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
