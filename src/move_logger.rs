use std::env;
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use chrono::Utc;
use crate::card::Card;
use crate::engine::EngineError;
use crate::zones::Zone;

const DEFAULT_MOVES_LOG_PATH: &str = "data/moves_log.csv";

pub fn moves_log_path() -> PathBuf {
    env::var("MOVES_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_MOVES_LOG_PATH))
}

#[derive(Debug, Clone)]
pub struct MoveLogEntry {
    pub timestamp: String,
    pub action: String,
    pub card_id: String,
    pub card_name: String,
    pub from_zone: String,
    pub to_zone: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MoveLogger {
    pub path: PathBuf,
}

impl MoveLogger {
    pub fn new(path: Option<&Path>) -> Self {
        Self {
            path: path.map(PathBuf::from).unwrap_or_else(moves_log_path),
        }
    }

    pub fn append_move(
        &self,
        action: &str,
        card: &Card,
        from_zone: Zone,
        to_zone: Zone,
        notes: Option<&str>,
    ) -> Result<MoveLogEntry, EngineError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(EngineError::Io)?;
        }

        let (file, should_write_header) = match OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(&self.path)
        {
            Ok(file) => (file, true),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                let should_write_header =
                    fs::metadata(&self.path).map_err(EngineError::Io)?.len() == 0;
                let file = OpenOptions::new()
                    .append(true)
                    .open(&self.path)
                    .map_err(EngineError::Io)?;
                (file, should_write_header)
            }
            Err(err) => return Err(EngineError::Io(err)),
        };

        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);

        if should_write_header {
            writer
                .write_record([
                    "timestamp",
                    "action",
                    "card_id",
                    "card_name",
                    "from_zone",
                    "to_zone",
                    "notes",
                ])
                .map_err(EngineError::Csv)?;
        }

        let entry = MoveLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            action: action.to_string(),
            card_id: card.id.clone(),
            card_name: card.name.clone(),
            from_zone: from_zone.to_string(),
            to_zone: to_zone.to_string(),
            notes: notes.map(ToString::to_string),
        };

        writer
            .write_record([
                entry.timestamp.as_str(),
                entry.action.as_str(),
                entry.card_id.as_str(),
                entry.card_name.as_str(),
                entry.from_zone.as_str(),
                entry.to_zone.as_str(),
                entry.notes.as_deref().unwrap_or(""),
            ])
            .map_err(EngineError::Csv)?;
        writer.flush().map_err(EngineError::Io)?;

        Ok(entry)
    }
}
