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
const takeCalls = [];
page.on('request', (r) => {
  if (r.url().includes('/api/render')) renderCalls.push({ url: r.url(), body: r.postDataJSON?.() });
  if (r.url().includes('/api/take')) takeCalls.push({ url: r.url(), body: r.postDataJSON?.() });
});
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

/* Missing evidence generates on demand: first the take, then matching GPU coverage. */
await page.waitForFunction(() => {
  const img = document.querySelector('#engine-render');
  return img && !img.hidden && img.getAttribute('src');
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
check('the engine shot matches the diagnostic action tick exactly',
  tickOf(after.note) === 40, `action tick ${tickOf(after.note)} for take frame 40`);
await page.screenshot({ path: `${SHOTS}/02-upb-frame40.png` });

/* With stride 2, action tick 41 has no engine image. It must never borrow 40. */
await page.$eval('#take-scrub', (s) => { s.value = '41'; s.dispatchEvent(new Event('input')); });
await page.waitForTimeout(150);
const odd = {
  src: await page.getAttribute('#engine-render', 'src'),
  note: await page.textContent('#engine-render-note'),
  hidden: await page.$eval('#engine-render', (i) => i.hidden),
};
check('an unsampled action tick paints no stale GPU frame', odd.hidden && !odd.src, odd.src || odd.note);
check('the unsampled action tick is explicit', /No GPU sample for action tick 41/.test(odd.note), odd.note);
await page.$eval('#take-scrub', (s) => { s.value = '40'; s.dispatchEvent(new Event('input')); });
await page.waitForTimeout(100);

/* ---- render coverage follows the runtime take horizon ---- */
const doc = await page.evaluate(async () => {
  const through = Number(document.querySelector('#take-scrub').max);
  const response = await fetch('/api/render', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      scenario: {
        subject: 'npc_pirate_admiral', target: 'npc_pirate_admiral',
        target_behavior: 'passive', verb: 'special_up', spacing: 40,
        chain: null, hold_policy: 'move_exercise_default',
      },
      stride: 2, through_tick: through,
    }),
  });
  return response.json();
});
const horizon = await page.$eval('#take-scrub', (s) => Number(s.max));
const lastSample = (doc.shots || []).at(-1)?.action_tick;
check('GPU render coverage spans the diagnostic horizon',
  lastSample >= horizon - 1, `last sampled ${lastSample}, diagnostic last ${horizon}`);
check('the manifest preserves the canonical scenario',
  doc.scenario?.subject === 'npc_pirate_admiral' &&
  doc.scenario?.target === 'npc_pirate_admiral' &&
  doc.scenario?.target_behavior === 'passive' && doc.scenario?.spacing === 40,
  JSON.stringify(doc.scenario));
check('the render reached the intended move', doc.reached_intended_move === true, doc.outcome);

/* Regenerate Take is a real operation with a visible loading state. */
const takeCallsBefore = takeCalls.length;
await page.click('#regen-take');
await page.waitForFunction(() => !document.querySelector('#take-loading').hidden, null, { timeout: 3000 });
check('Regenerate Take visibly enters a loading/generating state',
  await page.$eval('#take-loading', (n) => !n.hidden));
await page.waitForFunction(() => document.querySelector('#take-loading').hidden, null, { timeout: 5 * 60 * 1000 });
check('Regenerate Take executes the one-scenario endpoint', takeCalls.length > takeCallsBefore);

/* ---- play does not re-spawn the renderer ---- */
const callsBefore = renderCalls.length;
const frameOf = () => page.$eval('#take-scrub', (s) => Number(s.value));
const playFrom = await frameOf();
const playNote = await page.textContent('#engine-render-note');
await page.click('#take-play');
await page.waitForTimeout(4000);
const playTo = await frameOf();
const playNoteAfter = await page.textContent('#engine-render-note');
await page.click('#take-play');
/* Play WRAPS at the end of the take (`(frame + 1) % length`), so four seconds
 * at 30fps from frame 40 of 150 lands back near the beginning. "It moved" is
 * the claim; "the number got bigger" is not. */
check('play advances the take', playTo !== playFrom, `frame ${playFrom} -> ${playTo} of 150`);
check('play advances the engine synchronization state too', playNoteAfter !== playNote, `${playNote} -> ${playNoteAfter}`);
check('play does not re-request the renderer', renderCalls.length === callsBefore,
  `${renderCalls.length - callsBefore} extra request(s)`);
await page.screenshot({ path: `${SHOTS}/03-after-play.png` });

/* A chain render refusal is pinned by the server contract test; this browser
 * suite stays on a production renderable move so it can test paint/sync. */

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
