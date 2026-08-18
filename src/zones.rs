use std::collections::HashMap;

use crate::card::Card;
use crate::engine::EngineError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    MainStack,
    CommanderPile,
    Hand,
    LandPile,
    Deck,
    Discard,
    Exile,
    ArtifactList,
    EnchantmentList,
    CreatureList,
}

impl Zone {
    pub const ALL: &'static [Zone] = &[
        Zone::MainStack,
        Zone::CommanderPile,
        Zone::Hand,
        Zone::LandPile,
        Zone::Deck,
        Zone::Discard,
        Zone::Exile,
        Zone::ArtifactList,
        Zone::EnchantmentList,
        Zone::CreatureList,
    ];

    pub fn is_battlefield(self) -> bool {
        matches!(
            self,
            Zone::ArtifactList | Zone::EnchantmentList | Zone::CreatureList
        )
    }
}

impl std::fmt::Display for Zone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::MainStack => write!(f, "MainStack"),
            Zone::CommanderPile => write!(f, "CommanderPile"),
            Zone::Hand => write!(f, "Hand"),
            Zone::LandPile => write!(f, "LandPile"),
            Zone::Deck => write!(f, "Deck"),
            Zone::Discard => write!(f, "Discard"),
            Zone::Exile => write!(f, "Exile"),
            Zone::ArtifactList => write!(f, "ArtifactList"),
            Zone::EnchantmentList => write!(f, "EnchantmentList"),
            Zone::CreatureList => write!(f, "CreatureList"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub cards: HashMap<String, Card>,
    pub zones: HashMap<Zone, Vec<String>>,
}

impl GameState {
    pub fn new(cards: Vec<Card>) -> Self {
        let cards = cards
            .into_iter()
            .map(|card| (card.id.clone(), card))
            .collect::<HashMap<_, _>>();

        let mut zones = HashMap::new();
        for zone in Zone::ALL {
            zones.insert(*zone, Vec::new());
        }

        Self { cards, zones }
    }

    pub fn set_zone_cards(&mut self, zone: Zone, card_ids: Vec<String>) -> Result<(), EngineError> {
        self.ensure_cards_exist(&card_ids)?;
        if zone == Zone::CommanderPile {
            self.validate_commander_contents(&card_ids)?;
        }

        self.zones.insert(zone, card_ids);
        Ok(())
    }

    pub fn zone_cards(&self, zone: Zone) -> &[String] {
        self.zones.get(&zone).map(Vec::as_slice).unwrap_or(&[])
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
            self.validate_commander_contents(&commander_cards)?;
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

    fn ensure_cards_exist(&self, card_ids: &[String]) -> Result<(), EngineError> {
        for card_id in card_ids {
            if !self.cards.contains_key(card_id) {
                return Err(EngineError::Validation(format!(
                    "Unknown card id '{card_id}'"
                )));
            }
        }
        Ok(())
    }

    fn validate_commander_contents(&self, card_ids: &[String]) -> Result<(), EngineError> {
        match card_ids.len() {
            0 | 1 => Ok(()),
            2 => {
                let first = self.card_by_id(&card_ids[0])?;
                let second = self.card_by_id(&card_ids[1])?;
                if first.is_partner && second.is_partner {
                    Ok(())
                } else {
                    Err(EngineError::Validation(
                        "CommanderPile can only have 2 cards when both cards are partners"
                            .to_string(),
                    ))
                }
            }
            _ => Err(EngineError::Validation(
                "CommanderPile cannot contain more than 2 cards".to_string(),
            )),
        }
    }

    pub fn card_by_id(&self, card_id: &str) -> Result<&Card, EngineError> {
        self.cards
            .get(card_id)
            .ok_or_else(|| EngineError::Validation(format!("Unknown card id '{card_id}'")))
    }
}
