/* User-visible runtime-evidence invariants that do not require generated data or
 * a GPU. This executes the browser's real scenario, Fighter-draw, take request,
 * render-coverage, and render/tick matching helpers against a small runtime take.
 *
 *   node tools/ambition_moveset_inspector/check_runtime_contract.mjs
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const src = readFileSync(join(here, "web", "app.js"), "utf8");
const html = readFileSync(join(here, "web", "index.html"), "utf8");
const exporter = readFileSync(join(here, "..", "..", "game", "ambition_app_tools", "src", "bin", "moveset_export.rs"), "utf8");

let arcs = 0;
const ctx = new Proxy({}, {
  get: (_, key) => key === "arc" ? (() => { arcs++; }) : (() => {}),
  set: () => true,
});
const nodes = new Map();
function makeNode(tag = "div") {
  return new Proxy({
    tagName: tag.toUpperCase(), nodeType: 1,
    classList: { toggle() {}, add() {}, remove() {} },
    addEventListener() {}, replaceChildren() {}, append() {}, prepend() {},
    removeAttribute(name) { if (name === "src") this.src = null; },
    setAttribute(name, value) { this[name] = String(value); },
    getAttribute(name) { return this[name] ?? null; },
    getContext: () => ctx, hidden: false, style: {}, dataset: {},
    clientWidth: tag === "canvas" ? 760 : 1000,
    textContent: "", title: "", value: "0", max: "0", src: null,
  }, { get: (t, k) => (k in t ? t[k] : makeNode()), set: (t, k, v) => (t[k] = v, true) });
}
globalThis.document = {
  querySelector(selector) {
    if (!nodes.has(selector)) nodes.set(selector, makeNode(selector.includes("canvas") ? "canvas" : "div"));
    return nodes.get(selector);
  },
  querySelectorAll: () => [],
  createElement: (tag) => makeNode(tag),
  createTextNode: (text) => ({ nodeType: 3, textContent: String(text) }),
};
globalThis.window = { devicePixelRatio: 1 };
globalThis.Image = class {
  constructor() { this.complete = false; this.naturalWidth = 0; }
  addEventListener() {}
};
let fetchCalls = [];
globalThis.fetch = async (url, init = {}) => {
  fetchCalls.push({ url: String(url), init });
  return { ok: false, status: 503, json: async () => ({ reason: "fixture refusal" }) };
};

const api = new Function(
  `${src.replace(/\nboot\(\);\s*$/, "\n")}\n` +
  `return { canonicalScenario, scenarioKey, sameScenario, renderRequestKey,
    renderMoveDetail, drawRuntimeDiagnostic, syncEngineRender,
    requestTakeEvidence, requestRenderEvidence,
    sampledPlaybackTicks, nextPlaybackTick, playbackDelayMs, artUrl, rosterPortrait,
    TAKE_EVIDENCE, RENDERS,
    get state(){return state}, set BUNDLE(v){BUNDLE=v} };`
)();

const pass = [], fail = [];
const check = (name, condition, detail = "") =>
  (condition ? pass : fail).push(`${name}${detail ? " — " + detail : ""}`);

const move = {
  id: "down_smash", display_name: "Down Smash", verbs: ["smash_down"],
  derived: { startup_f: 8, active_f: 5, endlag_f: 18, max_damage: 14, max_knockback: 90 },
  duration_f: 31, windows: [],
};
const fighter = {
  id: "george", display_name: "George", on_smash_grid: true,
  verbs: { smash_down: "down_smash" }, moves: [move], vitals: {},
};
api.BUNDLE = { sim_hz: 60, sheets: {}, characters: [fighter], smash_grid: ["george"] };
api.state.fighter = "george";
api.state.move = "down_smash";
api.state.view = "fighter";
api.state.scenarioTarget = "__mirror__";
api.state.scenarioBehavior = "cpu";
api.state.scenarioSpacing = 40;

const scenario = api.canonicalScenario("george", "smash_down");
check("a mirror target is explicit", scenario.target === "george", JSON.stringify(scenario));
const passive = { ...scenario, target_behavior: "passive" };
check("CPU and passive mirrors have distinct browser identity",
  api.scenarioKey(scenario) !== api.scenarioKey(passive));

const subject = {
  id: "subject", role: "subject", label: "george", pos: [0, 0], half: [12, 24], facing: 1,
  hurtbox_source: "runtime_exact", hurtboxes: [{ pos: [0, 0], half: [10, 20] }],
  move_state: { id: "down_smash", phase: "Active", elapsed_s: 0.15, duration_s: 0.52,
                attack_facing: 1, landed_hit: false, instance: 0 },
};
const target = {
  id: "target", role: "target", label: "george", pos: [40, 0], half: [12, 24], facing: -1,
  hurtbox_source: "runtime_exact", hurtboxes: [{ pos: [40, 0], half: [10, 20] }],
};
const circle = {
  id: "strike", role: "subject_owned", pos: [20, 0], half: [9, 9],
  shape: { kind: "circle", center: [20, 0], radius: 9 }, overlaps: [],
};
const frame = { bodies: [subject, target], hitboxes: [circle], projectiles: [], contacts: [] };
const take = {
  character: "george", subject: "george", target: "george", target_behavior: "cpu",
  verb: "smash_down", requested_spacing: 40, intended_move: "down_smash",
  view: [-80, -60, 100, 60], platforms: [], frames: [frame, frame, frame, frame],
};
const measurements = {
  startup: { first_tick: 0, last_tick: 7, ticks: 8 },
  active: { first_tick: 8, last_tick: 12, ticks: 5 },
  recovery: { first_tick: 13, last_tick: 30, ticks: 18 },
  invuln: null, armor: null, first_active_tick: 8,
  live_volume_windows: [{ first_tick: 8, last_tick: 9, ticks: 2 }, { first_tick: 11, last_tick: 12, ticks: 2 }],
  live_volume_gaps: [{ first_tick: 10, last_tick: 10, ticks: 1 }],
  subject_travel_before_active: 0, subject_travel_during_active: 2,
  aabb_reach_bound_px: 29, target_overlap_ticks: 0, target_overlap_source: "runtime_exact",
  aabb_overlap_ticks: 1, aabb_overlap_source: "runtime_shape_bounds",
  first_contact_tick: null, target_launch_speed: null, spawns: [],
};
api.TAKE_EVIDENCE.set(api.scenarioKey(scenario), {
  state: "ready", scenario,
  doc: { scenario, cache_source: "scenario_cache", take, report: { measurements } },
});
arcs = 0;
api.renderMoveDetail(fighter, move);
check("Fighter view reaches the runtime circle/shape drawing path", arcs > 0, `${arcs} arc call(s)`);
check("the old Fighter prepared-volume renderer is gone",
  !src.includes("function drawHitboxes(") && !src.includes("m.windows.flatMap"));
check("both Fighter and Takes call the same runtime diagnostic renderer",
  (src.match(/drawRuntimeDiagnostic\(/g) || []).length >= 3);

/* Exact sampled-tick synchronization. */
api.state.view = "takes";
const renderKey = api.renderRequestKey(scenario, take.frames.length, 2);
api.RENDERS.set(renderKey, {
  state: "ready", scenario,
  doc: {
    available: true, scenario, stride: 2, renderer: "moveset_render",
    outcome: "ok", mismatch: false,
    shots: [
      { action_tick: 0, sim_tick: 100, file: "000.png" },
      { action_tick: 2, sim_tick: 102, file: "002.png" },
    ],
    urls: ["/render/000.png", "/render/002.png"],
  },
});
api.syncEngineRender(take, 1, scenario);
const image = nodes.get("#engine-render");
const note = nodes.get("#engine-render-note");
check("an unsampled diagnostic tick hides the GPU image", image.hidden === true);
check("an unsampled tick is explicit", /No GPU sample for action tick 1/.test(note.textContent), note.textContent);
api.syncEngineRender(take, 2, scenario);
check("an exact sampled tick is shown", image.hidden === false && image.src === "/render/002.png", `${image.hidden} ${image.src}`);
check("the displayed note names the exact action tick", /action tick 2/.test(note.textContent), note.textContent);
check("playback advances across sampled GPU ticks instead of visiting the unsampled gap",
  api.nextPlaybackTick(0, scenario, take.frames.length, 2) === 2,
  String(api.nextPlaybackTick(0, scenario, take.frames.length, 2)));
check("sample-aligned playback preserves real-time cadence",
  Math.abs(api.playbackDelayMs(scenario, take.frames.length, 2) - (1000 / 30)) < 0.01,
  String(api.playbackDelayMs(scenario, take.frames.length, 2)));
check("Fighter view pairs its runtime canvas with the same synchronization helper",
  /syncEngineRender\(take, state\.fighterFrame, scenario, \{ img: gpuImg/.test(src));

api.RENDERS.set(renderKey, {
  state: "ready", scenario,
  doc: {
    available: true, scenario: passive, stride: 2, outcome: "ok", mismatch: false,
    shots: [{ action_tick: 0, sim_tick: 100, file: "000.png" }],
    urls: ["/render/wrong.png"],
  },
});
api.syncEngineRender(take, 0, scenario);
check("browser refuses a GPU manifest for another scenario",
  image.hidden === true && /MISMATCH/.test(note.textContent), note.textContent);
const chainScenario = { ...scenario, chain: { verb: "attack", at: 37 } };
api.syncEngineRender(take, 0, chainScenario);
check("browser refuses GPU evidence for an unsupported chain scenario",
  image.hidden === true && /not available for this chain scenario/.test(note.textContent), note.textContent);

/* Coverage comes from the take horizon, never a fixed 24-frame request. */
api.state.view = "roster";
fetchCalls = [];
await api.requestRenderEvidence({ ...scenario, verb: "attack" }, 150, { stride: 2, force: true }).catch(() => {});
const renderCall = fetchCalls.find((row) => row.url === "/api/render");
const renderBody = renderCall ? JSON.parse(renderCall.init.body) : {};
check("render request covers the selected take horizon", renderBody.through_tick === 149, JSON.stringify(renderBody));
check("render request preserves stride explicitly", renderBody.stride === 2, JSON.stringify(renderBody));

/* Missing take asks the server for precisely one canonical scenario. */
fetchCalls = [];
const newScenario = { ...scenario, verb: "attack", spacing: 55 };
await api.requestTakeEvidence(newScenario, { force: true }).catch(() => {});
const takeCall = fetchCalls.find((row) => row.url === "/api/take");
const takeBody = takeCall ? JSON.parse(takeCall.init.body) : {};
check("interactive take generation calls the one-scenario API", !!takeCall);
check("take generation sends the explicit CPU mirror", takeBody.scenario &&
  takeBody.scenario.subject === "george" && takeBody.scenario.target === "george" &&
  takeBody.scenario.target_behavior === "cpu", JSON.stringify(takeBody));
check("regeneration is carried as a force request", takeBody.force === true, JSON.stringify(takeBody));

check("the UI exposes all three regeneration controls",
  /id="regen-take"/.test(html) && /id="regen-render"/.test(html) && /id="regen-all"/.test(html));
check("Fighter exposes its own Play/Pause control",
  /fighterPlaying/.test(src) && /runFighterPlayback/.test(src) && /fighter-runtime-scrub/.test(src));
check("asset-relative portrait paths normalize onto the existing art route",
  api.artUrl("sprites/george_portraits.png") === "/art/george_portraits.png",
  api.artUrl("sprites/george_portraits.png"));
check("the roster renders resolved portrait art when exported",
  /rosterPortrait\(c\)/.test(src) && /portrait_art/.test(exporter) && /resolve_still/.test(exporter));

console.log("== PASS ==");
for (const row of pass) console.log("  ok   " + row);
if (fail.length) { console.log("== FAIL =="); for (const row of fail) console.log("  FAIL " + row); }
console.log(`\n${pass.length} passed, ${fail.length} failed`);
process.exit(fail.length ? 1 : 0);
