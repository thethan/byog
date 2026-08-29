use crate::card::{Card, CardType};
use crate::engine::EngineError;
use crate::token_pool::TokenPool;
use std::collections::HashMap;
use std::env;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use Pile;

const DEFAULT_CARDS_CSV_PATH: &str = "data/cards.csv";
const DEFAULT_PILES_CSV_PATH: &str = "data/piles.csv";

pub fn cards_csv_path() -> PathBuf {
    env::var("CARDS_CSV_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_CARDS_CSV_PATH))
}

pub fn piles_csv_path() -> PathBuf {
    env::var("PILES_CSV_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_PILES_CSV_PATH))
}

pub fn load_cards(path: Option<&Path>) -> Result<Vec<Card>, EngineError> {
    let path = path.map(PathBuf::from).unwrap_or_else(cards_csv_path);

    let mut reader = csv::Reader::from_path(&path).map_err(EngineError::Csv)?;
    let headers = reader
        .headers()
        .map_err(EngineError::Csv)?
        .iter()
        .enumerate()
        .map(|(idx, h)| (h.trim().to_ascii_lowercase(), idx))
        .collect::<HashMap<_, _>>();

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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::load_cards;

    #[test]
    fn parses_cards_and_tolerates_missing_optional_columns() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cards_test_{unique}.csv"));
        let csv = "id,name,card_type,is_commander,token_pools\n1,Sol Ring,Artifact,true,\"id=charge|token=fa-bolt|background=amber|starting=1|max=3|active=true\"\n2,Island,Land,,\n";
        fs::write(&path, csv).expect("write temp csv");

        let cards = load_cards(Some(&path)).expect("load cards");
        fs::remove_file(path).ok();

        assert_eq!(cards.len(), 2);
        assert_eq!(cards[0].id, "1");
        assert_eq!(cards[0].name, "Sol Ring");
        assert_eq!(cards[0].token_pools[0].token(), "fa-bolt");
        assert_eq!(cards[1].mana_cost, None);
        assert_eq!(cards[1].oracle_text, None);
        assert!(cards[1].token_pools.is_empty());
    }
}
