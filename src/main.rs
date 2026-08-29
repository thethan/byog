extern crate byog;

use byog::{cards_csv_path, load_cards, moves_log_path, CardEngine, EngineError, TokenPool, Zone};

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
    let energy_pool = TokenPool::configured(
        "energy",
        "Energy",
        "fa-bolt",
        Some("slate".to_string()),
        1,
        Some(0),
        Some(5),
        true,
    )
    .map_err(EngineError::Validation)?;
    engine.set_zone_token_pools(Zone::Hand, vec![energy_pool])?;

    let mut main_stack = Vec::new();

    for card in cards {

            main_stack.push(card.id.clone());

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
        println!(
            "Drew card: {} (type: {})",
            draw_entry.card_id, card_type
        );
    }

    if !engine.state.zone_cards(Zone::MainStack).is_empty() {
        let second_draw = engine.draw()?;
        engine.discard(Zone::Hand, &second_draw.card_id)?;
    }

    engine.add_tokens_to_zone_pool(Zone::Hand, "energy", 1)?;

    println!("Loaded cards from: {}", cards_path.display());
    println!("Move log written to: {}", move_log_path.display());
    println!(
        "Hand energy token: {} / background: {}",
        engine.state.zone_token_pool_icon(Zone::Hand, "energy")?,
        engine
            .state
            .zone_token_pool_background(Zone::Hand, "energy")?
            .unwrap_or("none")
    );
    Ok(())
}
