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
- `token_pools`

Example row:

```csv
id,name,card_type,mana_cost,colors,oracle_text,power,toughness,is_commander,is_partner,token_pools
art1,Sol Ring,Artifact,1,,{T}: Add {C}{C}.,,,false,false,"id=charge|label=Charge|token=fa-bolt|background=amber|starting=1|min=0|max=3|active=true"
```

`token_pools` accepts one or more semicolon-separated pools. Each pool uses `|`-separated `key=value` fields:

- `id` (required)
- `label` (optional, defaults to `id`)
- `token` or `icon` (optional, defaults to `label`; use a Font Awesome token such as `fa-bolt`)
- `background` (optional)
- `starting` or `count` (optional, defaults to `0`)
- `min` / `max` (optional)
- `active` (optional, defaults to `false`)

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
- seeds a startup zone token pool on `Hand`
- performs sample actions (`draw`, `play_land` or `cast_to_battlefield`, `discard`)
- adds a token to the `Hand` pool at runtime
- appends move rows to the move log CSV

## Token pools

Token pools can belong to zones or to individual cards.

- Zone pools are managed on `GameState` / `CardEngine` with `set_zone_token_pools`, `add_zone_token_pool`, `activate_zone_token_pool`, and `add_tokens_to_zone_pool`.
- Card pools are loaded from `token_pools` CSV data or added later with `add_card_token_pool`.
- Pools expose `token()` for the display token/icon and `background()` for UI styling.
- `min` and `max` bounds are validated when pools are created and when tokens are added.

## Move log format

Moves are appended to `data/moves_log.csv` (or `MOVES_LOG_PATH`) with header:

```csv
timestamp,action,card_id,card_name,from_zone,to_zone,notes
```

Sample move row:

```csv
2026-08-18T15:00:00Z,draw,land1,Plains,MainStack,Hand,
```
