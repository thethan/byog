use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardType {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub background_image: Option<String>,
    pub background_color: Option<String>,
    /// Font Awesome Pro identifier, for example `fa-solid fa-dragon`.
    pub icon: Option<String>,
    pub back_logo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardCostComponent {
    pub resource: String,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardCost {
    pub components: Vec<CardCostComponent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub card_type_id: String,
    pub description: Option<String>,
    pub cost: Option<CardCost>,
    pub visual: CardVisual,
    pub back_logo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardVisual {
    /// Generated layout containing name, type, cost, description, and optional artwork.
    Generated {
        image: Option<String>,
        background_image: Option<String>,
        background_color: Option<String>,
        icon: Option<String>,
    },
    /// Complete pre-rendered card. The renderer displays only this image.
    FullImage { image: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAppearance {
    pub image: Option<String>,
    pub background_image: Option<String>,
    pub background_color: Option<String>,
    pub icon: Option<String>,
    pub back_logo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvIngestionError {
    pub row: usize,
    pub message: String,
}

impl CsvIngestionError {
    fn new(row: usize, message: impl Into<String>) -> Self { Self { row, message: message.into() } }
}

impl Display for CsvIngestionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "row {}: {}", self.row, self.message) }
}

impl Error for CsvIngestionError {}

fn normalize_id(value: &str) -> String {
    value.trim().to_ascii_lowercase().chars().fold(String::new(), |mut out, ch| {
        if ch == '_' || ch.is_ascii_whitespace() {
            if !out.ends_with('-') { out.push('-'); }
        } else { out.push(ch); }
        out
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-') && !value.ends_with('-')
        && value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !value.contains("--")
}

fn optional(value: Option<&String>) -> Option<String> {
    value.map(|v| v.trim()).filter(|v| !v.is_empty()).map(str::to_owned)
}

pub fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, CsvIngestionError> {
    let chars: Vec<char> = input.chars().collect();
    let (mut rows, mut row, mut field, mut quoted) = (Vec::new(), Vec::new(), String::new(), false);
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if quoted {
            if ch == '"' && chars.get(i + 1) == Some(&'"') { field.push('"'); i += 1; }
            else if ch == '"' { quoted = false; }
            else { field.push(ch); }
        } else if ch == '"' && field.is_empty() { quoted = true; }
        else if ch == ',' { row.push(std::mem::take(&mut field)); }
        else if ch == '\n' {
            row.push(std::mem::take(&mut field).trim_end_matches('\r').to_owned());
            if row.iter().any(|v| !v.is_empty()) { rows.push(std::mem::take(&mut row)); } else { row.clear(); }
        } else { field.push(ch); }
        i += 1;
    }
    if quoted { return Err(CsvIngestionError::new(rows.len() + 1, "unterminated quoted field")); }
    row.push(field.trim_end_matches('\r').to_owned());
    if row.iter().any(|v| !v.is_empty()) { rows.push(row); }
    Ok(rows)
}

fn records(input: &str, required: &[&str]) -> Result<Vec<(usize, HashMap<String, String>)>, CsvIngestionError> {
    let rows = parse_csv(input)?;
    if rows.is_empty() { return Ok(Vec::new()); }
    let headers: Vec<String> = rows[0].iter().map(|v| v.trim().to_ascii_lowercase()).collect();
    for name in required {
        if !headers.iter().any(|header| header == name) { return Err(CsvIngestionError::new(1, format!("missing required column '{name}'"))); }
    }
    Ok(rows.into_iter().skip(1).enumerate().map(|(index, values)| {
        let record = headers.iter().enumerate().map(|(column, header)| (header.clone(), values.get(column).cloned().unwrap_or_default())).collect();
        (index + 2, record)
    }).collect())
}

fn required_value<'a>(values: &'a HashMap<String, String>, name: &str, row: usize) -> Result<&'a str, CsvIngestionError> {
    values.get(name).map(String::as_str).map(str::trim).filter(|v| !v.is_empty())
        .ok_or_else(|| CsvIngestionError::new(row, format!("'{name}' is required")))
}

fn checked_id(value: &str, field: &str, row: usize) -> Result<String, CsvIngestionError> {
    let id = normalize_id(value);
    valid_id(&id).then_some(id).ok_or_else(|| CsvIngestionError::new(row, format!("'{field}' is not a valid identifier")))
}

pub fn parse_card_cost(value: &str, row: usize) -> Result<Option<CardCost>, CsvIngestionError> {
    if value.trim().is_empty() { return Ok(None); }
    let mut components = Vec::new();
    for part in value.split('|') {
        let token = part.trim();
        let pieces: Vec<&str> = token.split(':').collect();
        let (resource, amount) = match pieces.as_slice() { [amount] => ("generic", *amount), [resource, amount] => (*resource, *amount), _ => return Err(CsvIngestionError::new(row, format!("invalid cost component '{token}'"))) };
        let resource = normalize_id(resource);
        let amount = amount.parse::<u32>().ok().filter(|v| *v > 0);
        if !valid_id(&resource) || amount.is_none() { return Err(CsvIngestionError::new(row, format!("invalid cost component '{token}'"))); }
        components.push(CardCostComponent { resource, amount: amount.unwrap() });
    }
    Ok(Some(CardCost { components }))
}

pub fn ingest_card_types_csv(input: &str) -> Result<Vec<CardType>, CsvIngestionError> {
    let mut seen = HashSet::new();
    records(input, &["game_id", "id", "name"])?.into_iter().map(|(row, values)| {
        let game_id = checked_id(required_value(&values, "game_id", row)?, "game_id", row)?;
        let id = checked_id(required_value(&values, "id", row)?, "id", row)?;
        if !seen.insert(format!("{game_id}/{id}")) { return Err(CsvIngestionError::new(row, format!("duplicate card type '{game_id}/{id}'"))); }
        Ok(CardType { game_id, id, name: required_value(&values, "name", row)?.to_owned(), description: optional(values.get("description")), background_image: optional(values.get("background_image")), background_color: optional(values.get("background_color")), icon: optional(values.get("icon")), back_logo: optional(values.get("back_logo")) })
    }).collect()
}

pub fn ingest_cards_csv(input: &str, card_types: &[CardType]) -> Result<Vec<Card>, CsvIngestionError> {
    let type_keys: HashSet<String> = card_types.iter().map(|t| format!("{}/{}", t.game_id, t.id)).collect();
    let mut seen = HashSet::new();
    records(input, &["game_id", "id", "name", "card_type_id"])?.into_iter().map(|(row, values)| {
        let game_id = checked_id(required_value(&values, "game_id", row)?, "game_id", row)?;
        let id = checked_id(required_value(&values, "id", row)?, "id", row)?;
        let card_type_id = checked_id(required_value(&values, "card_type_id", row)?, "card_type_id", row)?;
        if !seen.insert(format!("{game_id}/{id}")) { return Err(CsvIngestionError::new(row, format!("duplicate card '{game_id}/{id}'"))); }
        if !type_keys.contains(&format!("{game_id}/{card_type_id}")) { return Err(CsvIngestionError::new(row, format!("unknown card type '{game_id}/{card_type_id}'"))); }
        let visual = match optional(values.get("full_image")) {
            Some(image) => CardVisual::FullImage { image },
            None => CardVisual::Generated {
                image: optional(values.get("image")),
                background_image: optional(values.get("background_image")),
                background_color: optional(values.get("background_color")),
                icon: optional(values.get("icon")),
            },
        };
        Ok(Card { game_id, id, card_type_id, name: required_value(&values, "name", row)?.to_owned(), description: optional(values.get("description")), cost: parse_card_cost(values.get("cost").map_or("", String::as_str), row)?, visual, back_logo: optional(values.get("back_logo")) })
    }).collect()
}

pub fn resolve_card_visual(card: &Card, card_type: &CardType) -> Result<CardVisual, String> {
    if card.game_id != card_type.game_id || card.card_type_id != card_type.id { return Err(format!("card type '{}/{}' does not belong to card '{}/{}'", card_type.game_id, card_type.id, card.game_id, card.id)); }
    Ok(match &card.visual {
        CardVisual::FullImage { image } => CardVisual::FullImage { image: image.clone() },
        CardVisual::Generated { image, background_image, background_color, icon } => CardVisual::Generated {
            image: image.clone(),
            background_image: background_image.clone().or_else(|| card_type.background_image.clone()),
            background_color: background_color.clone().or_else(|| card_type.background_color.clone()),
            icon: icon.clone().or_else(|| card_type.icon.clone()),
        },
    })
}

pub fn resolve_card_appearance(card: &Card, card_type: &CardType) -> Result<Option<CardAppearance>, String> {
    match resolve_card_visual(card, card_type)? {
        CardVisual::FullImage { .. } => Ok(None),
        CardVisual::Generated { image, background_image, background_color, icon } => Ok(Some(CardAppearance { image, background_image, background_color, icon, back_logo: card.back_logo.clone().or_else(|| card_type.back_logo.clone()) })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingests_cost_and_inherits_appearance() {
        let types = ingest_card_types_csv("game_id,id,name,background_color,icon,back_logo\narcana,creature,Creature,#123456,fa-solid fa-dragon,/back.svg").unwrap();
        let cards = ingest_cards_csv("game_id,id,name,card_type_id,description,cost,image,background_color,full_image\narcana,ember-drake,Ember Drake,creature,A dragon born from flame,2|fire:2,/art/ember.webp,#654321,", &types).unwrap();
        assert_eq!(cards[0].cost.as_ref().unwrap().components[1], CardCostComponent { resource: "fire".into(), amount: 2 });
        let appearance = resolve_card_appearance(&cards[0], &types[0]).unwrap().unwrap();
        assert_eq!(appearance.image.as_deref(), Some("/art/ember.webp"));
        assert_eq!(appearance.background_color.as_deref(), Some("#654321"));
        assert_eq!(appearance.icon.as_deref(), Some("fa-solid fa-dragon"));
    }

    #[test]
    fn full_image_suppresses_generated_appearance() {
        let types = ingest_card_types_csv("game_id,id,name\narcana,creature,Creature").unwrap();
        let cards = ingest_cards_csv("game_id,id,name,card_type_id,full_image\narcana,printed-drake,Printed Drake,creature,/cards/printed.webp", &types).unwrap();
        assert!(matches!(resolve_card_visual(&cards[0], &types[0]).unwrap(), CardVisual::FullImage { .. }));
        assert_eq!(resolve_card_appearance(&cards[0], &types[0]).unwrap(), None);
    }

    #[test]
    fn supports_quoted_commas() {
        let types = ingest_card_types_csv("game_id,id,name,description\narcana,event,Event,\"Resolve once, then discard\"").unwrap();
        assert_eq!(types[0].description.as_deref(), Some("Resolve once, then discard"));
    }
}
