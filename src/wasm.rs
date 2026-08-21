use std::io::Cursor;

use serde::Serialize;
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

    pub fn state_json(&self) -> Result<JsValue, JsValue> {
        self.snapshot_value()
            .map_err(|err| JsValue::from_str(&err.to_string()))
    }

    pub fn draw(&mut self) -> Result<JsValue, JsValue> {
        self.engine
            .draw()
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.state_json()
    }

    pub fn auto_play_first_hand_card(&mut self) -> Result<JsValue, JsValue> {
        let Some(card_id) = self.engine.state.zone_cards(Zone::Hand).first().cloned() else {
            return self.state_json();
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

        self.state_json()
    }

    pub fn discard_first_hand_card(&mut self) -> Result<JsValue, JsValue> {
        let Some(card_id) = self.engine.state.zone_cards(Zone::Hand).first().cloned() else {
            return self.state_json();
        };
        self.engine
            .discard(Zone::Hand, &card_id)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.state_json()
    }

    pub fn add_hand_energy(&mut self, amount: u32) -> Result<JsValue, JsValue> {
        self.engine
            .add_tokens_to_zone_pool(Zone::Hand, "energy", amount)
            .map_err(|err| JsValue::from_str(&err.to_string()))?;
        self.state_json()
    }
}

impl WasmGame {
    fn snapshot_value(&self) -> Result<JsValue, EngineError> {
        let zones = Zone::ALL
            .iter()
            .map(|zone| {
                let cards = self
                    .engine
                    .state
                    .zone_cards(*zone)
                    .iter()
                    .filter_map(|card_id| self.engine.state.card_by_id(card_id).ok())
                    .map(|card| CardView {
                        id: card.id.clone(),
                        name: card.name.clone(),
                        card_type: card.card_type.to_string(),
                    })
                    .collect::<Vec<_>>();

                let token_pools = self
                    .engine
                    .state
                    .get_zone_token_pools(*zone)?
                    .values()
                    .map(|pool| TokenPoolView {
                        id: pool.id.clone(),
                        label: pool.label.clone(),
                        token: pool.token.clone(),
                        background: pool.background.clone(),
                        count: pool.count,
                        active: pool.active,
                    })
                    .collect::<Vec<_>>();

                Ok(ZoneView {
                    id: zone.to_string(),
                    battlefield: zone.is_battlefield(),
                    cards,
                    token_pools,
                })
            })
            .collect::<Result<Vec<_>, EngineError>>()?;

        serde_wasm_bindgen::to_value(&GameSnapshot { zones })
            .map_err(|err| EngineError::Validation(err.to_string()))
    }
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

#[derive(Serialize)]
struct GameSnapshot {
    zones: Vec<ZoneView>,
}

#[derive(Serialize)]
struct ZoneView {
    id: String,
    battlefield: bool,
    cards: Vec<CardView>,
    token_pools: Vec<TokenPoolView>,
}

#[derive(Serialize)]
struct CardView {
    id: String,
    name: String,
    card_type: String,
}

#[derive(Serialize)]
struct TokenPoolView {
    id: String,
    label: String,
    token: String,
    background: Option<String>,
    count: u32,
    active: bool,
}
