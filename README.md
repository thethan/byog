# byog

A Rust card/deck engine that loads MTG-like card data from a local CSV and logs all moves to a local CSV file.

## Card CSV schema

Default path: `data/cards.csv` (override with `CARDS_CSV_PATH`).

Required headers:
- `id`
- `name`
- `card_type` (or `type`)

Optional headers/values:
- `mana_cost`
- `colors`
- `oracle_text`
- `power`
- `toughness`
- `is_commander` (defaults to `false`)
- `is_partner` (defaults to `false`)

Example row:

```csv
id,name,card_type,mana_cost,colors,oracle_text,power,toughness,is_commander,is_partner
art1,Sol Ring,Artifact,1,,{T}: Add {C}{C}.,,,false,false
```

## Environment variables

- `CARDS_CSV_PATH`: path to local card CSV (default `data/cards.csv`)
- `MOVES_LOG_PATH`: path to moves log CSV (default `data/moves_log.csv`)

## Run demo

```bash
cargo run
```

The demo:
- loads cards from CSV
- initializes `MainStack`, `CommanderPile`, and other zones
- performs sample actions (`draw`, `play_land` or `cast_to_battlefield`, `discard`)
- appends move rows to the move log CSV

## Move log format

Moves are appended to `data/moves_log.csv` (or `MOVES_LOG_PATH`) with header:

```csv
timestamp,action,card_id,card_name,from_zone,to_zone,notes
```

Sample move row:

```csv
2026-08-18T15:00:00Z,draw,land1,Plains,MainStack,Hand,
```
