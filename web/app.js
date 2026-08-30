import init, { WasmGame } from "../pkg/byog.js?v=14";
import protobuf from "https://cdn.jsdelivr.net/npm/protobufjs@7.4.0/+esm";

const statusEl = document.getElementById("status");
const boardEl = document.getElementById("board");
const handAreaEl = document.getElementById("hand-area");
const handSummaryCount = document.getElementById("hand-summary-count");
const handOwnerLabel = document.getElementById("hand-owner-label");
const activePlayerLabel = document.getElementById("active-player-label");
const zonesOwnerLabel = document.getElementById("zones-owner-label");
const zonesOwnerDescription = document.getElementById("zones-owner-description");
const opponentHandLabel = document.getElementById("opponent-hand-label");
const opponentHandCount = document.getElementById("opponent-hand-count");
const opponentHandArea = document.getElementById("opponent-hand-area");
const showOpponentHand = document.getElementById("show-opponent-hand");
const collapseOpponentHands = document.getElementById("collapse-opponent-hands");
const playerSelect = document.getElementById("player-select");
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
const cardModalPile = document.getElementById("card-modal-pile");
const cardModalTitle = document.getElementById("card-modal-title");
const cardViewName = document.getElementById("card-view-name");
const cardViewType = document.getElementById("card-view-type");
const cardViewId = document.getElementById("card-view-id");
const cardView = document.getElementById("card-view");
const cardModalTap = document.getElementById("card-modal-tap");
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

// ── Colour mappings ───────────────────────────────────────────────────────────
const zoneColorMap = {
  MainStack:    "border-indigo-300",
  CommanderPile:"border-violet-300",
  Hand:         "border-cyan-300",
  Lands:        "border-lime-300",
  Deck:         "border-sky-300",
  Discard:      "border-rose-300",
  Exile:        "border-orange-300",
  Artifacts:    "border-amber-300",
  Enchantments: "border-fuchsia-300",
  Creatures:    "border-emerald-300",
};

// Maps pile id → Zone display name (matches Zone::to_string() in Rust)
const pileIdToZoneId = {
  commander_pile: "CommanderPile",
  main_stack:     "MainStack",
  deck:           "Deck",
  lands:          "Lands",
  artifacts:      "Artifacts",
  enchantments:   "Enchantments",
  creatures:      "Creatures",
  discard:        "Discard",
  exile:          "Exile",
  hand:           "Hand",
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
}
message PileViewProto {
  string id = 1;
  string name = 2;
  string zone_id = 3;
  uint32 x = 4;
  uint32 y = 5;
  repeated string associated_piles = 6;
  bool visible = 7;
}
`;

const root = protobuf.parse(protoSchema, { keepCase: true }).root;
const GameStateSnapshotProto = root.lookupType("GameStateSnapshotProto");
const BoardLayoutProto = root.lookupType("BoardLayoutProto");

// ── State ─────────────────────────────────────────────────────────────────────
let game;
let games = [];
let playerStates = [];
let activePlayerIndex = 0;
let boardLayout = null;   // { zones: ZoneLayoutProto[], piles: PileViewProto[] }
let currentState = null;
let selectedCard = null;
let modalCard = null;
let modalOpponentIndex = null;
const revealedPileCards = new Map();
// pile id → panel element
const pilePanels = new Map();

// ── Decode helpers ────────────────────────────────────────────────────────────
function decodeState(bytes) {
  return GameStateSnapshotProto.toObject(GameStateSnapshotProto.decode(bytes), { defaults: true });
}
function decodeBoardLayout(bytes) {
  return BoardLayoutProto.toObject(BoardLayoutProto.decode(bytes), { defaults: true });
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

function cardCounterValue(card) {
  return cardPool(card, "counters")?.count ?? 0;
}

function cardCounterBounds(card) {
  const pool = cardPool(card, "counters");
  return { min: pool?.min ?? 0, max: pool?.max ?? null };
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

function createTokenPool(pool, controls = false) {
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
    remove.addEventListener("click", () => runAction(() => game.remove_player_tokens(pool.id, pool.minus), `Removed ${pool.minus} ${pool.label}`));
    const add = document.createElement("button");
    add.type = "button";
    add.className = "grid h-9 w-9 place-items-center rounded-full border border-current/30 text-2xl leading-none transition hover:bg-white/60 focus:outline-none focus:ring-2 focus:ring-current";
    add.setAttribute("aria-label", `Add ${pool.plus} ${pool.label}`);
    add.textContent = "+";
    add.addEventListener("click", () => runAction(() => game.add_player_tokens(pool.id, pool.plus), `Added ${pool.plus} ${pool.label}`));
    item.prepend(remove);
    item.append(add);
  }
  return item;
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
    const bytes = change > 0
      ? games[index].add_player_tokens(poolId, playerStates[index].players[0].token_pools.find((pool) => pool.id === poolId)?.plus ?? 1)
      : games[index].remove_player_tokens(poolId, playerStates[index].players[0].token_pools.find((pool) => pool.id === poolId)?.minus ?? 1);
    playerStates[index] = decodeState(bytes);
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
  renderPlayerTabs();
  switchPlayer(games.length - 1);
}

function updatePlayerControls() {
  const name = playerName(activePlayerIndex);
  activePlayerLabel.textContent = name;
  handOwnerLabel.textContent = `${name}'s hand`;
  zonesOwnerLabel.textContent = `${name} zones`;
  zonesOwnerDescription.textContent = `Showing ${name}'s piles and token counts.`;
  playerSelect.value = String(activePlayerIndex);
  playerBoardPanel.setAttribute("aria-label", `${name} board`);
}

function renamePlayer(index) {
  const currentName = playerName(index);
  const name = window.prompt("Player name", currentName)?.trim();
  if (!name || name === currentName) return;
  try {
    playerStates[index] = decodeState(games[index].set_player_name(name));
    renderPlayerTabs();
    updatePlayerControls();
    renderOpponentHand();
    if (modalOpponentIndex === index) renderOpponentModal();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>Renamed player to ${name}`;
  } catch (err) {
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-rose-400"></span>Rename failed: ${err}`;
  }
}

function switchPlayer(index) {
  if (!games[index] || index === activePlayerIndex) return;
  if (cardModal.open) closeCardModal();
  if (opponentModal.open) opponentModal.close();
  activePlayerIndex = index;
  game = games[index];
  selectedCard = null;
  selectionEl.textContent = "Select a card from the searched pile.";
  moveBtn.disabled = true;
  updatePlayerControls();
  renderState(playerStates[index]);
  statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>${playerName(index)}'s turn`;
}

function createCardComponent(card, pileId) {
  const item = document.createElement("li");
  const button = document.createElement("button");
  button.type = "button";
  const usesCardLayout = pileId === "hand"
    || pileId === "opponent-hand"
    || Boolean(boardLayout?.piles.some((pile) => pile.id === pileId));
  if (usesCardLayout) item.className = "w-24 shrink-0 snap-start sm:w-28";
  button.className = usesCardLayout
    ? `group flex aspect-[5/7] w-full flex-col overflow-hidden rounded-xl border bg-gradient-to-br from-gray-900 via-gray-800 to-gray-950 p-2.5 text-left text-white shadow-md transition hover:-translate-y-1 hover:border-cyan-400 hover:shadow-lg focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:ring-offset-2 ${isCardTapped(card) ? "rotate-2 border-amber-300" : "border-gray-700"}`
    : `group flex w-full items-center justify-between gap-2 rounded-lg border border-gray-200 bg-gray-50 px-2.5 py-2 text-left transition hover:border-brand-500 hover:bg-brand-50 focus:outline-none focus:ring-2 focus:ring-brand-500 focus:ring-offset-1 ${isCardTapped(card) ? "rotate-2 border-amber-300 bg-amber-50" : ""}`;
  button.title = `View ${card.name}`;
  button.innerHTML = usesCardLayout
    ? `<span class="block w-full truncate border-b border-white/15 pb-2 text-xs font-semibold" data-card-name></span><span class="my-2 grid w-full flex-1 place-items-center rounded-lg border border-white/10 bg-gradient-to-br from-cyan-500/35 to-violet-500/20 text-3xl text-white/75" aria-hidden="true">✦</span><span class="flex w-full items-end justify-between gap-2"><span class="min-w-0 truncate text-[10px] text-white/60" data-card-type></span><span data-counter-badge class="hidden min-w-5 rounded-full bg-brand-500 px-1.5 py-0.5 text-center text-[9px] font-semibold text-white"></span></span>`
    : `<span class="min-w-0"><span class="block truncate text-[11px] font-medium text-gray-800" data-card-name></span><span class="mt-0.5 block truncate text-[9px] text-gray-400" data-card-type></span></span><span class="flex shrink-0 items-center gap-1.5"><span data-counter-badge class="hidden min-w-5 rounded-full bg-brand-500 px-1.5 py-0.5 text-center text-[9px] font-semibold text-white"></span><span class="text-xs text-gray-300 transition group-hover:translate-x-0.5 group-hover:text-brand-500" aria-hidden="true">→</span></span>`;
  button.querySelector("[data-card-name]").textContent = card.name;
  button.querySelector("[data-card-type]").textContent = card.card_type;
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
  modalCard = { card, pileId };
  renderCardModal();
  cardModalTarget.innerHTML = boardLayout.piles
    .filter((pile) => pile.id !== pileId)
    .map((pile) => `<option value="${pile.id}">${pile.name}</option>`)
    .join("");
  cardModalMove.disabled = !cardModalTarget.value;
  cardModal.showModal();
}

function renderCardModal() {
  if (!modalCard) return;
  const { card, pileId } = modalCard;
  cardModalPile.textContent = pileName(pileId);
  cardModalTitle.textContent = card.name;
  cardViewName.textContent = card.name;
  cardViewType.textContent = card.card_type || "Card";
  cardViewId.textContent = `Card ID: ${card.id}`;
  const tapped = isCardTapped(card);
  cardView.classList.toggle("rotate-90", tapped);
  cardView.classList.toggle("my-10", tapped);
  cardModalTap.textContent = tapped ? "Untap" : "Tap";
  cardModalTap.setAttribute("aria-pressed", String(tapped));
  cardModalBottom.classList.toggle("hidden", pileId !== "deck");
  cardModalBottom.disabled = false;
  const counters = cardCounterValue(card);
  const { min, max } = cardCounterBounds(card);
  cardCounterCount.textContent = String(counters);
  cardCounterCount.title = `Allowed range: ${min}–${max ?? "unlimited"}`;
  cardCounterRemove.disabled = counters <= min;
  cardCounterAdd.disabled = max !== null && counters >= max;
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

  const parentZones = new Map();
  for (const zone of layout.zones.filter((candidate) =>
    layout.zones.some((child) => child.parent_zone === candidate.id)
  )) {
    const container = document.createElement("section");
    container.className = "zone-parent relative grid min-w-0 gap-2 rounded-xl border border-emerald-200 bg-emerald-50/40 px-2 pb-2 pt-8";
    container.style.gridTemplateColumns = `repeat(${zone.width}, minmax(0, 1fr))`;
    container.style.gridColumn = `${zone.x + 1} / span ${zone.width}`;
    container.style.gridRow = `${zone.y + 1} / span ${zone.height}`;
    container.dataset.zoneId = zone.id;
    const label = document.createElement("h2");
    label.className = "absolute left-3 top-2 text-xs font-semibold uppercase tracking-wide text-emerald-700";
    label.textContent = zone.name;
    container.appendChild(label);
    boardEl.appendChild(container);
    parentZones.set(zone.id, container);
  }

  // Pile panels
  for (const pile of layout.piles) {
    const panel = document.createElement("article");
    const zoneId = pileIdToZoneId[pile.id] ?? pile.id;
    const borderCls = zoneColorMap[zoneId] ?? "border-gray-200";
    panel.className = `pile-panel rounded-xl border bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md ${borderCls}`;
    const zoneLayout = layout.zones.find((zone) => zone.id === pile.zone_id);
    panel.style.gridColumn = `${(zoneLayout?.x ?? pile.x) + 1} / span ${zoneLayout?.width ?? 8}`;
    panel.style.gridRow = `${(zoneLayout?.y ?? pile.y) + 1} / span ${zoneLayout?.height ?? 4}`;
    panel.dataset.pileId   = pile.id;

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

    const cardList = document.createElement("ul");
    cardList.className = "flex w-full snap-x snap-proximity gap-2 overflow-x-auto overflow-y-hidden overscroll-x-contain pb-2";
    cardList.dataset.cardList = "true";
    panel.appendChild(cardList);

    if (!pile.visible) {
      const controls = document.createElement("div");
      controls.className = "mt-2 grid grid-cols-2 gap-1.5";
      const destinations = layout.piles
        .filter((candidate) => candidate.id !== pile.id)
        .map((candidate) => `<option value="${candidate.id}" ${candidate.id === "hand" ? "selected" : ""}>${candidate.name}</option>`)
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
        const zoneId = pileIdToZoneId[pile.id] ?? pile.id;
        const cards = currentState?.zones.find((zone) => zone.id === zoneId)?.cards ?? [];
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

    if (pile.id === "hand") {
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

// ── Update pile panels from game state ───────────────────────────────────────
function renderState(state) {
  currentState = state;
  playerStates[activePlayerIndex] = state;
  // Build a map zone display-id → zone data
  const zoneMap = Object.fromEntries(state.zones.map((z) => [z.id, z]));

  const playerTokens = document.getElementById("player-tokens");
  playerTokens.innerHTML = "";
  for (const player of state.players) {
    for (const pool of player.token_pools.filter((candidate) => candidate.active)) {
      playerTokens.appendChild(createTokenPool(pool, true));
    }
  }

  if (modalCard) {
    const freshCard = state.zones.flatMap((zone) => zone.cards).find((card) => card.id === modalCard.card.id);
    if (freshCard) {
      modalCard.card = freshCard;
      renderCardModal();
    }
  }

  for (const [pileId, panel] of pilePanels) {
    const zoneId = pileIdToZoneId[pileId];
    const zone = zoneMap[zoneId];
    if (!zone) continue;

    const countEl  = panel.querySelector("[data-card-count]");
    const listEl   = panel.querySelector("[data-card-list]");
    const tokenEl  = panel.querySelector("[data-token-wrap]");
    const hiddenActionEls = panel.querySelectorAll("[data-move-hidden], [data-reveal-hidden], [data-shuffle-hidden]");

    countEl.textContent = `${zone.cards.length} card${zone.cards.length !== 1 ? "s" : ""}`;
    for (const action of hiddenActionEls) action.disabled = zone.cards.length === 0;
    if (pileId === "hand") handSummaryCount.textContent = countEl.textContent;

    listEl.innerHTML = "";
    const pile = boardLayout.piles.find((candidate) => candidate.id === pileId);
    const query = searchEl.value.trim().toLocaleLowerCase();
    const isSelectedSearchPile = sourcePileEl.value === pileId;
    const searchableCards = zone.cards.filter((card) =>
      !query || [card.id, card.name, card.card_type].some((value) => value.toLocaleLowerCase().includes(query))
    );
    const revealed = revealedPileCards.get(pileId);
    const revealedCards = revealed ? zone.cards.filter((card) => revealed.has(card.id)) : [];
    const cardsToShow = pile?.visible ? (isSelectedSearchPile && query ? searchableCards : zone.cards) : (isSelectedSearchPile && query ? searchableCards : revealedCards);
    if (!cardsToShow.length) {
      const message = !pile?.visible && !query ? "Cards hidden" : (query && isSelectedSearchPile ? "No matches" : "Empty");
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
    const next = decodeState(await action());
    revealedPileCards.clear();
    statusEl.innerHTML = `<span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>${message}`;
    renderState(next);
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
    await init(new URL("../pkg/byog_bg.wasm?v=14", import.meta.url));
    games = [new WasmGame(), new WasmGame()];
    games[1].set_player_name("Player 2");
    game = games[activePlayerIndex];

    boardLayout = decodeBoardLayout(game.board_layout_proto());
    fillPileSelectors();
    buildBoard(boardLayout);
    applyLandscapeGroups(boardLayout);

    playerStates = games.map((playerGame) => decodeState(playerGame.state_proto()));
    const state = playerStates[activePlayerIndex];
    renderPlayerTabs();
    renderState(state);

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

playerSelect.addEventListener("change", () => switchPlayer(Number(playerSelect.value)));
addPlayerButton.addEventListener("click", addPlayer);
renamePlayerButton.addEventListener("click", () => renamePlayer(activePlayerIndex));
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
cardModalTap.addEventListener("click", async () => {
  if (!modalCard) return;
  const { card } = modalCard;
  cardModalTap.disabled = true;
  await runAction(
    () => game.set_card_tapped(card.id, !isCardTapped(card)),
    `${isCardTapped(card) ? "Untapped" : "Tapped"} ${card.name}`,
  );
  cardModalTap.disabled = false;
});
cardModalBottom.addEventListener("click", async () => {
  if (!modalCard || modalCard.pileId !== "deck") return;
  const { card } = modalCard;
  cardModalBottom.disabled = true;
  const moved = await runAction(
    () => game.move_card_to_bottom("deck", card.id),
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
