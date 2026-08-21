import init, { WasmGame } from "../pkg/byog.js";

const statusEl = document.getElementById("status");
const zonesEl = document.getElementById("zones");
const drawBtn = document.getElementById("draw-btn");
const autoPlayBtn = document.getElementById("auto-play-btn");
const discardBtn = document.getElementById("discard-btn");
const energyBtn = document.getElementById("energy-btn");

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

function parseState(json) {
  return JSON.parse(json);
}

function renderState(state) {
  zonesEl.innerHTML = "";

  for (const zone of state.zones) {
    const panel = document.createElement("article");
    panel.className = `rounded-lg border bg-slate-900/80 p-3 ${zoneClassMap[zone.id] ?? "border-slate-700"}`;

    const heading = document.createElement("div");
    heading.className = "mb-2 flex items-center justify-between";
    heading.innerHTML = `<h2 class=\"font-semibold\">${zone.id}</h2><span class=\"text-xs text-slate-400\">${zone.cards.length} cards</span>`;
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
        item.innerHTML = `<div class=\"text-sm font-medium\">${card.name}</div><div class=\"text-xs text-slate-400\">${card.card_type} · ${card.id}</div>`;
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
  try {
    const next = parseState(action());
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
    const state = parseState(game.state_json());
    renderState(state);
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
