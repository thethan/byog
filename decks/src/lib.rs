use std::io;
use std::path::{Path, PathBuf};
use cards::cards::Card;

struct Deck {
    cards: Vec<Card>
}

struct Pile {
    name: String,
    cards: Vec<String>, // TODO Store card IDs instead of Card objects
}


// todo replace E with EngineError
pub fn load_piles(path: Option<&Path>) -> Result<Vec<Pile>, io::Error> {
    let mut piles: Vec<Pile> = Vec::new();
    // let mut reader = csv::Reader::from_path(&path).map_err(EngineError::Csv)?;
    // // Placeholder for future implementation
    // let path = path.map(PathBuf::from).unwrap_or_else(cards_csv_path);
    //
    //
    // for (line_idx, record) in reader.records().enumerate() {
    //     let record = record.map_err(EngineError::Csv)?;
    //
    //     let id = required_value(&record, 0, "id", line_idx + 1)?;
    //     let name = required_value(&record, 0, "name", line_idx + 1)?;
    //
    //     piles.push(Pile { id, name });
    // }

    Ok(piles)
}
