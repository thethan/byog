use std::env;
use std::path::{Path, PathBuf};

use engine::EngineError;
use token_pools::TokenPool;

const DEFAULT_ZONES_CSV_PATH: &str = "data/zones.csv";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZoneScope {
    Game,
    #[default]
    Player,
}

impl ZoneScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Player => "player",
        }
    }

    fn parse(raw: &str, line: usize) -> Result<Self, EngineError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "game" => Ok(Self::Game),
            "player" | "" => Ok(Self::Player),
            value => Err(EngineError::Validation(format!(
                "Invalid zone scope '{value}' at zones CSV line {line}; expected 'game' or 'player'"
            ))),
        }
    }
}

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
    /// Whether the zone is shared by the game or owned separately by each player.
    pub scope: ZoneScope,
    /// Optional containing zone. Coordinates are relative to this zone.
    pub parent_zone: Option<String>,
    /// Grid column origin (0-based)
    pub x: u8,
    /// Grid row origin (0-based)
    pub y: u8,
    /// Number of grid columns spanned
    pub width: u8,
    /// Number of grid rows spanned
    pub height: u8,
    /// Token pools owned by this zone, parsed from the optional CSV column.
    pub token_pools: Vec<TokenPool>,
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
    let token_pools_idx = headers.get("token_pools").copied();
    let scope_idx = headers.get("scope").copied();
    let parent_zone_idx = headers.get("parent_zone").copied();
    let width_idx = headers.get("width").copied();
    let height_idx = headers.get("height").copied();

    let mut zones = Vec::new();
    for (line_idx, record) in reader.records().enumerate() {
        let record = record.map_err(EngineError::Csv)?;
        let line = line_idx + 2;

        let id = req(&record, id_idx, "id", line)?;
        let name = req(&record, name_idx, "name", line)?;
        let color = req(&record, color_idx, "color", line)?;
        let grid_raw = req(&record, grid_idx, "grid", line)?;
        let scope = ZoneScope::parse(
            scope_idx
                .and_then(|idx| record.get(idx))
                .unwrap_or("player"),
            line,
        )?;

        let (x, y, grid_width, grid_height) = parse_grid(&grid_raw, line)?;
        let width = optional_u8(&record, width_idx, "width", line)?.unwrap_or(grid_width);
        let height = optional_u8(&record, height_idx, "height", line)?.unwrap_or(grid_height);
        if width == 0 || height == 0 {
            return Err(EngineError::Validation(format!(
                "Zone width and height must be greater than zero at zones CSV line {line}"
            )));
        }
        let parent_zone = parent_zone_idx
            .and_then(|idx| record.get(idx))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let token_pools = record
            .get(token_pools_idx.unwrap_or(usize::MAX))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(TokenPool::parse_list)
            .transpose()
            .map_err(|message| {
                EngineError::Validation(format!(
                    "Invalid token_pools value at zones CSV line {line}: {message}"
                ))
            })?
            .unwrap_or_default();

        zones.push(ZoneLayout {
            id,
            name,
            color,
            scope,
            parent_zone,
            x,
            y,
            width,
            height,
            token_pools,
        });
    }
    let ids: std::collections::HashSet<_> = zones.iter().map(|zone| zone.id.as_str()).collect();
    for zone in &zones {
        if let Some(parent) = zone.parent_zone.as_deref() {
            if parent == zone.id || !ids.contains(parent) {
                return Err(EngineError::Validation(format!(
                    "Zone '{}' has invalid parent_zone '{}'",
                    zone.id, parent
                )));
            }
        }
    }
    Ok(zones)
}

fn optional_u8(
    record: &csv::StringRecord,
    idx: Option<usize>,
    field: &str,
    line: usize,
) -> Result<Option<u8>, EngineError> {
    let Some(raw) = idx
        .and_then(|idx| record.get(idx))
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    raw.parse::<u8>().map(Some).map_err(|_| {
        EngineError::Validation(format!(
            "Invalid {field} '{raw}' at zones CSV line {line}; expected 0-255"
        ))
    })
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

fn col(
    headers: &std::collections::HashMap<String, usize>,
    name: &str,
) -> Result<usize, EngineError> {
    headers.get(name).copied().ok_or_else(|| {
        EngineError::Validation(format!("Missing required '{name}' column in zones CSV"))
    })
}

fn req(
    record: &csv::StringRecord,
    idx: usize,
    field: &str,
    line: usize,
) -> Result<String, EngineError> {
    record
        .get(idx)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| {
            EngineError::Validation(format!(
                "Missing required '{field}' at zones CSV line {line}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::{ZoneScope, parse_zones_csv};

    #[test]
    fn parses_zones_csv() {
        let csv = "id,name,color,grid,scope,token_pools\n\
                   command_zone,Command Zone,violet,\"0,0.8x4\",player,\n\
                   battlefield,Battlefield,emerald,\"0,8.32x16\",game,\"id=energy|label=Energy|icon=fa-solid fa-bolt|active=true\"\n";
        let zones = parse_zones_csv(csv).expect("parse zones");
        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].id, "command_zone");
        assert_eq!(zones[0].x, 0);
        assert_eq!(zones[0].y, 0);
        assert_eq!(zones[0].width, 8);
        assert_eq!(zones[0].height, 4);
        assert_eq!(zones[0].scope, ZoneScope::Player);
        assert_eq!(zones[0].parent_zone, None);
        assert_eq!(zones[1].scope, ZoneScope::Game);
        assert_eq!(zones[1].width, 32);
        assert_eq!(zones[1].height, 16);
        assert_eq!(zones[1].token_pools[0].id, "energy");
        assert_eq!(zones[1].token_pools[0].token(), "fa-solid fa-bolt");
    }

    #[test]
    fn parses_parent_zone_and_explicit_width() {
        let csv = "id,name,color,grid,scope,parent_zone,width\n\
                   main,Main,emerald,\"0,0.32x8\",player,,32\n\
                   lands,Lands,lime,\"0,0.8x8\",player,main,12\n";
        let zones = parse_zones_csv(csv).expect("parse nested zones");
        assert_eq!(zones[1].parent_zone.as_deref(), Some("main"));
        assert_eq!(zones[1].width, 12);
    }

    #[test]
    fn rejects_invalid_grid_notation() {
        let csv = "id,name,color,grid\nbad,Bad,red,invalid\n";
        assert!(parse_zones_csv(csv).is_err());
    }

    #[test]
    fn defaults_missing_scope_to_player_and_rejects_unknown_scope() {
        let legacy = "id,name,color,grid\nhand,Hand,cyan,\"0,0.8x4\"\n";
        assert_eq!(parse_zones_csv(legacy).unwrap()[0].scope, ZoneScope::Player);

        let invalid = "id,name,color,grid,scope\nhand,Hand,cyan,\"0,0.8x4\",table\n";
        assert!(parse_zones_csv(invalid).is_err());
    }
}
