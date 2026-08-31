/* Does the takes view list what EXISTS, or only what has been RECORDED?
 *
 * ⛔⛔ THE FAILURE THIS EXISTS FOR LOOKS LIKE MISSING DATA. The fighter picker
 * derived its list from `TAKES.takes`, so a fighter appeared in this view only
 * once somebody had recorded a bulk take for it — and the page then answered
 * "why can I only select two fighters?" with a roster that was honestly
 * reporting the cache while appearing to report the game. A missing recording is
 * missing EVIDENCE; it is not a missing fighter, and the two must never look the
 * same.
 *
 *   node tools/ambition_moveset_inspector/check_takes_discovery.mjs
 *
 * ⭐ IT DRIVES THE REAL FUNCTIONS. `app.js` is loaded against the same DOM stub
 * `check_draw_path.mjs` uses and its own discovery path is called with
 * synthesised bundles, so this fails if the browser's answer changes rather than
 * if a phrase in the source does.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const src = readFileSync(join(here, "web", "app.js"), "utf8");

const ctx = new Proxy({}, { get: (_, k) => (k === "canvas" ? {} : () => {}), set: () => true });
const node = () => new Proxy({
  classList: { toggle() {}, add() {}, remove() {} },
  addEventListener() {}, replaceChildren() {}, append() {},
  removeAttribute() {}, setAttribute() {}, getAttribute() { return null; }, hidden: false,
  getContext: () => ctx, style: {}, dataset: {},
  clientWidth: 1000, textContent: "", title: "", value: "0", max: "0",
}, { get: (t, k) => (k in t ? t[k] : node()), set: (t, k, v) => (t[k] = v, true) });
globalThis.document = {
  querySelector: () => node(), querySelectorAll: () => [],
  createElement: () => node(),
  createTextNode: (t) => ({ nodeType: 3, textContent: String(t) }),
};
globalThis.window = { devicePixelRatio: 1 };
globalThis.Image = class { constructor() { this.complete = false; this.naturalWidth = 0; } addEventListener() {} };
globalThis.fetch = async () => ({ json: async () => ({}) });

const api = new Function(
  `${src.replace(/\nboot\(\);\s*$/, "\n")}\n` +
  "return { takeRoster, takeSlotsFor, roleOf, renderTakeOptions, drawTake," +
  " get state(){return state}, set BUNDLE(v){BUNDLE=v}, set TAKES(v){TAKES=v} };"
)();

const ok = [], bad = [];
const check = (name, cond, detail = "") => (cond ? ok : bad).push(`${name}${detail ? " — " + detail : ""}`);

/* A prepared cast of five grid fighters, each binding three moves. */
const fighter = (id) => ({
  id,
  display_name: id.toUpperCase(),
  on_smash_grid: true,
  verbs: { attack: {}, smash_forward: {}, special_up: {} },
  moves: [],
});
const GRID = ["alpha", "beta", "gamma", "delta", "epsilon"];
api.BUNDLE = { characters: GRID.map(fighter), smash_grid: GRID };

/* ---- 1. every prepared fighter is selectable with ZERO recordings ---- */
api.TAKES = null;
let roster = api.takeRoster();
check("a bundle of 5 fighters offers 5 with no takes at all",
  roster.length === 5, `offered ${roster.length}`);
check("and each is marked as having no recording",
  roster.every((r) => r.takes.length === 0));

/* ---- 2. two recordings do not shrink the picker to two ---- */
const take = (character, verb) => ({
  character, verb, label: verb, seat: 0,
  view: [0, 0, 320, 240],
  frames: [{ bodies: [], hitboxes: [], projectiles: [] }],
});
api.TAKES = { takes: [take("alpha", "attack"), take("beta", "smash_forward")] };
roster = api.takeRoster();
check("two recorded fighters still offer the whole prepared roster",
  roster.length === 5, `offered ${roster.length}`);
check("and the recorded ones are the ones marked recorded",
  roster.filter((r) => r.takes.length).map((r) => r.id).join(",") === "alpha,beta",
  roster.filter((r) => r.takes.length).map((r) => r.id).join(","));

/* ---- 3. an UNRECORDED fighter still exposes its moves ---- */
api.state.takeFighter = "gamma";
const slots = api.takeSlotsFor("gamma");
check("an unrecorded fighter still lists every move it binds",
  slots.length === 3, `${slots.length} move(s)`);
check("and every one of them reports no recording",
  slots.every((s) => s.take === null));
check("the moves are in button order, not alphabetical",
  slots.map((s) => s.verb).join(",") === "attack,smash_forward,special_up",
  slots.map((s) => s.verb).join(","));

/* ---- 4. a recorded move is joined to its take ---- */
const alpha = api.takeSlotsFor("alpha");
check("a recorded move points at its take",
  alpha.find((s) => s.verb === "attack").take === 0);
check("and its unrecorded siblings still appear",
  alpha.filter((s) => s.take === null).length === 2);

/* ---- 5. a recording of somebody the bundle dropped is still visible ---- */
api.TAKES = { takes: [take("retired_fighter", "attack")] };
roster = api.takeRoster();
check("a take for a fighter the bundle no longer resolves is still listed",
  roster.some((r) => r.id === "retired_fighter"),
  roster.map((r) => r.id).join(","));

/* ---- 6. selecting an UNRECORDED move draws, rather than throwing ---- */
/* ⛔⛔ THE PATH A READER HITS CONSTANTLY. Every move of every fighter nobody has
 * recorded goes through it, and `drawTake` used to return early on a null take —
 * so the canvas kept whatever the last move painted while the picker said
 * something else. */
api.state.takeFighter = "gamma";
api.state.takeVerb = null;
let threw = null;
try { api.renderTakeOptions(); } catch (e) { threw = e; }
check("selecting an unrecorded fighter draws without throwing", threw === null,
  threw && threw.message);
check("and it lands on one of that fighter's moves",
  api.state.takeVerb === "attack", String(api.state.takeVerb));
check("with no take behind it", api.state.take === null, String(api.state.take));

/* And a RECORDED move still loads its frames. (Step 5 replaced the recording
 * set; put the two-fighter one back.) */
api.TAKES = { takes: [take("alpha", "attack"), take("beta", "smash_forward")] };
api.state.takeFighter = "alpha";
api.state.takeVerb = null;
threw = null;
try { api.renderTakeOptions(); } catch (e) { threw = e; }
check("selecting a recorded fighter loads its take", threw === null && api.state.take === 0,
  threw ? threw.message : String(api.state.take));

/* ⛔ AND A TAKE WITH NO VIEW RECTANGLE STILL DRAWS. `view[2]` on an absent
 * rectangle throws, and a throw in the draw path kills the playback timer. */
api.TAKES = { takes: [{ ...take("alpha", "attack"), view: undefined }] };
api.state.takeFighter = "alpha";
api.state.takeVerb = null;
threw = null;
try { api.renderTakeOptions(); } catch (e) { threw = e; }
check("a take with no view rectangle draws instead of throwing", threw === null,
  threw && threw.message);

/* ---- 7. roles are read, and old takes still resolve ---- */
check("an explicit role wins",
  api.roleOf({ role: "target" }, { seat: 0 }) === "target");
check("a v1 body falls back to its seat",
  api.roleOf({ seat: 1 }, { seat: 0 }) === "target");
check("a v1 subject-owned strike is still the subject's",
  api.roleOf({ subject_owned: true }, { seat: 0 }) === "subject_owned");
/* ⛔ AND `subject_owned: false` IS NOT "THE TARGET'S". It said only "not the
 * subject's", which covered the target, the target's summon and a stage hazard
 * alike — the ambiguity roles exist to remove. */
check("a v1 unowned strike is not promoted to the target's",
  api.roleOf({ subject_owned: false }, { seat: 0 }) === "other");

console.log("== PASS ==");
for (const o of ok) console.log("  ok   " + o);
if (bad.length) { console.log("== FAIL =="); for (const b of bad) console.log("  FAIL " + b); }
console.log(`\n${ok.length} passed, ${bad.length} failed`);
process.exit(bad.length ? 1 : 0);
