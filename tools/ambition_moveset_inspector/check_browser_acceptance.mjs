/* Drive the Engine Takes view in a REAL browser and check what it PAINTS.
 *
 * ⛔⛔ EVERY OTHER CHECK IN THIS TOOL STOPS AT THE DATA. `check_bundle_contract`
 * reads the export, `check_draw_path` runs the draw against a DOM stub, and both
 * passed through a session in which the page showed nothing: `state.view` was
 * read in two places and assigned in none, so no image arriving ever repainted
 * and the engine panel sat on "rendering special_up…" with all 24 PNGs already
 * loaded. A stub cannot find that. Neither can an endpoint.
 *
 *   1. serve the inspector           (serve_inspector.sh, or the server module)
 *   2. node tools/ambition_moveset_inspector/check_browser_acceptance.mjs
 *
 * ⛔ IT NEEDS A BROWSER AND SAYS SO. `playwright-core` plus a Chromium; both are
 * developer-machine things rather than build dependencies, so this is not wired
 * into the gate — it is the check you run before claiming the view works.
 *
 *   npm install playwright-core
 *   PORT=8791 CHROME=/path/to/chrome node check_browser_acceptance.mjs
 *
 * ⛔ AND IT MEASURES PAINT, NOT PROPERTIES. `img.hidden` was true while a rule in
 * this tool's own stylesheet kept `display: block`, so a REFUSED render still
 * showed the broken-image icon and its alt text above the word UNBOUND. The
 * assertions ask for bounding boxes and computed styles wherever "is it on
 * screen" is the question.
 */
/* ⛔ ABSENCE IS NOT SUCCESS. A checker that exits 0 when it could not run is a
 * checker that reports "the browser view is fine" from a machine with no
 * browser — the same defect this repo's triage note raises about
 * `check_clip_handedness.py`. Missing tooling exits 2 and says what to install. */
let chromium;
try {
  ({ chromium } = await import('playwright-core'));
} catch {
  /* ⛔ ESM RESOLVES FROM THIS FILE'S DIRECTORY, NOT FROM `NODE_PATH`. This tool
   * has no `node_modules` of its own and should not grow one, so an install
   * living anywhere else is named explicitly. */
  const base = process.env.PLAYWRIGHT_PATH;
  try {
    const { createRequire } = await import('node:module');
    ({ chromium } = createRequire(base.replace(/\/?$/, '/') + 'anchor.cjs')('playwright-core'));
  } catch {
    console.error(
      '[browser-acceptance] SKIPPED - playwright-core is not installed.\n' +
      '  npm install playwright-core   then  PLAYWRIGHT_PATH=<dir>/node_modules\n' +
      '  and point CHROME at a Chromium binary if playwright has not downloaded one.'
    );
    process.exit(2);
  }
}

const PORT = process.env.PORT || '8791';
const BASE = `http://127.0.0.1:${PORT}/`;
const SHOTS = process.env.SHOTS || '/tmp/moveset_inspector_acceptance';
const ok = [], bad = [];
const check = (name, cond, detail = '') => (cond ? ok : bad).push(`${name}${detail ? ' — ' + detail : ''}`);

import { mkdirSync } from 'node:fs';
mkdirSync(SHOTS, { recursive: true });
/* Playwright's own download if there is one; `CHROME` otherwise. */
const browser = await chromium.launch({
  ...(process.env.CHROME ? { executablePath: process.env.CHROME } : {}),
  args: ['--no-sandbox', '--disable-gpu'],
});
const page = await browser.newPage({ viewport: { width: 1680, height: 1000 } });

const renderCalls = [];
page.on('request', (r) => { if (r.url().includes('/api/render')) renderCalls.push(r.url()); });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e)));
/* `/api/review?subject=…` answers 404 for "no review written yet", which is the
 * designed answer and not a fault; the browser logs it as a console error all
 * the same. Everything else counts. */
const benign = (t) => /api\/review/.test(t) || /favicon/.test(t);
page.on('response', (r) => { if (r.status() >= 400 && !benign(r.url())) errors.push(`HTTP ${r.status()} ${r.url()}`); });

await page.goto(BASE, { waitUntil: 'domcontentloaded', timeout: 120000 });
/* The bundle and the takes are megabytes; wait for the app to have them rather
 * than for the network to go quiet. */
await page.waitForFunction(() => {
  const w = document.querySelector('#take-fighter');
  return w && w.options.length && w.options[0].value;
}, null, { timeout: 180000 });
await page.click('nav.tabs button[data-view="takes"]');
await page.waitForTimeout(1500);

/* ---- fighter + verb ---- */
await page.selectOption('#take-fighter', 'npc_pirate_admiral');
await page.waitForTimeout(600);
const options = await page.$$eval('#take-pick option', (os) => os.map((o) => ({ v: o.value, t: o.textContent })));
console.log('takes offered:', options.map((o) => o.t).join(' | '));
const upb = options.find((o) => /Up B/.test(o.t));
check('the Up-B take is offered', !!upb, upb && upb.t);
await page.selectOption('#take-pick', upb.v);

/* The engine render is produced on demand; the first ask can take minutes. */
await page.waitForFunction(() => {
  const n = document.querySelector('#engine-render-note');
  return n && !/^rendering/.test(n.textContent);
}, null, { timeout: 20 * 60 * 1000 });

const shot0 = {
  src: await page.getAttribute('#engine-render', 'src'),
  note: await page.textContent('#engine-render-note'),
  hidden: await page.$eval('#engine-render', (i) => i.hidden),
};
console.log('engine note:', shot0.note);
check('the engine panel shows an image', !!shot0.src && !shot0.hidden, shot0.src || 'no src');
check('the note names moveset_render', /moveset_render/.test(shot0.note));
check('the note carries an action tick', /action tick \d+/.test(shot0.note));
check('the note carries a sim tick', /sim tick \d+/.test(shot0.note));
check('the note carries build provenance', /built \d{4}-\d\d-\d\d/.test(shot0.note));
check('no capture_scene label', !/capture_scene/.test(shot0.note));
const canvasBox = await page.$eval('#take-canvas', (c) => c.getBoundingClientRect().toJSON());
const imgBox = await page.$eval('#engine-render', (c) => c.getBoundingClientRect().toJSON());
console.log('layout: img', JSON.stringify(imgBox), 'canvas', JSON.stringify(canvasBox));
check('both panels are on screen', imgBox.width > 200 && canvasBox.width > 400,
  `img ${Math.round(imgBox.width)}px, canvas ${Math.round(canvasBox.width)}px`);
check('they are side by side, not stacked',
  Math.abs(imgBox.y - canvasBox.y) < 120 && imgBox.x + imgBox.width <= canvasBox.x + 4,
  `img.y=${Math.round(imgBox.y)} canvas.y=${Math.round(canvasBox.y)}`);
check('no page has scrolled sideways',
  await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1));
/* ⛔ AND NOTHING RUNS OFF THE RIGHT. A long unbreakable id widened the Frame
 * panel's own column past the panel, so its values were painted outside the
 * panel and off the viewport while the document still reported no overflow. */
const overflow = await page.evaluate(() => {
  const w = window.innerWidth;
  return [...document.querySelectorAll('#view-takes .panel, #view-takes .panel *')]
    .filter((e) => e.getBoundingClientRect().right > w + 1)
    .map((e) => `${e.tagName}.${e.className || e.id}`).slice(0, 4);
});
check('nothing in Engine Takes runs past the viewport', overflow.length === 0, overflow.join(', '));
await page.screenshot({ path: `${SHOTS}/01-upb-frame0.png` });

/* ---- scrubbing moves both panels ---- */
const canvasHash = () => page.$eval('#take-canvas', (c) => c.toDataURL().length + ':' + c.toDataURL().slice(-64));
const before = { src: shot0.src, canvas: await canvasHash() };
await page.$eval('#take-scrub', (s) => { s.value = '40'; s.dispatchEvent(new Event('input')); });
await page.waitForTimeout(900);
const after = {
  src: await page.getAttribute('#engine-render', 'src'),
  note: await page.textContent('#engine-render-note'),
  canvas: await canvasHash(),
};
console.log('after scrub:', after.note);
check('scrubbing changes the engine image', after.src !== before.src, `${before.src} -> ${after.src}`);
check('scrubbing changes the diagnostic canvas', after.canvas !== before.canvas);
const tickOf = (s) => Number((s.match(/action tick (\d+)/) || [])[1]);
check('the engine shot is the nearest at or before the take frame',
  tickOf(after.note) <= 40 && tickOf(after.note) > tickOf(shot0.note),
  `action tick ${tickOf(after.note)} for take frame 40`);
await page.screenshot({ path: `${SHOTS}/02-upb-frame40.png` });

/* ---- the shark: does any rendered frame carry it? ---- */
const doc = await page.evaluate(async () =>
  (await (await fetch('/api/render?character=npc_pirate_admiral&verb=special_up&frames=24&stride=2')).json()));
console.log('manifest:', JSON.stringify({
  intended: doc.intended_move, observed: doc.observed_moves, outcome: doc.outcome,
  prepared: doc.prepared, reached: doc.reached_intended_move, release: doc.release_reached,
  last: doc.last_action_tick, hold: doc.hold_ticks, pumps: doc.zero_time_pumps,
  ticks: (doc.shots || []).map((s) => s.sim_tick),
}));
check('the render reached the intended move', doc.reached_intended_move === true, doc.outcome);
check('the render was prepared airborne', doc.prepared === true);
check('the run crossed the release', doc.release_reached === true,
  `last action tick ${doc.last_action_tick} of ${doc.hold_ticks}`);

/* ---- play does not re-spawn the renderer ---- */
const callsBefore = renderCalls.length;
const frameOf = () => page.$eval('#take-scrub', (s) => Number(s.value));
const playFrom = await frameOf();
const playSrc = await page.getAttribute('#engine-render', 'src');
await page.click('#take-play');
await page.waitForTimeout(4000);
const playTo = await frameOf();
const playSrcAfter = await page.getAttribute('#engine-render', 'src');
await page.click('#take-play');
/* Play WRAPS at the end of the take (`(frame + 1) % length`), so four seconds
 * at 30fps from frame 40 of 150 lands back near the beginning. "It moved" is
 * the claim; "the number got bigger" is not. */
check('play advances the take', playTo !== playFrom, `frame ${playFrom} -> ${playTo} of 150`);
check('play advances the engine panel too', playSrcAfter !== playSrc, `${playSrc} -> ${playSrcAfter}`);
check('play does not re-request the renderer', renderCalls.length === callsBefore,
  `${renderCalls.length - callsBefore} extra request(s)`);
await page.screenshot({ path: `${SHOTS}/03-after-play.png` });

/* ---- the mismatch / unbound case ---- */
const air = options.find((o) => /Down B \(air\)/.test(o.t));
check('the unbound take is offered', !!air, air && air.t);
await page.selectOption('#take-pick', air.v);
await page.waitForFunction(() => {
  const n = document.querySelector('#engine-render-note');
  return n && !/^rendering/.test(n.textContent);
}, null, { timeout: 20 * 60 * 1000 });
const mism = {
  src: await page.getAttribute('#engine-render', 'src'),
  note: await page.textContent('#engine-render-note'),
  hidden: await page.$eval('#engine-render', (i) => i.hidden),
};
console.log('unbound note:', mism.note);
/* ⛔ MEASURED AS PAINT, NOT AS A PROPERTY. `img.hidden` was true while a CSS
 * rule in this very stylesheet kept `display: block`, so the broken-image icon
 * and its alt text were on screen above the refusal. */
const mismBox = await page.$eval('#engine-render', (i) => {
  const r = i.getBoundingClientRect();
  return { w: r.width, h: r.height, display: getComputedStyle(i).display };
});
check('the unbound case shows NO image', !mism.src && mism.hidden, mism.src || 'no src');
check('and nothing is painted where it would be',
  mismBox.w === 0 && mismBox.h === 0, `${mismBox.w}x${mismBox.h}, display:${mismBox.display}`);
check('the unbound case says so', /UNBOUND|MISMATCH/.test(mism.note), mism.note);
check('the unbound case names what the engine played', /heave_to/.test(mism.note));
check('the diagnostic canvas is still drawn', (await canvasHash()) !== before.canvas);
await page.screenshot({ path: `${SHOTS}/04-unbound.png` });

/* ---- the status view still answers ---- */
await page.click('nav.tabs button[data-view="status"]');
await page.waitForTimeout(1200);
const statusText = await page.textContent('#status-body');
check('the status view answers', /moveset_render/.test(statusText), statusText.slice(0, 80));
await page.screenshot({ path: `${SHOTS}/05-status.png`, fullPage: true });

check('no page errors', errors.length === 0, errors.slice(0, 3).join(' | '));

console.log('\n== PASS ==');
for (const o of ok) console.log('  ok   ' + o);
if (bad.length) { console.log('== FAIL =='); for (const b of bad) console.log('  FAIL ' + b); }
console.log(`\n${ok.length} passed, ${bad.length} failed`);
await browser.close();
process.exit(bad.length ? 1 : 0);
