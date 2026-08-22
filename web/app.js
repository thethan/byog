import init, { WasmGame } from "../pkg/byog.js";
import protobuf from "https://cdn.jsdelivr.net/npm/protobufjs@7.4.0/+esm";

const statusEl = document.getElementById("status");
const boardEl = document.getElementById("board");
const drawBtn = document.getElementById("draw-btn");
const autoPlayBtn = document.getElementById("auto-play-btn");
const discardBtn = document.getElementById("discard-btn");
const energyBtn = document.getElementById("energy-btn");
const controls = [drawBtn, autoPlayBtn, discardBtn, energyBtn];

// ── Colour mappings ───────────────────────────────────────────────────────────
const zoneColorMap = {
  MainStack:    "border-indigo-700/70",
  CommanderPile:"border-violet-700/70",
  Hand:         "border-cyan-700/70",
  Lands:        "border-lime-700/70",
  Deck:         "border-sky-700/70",
  Discard:      "border-rose-700/70",
  Exile:        "border-orange-700/70",
  Artifacts:    "border-amber-700/70",
  Enchantments: "border-fuchsia-700/70",
  Creatures:    "border-emerald-700/70",
};

// Tailwind background colours for zone region overlays (semi-transparent)
const regionBgMap = {
  violet:  "bg-violet-900",
  cyan:    "bg-cyan-900",
  emerald: "bg-emerald-900",
  sky:     "bg-sky-900",
  indigo:  "bg-indigo-900",
  rose:    "bg-rose-900",
  orange:  "bg-orange-900",
  lime:    "bg-lime-900",
  amber:   "bg-amber-900",
  fuchsia: "bg-fuchsia-900",
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
}
message TokenPoolViewProto {
  string id = 1;
  string label = 2;
  string token = 3;
  optional string background = 4;
  uint32 count = 5;
  bool active = 6;
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
}
message PileViewProto {
  string id = 1;
  string name = 2;
  string zone_id = 3;
  uint32 x = 4;
  uint32 y = 5;
  repeated string associated_piles = 6;
}
`;

const root = protobuf.parse(protoSchema, { keepCase: true }).root;
const GameStateSnapshotProto = root.lookupType("GameStateSnapshotProto");
const BoardLayoutProto = root.lookupType("BoardLayoutProto");

// ── State ─────────────────────────────────────────────────────────────────────
let game;
let boardLayout = null;   // { zones: ZoneLayoutProto[], piles: PileViewProto[] }
// pile id → panel element
const pilePanels = new Map();

for (const control of controls) control.disabled = true;

// ── Decode helpers ────────────────────────────────────────────────────────────
function decodeState(bytes) {
  return GameStateSnapshotProto.toObject(GameStateSnapshotProto.decode(bytes), { defaults: true });
}
function decodeBoardLayout(bytes) {
  return BoardLayoutProto.toObject(BoardLayoutProto.decode(bytes), { defaults: true });
}

// ── Render board layout (zone regions + pile panels) ─────────────────────────
function buildBoard(layout) {
  boardEl.innerHTML = "";
  pilePanels.clear();

  // Zone region overlays
  for (const zone of layout.zones) {
    const overlay = document.createElement("div");
    const bg = regionBgMap[zone.color] ?? "bg-slate-800";
    overlay.className = `zone-region ${bg}`;
    overlay.style.gridColumn = `${zone.x + 1} / span ${zone.width}`;
    overlay.style.gridRow    = `${zone.y + 1} / span ${zone.height}`;
    // Add zone name label in corner
    const label = document.createElement("span");
    label.className = "absolute top-1 left-1 text-[9px] font-semibold text-white/40 uppercase tracking-wider pointer-events-none";
    label.textContent = zone.name;
    overlay.style.position = "relative";
    overlay.appendChild(label);
    boardEl.appendChild(overlay);
  }

  // Pile panels
  for (const pile of layout.piles) {
    const panel = document.createElement("article");
    const zoneId = pileIdToZoneId[pile.id] ?? pile.id;
    const borderCls = zoneColorMap[zoneId] ?? "border-slate-700";
    panel.className = `pile-panel rounded border bg-slate-900/90 p-2 ${borderCls}`;
    panel.style.gridColumn = `${pile.x + 1} / span 8`;
    panel.style.gridRow    = `${pile.y + 1} / span 4`;
    panel.dataset.pileId   = pile.id;

    const heading = document.createElement("div");
    heading.className = "mb-1 flex items-center justify-between";

    const title = document.createElement("h2");
    title.className = "text-xs font-semibold truncate";
    title.textContent = pile.name;

    const count = document.createElement("span");
    count.className = "text-[10px] text-slate-400 ml-1 shrink-0";
    count.dataset.cardCount = "true";
    count.textContent = "0 cards";

    heading.appendChild(title);
    heading.appendChild(count);
    panel.appendChild(heading);

    const cardList = document.createElement("ul");
    cardList.className = "space-y-0.5 overflow-y-auto max-h-[60%]";
    cardList.dataset.cardList = "true";
    panel.appendChild(cardList);

    const tokenWrap = document.createElement("div");
    tokenWrap.className = "flex flex-wrap gap-1 mt-1";
    tokenWrap.dataset.tokenWrap = "true";
    panel.appendChild(tokenWrap);

    boardEl.appendChild(panel);
    pilePanels.set(pile.id, panel);
  }
}

// ── Update pile panels from game state ───────────────────────────────────────
function renderState(state) {
  // Build a map zone display-id → zone data
  const zoneMap = Object.fromEntries(state.zones.map((z) => [z.id, z]));

  for (const [pileId, panel] of pilePanels) {
    const zoneId = pileIdToZoneId[pileId];
    const zone = zoneMap[zoneId];
    if (!zone) continue;

    const countEl  = panel.querySelector("[data-card-count]");
    const listEl   = panel.querySelector("[data-card-list]");
    const tokenEl  = panel.querySelector("[data-token-wrap]");

    countEl.textContent = `${zone.cards.length} card${zone.cards.length !== 1 ? "s" : ""}`;

    listEl.innerHTML = "";
    if (!zone.cards.length) {
      listEl.innerHTML = '<li class="text-[10px] text-slate-500 italic">Empty</li>';
    } else {
      for (const card of zone.cards) {
        const li = document.createElement("li");
        li.className = "text-[10px] leading-tight border border-slate-700/50 rounded px-1 py-0.5 bg-slate-950 truncate";
        li.title = `${card.name} · ${card.card_type} · ${card.id}`;
        li.textContent = card.name;
        listEl.appendChild(li);
      }
    }

    tokenEl.innerHTML = "";
    for (const pool of zone.token_pools) {
      const pill = document.createElement("span");
      pill.className = "rounded-full border border-slate-700 bg-slate-800 px-1.5 py-0.5 text-[10px]";
      pill.textContent = `${pool.label}: ${pool.count}`;
      tokenEl.appendChild(pill);
    }
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
  if (!game) { statusEl.textContent = "Game is still initialising."; return; }
  try {
    const next = decodeState(await action());
    statusEl.textContent = message;
    renderState(next);
  } catch (err) {
    statusEl.textContent = `Action failed: ${err}`;
  }
}

async function boot() {
  try {
    await init();
    game = new WasmGame();

    boardLayout = decodeBoardLayout(game.board_layout_proto());
    buildBoard(boardLayout);
    applyLandscapeGroups(boardLayout);

    const state = decodeState(game.state_proto());
    renderState(state);

    for (const control of controls) control.disabled = false;
    statusEl.textContent = "Ready";
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

drawBtn.addEventListener("click",    () => runAction(() => game.draw(), "Drew one card"));
autoPlayBtn.addEventListener("click",() => runAction(() => game.auto_play_first_hand_card(), "Auto played first hand card"));
discardBtn.addEventListener("click", () => runAction(() => game.discard_first_hand_card(), "Discarded first hand card"));
energyBtn.addEventListener("click",  () => runAction(() => game.add_hand_energy(1), "Added one hand energy"));

boot();

const statusEl = document.getElementById("status");
const zonesEl = document.getElementById("zones");
const drawBtn = document.getElementById("draw-btn");
const autoPlayBtn = document.getElementById("auto-play-btn");
const discardBtn = document.getElementById("discard-btn");
const energyBtn = document.getElementById("energy-btn");
const controls = [drawBtn, autoPlayBtn, discardBtn, energyBtn];

const zoneClassMap = {
  MainStack: "border-indigo-700/70",
  CommanderPile: "border-violet-700/70",
  Hand: "border-cyan-700/70",
  LandPile: "border-lime-700/70",
  Deck: "border-sky-700/70",
  Discard: "border-rose-700/70",
  Exile: "border-orange-700/70",
  ArtifactList: "border-amber-700/70",
  EnchantmentList: "border-fuchsia-700/70",
  CreatureList: "border-emerald-700/70",
};

let game;
const protoSchema = `
syntax = "proto3";

message GameStateSnapshotProto {
  repeated ZoneViewProto zones = 1;
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
}

message TokenPoolViewProto {
  string id = 1;
  string label = 2;
  string token = 3;
  optional string background = 4;
  uint32 count = 5;
  bool active = 6;
}
`;
const root = protobuf.parse(protoSchema, { keepCase: true }).root;
const GameStateSnapshotProto = root.lookupType("GameStateSnapshotProto");

for (const control of controls) {
  control.disabled = true;
}

function decodeState(protoBytes) {
  const state = GameStateSnapshotProto.decode(protoBytes);
  return GameStateSnapshotProto.toObject(state, {
    defaults: true,
  });
}

function renderState(state) {
  zonesEl.innerHTML = "";

  for (const zone of state.zones) {
    const panel = document.createElement("article");
    panel.className = `rounded-lg border bg-slate-900/80 p-3 ${zoneClassMap[zone.id] ?? "border-slate-700"}`;

    const heading = document.createElement("div");
    heading.className = "mb-2 flex items-center justify-between";
    const headingTitle = document.createElement("h2");
    headingTitle.className = "font-semibold";
    headingTitle.textContent = zone.id;
    const headingCount = document.createElement("span");
    headingCount.className = "text-xs text-slate-400";
    headingCount.textContent = `${zone.cards.length} cards`;
    heading.appendChild(headingTitle);
    heading.appendChild(headingCount);
    panel.appendChild(heading);

    const cards = document.createElement("ul");
    cards.className = zone.battlefield
      ? "mb-2 grid grid-cols-1 gap-2 sm:grid-cols-2"
      : "mb-2 space-y-2";

    if (!zone.cards.length) {
      cards.innerHTML = '<li class="rounded border border-dashed border-slate-700 px-2 py-1 text-xs text-slate-500">Empty</li>';
    } else {
      for (const card of zone.cards) {
        const item = document.createElement("li");
        item.className = "rounded border border-slate-700 bg-slate-950 px-2 py-1";
        const title = document.createElement("div");
        title.className = "text-sm font-medium";
        title.textContent = card.name;
        const meta = document.createElement("div");
        meta.className = "text-xs text-slate-400";
        meta.textContent = `${card.card_type} · ${card.id}`;
        item.appendChild(title);
        item.appendChild(meta);
        cards.appendChild(item);
      }
    }
    panel.appendChild(cards);

    const tokenWrap = document.createElement("div");
    tokenWrap.className = "flex flex-wrap gap-2";
    if (zone.token_pools.length) {
      for (const pool of zone.token_pools) {
        const pill = document.createElement("span");
        pill.className = "rounded-full border border-slate-700 bg-slate-800 px-2 py-1 text-xs";
        pill.textContent = `${pool.label}: ${pool.count}`;
        tokenWrap.appendChild(pill);
      }
    }
    panel.appendChild(tokenWrap);

    zonesEl.appendChild(panel);
  }
}

async function runAction(action, message) {
  if (!game) {
    statusEl.textContent = "Game is still initializing.";
    return;
  }
  try {
    const next = decodeState(await action());
    statusEl.textContent = message;
    renderState(next);
  } catch (error) {
    statusEl.textContent = `Action failed: ${error}`;
  }
}

async function boot() {
  try {
    await init();
    game = new WasmGame();
    const state = decodeState(game.state_proto());
    renderState(state);
    for (const control of controls) {
      control.disabled = false;
    }
    statusEl.textContent = "Ready";
  } catch (error) {
    statusEl.textContent = `Initialization failed: ${error}`;
  }
}

drawBtn.addEventListener("click", () => runAction(() => game.draw(), "Drew one card"));
autoPlayBtn.addEventListener("click", () =>
  runAction(() => game.auto_play_first_hand_card(), "Auto played first hand card")
);
discardBtn.addEventListener("click", () =>
  runAction(() => game.discard_first_hand_card(), "Discarded first hand card")
);
energyBtn.addEventListener("click", () =>
  runAction(() => game.add_hand_energy(1), "Added one hand energy")
);

boot();
