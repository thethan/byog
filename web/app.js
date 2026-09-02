import init, { WasmGame } from "../pkg/byog.js?v=25";
import protobuf from "https://cdn.jsdelivr.net/npm/protobufjs@7.4.0/+esm";

const statusEl = document.getElementById("status");
const boardEl = document.getElementById("board");
const handAreaEl = document.getElementById("hand-area");
const handSummaryCount = document.getElementById("hand-summary-count");
const handOwnerLabel = document.getElementById("hand-owner-label");
const activePlayerLabel = document.getElementById("active-player-label");
const tokenPoolPlayerSelect = document.getElementById("token-pool-player-select");
const zonesOwnerLabel = document.getElementById("zones-owner-label");
const zonesOwnerDescription = document.getElementById("zones-owner-description");
const opponentHandLabel = document.getElementById("opponent-hand-label");
const opponentHandCount = document.getElementById("opponent-hand-count");
const opponentHandArea = document.getElementById("opponent-hand-area");
const showOpponentHand = document.getElementById("show-opponent-hand");
const collapseOpponentHands = document.getElementById("collapse-opponent-hands");
const playerSelect = document.getElementById("player-select");
const gameSelect = document.getElementById("game-select");
const newGameButton = document.getElementById("new-game");
const deleteGameButton = document.getElementById("delete-game");
const addPlayerButton = document.getElementById("add-player");
const renamePlayerButton = document.getElementById("rename-player");
const playerBoardPanel = document.getElementById("player-board-panel");
const sourcePileEl = document.getElementById("source-pile");
const targetPileEl = document.getElementById("target-pile");
const searchEl = document.getElementById("card-search");
const moveBtn = document.getElementById("move-btn");
const selectionEl = document.getElementById("selection");
const moveMenuBtn = document.getElementById("move-menu-btn");
const moveMenu = document.getElementById("move-menu");
const moveChevron = document.getElementById("move-chevron");
const cardModal = document.getElementById("card-modal");
const cardModalClose = document.getElementById("card-modal-close");
const cardModalPrevious = document.getElementById("card-modal-previous");
const cardModalNext = document.getElementById("card-modal-next");
const cardModalPile = document.getElementById("card-modal-pile");
const cardModalTitle = document.getElementById("card-modal-title");
const cardViewName = document.getElementById("card-view-name");
const cardViewType = document.getElementById("card-view-type");
const cardViewId = document.getElementById("card-view-id");
const cardView = document.getElementById("card-view");
const cardModalTap = document.getElementById("card-modal-tap");
const cardModalFlip = document.getElementById("card-modal-flip");
const cardModalBottom = document.getElementById("card-modal-bottom");
const cardCounterRemove = document.getElementById("card-counter-remove");
const cardCounterCount = document.getElementById("card-counter-count");
const cardCounterAdd = document.getElementById("card-counter-add");
const cardModalTarget = document.getElementById("card-modal-target");
const cardModalMove = document.getElementById("card-modal-move");
const opponentModal = document.getElementById("opponent-modal");
const opponentModalTitle = document.getElementById("opponent-modal-title");
const opponentModalClose = document.getElementById("opponent-modal-close");
const opponentTokenPools = document.getElementById("opponent-token-pools");
const opponentRename = document.getElementById("opponent-rename");
const undoBtn = document.getElementById("undo-btn");
const changeHistoryEl = document.getElementById("change-history");
const turnLabel = document.getElementById("turn-label");
const endTurnButton = document.getElementById("end-turn");
const createCardButton = document.getElementById("create-card-button");
const createCardDialog = document.getElementById("create-card-dialog");
const createCardForm = document.getElementById("create-card-form");
const createCardClose = document.getElementById("create-card-close");
const createCardCancel = document.getElementById("create-card-cancel");
const createCardName = document.getElementById("create-card-name");
const createCardType = document.getElementById("create-card-type");
const createCardPile = document.getElementById("create-card-pile");
const createCardOffense = document.getElementById("create-card-offense");
const createCardDefense = document.getElementById("create-card-defense");
const createCardText = document.getElementById("create-card-text");

// ── Colour mappings ───────────────────────────────────────────────────────────
const zoneColorMap = {
  indigo: "border-indigo-300", violet: "border-violet-300", cyan: "border-cyan-300",
  lime: "border-lime-300", sky: "border-sky-300", rose: "border-rose-300",
  orange: "border-orange-300", amber: "border-amber-300",
  fuchsia: "border-fuchsia-300", emerald: "border-emerald-300",
};

// ── Protobuf schema ───────────────────────────────────────────────────────────
const protoSchema = `
syntax = "proto3";

message GameStateSnapshotProto {
  repeated ZoneViewProto zones = 1;
  repeated PlayerViewProto players = 2;
}
message PlayerViewProto {
  string id = 1;
  string name = 2;
  repeated TokenPoolViewProto token_pools = 3;
}
message ZoneViewProto {
  string id = 1;
  bool battlefield = 2;
  repeated CardViewProto cards = 3;
  repeated TokenPoolViewProto token_pools = 4;
}
message CardViewProto {
  string id = 1;
  string name = 2;
  string card_type = 3;
  repeated TokenPoolViewProto token_pools = 4;
  optional string mana = 5;
  optional string oracle_text = 6;
  optional string image = 7;
  optional string background_image = 8;
  optional string colors = 9;
  optional string power = 10;
  optional string toughness = 11;
  optional string back_image = 12;
}
message TokenPoolViewProto {
  string id = 1;
  string label = 2;
  string token = 3;
  optional string background = 4;
  uint32 count = 5;
  bool active = 6;
  optional uint32 min = 7;
  optional uint32 max = 8;
  uint32 plus = 9;
  uint32 minus = 10;
  uint32 starting = 11;
  optional string parent_id = 12;
  optional string icon_color = 13;
}

message BoardLayoutProto {
  repeated ZoneLayoutProto zones = 1;
  repeated PileViewProto piles = 2;
}
message ZoneLayoutProto {
  string id = 1;
  string name = 2;
  string color = 3;
  uint32 x = 4;
  uint32 y = 5;
  uint32 width = 6;
  uint32 height = 7;
  string scope = 8;
  optional string parent_zone = 9;
  repeated string allowed_card_types = 10;
  optional uint32 max_cards = 11;
}
message PileViewProto {
  string id = 1;
  string name = 2;
  string zone_id = 3;
  uint32 x = 4;
  uint32 y = 5;
  repeated string associated_piles = 6;
  bool visible = 7;
  optional string role = 8;
}
`;

const root = protobuf.parse(protoSchema, { keepCase: true }).root;
const GameStateSnapshotProto = root.lookupType("GameStateSnapshotProto");
const BoardLayoutProto = root.lookupType("BoardLayoutProto");

// ── State ─────────────────────────────────────────────────────────────────────
let game;
let games = [];
let playerStates = [];
const defaultPlayerDecks = [
  { name: "Player 1", url: "../data/players/player-1/cards.csv" },
  { name: "Player 2", url: "../data/players/player-2/cards.csv" },
];
let playerDecks = [...defaultPlayerDecks];
let gameInstances = [];
let activeGameInstanceId = null;
let activePlayerIndex = 0;
let turnState = null;
let boardLayout = null;   // { zones: ZoneLayoutProto[], piles: PileViewProto[] }
let defaultDeckCsvs = [];
let currentState = null;
let selectedCard = null;
let modalCard = null;
let modalOpponentIndex = null;
let tokenPoolPlayerIndex = 0;
let activePileIndex = -1;
const revealedPileCards = new Map();
const savedStatePrefix = "byog.game-state.v1";
const changeHistoryPrefix = "byog.change-history.v1";
const maxUndoChanges = 20;
const gameInstancesKey = "byog.game-instances.v1";
const activeGameInstanceKey = "byog.active-game-instance.v1";
const turnStatePrefix = "byog.turn-state.v1";
const customCardsPrefix = "byog.custom-cards.v1";
// pile id → panel element
const pilePanels = new Map();
const parentZoneSearches = new Map();

// ── Decode helpers ────────────────────────────────────────────────────────────
function decodeState(bytes) {
  return GameStateSnapshotProto.toObject(GameStateSnapshotProto.decode(bytes), { defaults: true });
}
function decodeBoardLayout(bytes) {
  return BoardLayoutProto.toObject(BoardLayoutProto.decode(bytes), { defaults: true });
}

function savedStateKey(index) {
  return `${savedStatePrefix}:${activeGameInstanceId}:player-${index + 1}`;
}

function changeHistoryKey(index) {
  return `${changeHistoryPrefix}:${activeGameInstanceId}:player-${index + 1}`;
}

function turnStateKey() {
  return `${turnStatePrefix}:${activeGameInstanceId}`;
}

function customCardsKey(index) {
  return `${customCardsPrefix}:${activeGameInstanceId}:player-${index + 1}`;
}

function readCustomCards(index) {
  try {
    const cards = JSON.parse(localStorage.getItem(customCardsKey(index)) || "[]");
    return Array.isArray(cards) ? cards : [];
  } catch (err) {
    console.warn("Could not read custom cards", err);
    return [];
  }
}

function writeCustomCards(index, cards) {
  localStorage.setItem(customCardsKey(index), JSON.stringify(cards));
}

function csvField(value) {
  const valueText = String(value ?? "");
  return /[",\r\n]/.test(valueText) ? `"${valueText.replaceAll('"', '""')}"` : valueText;
}

function withCustomCards(csv, index) {
  const rows = readCustomCards(index).map((card) => [
    card.id, card.name, card.cardType, "", "", card.text, card.offense, card.defense,
    "false", "false", "", card.pileId, "", "", "", "",
  ].map(csvField).join(","));
  return rows.length ? `${csv.trimEnd()}\n${rows.join("\n")}\n` : csv;
}

function readTurnState() {
  try {
    const saved = JSON.parse(localStorage.getItem(turnStateKey()) || "null");
    if (saved && Number.isInteger(saved.number) && Number.isInteger(saved.playerIndex)) return saved;
  } catch (err) {
    console.warn("Could not read turn state", err);
  }
  return { number: 1, playerIndex: 0, startedAt: new Date().toISOString() };
}

function saveTurnState() {
  localStorage.setItem(turnStateKey(), JSON.stringify(turnState));
}

function makeGameInstance(name) {
  const id = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`;
  return { id, name, playerCount: defaultPlayerDecks.length, createdAt: new Date().toISOString() };
}

function readGameInstances() {
  try {
    const parsed = JSON.parse(localStorage.getItem(gameInstancesKey) || "[]");
    if (Array.isArray(parsed) && parsed.length) return parsed;
  } catch (err) {
    console.warn("Could not read game instances", err);
  }
  const first = makeGameInstance("Game 1");
  localStorage.setItem(gameInstancesKey, JSON.stringify([first]));
  return [first];
}

function saveGameInstances() {
  localStorage.setItem(gameInstancesKey, JSON.stringify(gameInstances));
  localStorage.setItem(activeGameInstanceKey, activeGameInstanceId);
}

function renderGameInstances() {
  gameSelect.innerHTML = "";
  for (const instance of gameInstances) {
    const option = document.createElement("option");
    option.value = instance.id;
    option.textContent = instance.name;
    gameSelect.appendChild(option);
  }
  gameSelect.value = activeGameInstanceId;
}

function bytesToBase64(bytes) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function base64ToBytes(value) {
  const binary = atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

function saveGameState(index) {
  try {
    localStorage.setItem(savedStateKey(index), bytesToBase64(games[index].state_proto()));
  } catch (err) {
    console.warn("Could not save game state", err);
  }
}

function readChangeHistory(index) {
  try {
    const parsed = JSON.parse(localStorage.getItem(changeHistoryKey(index)) || "[]");
    return Array.isArray(parsed) ? parsed.filter((entry) => entry && typeof entry.message === "string") : [];
  } catch (err) {
    console.warn("Could not read change history", err);
    return [];
  }
}

function writeChangeHistory(index, history) {
  localStorage.setItem(changeHistoryKey(index), JSON.stringify(history));
}

function recordChange(index, message, before, after) {
  const entry = {
    id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
    timestamp: new Date().toISOString(),
    message,
    turnNumber: turnState?.number ?? 1,
    playerIndex: turnState?.playerIndex ?? index,
    playerName: playerName(turnState?.playerIndex ?? index),
    before: bytesToBase64(before),
    undone: false,
  };
  let history = [...readChangeHistory(index), entry];
  const undoSnapshots = history.map((item, itemIndex) => item.before ? itemIndex : -1).filter((itemIndex) => itemIndex >= 0);
  for (const itemIndex of undoSnapshots.slice(0, -maxUndoChanges)) history[itemIndex].before = null;
  try {
    writeChangeHistory(index, history);
    localStorage.setItem(savedStateKey(index), bytesToBase64(after));
  } catch (err) {
    // Storage quotas vary. Retain the newest changes and retry with older undo
    // snapshots removed before giving up on persistence entirely.
    let snapshotIndex = history.findIndex((item) => item.before);
    while (snapshotIndex >= 0 && snapshotIndex < history.length - 1) {
      history[snapshotIndex].before = null;
      try {
        writeChangeHistory(index, history);
        localStorage.setItem(savedStateKey(index), bytesToBase64(after));
        renderChangeHistory();
        return;
      } catch (_) {}
      snapshotIndex = history.findIndex((item) => item.before);
    }
    console.warn("Could not save change history", err);
  }
  renderChangeHistory();
}

function renderChangeHistory() {
  if (!changeHistoryEl || !undoBtn) return;
  const history = games.flatMap((_, index) => readChangeHistory(index).map((entry) => ({ ...entry, playerIndex: entry.playerIndex ?? index })));
  history.sort((a, b) => new Date(a.timestamp) - new Date(b.timestamp));
  changeHistoryEl.innerHTML = "";
  const activeHistory = readChangeHistory(activePlayerIndex);
  undoBtn.disabled = !activeHistory.some((entry) => entry.before && !entry.undone && (entry.turnNumber ?? turnState?.number) === turnState?.number);
  if (!history.length) {
    const empty = document.createElement("li");
    empty.className = "italic text-gray-400";
    empty.textContent = "No saved changes yet.";
    changeHistoryEl.appendChild(empty);
    return;
  }
  const groups = new Map();
  for (const entry of history) {
    const number = entry.turnNumber ?? 0;
    if (!groups.has(number)) groups.set(number, []);
    groups.get(number).push(entry);
  }
  for (const [number, entries] of [...groups.entries()].reverse()) {
    const group = document.createElement("li");
    group.className = "rounded-lg border border-gray-200 bg-white p-2.5";
    const heading = document.createElement("p");
    heading.className = "mb-2 font-semibold text-gray-800";
    const owner = entries[0].playerName || playerName(entries[0].playerIndex);
    heading.textContent = number ? `Turn ${number} · ${owner}` : "Earlier activity";
    const list = document.createElement("ol");
    list.className = "space-y-1.5 border-l-2 border-emerald-100 pl-2";
    for (const entry of entries) {
      const item = document.createElement("li");
      item.className = entry.undone ? "text-gray-400 line-through" : "text-gray-700";
      item.textContent = entry.message;
      list.appendChild(item);
    }
    group.append(heading, list);
    changeHistoryEl.appendChild(group);
  }
}

function undoLastChange() {
  const history = readChangeHistory(activePlayerIndex);
  const entryIndex = history.findLastIndex((entry) => entry.before && !entry.undone && (entry.turnNumber ?? turnState?.number) === turnState?.number);
  if (entryIndex < 0) return;
  try {
    const nextBytes = games[activePlayerIndex].restore_state_proto(base64ToBytes(history[entryIndex].before));
    history[entryIndex].undone = true;
    history[entryIndex].undoneAt = new Date().toISOString();
    writeChangeHistory(activePlayerIndex, history);
    saveGameState(activePlayerIndex);
    playerStates[activePlayerIndex] = decodeState(nextBytes);
    selectedCard = null;
    revealedPileCards.clear();
    renderPlayerTabs();
    renderState(playerStates[activePlayerIndex]);
    renderChangeHistory();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Undid: ${history[entryIndex].message}`;
  } catch (err) {
    console.error("[UNDO ERROR]", err);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Undo failed: ${err}`;
  }
}

function restoreGameState(playerGame, index) {
  try {
    const saved = localStorage.getItem(savedStateKey(index));
    if (!saved) return;
    playerGame.restore_state_proto(base64ToBytes(saved));
  } catch (err) {
    console.warn("Ignoring incompatible saved game state", err);
    try { localStorage.removeItem(savedStateKey(index)); } catch (_) {}
  }
}

function pileName(pileId) {
  return boardLayout?.piles.find((pile) => pile.id === pileId)?.name ?? pileId;
}

function playerName(index) {
  return playerStates[index]?.players[0]?.name || `Player ${index + 1}`;
}

function cardPool(card, poolId) {
  return card.token_pools?.find((pool) => pool.id === poolId);
}

function isCardTapped(card) {
  return Boolean(cardPool(card, "tapped")?.active);
}

function isCardFlipped(card) {
  return Boolean(cardPool(card, "flipped")?.active);
}

function visibleCardImage(card) {
  return isCardFlipped(card) && card.back_image ? card.back_image : card.image;
}

function cardCounterValue(card) {
  return cardPool(card, "counters")?.count ?? 0;
}

function cardCounterBounds(card) {
  const pool = cardPool(card, "counters");
  return { min: pool?.min ?? 0, max: pool?.max ?? null };
}

function pileAcceptsCard(pile, card) {
  const zone = boardLayout?.zones.find((candidate) => candidate.id === pile.zone_id);
  return !zone?.allowed_card_types?.length
    || zone.allowed_card_types.includes((card.card_type || "").trim().toLocaleLowerCase().replaceAll(" ", "-"));
}

function moveDestinations(card, sourcePileId) {
  return boardLayout.piles.filter((pile) => pile.id !== sourcePileId && pileAcceptsCard(pile, card));
}

function pileHasRole(pileId, role) {
  return boardLayout?.piles.find((pile) => pile.id === pileId)?.role === role;
}

function defaultDestination(card, sourcePileId, destinations) {
  if (pileHasRole(sourcePileId, "hand")) {
    const typedZone = destinations.find((pile) => {
      const allowed = boardLayout?.zones.find((zone) => zone.id === pile.zone_id)?.allowed_card_types ?? [];
      return allowed.includes((card.card_type || "").trim().toLocaleLowerCase().replaceAll(" ", "-"));
    });
    return typedZone?.id ?? destinations.find((pile) => pile.role === "play_default")?.id ?? destinations[0]?.id;
  }
  return destinations[0]?.id;
}

const tokenBackgrounds = {
  rose: "bg-rose-100 text-rose-700 border-rose-200",
  amber: "bg-amber-100 text-amber-700 border-amber-200",
  emerald: "bg-emerald-100 text-emerald-700 border-emerald-200",
  sky: "bg-sky-100 text-sky-700 border-sky-200",
  violet: "bg-violet-100 text-violet-700 border-violet-200",
};
const tokenIconColors = {
  red: "text-red-600",
  black: "text-black",
  blue: "text-blue-600",
  green: "text-green-600",
  yellow: "text-yellow-500",
  white: "text-amber-100",
};

function createTokenPool(pool, controls = false, playerIndex = activePlayerIndex) {
  const item = document.createElement("span");
  item.className = `flex h-14 w-full shrink-0 items-center justify-center border font-semibold ${controls ? "gap-3 rounded-xl px-3 text-xl" : "gap-2 rounded-xl px-3 text-sm"} ${tokenBackgrounds[pool.background] ?? "border-gray-200 bg-white text-gray-700"}`;
  item.setAttribute("aria-label", `${pool.label}: ${pool.count}`);
  item.title = `${pool.label}: ${pool.count}`;
  const icon = document.createElement("i");
  icon.className = `${pool.token} ${tokenIconColors[pool.icon_color] ?? ""}`;
  icon.setAttribute("aria-hidden", "true");
  const label = document.createElement("span");
  label.textContent = pool.count;
  label.setAttribute("aria-hidden", "true");
  item.append(icon, label);
  if (controls) {
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "grid h-9 w-9 place-items-center rounded-full border border-current/30 text-2xl leading-none transition hover:bg-white/60 focus:outline-none focus:ring-2 focus:ring-current";
    remove.setAttribute("aria-label", `Remove ${pool.minus} ${pool.label}`);
    remove.textContent = "−";
    remove.addEventListener("click", () => adjustPlayerTokenPool(playerIndex, pool, -pool.minus));
    const add = document.createElement("button");
    add.type = "button";
    add.className = "grid h-9 w-9 place-items-center rounded-full border border-current/30 text-2xl leading-none transition hover:bg-white/60 focus:outline-none focus:ring-2 focus:ring-current";
    add.setAttribute("aria-label", `Add ${pool.plus} ${pool.label}`);
    add.textContent = "+";
    add.addEventListener("click", () => adjustPlayerTokenPool(playerIndex, pool, pool.plus));
    item.prepend(remove);
    item.append(add);
  }
  return item;
}

function renderPlayerTokenPools() {
  tokenPoolPlayerIndex = Math.min(Math.max(tokenPoolPlayerIndex, 0), playerStates.length - 1);
  tokenPoolPlayerSelect.innerHTML = playerStates.map((_, index) =>
    `<option value="${index}">${playerName(index)}</option>`).join("");
  tokenPoolPlayerSelect.value = String(tokenPoolPlayerIndex);
  activePlayerLabel.textContent = `Token pools for ${playerName(tokenPoolPlayerIndex)}`;
  const playerTokens = document.getElementById("player-tokens");
  playerTokens.innerHTML = "";
  const pools = playerStates[tokenPoolPlayerIndex]?.players
    .flatMap((player) => player.token_pools)
    .filter((pool) => pool.active) ?? [];
  if (!pools.length) {
    playerTokens.innerHTML = '<span class="text-sm italic text-gray-400">No active token pools</span>';
    return;
  }
  for (const pool of pools) playerTokens.appendChild(createTokenPool(pool, true, tokenPoolPlayerIndex));
}

async function adjustPlayerTokenPool(index, pool, amount) {
  try {
    const playerGame = games[index];
    const before = playerGame.state_proto();
    const bytes = amount > 0
      ? playerGame.add_player_tokens(pool.id, amount)
      : playerGame.remove_player_tokens(pool.id, Math.abs(amount));
    playerStates[index] = decodeState(bytes);
    const message = `${amount > 0 ? "Added" : "Removed"} ${Math.abs(amount)} ${pool.label} for ${playerName(index)}`;
    recordChange(index, message, before, bytes);
    if (index === activePlayerIndex) renderState(playerStates[index]);
    else {
      renderPlayerTokenPools();
      renderOpponentHand();
      if (modalOpponentIndex === index) renderOpponentModal();
    }
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>${message}`;
  } catch (err) {
    console.error("[TOKEN POOL ERROR]", err);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Action failed: ${err}`;
  }
}

function renderOpponentHand() {
  const opponents = playerStates
    .map((state, index) => ({ state, index }))
    .filter(({ index }) => index !== activePlayerIndex);
  const totalCards = opponents.reduce((total, { state }) => total + (state?.zones.find((zone) => zone.id === "Hand")?.cards.length ?? 0), 0);
  opponentHandLabel.textContent = "Opponent hands";
  opponentHandCount.textContent = `${opponents.length} opponent${opponents.length !== 1 ? "s" : ""} · ${totalCards} card${totalCards !== 1 ? "s" : ""} · ${showOpponentHand.checked ? "visible" : "hidden"}`;
  opponentHandArea.innerHTML = "";

  for (const { state, index } of opponents) {
    const hand = state?.zones.find((zone) => zone.id === "Hand")?.cards ?? [];
    const group = document.createElement("section");
    group.className = "min-w-0 cursor-pointer rounded-lg border border-violet-100 bg-white/70 p-2 transition hover:border-violet-300 hover:bg-violet-50 focus:outline-none focus:ring-2 focus:ring-violet-400";
    group.tabIndex = 0;
    group.setAttribute("role", "button");
    group.setAttribute("aria-label", `Adjust ${playerName(index)} token pools`);
    group.addEventListener("click", () => openOpponentModal(index));
    group.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openOpponentModal(index);
      }
    });
    const title = document.createElement("h3");
    title.className = "mb-2 text-xs font-semibold text-violet-700";
    title.textContent = `${playerName(index)} · ${hand.length} card${hand.length !== 1 ? "s" : ""}`;
    const cards = document.createElement("ul");
    cards.className = "flex min-h-20 gap-2 overflow-x-auto";
    group.append(title, cards);
    opponentHandArea.appendChild(group);

    if (!hand.length) {
      cards.innerHTML = '<li class="self-center text-xs italic text-gray-400">Empty</li>';
    }

    for (const card of hand) {
      if (showOpponentHand.checked) {
        const item = createCardComponent(card, "opponent-hand");
        item.className = "w-24 shrink-0 sm:w-28";
        const button = item.querySelector("button");
        button.disabled = true;
        button.removeAttribute("title");
        button.setAttribute("aria-label", `${card.name}, ${card.card_type}`);
        button.classList.add("cursor-default");
        cards.appendChild(item);
        continue;
      }
      const back = document.createElement("li");
      back.className = "grid aspect-[5/7] w-16 shrink-0 place-items-center rounded-xl border border-violet-300 bg-gradient-to-br from-violet-800 via-indigo-900 to-gray-950 text-2xl text-white/70 shadow-sm";
      back.setAttribute("aria-label", `${playerName(index)} hidden card`);
      back.textContent = "✦";
      cards.appendChild(back);
    }
  }
}

function openOpponentModal(index) {
  if (!playerStates[index] || index === activePlayerIndex) return;
  modalOpponentIndex = index;
  renderOpponentModal();
  opponentModal.showModal();
}

function renderOpponentModal() {
  if (modalOpponentIndex === null) return;
  const state = playerStates[modalOpponentIndex];
  const pools = state?.players.flatMap((player) => player.token_pools).filter((pool) => pool.active) ?? [];
  opponentModalTitle.textContent = playerName(modalOpponentIndex);
  opponentTokenPools.innerHTML = "";
  if (!pools.length) {
    opponentTokenPools.innerHTML = '<p class="text-sm italic text-gray-400">No active token pools</p>';
    return;
  }
  for (const pool of pools) {
    const row = document.createElement("div");
    row.className = "flex h-14 w-full shrink-0 items-center justify-center rounded-xl border border-gray-200 bg-gray-50 px-3";
    row.setAttribute("aria-label", `${pool.label}: ${pool.count}. Allowed range ${pool.min ?? 0} to ${pool.max ?? "unlimited"}`);
    row.title = `${pool.label}: ${pool.count}`;
    const controls = document.createElement("div");
    controls.className = "inline-flex items-center gap-2";
    const remove = document.createElement("button");
    remove.type = "button";
    remove.className = "grid h-10 w-10 place-items-center rounded-full border border-gray-300 bg-white text-xl text-gray-600 transition hover:bg-gray-100 disabled:opacity-35";
    remove.textContent = "−";
    remove.disabled = pool.count <= (pool.min ?? 0);
    remove.setAttribute("aria-label", `Remove ${pool.minus} ${pool.label} from ${playerName(modalOpponentIndex)}`);
    const count = document.createElement("span");
    count.className = "min-w-12 px-3 text-center text-sm font-semibold text-gray-800";
    count.textContent = pool.count;
    const add = document.createElement("button");
    add.type = "button";
    add.className = "grid h-10 w-10 place-items-center rounded-full border border-gray-300 bg-white text-xl text-gray-600 transition hover:bg-gray-100 disabled:opacity-35";
    add.textContent = "+";
    add.disabled = pool.max !== undefined && pool.max !== null && pool.count >= pool.max;
    add.setAttribute("aria-label", `Add ${pool.plus} ${pool.label} to ${playerName(modalOpponentIndex)}`);
    remove.addEventListener("click", () => adjustOpponentPool(pool.id, -1));
    add.addEventListener("click", () => adjustOpponentPool(pool.id, 1));
    controls.append(remove, count, add);
    row.appendChild(controls);
    opponentTokenPools.appendChild(row);
  }
}

async function adjustOpponentPool(poolId, change) {
  if (modalOpponentIndex === null) return;
  const index = modalOpponentIndex;
  try {
    const before = games[index].state_proto();
    const pool = playerStates[index].players[0].token_pools.find((candidate) => candidate.id === poolId);
    const bytes = change > 0
      ? games[index].add_player_tokens(poolId, pool?.plus ?? 1)
      : games[index].remove_player_tokens(poolId, pool?.minus ?? 1);
    playerStates[index] = decodeState(bytes);
    recordChange(index, `${change > 0 ? "Added" : "Removed"} ${pool?.label ?? poolId} for ${playerName(index)}`, before, bytes);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Updated ${playerName(index)}`;
    renderOpponentHand();
    renderOpponentModal();
  } catch (err) {
    console.error("[WASM ACTION ERROR] Updating opponent token pool", err);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Action failed: ${err}`;
  }
}

function renderPlayerTabs() {
  playerSelect.innerHTML = "";
  games.forEach((_, index) => {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = playerName(index);
    playerSelect.appendChild(option);
  });
  updatePlayerControls();
}

function addPlayer() {
  const newGame = new WasmGame();
  const nextName = `Player ${games.length + 1}`;
  newGame.set_player_name(nextName);
  games.push(newGame);
  playerStates.push(decodeState(newGame.state_proto()));
  playerDecks.push({ name: nextName, url: null });
  const instance = gameInstances.find((candidate) => candidate.id === activeGameInstanceId);
  if (instance) instance.playerCount = games.length;
  saveGameInstances();
  saveGameState(games.length - 1);
  renderPlayerTabs();
  renderPlayerTokenPools();
  renderOpponentHand();
  statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Added ${nextName}`;
}

function createPlayerGame(index) {
  const baseCsv = defaultDeckCsvs[index] ?? defaultDeckCsvs[0];
  const playerGame = WasmGame.fromCardsCsv(withCustomCards(baseCsv, index));
  playerGame.set_player_name(`Player ${index + 1}`);
  restoreGameState(playerGame, index);
  return playerGame;
}

function loadGameInstance(instanceId) {
  const instance = gameInstances.find((candidate) => candidate.id === instanceId);
  if (!instance) return;
  if (cardModal.open) closeCardModal();
  if (opponentModal.open) opponentModal.close();
  activeGameInstanceId = instance.id;
  const playerCount = Math.max(defaultPlayerDecks.length, Number(instance.playerCount) || 0);
  playerDecks = Array.from({ length: playerCount }, (_, index) =>
    defaultPlayerDecks[index] ?? { name: `Player ${index + 1}`, url: null });
  games = playerDecks.map((_, index) => createPlayerGame(index));
  playerStates = games.map((playerGame) => decodeState(playerGame.state_proto()));
  turnState = readTurnState();
  turnState.playerIndex = Math.min(Math.max(turnState.playerIndex, 0), games.length - 1);
  activePlayerIndex = turnState.playerIndex;
  tokenPoolPlayerIndex = activePlayerIndex;
  game = games[activePlayerIndex];
  saveTurnState();
  selectedCard = null;
  revealedPileCards.clear();
  saveGameInstances();
  renderGameInstances();
  renderPlayerTabs();
  if (boardLayout) {
    renderState(playerStates[activePlayerIndex]);
    renderChangeHistory();
  }
}

function createGameInstance() {
  const nextNumber = Math.max(0, ...gameInstances.map((candidate) => {
    const match = /^Game (\d+)$/.exec(candidate.name);
    return match ? Number(match[1]) : 0;
  })) + 1;
  const instance = makeGameInstance(`Game ${nextNumber}`);
  gameInstances.push(instance);
  saveGameInstances();
  loadGameInstance(instance.id);
  statusEl.innerHTML = '<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Created a new game';
}

function deleteGameInstance() {
  const instance = gameInstances.find((candidate) => candidate.id === activeGameInstanceId);
  if (!instance || !window.confirm(`Delete ${instance.name}? All of its saved state and history will be removed.`)) return;
  for (let index = 0; index < Math.max(instance.playerCount || 0, games.length); index += 1) {
    localStorage.removeItem(savedStateKey(index));
    localStorage.removeItem(changeHistoryKey(index));
    localStorage.removeItem(customCardsKey(index));
  }
  localStorage.removeItem(turnStateKey());
  gameInstances = gameInstances.filter((candidate) => candidate.id !== instance.id);
  if (!gameInstances.length) gameInstances.push(makeGameInstance("Game 1"));
  activeGameInstanceId = gameInstances[0].id;
  saveGameInstances();
  loadGameInstance(activeGameInstanceId);
  statusEl.innerHTML = '<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Game deleted';
}

function updatePlayerControls() {
  const name = playerName(activePlayerIndex);
  handOwnerLabel.textContent = `${name}'s hand`;
  zonesOwnerLabel.textContent = `${name} zones`;
  zonesOwnerDescription.textContent = `Showing ${name}'s piles and token counts.`;
  playerSelect.value = String(activePlayerIndex);
  playerBoardPanel.setAttribute("aria-label", `${name} board`);
  turnLabel.textContent = `Turn ${turnState?.number ?? 1} · ${playerName(turnState?.playerIndex ?? activePlayerIndex)}`;
  endTurnButton.disabled = games.length < 2;
}

function renamePlayer(index) {
  const currentName = playerName(index);
  const name = window.prompt("Player name", currentName)?.trim();
  if (!name || name === currentName) return;
  try {
    const before = games[index].state_proto();
    const bytes = games[index].set_player_name(name);
    playerStates[index] = decodeState(bytes);
    recordChange(index, `Renamed ${currentName} to ${name}`, before, bytes);
    renderPlayerTabs();
    updatePlayerControls();
    renderPlayerTokenPools();
    renderOpponentHand();
    if (modalOpponentIndex === index) renderOpponentModal();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Renamed player to ${name}`;
  } catch (err) {
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Rename failed: ${err}`;
  }
}

function switchPlayer(index) {
  if (!games[index]) return;
  if (cardModal.open) closeCardModal();
  if (opponentModal.open) opponentModal.close();
  activePlayerIndex = index;
  tokenPoolPlayerIndex = index;
  game = games[index];
  selectedCard = null;
  selectionEl.textContent = "Select a card from the searched pile.";
  moveBtn.disabled = true;
  updatePlayerControls();
  renderState(playerStates[index]);
  renderChangeHistory();
  const turnOwner = playerName(turnState?.playerIndex ?? index);
  statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Viewing ${playerName(index)}'s board · ${turnOwner}'s turn`;
}

function cyclePlayerBoard(direction) {
  if (games.length < 2) return;
  const nextIndex = (activePlayerIndex + direction + games.length) % games.length;
  switchPlayer(nextIndex);
}

function cycleBoardPiece(direction) {
  const panels = [...pilePanels.values()].filter((panel) => !panel.classList.contains("hidden"));
  if (!panels.length) return;
  panels[activePileIndex]?.classList.remove("ring-4", "ring-brand-300", "ring-offset-2");
  activePileIndex = (activePileIndex + direction + panels.length) % panels.length;
  const panel = panels[activePileIndex];
  panel.classList.add("ring-4", "ring-brand-300", "ring-offset-2");
  panel.focus({ preventScroll: true });
  panel.scrollIntoView({ behavior: "smooth", block: "center", inline: "nearest" });
  statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-brand-500"></span>${pileName(panel.dataset.pileId)}`;
}

function endTurn() {
  if (games.length < 2 || !turnState) return;
  const previousPlayer = playerName(turnState.playerIndex);
  turnState = {
    number: turnState.number + 1,
    playerIndex: (turnState.playerIndex + 1) % games.length,
    startedAt: new Date().toISOString(),
  };
  saveTurnState();
  switchPlayer(turnState.playerIndex);
  renderChangeHistory();
  statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>${previousPlayer} ended their turn. ${playerName(activePlayerIndex)} is up.`;
}

function createCardComponent(card, pileId) {
  const item = document.createElement("li");
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.cardId = card.id;
  const tapped = isCardTapped(card);
  const usesCardLayout = pileHasRole(pileId, "hand")
    || pileId === "opponent-hand"
    || Boolean(boardLayout?.piles.some((pile) => pile.id === pileId));
  if (usesCardLayout) {
    item.className = tapped
      ? "relative h-28 w-36 shrink-0 snap-start"
      : "w-24 shrink-0 snap-start sm:w-28";
  }
  button.className = usesCardLayout
    ? `group flex aspect-[5/7] flex-col overflow-hidden rounded-xl border bg-gradient-to-br from-gray-900 via-gray-800 to-gray-950 p-2.5 text-left text-white shadow-md transition hover:border-cyan-400 hover:shadow-lg focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:ring-offset-2 ${tapped ? "absolute left-1/2 top-1/2 w-20 -translate-x-1/2 -translate-y-1/2 rotate-90 border-amber-300 hover:scale-[1.02] sm:w-24" : "w-full border-gray-700 hover:-translate-y-1"}`
    : `group flex w-full items-center justify-between gap-2 rounded-lg border border-gray-200 bg-gray-50 px-2.5 py-2 text-left transition hover:border-brand-500 hover:bg-brand-50 focus:outline-none focus:ring-2 focus:ring-brand-500 focus:ring-offset-1 ${tapped ? "rotate-90 border-amber-300 bg-amber-50" : ""}`;
  button.title = `View ${card.name}`;
  button.innerHTML = usesCardLayout
    ? `<span class="flex w-full items-start justify-between gap-1 border-b border-white/15 pb-2"><span class="min-w-0 truncate text-xs font-semibold" data-card-name></span><span class="shrink-0 text-[9px] font-bold text-amber-200" data-card-mana></span></span><span class="my-2 block w-full flex-1 rounded-lg border border-white/10 bg-gradient-to-br from-cyan-500/35 to-violet-500/20 bg-cover bg-center" data-card-art aria-hidden="true"></span><span class="flex w-full items-end justify-between gap-2"><span class="min-w-0 truncate text-[10px] text-white/60" data-card-type></span><span data-counter-badge class="hidden min-w-5 rounded-full bg-brand-500 px-1.5 py-0.5 text-center text-[9px] font-semibold text-white"></span></span>`
    : `<span class="min-w-0"><span class="block truncate text-[11px] font-medium text-gray-800" data-card-name></span><span class="mt-0.5 block truncate text-[9px] text-gray-400" data-card-type></span></span><span class="flex shrink-0 items-center gap-1.5"><span data-counter-badge class="hidden min-w-5 rounded-full bg-brand-500 px-1.5 py-0.5 text-center text-[9px] font-semibold text-white"></span><span class="text-xs text-gray-300 transition group-hover:translate-x-0.5 group-hover:text-brand-500" aria-hidden="true">→</span></span>`;
  button.querySelector("[data-card-name]").textContent = card.name;
  button.querySelector("[data-card-type]").textContent = card.card_type;
  if (usesCardLayout) {
    button.querySelector("[data-card-mana]").textContent = formatMana(card.mana);
    const art = button.querySelector("[data-card-art]");
    const image = visibleCardImage(card);
    if (image) art.style.backgroundImage = `url("${image}")`;
  }
  const counterBadge = button.querySelector("[data-counter-badge]");
  const counters = cardCounterValue(card);
  if (counters > 0) {
    counterBadge.textContent = counters;
    counterBadge.classList.remove("hidden");
  }
  button.addEventListener("click", () => openCardModal(card, pileId));
  item.appendChild(button);
  return item;
}

function openCardModal(card, pileId) {
  modalCard = { card, pileId, navigationIds: viewableCardsForPile(pileId).map((candidate) => candidate.id) };
  renderCardModal();
  cardModal.showModal();
}

function viewableCardsForPile(pileId) {
  const zoneCards = currentState?.zones.find((zone) => zone.id === pileId)?.cards ?? [];
  const pile = boardLayout?.piles.find((candidate) => candidate.id === pileId);
  const panelQuery = pilePanels.get(pileId)?.querySelector("[data-pile-search]")?.value.trim().toLocaleLowerCase() ?? "";
  const actionQuery = sourcePileEl.value === pileId ? searchEl.value.trim().toLocaleLowerCase() : "";
  const query = panelQuery || actionQuery;
  if (pile?.visible) return query ? zoneCards.filter((card) => cardMatchesSearch(card, query)) : zoneCards;
  if (query) {
    return zoneCards.filter((card) => cardMatchesSearch(card, query));
  }
  const revealed = revealedPileCards.get(pileId);
  return revealed ? zoneCards.filter((card) => revealed.has(card.id)) : [];
}

function updateCardModalDestinations() {
  if (!modalCard) return;
  const { card, pileId } = modalCard;
  const destinations = moveDestinations(card, pileId);
  cardModalTarget.innerHTML = destinations
    .map((pile) => `<option value="${pile.id}">${pile.name}</option>`)
    .join("");
  cardModalTarget.value = defaultDestination(card, pileId, destinations) ?? "";
  cardModalMove.disabled = !cardModalTarget.value;
}

function navigateModalCard(offset) {
  if (!modalCard) return;
  const currentIndex = modalCard.navigationIds.indexOf(modalCard.card.id);
  const nextId = modalCard.navigationIds[currentIndex + offset];
  if (!nextId) return;
  const nextCard = currentState?.zones
    .find((zone) => zone.id === modalCard.pileId)?.cards
    .find((card) => card.id === nextId);
  if (!nextCard) return;
  modalCard.card = nextCard;
  renderCardModal();
}

function renderCardModal() {
  if (!modalCard) return;
  const { card, pileId } = modalCard;
  cardModalPile.textContent = pileName(pileId);
  cardModalTitle.textContent = card.name;
  cardViewName.textContent = card.name;
  cardViewType.textContent = `${card.card_type || "Card"}${card.power ? ` · ${card.power}/${card.toughness}` : ""}`;
  cardViewId.textContent = card.oracle_text || `Card ID: ${card.id}`;
  const image = visibleCardImage(card);
  cardView.style.backgroundImage = image ? `url("${image}")` : "";
  cardView.classList.toggle("card-full-art", Boolean(image));
  const tapped = isCardTapped(card);
  cardView.classList.toggle("rotate-90", tapped);
  cardView.classList.toggle("my-10", tapped);
  cardModalTap.innerHTML = `${tapped ? "Untap" : "Tap"} <span class="ml-1 text-xs text-gray-400">T</span>`;
  cardModalTap.setAttribute("aria-pressed", String(tapped));
  const navigationIndex = modalCard.navigationIds.indexOf(card.id);
  cardModalPrevious.disabled = navigationIndex <= 0;
  cardModalNext.disabled = navigationIndex < 0 || navigationIndex >= modalCard.navigationIds.length - 1;
  const flipped = isCardFlipped(card);
  cardModalFlip.classList.toggle("hidden", !card.back_image);
  cardModalFlip.textContent = flipped ? "Show front" : "Show back";
  cardModalFlip.setAttribute("aria-pressed", String(flipped));
  cardModalBottom.classList.toggle("hidden", !pileHasRole(pileId, "draw"));
  cardModalBottom.disabled = false;
  const counters = cardCounterValue(card);
  const { min, max } = cardCounterBounds(card);
  cardCounterCount.textContent = String(counters);
  cardCounterCount.title = `Allowed range: ${min}–${max ?? "unlimited"}`;
  cardCounterRemove.disabled = counters <= min;
  cardCounterAdd.disabled = max !== null && counters >= max;
  updateCardModalDestinations();
}

async function toggleModalCardTapped() {
  if (!modalCard || cardModalTap.disabled) return;
  const { card } = modalCard;
  const tapping = !isCardTapped(card);
  cardModalTap.disabled = true;
  const changed = await runAction(
    () => game.set_card_tapped(card.id, tapping),
    `${tapping ? "Tapped" : "Untapped"} ${card.name}`,
  );
  if (changed) animateCardRotation(card.id, tapping);
  cardModalTap.disabled = false;
}

function animateCardRotation(cardId, tapping) {
  if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const cardButton = document.querySelector(`[data-card-id="${CSS.escape(cardId)}"]`);
  if (!cardButton) return;
  const finalTransform = getComputedStyle(cardButton).transform;
  cardButton.animate(
    [
      { transform: tapping ? "translate(-50%, -50%) rotate(0deg)" : "rotate(90deg)", opacity: 0.78 },
      { transform: finalTransform, opacity: 1 },
    ],
    { duration: 360, easing: "cubic-bezier(.2,.8,.2,1)" },
  );
}

function formatMana(mana = "") {
  return mana.replaceAll("{", "").replaceAll("}", "");
}

function cardMatchesSearch(card, query) {
  return !query || [card.id, card.name, card.card_type, card.oracle_text]
    .some((value) => (value || "").toLocaleLowerCase().includes(query));
}

function closeCardModal() {
  cardModal.close();
  modalCard = null;
}

// ── Render pile layout ───────────────────────────────────────────────────────
function buildBoard(layout) {
  boardEl.innerHTML = "";
  handAreaEl.innerHTML = "";
  pilePanels.clear();
  parentZoneSearches.clear();
  activePileIndex = -1;

  const parentZones = new Map();
  for (const zone of layout.zones.filter((candidate) =>
    layout.zones.some((child) => child.parent_zone === candidate.id)
  )) {
    const container = document.createElement("section");
    container.className = "zone-parent relative flex min-w-0 flex-col gap-2 rounded-xl border border-emerald-200 bg-emerald-50/40 p-2";
    container.style.gridColumn = `${zone.x + 1} / span ${zone.width}`;
    container.style.gridRow = `${zone.y + 1} / span ${zone.height}`;
    container.dataset.zoneId = zone.id;
    const childGrid = document.createElement("div");
    childGrid.className = "grid min-h-0 flex-1 gap-2";
    childGrid.style.gridTemplateColumns = `repeat(${zone.width}, minmax(0, 1fr))`;
    const hasOwnPile = layout.piles.some((pile) => pile.zone_id === zone.id);
    if (!hasOwnPile) {
      const search = document.createElement("section");
      search.className = "rounded-lg border border-emerald-200 bg-white/90 p-2 shadow-sm";
      search.setAttribute("aria-label", `${zone.name} cards`);
      search.innerHTML = `<label class="block"><span class="sr-only">Search ${zone.name}</span><span class="relative block"><i class="fa-solid fa-magnifying-glass pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-xs text-emerald-600" aria-hidden="true"></i><input data-parent-search type="search" placeholder="Search ${zone.name}…" class="w-full rounded-lg border border-emerald-200 bg-emerald-50/50 py-2 pl-9 pr-3 text-xs text-gray-700 outline-none transition placeholder:text-gray-400 focus:border-brand-500 focus:bg-white focus:ring-2 focus:ring-brand-50" /></span></label><ul data-parent-card-list class="mt-2 flex max-h-40 gap-2 overflow-x-auto overflow-y-hidden pb-1"></ul>`;
      search.querySelector("[data-parent-search]").addEventListener("input", renderParentZoneSearches);
      container.appendChild(search);
      parentZoneSearches.set(zone.id, search);
    }
    container.appendChild(childGrid);
    boardEl.appendChild(container);
    parentZones.set(zone.id, childGrid);
  }

  // Pile panels
  for (const pile of layout.piles) {
    const panel = document.createElement("article");
    const zoneLayout = layout.zones.find((zone) => zone.id === pile.zone_id);
    const borderCls = zoneColorMap[zoneLayout?.color] ?? "border-gray-200";
    panel.className = `pile-panel rounded-xl border bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md ${borderCls}`;
    panel.style.gridColumn = `${(zoneLayout?.x ?? pile.x) + 1} / span ${zoneLayout?.width ?? 8}`;
    panel.style.gridRow = `${(zoneLayout?.y ?? pile.y) + 1} / span ${zoneLayout?.height ?? 4}`;
    panel.dataset.pileId   = pile.id;
    panel.tabIndex = -1;
    panel.setAttribute("aria-label", `${pile.name} board piece`);

    const heading = document.createElement("div");
    heading.className = "mb-2 flex items-center justify-between";

    const title = document.createElement("h2");
    title.className = "truncate text-xs font-semibold text-gray-800";
    title.textContent = pile.name;
    heading.appendChild(title);

    if (zoneLayout?.scope === "game") {
      const scope = document.createElement("span");
      scope.className = "ml-auto mr-2 shrink-0 rounded-full bg-indigo-50 px-1.5 py-0.5 text-[9px] font-semibold uppercase tracking-wide text-indigo-600";
      scope.textContent = "Shared";
      scope.title = "This zone belongs to the game and is shared by all players";
      heading.appendChild(scope);
    }

    const count = document.createElement("span");
    count.className = "ml-1 shrink-0 rounded-full bg-gray-100 px-1.5 py-0.5 text-[10px] font-medium text-gray-500";
    count.dataset.cardCount = "true";
    count.textContent = "0 cards";

    heading.appendChild(count);
    panel.appendChild(heading);

    const pileSearch = document.createElement("label");
    pileSearch.className = "mb-2 block";
    pileSearch.innerHTML = `<span class="sr-only">Search ${pile.name}</span><span class="relative block"><i class="fa-solid fa-magnifying-glass pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[10px] text-gray-400" aria-hidden="true"></i><input data-pile-search type="search" placeholder="Search ${pile.name}…" class="w-full rounded-lg border border-gray-200 bg-gray-50 py-1.5 pl-7 pr-2 text-[10px] text-gray-700 outline-none transition placeholder:text-gray-400 focus:border-brand-500 focus:bg-white focus:ring-2 focus:ring-brand-50" /></span>`;
    pileSearch.querySelector("[data-pile-search]").addEventListener("input", () => {
      selectedCard = null;
      moveBtn.disabled = true;
      if (currentState) renderState(currentState);
    });
    panel.appendChild(pileSearch);

    const cardList = document.createElement("ul");
    cardList.className = "flex w-full snap-x snap-proximity gap-2 overflow-x-auto overflow-y-hidden overscroll-x-contain pb-2";
    cardList.dataset.cardList = "true";
    panel.appendChild(cardList);

    if (!pile.visible) {
      const controls = document.createElement("div");
      controls.className = "mt-2 grid grid-cols-2 gap-1.5";
      const destinations = layout.piles
        .filter((candidate) => candidate.id !== pile.id)
        .map((candidate) => `<option value="${candidate.id}" ${candidate.role === "hand" ? "selected" : ""}>${candidate.name}</option>`)
        .join("");
      controls.innerHTML = `<label class="text-[9px] text-gray-500">Count<input data-hidden-count type="number" min="1" value="1" class="mt-0.5 w-full rounded border border-gray-200 px-1.5 py-1 text-[10px]"></label><label class="text-[9px] text-gray-500">Choose<select data-hidden-mode class="mt-0.5 w-full rounded border border-gray-200 px-1.5 py-1 text-[10px]"><option value="top">From top</option><option value="random">At random</option></select></label><label class="col-span-2 text-[9px] text-gray-500">Move to<select data-hidden-target class="mt-0.5 w-full rounded border border-gray-200 px-1.5 py-1 text-[10px]">${destinations}</select></label><button type="button" data-move-hidden class="rounded-lg bg-gray-900 px-2 py-1.5 text-[10px] font-semibold text-white transition hover:bg-gray-700 disabled:opacity-40">Move</button><button type="button" data-reveal-hidden class="rounded-lg border border-gray-300 bg-white px-2 py-1.5 text-[10px] font-semibold text-gray-700 transition hover:bg-gray-100 disabled:opacity-40">Reveal</button><button type="button" data-shuffle-hidden class="col-span-2 rounded-lg border border-violet-200 bg-violet-50 px-2 py-1.5 text-[10px] font-semibold text-violet-700 transition hover:bg-violet-100 disabled:opacity-40">Shuffle pile</button>`;
      const selection = () => ({
        count: Math.max(1, Number.parseInt(controls.querySelector("[data-hidden-count]").value, 10) || 1),
        random: controls.querySelector("[data-hidden-mode]").value === "random",
      });
      controls.querySelector("[data-move-hidden]").addEventListener("click", () => {
        const { count, random } = selection();
        const destination = controls.querySelector("[data-hidden-target]").value;
        runAction(
          () => game.move_cards(pile.id, destination, count, random),
          `Moved up to ${count} ${random ? "random" : "top"} card${count === 1 ? "" : "s"} from ${pile.name}`,
        );
      });
      controls.querySelector("[data-reveal-hidden]").addEventListener("click", () => {
        const { count, random } = selection();
        const cards = currentState?.zones.find((zone) => zone.id === pile.id)?.cards ?? [];
        const available = [...cards];
        const selected = [];
        const revealCount = Math.min(count, available.length);
        if (random) {
          while (selected.length < revealCount) selected.push(available.splice(Math.floor(Math.random() * available.length), 1)[0]);
        } else {
          selected.push(...available.slice(-revealCount).reverse());
        }
        revealedPileCards.set(pile.id, new Set(selected.map((card) => card.id)));
        renderState(currentState);
      });
      controls.querySelector("[data-shuffle-hidden]").addEventListener("click", () => {
        runAction(() => game.shuffle_pile(pile.id), `Shuffled ${pile.name}`);
      });
      panel.appendChild(controls);
    }

    const tokenWrap = document.createElement("div");
    tokenWrap.className = "mt-1 flex max-h-24 flex-col items-start gap-1 overflow-x-hidden overflow-y-auto overscroll-y-contain pr-1";
    tokenWrap.dataset.tokenWrap = "true";
    panel.appendChild(tokenWrap);

    if (pile.role === "hand") {
      panel.style.gridColumn = "";
      panel.style.gridRow = "";
      panel.className = "h-full bg-transparent";
      heading.classList.add("hidden");
      tokenWrap.classList.add("hidden");
      cardList.className = "flex w-full snap-x snap-proximity gap-3 overflow-x-auto overscroll-x-contain pb-2";
      handAreaEl.appendChild(panel);
    } else {
      const parent = zoneLayout?.parent_zone ? parentZones.get(zoneLayout.parent_zone) : null;
      (parent ?? boardEl).appendChild(panel);
    }
    pilePanels.set(pile.id, panel);
  }
}

function renderParentZoneSearches() {
  if (!currentState || !boardLayout) return;
  for (const [parentId, search] of parentZoneSearches) {
    const childZoneIds = new Set(boardLayout.zones
      .filter((zone) => zone.parent_zone === parentId)
      .map((zone) => zone.id));
    const childPileIds = new Set(boardLayout.piles
      .filter((pile) => childZoneIds.has(pile.zone_id))
      .map((pile) => pile.id));
    const query = search.querySelector("[data-parent-search]").value.trim().toLocaleLowerCase();
    const list = search.querySelector("[data-parent-card-list]");
    list.innerHTML = "";
    if (!query) {
      list.innerHTML = '<li class="self-center px-1 text-[10px] italic text-gray-400">Search to show cards in this zone</li>';
      continue;
    }
    const cards = currentState.zones
      .filter((zone) => childPileIds.has(zone.id))
      .flatMap((zone) => zone.cards
        .filter((card) => cardMatchesSearch(card, query))
        .map((card) => ({ card, pileId: zone.id })));
    if (!cards.length) {
      list.innerHTML = '<li class="self-center px-1 text-[10px] italic text-gray-400">No matches</li>';
      continue;
    }
    for (const { card, pileId } of cards) list.appendChild(createCardComponent(card, pileId));
  }
}

// ── Update pile panels from game state ───────────────────────────────────────
function renderState(state) {
  currentState = state;
  playerStates[activePlayerIndex] = state;
  const deckBackground = state.zones
    .flatMap((zone) => zone.cards)
    .find((card) => card.background_image)?.background_image;
  if (deckBackground) document.body.style.backgroundImage = `url("${deckBackground}")`;
  // Build a map zone display-id → zone data
  const zoneMap = Object.fromEntries(state.zones.map((z) => [z.id, z]));

  renderPlayerTokenPools();

  if (modalCard) {
    const freshCard = state.zones.flatMap((zone) => zone.cards).find((card) => card.id === modalCard.card.id);
    if (freshCard) {
      modalCard.card = freshCard;
      renderCardModal();
    }
  }

  for (const [pileId, panel] of pilePanels) {
    const zone = zoneMap[pileId];
    if (!zone) continue;

    const countEl  = panel.querySelector("[data-card-count]");
    const listEl   = panel.querySelector("[data-card-list]");
    const tokenEl  = panel.querySelector("[data-token-wrap]");
    const hiddenActionEls = panel.querySelectorAll("[data-move-hidden], [data-reveal-hidden], [data-shuffle-hidden]");

    countEl.textContent = `${zone.cards.length} card${zone.cards.length !== 1 ? "s" : ""}`;
    for (const action of hiddenActionEls) action.disabled = zone.cards.length === 0;
    if (pileHasRole(pileId, "hand")) handSummaryCount.textContent = countEl.textContent;

    listEl.innerHTML = "";
    const pile = boardLayout.piles.find((candidate) => candidate.id === pileId);
    const localQuery = panel.querySelector("[data-pile-search]")?.value.trim().toLocaleLowerCase() ?? "";
    const isSelectedSearchPile = sourcePileEl.value === pileId;
    const actionQuery = isSelectedSearchPile ? searchEl.value.trim().toLocaleLowerCase() : "";
    const query = localQuery || actionQuery;
    const searchableCards = zone.cards.filter((card) => cardMatchesSearch(card, query));
    const revealed = revealedPileCards.get(pileId);
    const revealedCards = revealed ? zone.cards.filter((card) => revealed.has(card.id)) : [];
    const cardsToShow = pile?.visible ? (query ? searchableCards : zone.cards) : (query ? searchableCards : revealedCards);
    if (!cardsToShow.length) {
      const message = !pile?.visible && !query ? "Cards hidden" : (query ? "No matches" : "Empty");
      listEl.innerHTML = `<li class="text-[10px] italic text-gray-400">${message}</li>`;
    } else {
      for (const card of cardsToShow) {
        listEl.appendChild(createCardComponent(card, pileId));
      }
    }

    tokenEl.innerHTML = "";
    for (const pool of zone.token_pools) {
      if (pool.active) tokenEl.appendChild(createTokenPool(pool));
    }
  }
  renderParentZoneSearches();
  renderOpponentHand();
}

function selectCard(card, pileId) {
  selectedCard = { card, pileId };
  selectionEl.textContent = `Selected ${card.name} from ${boardLayout.piles.find((p) => p.id === pileId)?.name ?? pileId}.`;
  moveBtn.disabled = targetPileEl.value === pileId;
  if (currentState) renderState(currentState);
}

function fillPileSelectors() {
  const options = boardLayout.piles.map((pile) => `<option value="${pile.id}">${pile.name}</option>`).join("");
  sourcePileEl.innerHTML = options;
  targetPileEl.innerHTML = options;
  targetPileEl.value = boardLayout.piles.find((p) => p.id !== sourcePileEl.value)?.id ?? sourcePileEl.value;
}

function fillCreateCardPiles() {
  if (!boardLayout) return;
  const card = { card_type: createCardType.value };
  const choices = boardLayout.piles.filter((pile) => pileAcceptsCard(pile, card));
  createCardPile.innerHTML = choices.map((pile) => `<option value="${pile.id}">${pile.name}</option>`).join("");
  const typedPile = choices.find((pile) => {
    const allowed = boardLayout.zones.find((zone) => zone.id === pile.zone_id)?.allowed_card_types ?? [];
    return allowed.includes(createCardType.value);
  });
  createCardPile.value = typedPile?.id ?? choices.find((pile) => pile.role === "play_default")?.id ?? choices[0]?.id ?? "";
}

function openCreateCardDialog() {
  createCardForm.reset();
  fillCreateCardPiles();
  createCardDialog.showModal();
  createCardName.focus();
}

async function submitCustomCard(event) {
  event.preventDefault();
  const definition = {
    id: `custom-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`}`,
    name: createCardName.value.trim(),
    cardType: createCardType.value,
    text: createCardText.value.trim(),
    offense: createCardOffense.value.trim(),
    defense: createCardDefense.value.trim(),
    pileId: createCardPile.value,
  };
  if (!definition.name || !definition.pileId) return;
  const submit = document.getElementById("create-card-submit");
  submit.disabled = true;
  try {
    const bytes = game.createCard(
      definition.id,
      definition.name,
      definition.cardType,
      definition.text,
      definition.offense,
      definition.defense,
      definition.pileId,
    );
    writeCustomCards(activePlayerIndex, [...readCustomCards(activePlayerIndex), definition]);
    playerStates[activePlayerIndex] = decodeState(bytes);
    saveGameState(activePlayerIndex);
    renderState(playerStates[activePlayerIndex]);
    createCardDialog.close();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Created ${definition.name} in ${pileName(definition.pileId)}`;
  } catch (err) {
    console.error("[CREATE CARD ERROR]", err);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Could not create card: ${err}`;
  } finally {
    submit.disabled = false;
  }
}

// ── Landscape phone: group associated piles ───────────────────────────────────
function applyLandscapeGroups(layout) {
  if (!isLandscapePhone()) return;

  // Build association sets
  const visited = new Set();
  for (const pile of layout.piles) {
    if (visited.has(pile.id) || !pile.associated_piles?.length) continue;
    const group = [pile.id, ...pile.associated_piles];
    group.forEach((id) => visited.add(id));

    const wrapper = document.createElement("div");
    wrapper.className = "pile-group";
    const firstPanel = pilePanels.get(group[0]);
    if (!firstPanel) continue;
    boardEl.insertBefore(wrapper, firstPanel);
    for (const id of group) {
      const panel = pilePanels.get(id);
      if (panel) wrapper.appendChild(panel);
    }
  }
}

function isLandscapePhone() {
  return window.matchMedia("(orientation: landscape) and (max-height: 500px)").matches;
}

// ── Boot ──────────────────────────────────────────────────────────────────────
async function runAction(action, message) {
  if (!game) { statusEl.textContent = "Game is still initialising."; return false; }
  try {
    const before = game.state_proto();
    const nextBytes = await action();
    const next = decodeState(nextBytes);
    playerStates[activePlayerIndex] = next;
    revealedPileCards.clear();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>${message}`;
    renderState(next);
    recordChange(activePlayerIndex, message, before, nextBytes);
    return true;
  } catch (err) {
    console.error(`[WASM ACTION ERROR] ${message}`, err);
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Action failed: ${err}`;
    return false;
  }
}

window.addEventListener("error", (event) => {
  console.error("[WEB ERROR]", event.error ?? event.message);
});
window.addEventListener("unhandledrejection", (event) => {
  console.error("[WEB UNHANDLED REJECTION]", event.reason);
});

async function boot() {
  try {
    await init(new URL("../pkg/byog_bg.wasm?v=25", import.meta.url));
    defaultDeckCsvs = await Promise.all(defaultPlayerDecks.map(async ({ url }) => {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`Could not load ${url} (${response.status})`);
      return response.text();
    }));
    gameInstances = readGameInstances();
    const savedActiveId = localStorage.getItem(activeGameInstanceKey);
    activeGameInstanceId = gameInstances.some((instance) => instance.id === savedActiveId)
      ? savedActiveId
      : gameInstances[0].id;
    loadGameInstance(activeGameInstanceId);

    boardLayout = decodeBoardLayout(game.board_layout_proto());
    fillPileSelectors();
    buildBoard(boardLayout);
    applyLandscapeGroups(boardLayout);

    renderState(playerStates[activePlayerIndex]);

    sourcePileEl.disabled = false;
    targetPileEl.disabled = false;
    searchEl.disabled = false;
    statusEl.innerHTML = '<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Ready';
  } catch (err) {
    statusEl.textContent = `Initialisation failed: ${err}`;
  }
}

// Re-build board on orientation change
window.addEventListener("resize", () => {
  if (boardLayout) {
    buildBoard(boardLayout);
    applyLandscapeGroups(boardLayout);
    if (game) {
      try { renderState(decodeState(game.state_proto())); } catch (_) {}
    }
  }
});

endTurnButton.addEventListener("click", endTurn);
gameSelect.addEventListener("change", () => loadGameInstance(gameSelect.value));
playerSelect.addEventListener("change", () => switchPlayer(Number(playerSelect.value)));
newGameButton.addEventListener("click", createGameInstance);
deleteGameButton.addEventListener("click", deleteGameInstance);
undoBtn.addEventListener("click", undoLastChange);
addPlayerButton.addEventListener("click", addPlayer);
renamePlayerButton.addEventListener("click", () => renamePlayer(activePlayerIndex));
tokenPoolPlayerSelect.addEventListener("change", () => {
  tokenPoolPlayerIndex = Number(tokenPoolPlayerSelect.value);
  renderPlayerTokenPools();
});
createCardButton.addEventListener("click", openCreateCardDialog);
createCardClose.addEventListener("click", () => createCardDialog.close());
createCardCancel.addEventListener("click", () => createCardDialog.close());
createCardType.addEventListener("change", fillCreateCardPiles);
createCardForm.addEventListener("submit", submitCustomCard);
createCardDialog.addEventListener("click", (event) => {
  if (event.target === createCardDialog) createCardDialog.close();
});
opponentRename.addEventListener("click", () => {
  if (modalOpponentIndex !== null) renamePlayer(modalOpponentIndex);
});
showOpponentHand.addEventListener("change", renderOpponentHand);
collapseOpponentHands.addEventListener("click", () => {
  const expanded = collapseOpponentHands.getAttribute("aria-expanded") === "true";
  collapseOpponentHands.setAttribute("aria-expanded", String(!expanded));
  opponentHandArea.classList.toggle("hidden", expanded);
  collapseOpponentHands.querySelector("[data-collapse-label]").textContent = expanded ? "Expand" : "Collapse";
  collapseOpponentHands.querySelector("[data-collapse-chevron]").classList.toggle("rotate-180", expanded);
});

sourcePileEl.addEventListener("change", () => { selectedCard = null; selectionEl.textContent = "Select a card from the searched pile."; moveBtn.disabled = true; if (currentState) renderState(currentState); });
searchEl.addEventListener("input", () => { selectedCard = null; moveBtn.disabled = true; if (currentState) renderState(currentState); });
targetPileEl.addEventListener("change", () => { moveBtn.disabled = !selectedCard || selectedCard.pileId === targetPileEl.value; });
moveMenuBtn.addEventListener("click", () => {
  const isOpen = !moveMenu.classList.contains("hidden");
  moveMenu.classList.toggle("hidden", isOpen);
  moveMenuBtn.setAttribute("aria-expanded", String(!isOpen));
  moveChevron.classList.toggle("rotate-180", !isOpen);
  if (!isOpen) sourcePileEl.focus();
});
moveBtn.addEventListener("click", () => {
  if (!selectedCard) return;
  const { card, pileId } = selectedCard;
  runAction(() => game.move_card(card.id, pileId, targetPileEl.value), `Moved ${card.name}`).then(() => { selectedCard = null; moveBtn.disabled = true; selectionEl.textContent = "Select another card."; });
});
cardModalClose.addEventListener("click", closeCardModal);
cardModalPrevious.addEventListener("click", () => navigateModalCard(-1));
cardModalNext.addEventListener("click", () => navigateModalCard(1));
opponentModalClose.addEventListener("click", () => opponentModal.close());
opponentModal.addEventListener("close", () => { modalOpponentIndex = null; });
opponentModal.addEventListener("click", (event) => {
  if (event.target === opponentModal) opponentModal.close();
});
cardModal.addEventListener("close", () => { modalCard = null; });
cardModal.addEventListener("click", (event) => {
  if (event.target === cardModal) closeCardModal();
});
cardModalMove.addEventListener("click", async () => {
  if (!modalCard || !cardModalTarget.value) return;
  const { card, pileId } = modalCard;
  cardModalMove.disabled = true;
  const destination = cardModalTarget.value;
  const moved = await runAction(() => game.move_card(card.id, pileId, destination), `Moved ${card.name} to ${pileName(destination)}`);
  if (moved) closeCardModal();
  else cardModalMove.disabled = false;
});
cardModalTap.addEventListener("click", toggleModalCardTapped);
document.addEventListener("keydown", (event) => {
  if (!cardModal.open || event.metaKey || event.ctrlKey || event.altKey) return;
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement || target?.isContentEditable) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    navigateModalCard(-1);
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    navigateModalCard(1);
  } else if (event.key.toLocaleLowerCase() === "t" && !event.repeat) {
    event.preventDefault();
    toggleModalCardTapped();
  }
});
document.addEventListener("keydown", (event) => {
  if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
  if (cardModal.open || opponentModal.open || createCardDialog.open) return;
  const target = event.target;
  if (target instanceof HTMLInputElement || target instanceof HTMLSelectElement || target instanceof HTMLTextAreaElement || target?.isContentEditable) return;
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    cycleBoardPiece(event.key === "ArrowDown" ? 1 : -1);
  } else if (event.key === "[" || event.key === "]") {
    event.preventDefault();
    cyclePlayerBoard(event.key === "]" ? 1 : -1);
  }
});
cardModalFlip.addEventListener("click", async () => {
  if (!modalCard?.card.back_image) return;
  const { card } = modalCard;
  cardModalFlip.disabled = true;
  await runAction(
    () => game.set_card_flipped(card.id, !isCardFlipped(card)),
    `${isCardFlipped(card) ? "Showing front of" : "Showing back of"} ${card.name}`,
  );
  cardModalFlip.disabled = false;
});
cardModalBottom.addEventListener("click", async () => {
  if (!modalCard || !pileHasRole(modalCard.pileId, "draw")) return;
  const { card } = modalCard;
  cardModalBottom.disabled = true;
  const moved = await runAction(
    () => game.move_card_to_bottom(modalCard.pileId, card.id),
    `Moved ${card.name} to the bottom of the deck`,
  );
  if (moved) closeCardModal();
  else cardModalBottom.disabled = false;
});
cardCounterAdd.addEventListener("click", async () => {
  if (!modalCard) return;
  const { max } = cardCounterBounds(modalCard.card);
  if (max !== null && cardCounterValue(modalCard.card) >= max) return;
  cardCounterAdd.disabled = true;
  await runAction(() => game.add_card_counter(modalCard.card.id), `Added a counter to ${modalCard.card.name}`);
  if (modalCard) {
    const bounds = cardCounterBounds(modalCard.card);
    cardCounterAdd.disabled = bounds.max !== null && cardCounterValue(modalCard.card) >= bounds.max;
  }
});
cardCounterRemove.addEventListener("click", async () => {
  if (!modalCard || cardCounterValue(modalCard.card) <= cardCounterBounds(modalCard.card).min) return;
  cardCounterRemove.disabled = true;
  await runAction(() => game.remove_card_counter(modalCard.card.id), `Removed a counter from ${modalCard.card.name}`);
  if (modalCard) cardCounterRemove.disabled = cardCounterValue(modalCard.card) <= cardCounterBounds(modalCard.card).min;
});
boot();
