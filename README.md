# byog

A Rust card/deck engine that loads MTG-like card data from a local CSV and logs all moves to a local CSV file.

## Card CSV schema

Default path: `data/cards.csv` (override with `CARDS_CSV_PATH`).

Required headers:
- `id`
- `name`
- `card_type` (or `type`)

Optional headers/values:
- `mana` (resource names can match token type IDs from `data/token_types.csv`)
- `colors`
- `oracle_text`
- `power`
- `toughness`
- `is_commander` (defaults to `false`)
- `is_partner` (defaults to `false`)
- `token_pools`

Example row:

```csv
id,name,card_type,mana,colors,oracle_text,power,toughness,is_commander,is_partner,token_pools
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

## Dice helpers

The library also exposes dice roll helpers:

- `roll_die(sides)` for a single die
- `roll_dice(count, sides)` for individual results
- `roll_dice_total(count, sides)` for a summed total

### Icon-based dice sides

When a die side is represented by an icon, the icon value must be a valid
[Font Awesome](https://fontawesome.com/) class name.

**Accepted formats**

| Form | Example |
|------|---------|
| Single token | `fa-skull` |
| Single token | `fa-dice-d20` |
| Style + icon | `fa-solid fa-skull` |
| Style + icon | `fa-regular fa-star` |
| Style + icon | `fa-brands fa-github` |

Rules:
- Must start with `fa-` followed by one or more lowercase letters, digits, or hyphens.
- Optional style prefix must be one of `fa-solid`, `fa-regular`, or `fa-brands`.
- No uppercase letters, underscores, or other icon systems (e.g. `mdi-*`).

**Invalid examples**: `""`, `Fa-Skull`, `mdi-sword-cross`, `fa_heart`, `fa-duotone fa-skull`

Use `validate_fa_icon(icon)` to check an icon string; invalid values return
`EngineError::Validation`.

Token types are defined in `data/token_types.csv`, similarly to card types. A `Token`
references a token type and may optionally reference a related card.

Token pools are defined and assigned in `data/token_pools.csv`. Each pool has a
`starting`, `plus`, and `minus` amount, a required Font Awesome icon inherited
from its token type, and an owner: `player`, `card`, `creature`, `zone`, or
`battlefield`. The optional `parent_id` propagates every increase or decrease to
the parent pool atomically.

Legacy inline token pools remain supported for cards and zones.

Add the same optional `token_pools` column to `data/zones.csv` to configure a zone-owned pool. Zone and card pools use the format documented in the Card CSV schema above, including full Font Awesome classes in `icon`/`token` (for example `fa-solid fa-bolt`).

Zones may also include a `scope` column. Use `player` for a separate copy owned by each player (such as a hand or deck), or `game` for a zone shared by everyone (such as a stack). The default is `player` when the column or value is omitted. Any other value is rejected during loading.

Every new game also creates `player-1` with an active `life` token pool starting at 20. Player pools can be changed through `add_tokens_to_player_pool` and `remove_tokens_from_player_pool`.

- Zone pools are managed on `GameState` / `CardEngine` with `set_zone_token_pools`, `add_zone_token_pool`, `activate_zone_token_pool`, `add_tokens_to_zone_pool`, and `remove_tokens_from_zone_pool`.
- Card pools are loaded from `token_pools` CSV data or added later with `add_card_token_pool`.
- Pools expose `token()` for the display token/icon and `background()` for UI styling.
- Token icons use Font Awesome classes (for example `fa-solid fa-heart`); `background` selects the card-like colored surface behind the icon and count.
- `min` and `max` bounds are validated when pools are created and whenever token counts change.

## Move log format

Moves are appended to `data/moves_log.csv` (or `MOVES_LOG_PATH`) with header:

```csv
timestamp,action,card_id,card_name,from_pile,to_pile,notes
```

Sample move row:

```csv
2026-08-18T15:00:00Z,draw,land1,Plains,MainStack,Hand,
```

## Web app (Tailwind + WebAssembly)

The repository now includes a playable browser app with a Tailwind layout for every zone:

- MainStack
- CommanderPile
- Hand
- LandPile
- Deck
- Discard
- Exile
- ArtifactList
- EnchantmentList
- CreatureList

The WebAssembly interface returns game-state snapshots as Protocol Buffer bytes,
and the frontend decodes those payloads before rendering.

### Build Rust to WebAssembly

```bash
cargo install wasm-pack
wasm-pack build --target web --out-dir pkg
```

### Run the app

Serve the repository root with any static server and open `/web/`:

```bash
python -m http.server 8000
```

Then visit `http://localhost:8000/web/`.

### Move control

- **Move selected**: choose a source pile, select a matching card, and move it to a target pile

Piles are defined in `data/piles.csv`; the optional `deck_id` column associates each pile with a deck. Set the optional `visible` column to `false` to hide card identities (for example, a deck). Hidden piles show their count and can be shuffled, move or temporarily reveal any number of cards selected from the top or at random. They also support explicit searches by card name, type, or ID. `visible` defaults to `true`, and every deck can be selected as a search source. Native runs append every card move to `data/moves_log.csv`. Browser runs keep the same audit in memory.

Card counter bounds use the existing `token_pools` syntax with `id=counters`, for example `id=counters|label=Counters|token=fa-solid fa-plus|starting=1|min=1|max=5|active=true`. Cards without an explicit counters pool default to a minimum of zero and no maximum.
