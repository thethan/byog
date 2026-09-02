# Card sources

Cards are separated by owner:

- `players/player-1/cards.csv` is Player 1's private deck definition.
- `players/player-2/cards.csv` is Player 2's private deck definition.
- `table/cards.csv` is reserved for cards owned by the shared table rather than a player.

The browser creates a separate `WasmGame` from each player CSV, so hands, decks,
command zones, graveyards, exile zones, and battlefields no longer share one
global card list. Add or remove entries in a player's own file to change only
that player's deck.

Cards default to their front image. To make a card flippable, add a `back_image`
column and place the reverse-face image URL in that card's row. The reverse face
uses the same name, rules text, mana cost, counters, and other attributes as the
front; only the displayed image changes.
