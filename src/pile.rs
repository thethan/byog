use std::env;
use std::path::{Path, PathBuf};

use crate::engine::EngineError;

const DEFAULT_PILES_CSV_PATH: &str = "data/piles.csv";

pub fn piles_csv_path() -> PathBuf {
    env::var("PILES_CSV_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_PILES_CSV_PATH))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pile {
    pub id: String,
    pub name: String,
    pub zone_id: String,
    pub x: u8,
    pub y: u8,
    pub associated_piles: Vec<String>,
}

pub fn load_piles(path: Option<&Path>) -> Result<Vec<Pile>, EngineError> {
    let path = path.map(PathBuf::from).unwrap_or_else(piles_csv_path);
    parse_piles_csv(&std::fs::read_to_string(&path).map_err(EngineError::Io)?)
}

pub fn parse_piles_csv(raw: &str) -> Result<Vec<Pile>, EngineError> {
    let mut reader = csv::Reader::from_reader(raw.as_bytes());
    let headers: std::collections::HashMap<String, usize> = reader
        .headers()
        .map_err(EngineError::Csv)?
        .iter()
        .enumerate()
        .map(|(i, h)| (h.trim().to_ascii_lowercase(), i))
        .collect();

    let id_idx = col(&headers, "id")?;
    let name_idx = col(&headers, "name")?;
    let zone_id_idx = col(&headers, "zone_id")?;
    let x_idx = col(&headers, "x")?;
    let y_idx = col(&headers, "y")?;
    let assoc_idx = headers.get("associated_piles").copied();

    let mut piles = Vec::new();
    for (line_idx, record) in reader.records().enumerate() {
        let record = record.map_err(EngineError::Csv)?;
        let line = line_idx + 2;

        let id = req(&record, id_idx, "id", line)?;
        let name = req(&record, name_idx, "name", line)?;
        let zone_id = req(&record, zone_id_idx, "zone_id", line)?;
        let x = opt_u8(&record, x_idx, "x", line)?;
        let y = opt_u8(&record, y_idx, "y", line)?;

        let associated_piles = assoc_idx
            .and_then(|idx| record.get(idx))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_default();

        piles.push(Pile {
            id,
            name,
            zone_id,
            x,
            y,
            associated_piles,
        });
    }
    Ok(piles)
}

fn col(headers: &std::collections::HashMap<String, usize>, name: &str) -> Result<usize, EngineError> {
    headers
        .get(name)
        .copied()
        .ok_or_else(|| EngineError::Validation(format!("Missing required '{name}' column in piles CSV")))
}

fn req(record: &csv::StringRecord, idx: usize, field: &str, line: usize) -> Result<String, EngineError> {
    record
        .get(idx)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| EngineError::Validation(format!("Missing required '{field}' at piles CSV line {line}")))
}

fn opt_u8(record: &csv::StringRecord, idx: usize, field: &str, line: usize) -> Result<u8, EngineError> {
    let raw = record
        .get(idx)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("0");
    raw.parse::<u8>().map_err(|_| {
        EngineError::Validation(format!("Invalid u8 for '{field}' at piles CSV line {line}: '{raw}'"))
    })
}

#[cfg(test)]
mod tests {
    use super::parse_piles_csv;

    #[test]
    fn parses_piles_csv() {
        let csv = "id,name,zone_id,x,y,associated_piles\n\
                   deck,Deck,deck_zone,0,4,\"main_stack,discard\"\n\
                   hand,Hand,hand,0,28,\n";
        let piles = parse_piles_csv(csv).expect("parse piles");
        assert_eq!(piles.len(), 2);
        assert_eq!(piles[0].id, "deck");
        assert_eq!(piles[0].x, 0);
        assert_eq!(piles[0].y, 4);
        assert_eq!(piles[0].associated_piles, vec!["main_stack", "discard"]);
        assert!(piles[1].associated_piles.is_empty());
    }
}
