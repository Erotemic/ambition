/* Draw every frame of every recorded take, and fail on the first exception.
 *
 * ⛔⛔ WHY THIS EXISTS. `node --check` says a file PARSES; it does not say the
 * identifiers resolve. A call to `drawHitboxShape` shipped with no such function
 * anywhere in the file — the edit that added the call landed and the edit that
 * added the definition did not — and `node --check` passed. In the browser
 * `drawTake` threw the instant a strike appeared, which killed the playback
 * `setTimeout` chain: the Jab's first hitbox is on frame 3, so Play ran for
 * exactly three frames and stopped, with the error only in a console nobody had
 * open.
 *
 * ⭐ SO THE CHECK IS TO ACTUALLY RUN IT. The canvas and DOM are stubbed just far
 * enough for the draw path; anything the code reaches for that is not stubbed
 * throws, which is the point rather than a limitation.
 *
 *     node tools/ambition_moveset_inspector/check_draw_path.mjs
 */
import { readFileSync } from "node:fs";
const root = "tools/ambition_moveset_inspector";
const src = readFileSync(`${root}/web/app.js`, "utf8");

// Minimal DOM/canvas stubs: enough for the draw path, loud about anything else.
const ctx = new Proxy({}, { get: (_, k) => (k === "canvas" ? {} : () => {}), set: () => true });
const node = () => new Proxy({
  classList: { toggle(){}, add(){}, remove(){} },
  addEventListener(){}, replaceChildren(){}, append(){},
  // The engine-render panel is an <img> the draw path shows and hides. The
  // catch-all below answers an unknown key with another NODE, so a missing
  // method here fails as "not a function" rather than as a missing stub.
  removeAttribute(){}, setAttribute(){}, getAttribute(){ return null; }, hidden: false,
  getContext: () => ctx, style: {}, dataset: {},
  clientWidth: 1000, textContent: "", title: "", value: "0", max: "0",
}, { get: (t, k) => (k in t ? t[k] : node()), set: (t,k,v) => (t[k]=v, true) });
globalThis.document = {
  querySelector: () => node(), querySelectorAll: () => [],
  createElement: () => node(),
  createTextNode: (t) => ({ nodeType: 3, textContent: String(t) }),
};
globalThis.window = { devicePixelRatio: 1 };
globalThis.Image = class { constructor(){ this.complete=false; this.naturalWidth=0; } addEventListener(){} };
globalThis.fetch = async () => ({ json: async () => ({}) });
globalThis.WeakMap = WeakMap;

// Load app.js without running boot().
const mod = src.replace(/\nboot\(\);\s*$/, "\n");
const run = new Function(`${mod}\nreturn { drawTake, get state(){return state}, set BUNDLE(v){BUNDLE=v}, set TAKES(v){TAKES=v}, get TAKES(){return TAKES} };`);
const api = run();
api.BUNDLE = JSON.parse(readFileSync(`${root}/data/moveset_bundle.json`, "utf8"));
api.TAKES = JSON.parse(readFileSync(`${root}/data/takes/takes.json`, "utf8"));

let checked = 0, failed = 0;
for (let ti = 0; ti < api.TAKES.takes.length; ti++) {
  api.state.take = ti;
  const t = api.TAKES.takes[ti];
  for (let f = 0; f < t.frames.length; f++) {
    api.state.takeFrame = f;
    try { api.drawTake(); checked++; }
    catch (e) {
      failed++;
      if (failed <= 3) console.log(`THROW take ${ti} "${t.label}" (${t.character}) frame ${f}: ${e.message}`);
      break;
    }
  }
}
if (failed) {
  console.error(`[draw-path] FAIL — ${failed} take(s) threw after ${checked} good frames`);
  process.exit(1);
}
console.log(`[draw-path] PASS — drew ${checked} frames across ${api.TAKES.takes.length} takes`);
