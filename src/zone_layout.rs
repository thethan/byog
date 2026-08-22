use std::env;
use std::path::{Path, PathBuf};

use crate::engine::EngineError;

const DEFAULT_ZONES_CSV_PATH: &str = "data/zones.csv";

pub fn zones_csv_path() -> PathBuf {
    env::var("ZONES_CSV_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_ZONES_CSV_PATH))
}

/// Layout of a named zone region on the 32×32 game board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZoneLayout {
    pub id: String,
    pub name: String,
    pub color: String,
    /// Grid column origin (0-based)
    pub x: u8,
    /// Grid row origin (0-based)
    pub y: u8,
    /// Number of grid columns spanned
    pub width: u8,
    /// Number of grid rows spanned
    pub height: u8,
}

pub fn load_zones(path: Option<&Path>) -> Result<Vec<ZoneLayout>, EngineError> {
    let path = path.map(PathBuf::from).unwrap_or_else(zones_csv_path);
    parse_zones_csv(&std::fs::read_to_string(&path).map_err(EngineError::Io)?)
}

/// Parse zones CSV. The `grid` column uses the notation `x,y.WxH`
/// e.g. `0,0.8x4` means origin (0,0), width 8, height 4.
pub fn parse_zones_csv(raw: &str) -> Result<Vec<ZoneLayout>, EngineError> {
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
    let color_idx = col(&headers, "color")?;
    let grid_idx = col(&headers, "grid")?;

    let mut zones = Vec::new();
    for (line_idx, record) in reader.records().enumerate() {
        let record = record.map_err(EngineError::Csv)?;
        let line = line_idx + 2;

        let id = req(&record, id_idx, "id", line)?;
        let name = req(&record, name_idx, "name", line)?;
        let color = req(&record, color_idx, "color", line)?;
        let grid_raw = req(&record, grid_idx, "grid", line)?;

        let (x, y, width, height) = parse_grid(&grid_raw, line)?;

        zones.push(ZoneLayout { id, name, color, x, y, width, height });
    }
    Ok(zones)
}

/// Parse `x,y.WxH` grid notation.
fn parse_grid(raw: &str, line: usize) -> Result<(u8, u8, u8, u8), EngineError> {
    let err = || {
        EngineError::Validation(format!(
            "Invalid grid notation '{raw}' at zones CSV line {line}; expected x,y.WxH"
        ))
    };

    let dot = raw.find('.').ok_or_else(err)?;
    let origin = &raw[..dot];
    let size = &raw[dot + 1..];

    let (xs, ys) = origin.split_once(',').ok_or_else(err)?;
    let (ws, hs) = size.split_once('x').ok_or_else(err)?;

    let parse_u8 = |s: &str| s.trim().parse::<u8>().map_err(|_| err());

    Ok((parse_u8(xs)?, parse_u8(ys)?, parse_u8(ws)?, parse_u8(hs)?))
}

fn col(headers: &std::collections::HashMap<String, usize>, name: &str) -> Result<usize, EngineError> {
    headers
        .get(name)
        .copied()
        .ok_or_else(|| EngineError::Validation(format!("Missing required '{name}' column in zones CSV")))
}

fn req(record: &csv::StringRecord, idx: usize, field: &str, line: usize) -> Result<String, EngineError> {
    record
        .get(idx)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| EngineError::Validation(format!("Missing required '{field}' at zones CSV line {line}")))
}

#[cfg(test)]
mod tests {
    use super::parse_zones_csv;

    #[test]
    fn parses_zones_csv() {
        let csv = "id,name,color,grid\n\
                   command_zone,Command Zone,violet,\"0,0.8x4\"\n\
                   battlefield,Battlefield,emerald,\"0,8.32x16\"\n";
        let zones = parse_zones_csv(csv).expect("parse zones");
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].id, "command_zone");
        assert_eq!(zones[0].x, 0);
        assert_eq!(zones[0].y, 0);
        assert_eq!(zones[0].width, 8);
        assert_eq!(zones[0].height, 4);
        assert_eq!(zones[1].width, 32);
        assert_eq!(zones[1].height, 16);
    }

    #[test]
    fn rejects_invalid_grid_notation() {
        let csv = "id,name,color,grid\nbad,Bad,red,invalid\n";
        assert!(parse_zones_csv(csv).is_err());
    }
}
