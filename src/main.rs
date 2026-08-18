use std::path::PathBuf;

use byog::{CardEngine, CardType, EngineError, Zone, cards_csv_path, load_cards, moves_log_path};

fn main() {
    if let Err(err) = run_demo() {
        eprintln!("Demo failed: {err}");
        std::process::exit(1);
    }
}

fn run_demo() -> Result<(), EngineError> {
    let cards_path = cards_csv_path();
    let move_log_path = moves_log_path();

    let cards = load_cards(Some(&cards_path))?;
    if cards.is_empty() {
        return Err(EngineError::Validation(format!(
            "No cards loaded from {}",
            cards_path.display()
        )));
    }

    let mut engine = CardEngine::new(cards.clone(), Some(&move_log_path));

    let mut commander_ids = Vec::new();
    let mut main_stack = Vec::new();

    for card in cards {
        if card.is_commander && commander_ids.len() < 2 {
            commander_ids.push(card.id.clone());
        } else {
            main_stack.push(card.id.clone());
        }
    }

    if !commander_ids.is_empty() {
        engine
            .state
            .set_zone_cards(Zone::CommanderPile, commander_ids)?;
    }

    engine.state.set_zone_cards(Zone::MainStack, main_stack)?;

    let _peek = engine.peek_main_stack(2);

    let draw_entry = engine.draw()?;

    if let Some(card_type) = engine
        .state
        .card_by_id(&draw_entry.card_id)
        .ok()
        .map(|card| card.card_type.clone())
    {
        match card_type {
            CardType::Land => {
                engine.play_land(&draw_entry.card_id)?;
            }
            CardType::Artifact | CardType::Enchantment | CardType::Creature => {
                engine.cast_to_battlefield(&draw_entry.card_id)?;
            }
            _ => {
                engine.discard(Zone::Hand, &draw_entry.card_id)?;
            }
        }
    }

    if !engine.state.zone_cards(Zone::MainStack).is_empty() {
        let second_draw = engine.draw()?;
        engine.discard(Zone::Hand, &second_draw.card_id)?;
    }

    let confirmed: PathBuf = move_log_path;
    println!("Loaded cards from: {}", cards_path.display());
    println!("Move log written to: {}", confirmed.display());
    Ok(())
}
