"use strict";
/* Ambition moveset balance inspector.
 *
 * The prepared `moveset_export` bundle supplies roster and authoring context.
 * Selected-move timing and geometry come from a canonical runtime take produced
 * by the composed simulation; the browser only presents those observations and
 * server-derived measurements rather than reimplementing combat.
 */

/* The genre's slot order. A moveset table sorted alphabetically by move id is
 * unreadable; sorted by the button a player presses, it is the character. */
const SLOTS = [
  ["attack", "Jab"],
  ["attack_dash", "Dash attack"],
  ["attack_forward", "F-tilt"],
  ["attack_up", "U-tilt"],
  ["attack_down", "D-tilt"],
  ["smash_forward", "F-smash"],
  ["smash_up", "U-smash"],
  ["smash_down", "D-smash"],
  ["attack_air", "N-air"],
  ["attack_air_forward", "F-air"],
  ["attack_air_back", "B-air"],
  ["attack_air_up", "U-air"],
  ["attack_air_down", "D-air"],
  ["special", "Neutral B"],
  ["special_forward", "Side B"],
  ["special_up", "Up B"],
  ["special_down", "Down B"],
  ["special_air_down", "Down B (air)"],
  ["grab", "Grab"],
  ["grab_dash", "Dash grab"],
  ["capture_pummel", "Pummel"],
  ["capture_throw_forward", "F-throw"],
  ["capture_throw_back", "B-throw"],
  ["capture_throw_up", "U-throw"],
  ["capture_throw_down", "D-throw"],
  ["taunt", "Taunt"],
];
const SLOT_LABEL = new Map(SLOTS);
const SLOT_ORDER = new Map(SLOTS.map(([v], i) => [v, i]));

const ISSUE_TAGS = [
  "too-strong", "too-weak", "startup", "endlag", "range",
  "knockback", "recovery", "feel", "unclear-read", "animation",
];

let BUNDLE = null;
let TAKES = null;

/* ⭐⭐ THE SPRITE SHEETS, LAZILY. A page that eagerly loaded every sheet would
 * pull tens of megabytes to draw one fighter; a page that loaded them
 * synchronously per frame would stutter the scrubber. Each sheet is fetched once
 * on first use and the canvas simply draws the box alone until it arrives, so
 * the view is never blocked on art and never blank because of it. */
/* THE GPU RENDER, ON DEMAND. `/api/render` asks the engine to actually draw a
 * fighter; it may legitimately answer "not available" (no GPU, no binary) and
 * the derived sprite blit below is what the view falls back to. Requested once
 * per fighter per session and never awaited by the draw path — the canvas keeps
 * drawing the fallback until real frames arrive, so the view is never blocked
 * and never blank. */
/* Does the loaded recording carry sprite art at all? Memoised: it is a scan of
 * every body of every frame and the answer cannot change without a reload. */
let TAKES_HAVE_ART = null;
function takesCarryArt() {
  if (TAKES_HAVE_ART !== null) return TAKES_HAVE_ART;
  const rows = (TAKES && (TAKES.takes || TAKES)) || [];
  TAKES_HAVE_ART = rows.some((t) =>
    (t.frames || []).some((f) => (f.bodies || []).some((b) => b.art)));
  return TAKES_HAVE_ART;
}

const RENDERS = new Map();
const TAKE_EVIDENCE = new Map();
const TAKE_PENDING = new Map();

function stableScenarioDocument(raw) {
  const subject = raw.subject || raw.character;
  return {
    subject,
    target: raw.target || subject,
    target_behavior: raw.target_behavior || raw.behavior || "passive",
    verb: raw.verb,
    spacing: raw.spacing === undefined ? null : raw.spacing,
    chain: raw.chain || null,
    hold_policy: raw.hold_policy || "move_exercise_default",
  };
}

function scenarioKey(raw) {
  return JSON.stringify(stableScenarioDocument(raw));
}

function sameScenario(left, right) {
  return scenarioKey(left) === scenarioKey(right);
}

function canonicalScenario(subject, verb, chain = null) {
  const target = state.scenarioTarget && state.scenarioTarget !== "__mirror__"
    ? state.scenarioTarget
    : subject;
  return stableScenarioDocument({
    subject,
    target,
    target_behavior: state.scenarioBehavior || "passive",
    verb,
    spacing: state.scenarioSpacing,
    chain,
  });
}

function evidenceRecord(scenario) {
  return TAKE_EVIDENCE.get(scenarioKey(scenario)) || null;
}

function repaintEvidenceUsers() {
  if (state.view === "fighter" && state.fighter && state.move) {
    const c = fighterById(state.fighter);
    const m = c && c.moves.find((row) => row.id === state.move);
    if (c && m) renderMoveDetail(c, m);
  }
  if (state.view === "takes") drawTake();
}

async function requestTakeEvidence(scenario, { force = false } = {}) {
  const key = scenarioKey(scenario);
  const existing = TAKE_EVIDENCE.get(key);
  if (!force && existing && existing.state === "ready") return existing.doc;
  if (!force && TAKE_PENDING.has(key)) return TAKE_PENDING.get(key);

  TAKE_EVIDENCE.set(key, { state: "loading", scenario, message: "Loading runtime evidence…" });
  repaintEvidenceUsers();
  const slow = setTimeout(() => {
    const row = TAKE_EVIDENCE.get(key);
    if (row && row.state === "loading") {
      TAKE_EVIDENCE.set(key, { ...row, state: "generating", message: "Generating runtime take…" });
      repaintEvidenceUsers();
    }
  }, 250);

  const pending = fetch("/api/take", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ scenario, force }),
  })
    .then(async (response) => {
      const doc = await response.json();
      if (!response.ok || !doc || !doc.take) {
        const error = new Error(doc.reason || doc.error || `moveset_takes exited via HTTP ${response.status}`);
        error.detail = doc;
        throw error;
      }
      if (!sameScenario(doc.scenario, scenario)) {
        const error = new Error("generated take scenario does not match the selected scenario");
        error.detail = { asked: scenario, received: doc.scenario };
        throw error;
      }
      TAKE_EVIDENCE.set(key, { state: "ready", scenario, doc });
      return doc;
    })
    .catch((error) => {
      TAKE_EVIDENCE.set(key, {
        state: "error",
        scenario,
        message: error.message,
        detail: error.detail || null,
      });
      throw error;
    })
    .finally(() => {
      clearTimeout(slow);
      TAKE_PENDING.delete(key);
      repaintEvidenceUsers();
    });
  TAKE_PENDING.set(key, pending);
  return pending;
}

function renderRequestKey(scenario, takeLength, stride) {
  return `${scenarioKey(scenario)}|through=${Math.max(0, takeLength - 1)}|stride=${stride}`;
}

async function requestRenderEvidence(scenario, takeLength, { force = false, stride = 2 } = {}) {
  const key = renderRequestKey(scenario, takeLength, stride);
  const existing = RENDERS.get(key);
  if (!force && existing && existing.state === "ready") return existing.doc;
  if (!force && existing && existing.promise) return existing.promise;

  RENDERS.set(key, { state: "rendering", scenario, message: "Rendering engine frames…" });
  repaintEvidenceUsers();
  const promise = fetch("/api/render", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      scenario,
      stride,
      through_tick: Math.max(0, takeLength - 1),
      force,
    }),
  })
    .then(async (response) => {
      const doc = await response.json();
      if (!response.ok || !doc || !doc.available) {
        const error = new Error(doc.reason || doc.error || `moveset_render failed via HTTP ${response.status}`);
        error.detail = doc;
        throw error;
      }
      if (!sameScenario(doc.scenario, scenario)) {
        const error = new Error("GPU manifest scenario does not match the selected runtime take");
        error.detail = { asked: scenario, received: doc.scenario };
        throw error;
      }
      const images = (doc.urls || []).map((url) => {
        const image = new Image();
        image.src = url;
        image.addEventListener("load", repaintEvidenceUsers);
        return image;
      });
      RENDERS.set(key, { state: "ready", scenario, doc: { ...doc, images } });
      return { ...doc, images };
    })
    .catch((error) => {
      RENDERS.set(key, {
        state: error.detail && error.detail.state === "unsupported" ? "unsupported" : "error",
        scenario,
        message: error.message,
        detail: error.detail || null,
      });
      throw error;
    })
    .finally(repaintEvidenceUsers);
  RENDERS.set(key, { state: "rendering", scenario, message: "Rendering engine frames…", promise });
  return promise;
}

function renderedFramesFor(scenario, takeLength, options = {}) {
  if (!scenario || !scenario.subject || !scenario.verb || !takeLength) return null;
  const stride = options.stride || 2;
  const key = renderRequestKey(scenario, takeLength, stride);
  const have = RENDERS.get(key);
  if (!have) {
    requestRenderEvidence(scenario, takeLength, options).catch(() => {});
    return null;
  }
  return have;
}

let PLAYBACK_TIMER = null;
let PLAYBACK_TOKEN = 0;

function readyRenderFor(scenario, takeLength, stride = 2) {
  const record = RENDERS.get(renderRequestKey(scenario, takeLength, stride));
  if (!record || record.state !== "ready" || !record.doc || !sameScenario(record.doc.scenario, scenario)) return null;
  return record.doc;
}

function sampledPlaybackTicks(scenario, takeLength, stride = 2) {
  const doc = readyRenderFor(scenario, takeLength, stride);
  if (!doc) return null;
  const ticks = (doc.shots || [])
    .filter((shot, index) => Number.isInteger(shot.action_tick) && (doc.urls || [])[index])
    .map((shot) => shot.action_tick)
    .filter((tick) => tick >= 0 && tick < takeLength);
  return ticks.length ? ticks : null;
}

function nextPlaybackTick(current, scenario, takeLength, stride = 2) {
  const sampled = sampledPlaybackTicks(scenario, takeLength, stride);
  if (sampled) return sampled.find((tick) => tick > current) ?? sampled[0];
  return (current + 1) % Math.max(1, takeLength);
}

function playbackDelayMs(scenario, takeLength, stride = 2) {
  const sampled = sampledPlaybackTicks(scenario, takeLength, stride);
  const tickStep = sampled && sampled.length > 1
    ? Math.max(1, sampled[1] - sampled[0])
    : 1;
  const hz = Number((BUNDLE && BUNDLE.sim_hz) || 60);
  return Math.max(8, (1000 * tickStep) / Math.max(1, hz));
}

function cancelPlayback({ repaint = false } = {}) {
  PLAYBACK_TOKEN += 1;
  if (PLAYBACK_TIMER !== null) clearTimeout(PLAYBACK_TIMER);
  PLAYBACK_TIMER = null;
  const changed = state.playing || state.fighterPlaying;
  state.playing = false;
  state.fighterPlaying = false;
  const takePlay = $("#take-play");
  if (takePlay) {
    takePlay.classList.remove("on");
    takePlay.textContent = "Play";
  }
  if (repaint && changed) repaintEvidenceUsers();
}

function runTakePlayback() {
  cancelPlayback();
  state.playing = true;
  const button = $("#take-play");
  if (button) { button.classList.add("on"); button.textContent = "Pause"; }
  const token = ++PLAYBACK_TOKEN;
  const step = () => {
    if (token !== PLAYBACK_TOKEN || !state.playing || !state.takeFighter || !state.takeVerb) return;
    const scenario = canonicalScenario(state.takeFighter, state.takeVerb);
    const record = evidenceRecord(scenario);
    if (!record || record.state !== "ready") { cancelPlayback(); return; }
    const take = record.doc.take;
    state.takeFrame = nextPlaybackTick(state.takeFrame, scenario, take.frames.length, 2);
    drawTake();
    PLAYBACK_TIMER = setTimeout(step, playbackDelayMs(scenario, take.frames.length, 2));
  };
  PLAYBACK_TIMER = setTimeout(step, 0);
}

function runFighterPlayback(c, m, scenario, take, totalTicks) {
  cancelPlayback();
  state.fighterPlaying = true;
  const token = ++PLAYBACK_TOKEN;
  const step = () => {
    if (token !== PLAYBACK_TOKEN || !state.fighterPlaying || state.view !== "fighter" || state.fighter !== c.id || state.move !== m.id) return;
    state.fighterFrame = nextPlaybackTick(state.fighterFrame, scenario, take.frames.length, 2);
    updateFighterFrameView(c, m, scenario, take, totalTicks);
    PLAYBACK_TIMER = setTimeout(step, playbackDelayMs(scenario, take.frames.length, 2));
  };
  PLAYBACK_TIMER = setTimeout(step, 0);
}

function renderStatus(record) {
  const node = $("#take-source");
  if (!node) return;
  if (!record) {
    node.textContent = "engine render: idle";
    return;
  }
  if (record.state === "rendering") {
    node.textContent = "engine render: rendering current scenario";
    return;
  }
  if (record.state === "ready") {
    const doc = record.doc;
    node.textContent = `engine render: current · ${doc.renderer || "moveset_render"}` +
      (doc.renderer_built ? ` built ${doc.renderer_built}` : "");
    return;
  }
  node.textContent = `engine render: ${record.state} · ${record.message || "unavailable"}`;
}

const SHEETS = new Map();
const PORTRAIT_IMAGES = new Map();

function artUrl(path) {
  let clean = String(path || "").replace(/\\/g, "/").replace(/^\.\//, "");
  // `/art/` is rooted at the engine's `assets/sprites` directory. Catalog
  // portrait references are asset-relative (`sprites/foo.png`), while sheet
  // atlas records carry bare filenames. Normalize both onto that one route.
  if (clean.startsWith("sprites/")) clean = clean.slice("sprites/".length);
  return `/art/${clean.split("/").map(encodeURIComponent).join("/")}`;
}

function portraitImage(path) {
  const key = String(path || "");
  if (!key) return null;
  if (PORTRAIT_IMAGES.has(key)) return PORTRAIT_IMAGES.get(key);
  const image = new Image();
  image.src = artUrl(key);
  PORTRAIT_IMAGES.set(key, image);
  return image;
}

function portraitFallback(c) {
  const label = (c.display_name || c.id || "?").trim();
  const initials = label.split(/\s+/).filter(Boolean).slice(0, 2).map((part) => part[0]).join("").toUpperCase();
  return el("span", { class: "roster-portrait-fallback", "aria-hidden": "true" }, initials || "?");
}

function rosterPortrait(c) {
  const art = c && c.portrait_art;
  const frame = art && art.frame;
  const fallback = portraitFallback(c || {});
  const shell = el("div", {
    class: "roster-portrait-shell",
    title: art ? `portrait · ${art.clip || "still"}` : "portrait not available",
  }, fallback);
  if (!art || !art.image || !Array.isArray(frame) || frame.length !== 4) return shell;
  const [sx, sy, sw, sh] = frame.map(Number);
  if (![sx, sy, sw, sh].every(Number.isFinite) || sw <= 0 || sh <= 0) return shell;

  const canvas = el("canvas", { class: "roster-portrait", width: "120", height: "144", hidden: "" });
  shell.prepend(canvas);
  const image = portraitImage(art.image);
  const draw = () => {
    if (!image || !image.complete || !image.naturalWidth) return;
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    const scale = Math.min(canvas.width / sw, canvas.height / sh);
    const dw = sw * scale;
    const dh = sh * scale;
    const dx = (canvas.width - dw) / 2;
    const dy = (canvas.height - dh) / 2;
    ctx.drawImage(image, sx, sy, sw, sh, dx, dy, dw, dh);
    canvas.hidden = false;
    fallback.hidden = true;
  };
  if (image) {
    image.addEventListener("load", draw);
    image.addEventListener("error", () => { shell.title = `portrait image unavailable · ${art.image}`; });
    draw();
  }
  return shell;
}

function sheetImage(key) {
  if (SHEETS.has(key)) return SHEETS.get(key);
  const meta = BUNDLE && BUNDLE.sheets && BUNDLE.sheets[key];
  if (!meta) { SHEETS.set(key, null); return null; }
  const pages = (meta.images && meta.images.length ? meta.images : [meta.image]).map((name) => {
    const img = new Image();
    img.src = artUrl(name);
    /* A redraw when the bytes land, or the first frame a sheet appears on stays
     * a box until something else happens to repaint. */
    img.addEventListener("load", repaintEvidenceUsers);
    return img;
  });
  const entry = { meta, pages };
  SHEETS.set(key, entry);
  return entry;
}

/* Draw ONE body's current sheet frame so its art lands on its collision box.
 *
 * ⛔⛔ THE BODY'S OWN CENTRE, NOT THE FRAME'S. A sheet frame is a packed cell
 * sized by the widest pose and the art sits wherever the crop left it —
 * `projectile_polygon` is 17% of a 377px frame left of centre. Centring the cell
 * on the box reproduces here the exact defect the ENGINE's anchor had until
 * 2026-08-27, and a viewer that lies the same way the bug did is worse than no
 * viewer. `feet_pixel.x` is the horizontal origin; `body_pixel_bbox` gives the
 * scale, because the box the take recorded IS that rectangle.
 *
 * ⛔ TRIM IS APPLIED, not ignored. Frames are packed trimmed with an `off` into
 * the logical cell; drawing the sub-rect at the cell's origin puts every pose a
 * few pixels adrift, differently per frame, which reads as jitter. */
/* How far into its current animation row a body is, at this frame of the take.
 *
 * ⭐⭐ DERIVED FROM THE RECORDING, NOT RECORDED. The take stores which ROW a body
 * is drawn from on every tick; the frame within that row is then just "how many
 * consecutive ticks has it been on this row", divided by the row's own per-frame
 * duration. That keeps ONE clock — the sim tick the take was recorded at — where
 * a recorded frame index would be a second clock to keep in step with the first,
 * and the two drift the moment either changes.
 *
 * Memoised per take, because it is a scan from the beginning and the scrubber
 * asks for it sixty times a second. */
const ROW_CURSORS = new WeakMap();
function rowCursorsFor(take) {
  let cached = ROW_CURSORS.get(take);
  if (cached) return cached;
  cached = [];
  /* ⛔⛤ JOIN ON `id`, NOT ON WHAT A READER SEES. This keyed on `label + seat`,
   * and the recorder now says plainly that `label` is reader-facing and joins
   * nothing: a take deliberately seats TWO FIGHTERS WEARING THE SAME CHARACTER,
   * so the label names both of them. `id` is `SimId`, the engine's deterministic
   * identity, independent of Bevy entity allocation.
   *
   * ⭐ THE OLD KEY IS THE FALLBACK, not the rule: a recording made before `id`
   * existed still animates rather than collapsing two bodies into one cursor. */
  const held = new Map();
  take.frames.forEach((frame, i) => {
    const perFrame = new Map();
    for (const b of frame.bodies) {
      const id = b.id || `${b.label}#${b.seat ?? "-"}`;
      const key = b.art ? `${b.art[0]}:${b.art[1]}` : null;
      const prev = held.get(id);
      const ticks = prev && prev.key === key ? prev.ticks + 1 : 0;
      held.set(id, { key, ticks });
      perFrame.set(id, ticks);
    }
    cached[i] = perFrame;
  });
  ROW_CURSORS.set(take, cached);
  return cached;
}

function drawBodyArt(ctx, b, X, Y, scale, ticksOnRow) {
  if (!b.art) return false;
  const [key, row, holds] = b.art;
  const entry = sheetImage(key);
  if (!entry) return false;
  const { meta, pages } = entry;
  const sheetRow = (meta.rows || [])[row];
  if (!sheetRow || !sheetRow.rects || !sheetRow.rects.length) return false;
  /* ⛔⛔ `duration_secs` IS PER FRAME, NOT PER ROW. The engine's animator does
   * `elapsed -= row.duration_secs` once per frame advance, so a 6-frame row at
   * 0.13 runs for 0.78s. This divided by the frame count and played every
   * animation six times too slowly. A row with no duration is a still. */
  const simHz = (BUNDLE && BUNDLE.sim_hz) || 60;
  const count = sheetRow.rects.length;
  const perFrame = sheetRow.duration_secs > 0
    ? Math.max(1, Math.round(sheetRow.duration_secs * simHz))
    : 0;
  /* ⛔⛔ AND A CLIP HOLDS ITS LAST FRAME rather than looping — `tick_slot` sets
   * `clip_held` and stops. Looping a swing shows it restarting into its own
   * windup while the recovery is still running, which is a move that does not
   * exist. A resting pose loops; the recorder says which this is. */
  const raw = perFrame > 0 ? Math.floor((ticksOnRow || 0) / perFrame) : 0;
  const frameIndex = perFrame === 0 ? 0 : holds ? Math.min(raw, count - 1) : raw % count;
  const rect = sheetRow.rects[frameIndex];
  if (!rect) return false;
  const [sx, sy, sw, sh, page, offX, offY] = rect;
  const img = pages[Math.min(page || 0, pages.length - 1)];
  if (!img || !img.complete || !img.naturalWidth) return false;

  const fw = meta.frame_width || 1;
  const fh = meta.frame_height || 1;
  const bbox = meta.body_pixel_bbox;
  const feet = meta.feet_pixel;
  if (!bbox || !feet) return false;

  /* World pixels per sheet pixel: the recorded half-extent IS the body bbox. */
  const pxPerSheet = (b.half[0] * 2) / Math.max(1, bbox[2]);
  /* The body's origin inside the cell: feet x, and the bbox's bottom edge. */
  const originX = feet[0];
  const originY = bbox[1] + bbox[3];
  /* Where the trimmed sub-rect's top-left sits, in sheet pixels from the origin. */
  const dxSheet = (offX || 0) - originX;
  const dySheet = (offY || 0) - originY;

  const flip = (b.facing ?? 1) < 0;
  const dw = sw * pxPerSheet * scale;
  const dh = sh * pxPerSheet * scale;
  /* `+y` is gravity-down in both spaces, so the vertical term needs no flip;
   * the body's own bottom edge is `pos.y + half.y`. */
  const dy = Y(b.pos[1] + b.half[1] + dySheet * pxPerSheet);

  ctx.save();
  if (flip) {
    ctx.translate(X(b.pos[0]), 0);
    ctx.scale(-1, 1);
    ctx.drawImage(img, sx, sy, sw, sh, dxSheet * pxPerSheet * scale, dy, dw, dh);
  } else {
    ctx.drawImage(img, sx, sy, sw, sh, X(b.pos[0] + dxSheet * pxPerSheet), dy, dw, dh);
  }
  ctx.restore();
  return true;
}
let state = {
  fighter: null,
  move: null,
  slot: "smash_forward",
  gridOnly: true,
  compareGridOnly: true,
  sort: { key: "slot", asc: true },
  compareSort: { key: "fighter", asc: true },
  take: null,
  /* Which fighter's takes are listed. Seeded from `fighter` on entry so the
   * view follows the reader rather than starting over. */
  takeFighter: null,
  /* Which MOVE is selected — a verb from prepared content. Runtime frames live
   * in TAKE_EVIDENCE under the canonical scenario; `take` remains only as a
   * compatibility slot for older discovery helpers. */
  takeVerb: null,
  /* Whether the cyan damageable volumes are drawn. */
  takeHurt: true,
  takeFrame: 0,
  playing: false,
  fighterPlaying: false,
  /* ⛔⛔ WHICH VIEW IS ON SCREEN, and it was READ IN TWO PLACES AND WRITTEN IN
   * NONE. Both the sprite-sheet loader and the engine-render loader redraw with
   * `if (state.view === "takes") drawTake()`, and against a field nothing ever
   * assigned that condition is false forever — so an image arriving after the
   * last draw NEVER repainted. The engine panel sat on "rendering special_up…"
   * with all 24 PNGs already loaded in the page, and a sheet that finished late
   * left boxes where its art should have been. Nothing but a browser could find
   * this: every endpoint was correct and every file was served. */
  view: "roster",
  scenarioTarget: "__mirror__",
  scenarioBehavior: "passive",
  scenarioSpacing: 40,
  fighterFrame: 0,
};

/* ---------- small helpers ---------- */
const $ = (sel) => document.querySelector(sel);
const el = (tag, attrs = {}, ...kids) => {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === null || v === undefined || v === false) continue;
    if (k === "class") node.className = v;
    else if (k === "html") node.innerHTML = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v);
  }
  for (const kid of kids.flat()) {
    if (kid === null || kid === undefined || kid === false) continue;
    node.append(kid.nodeType ? kid : document.createTextNode(String(kid)));
  }
  return node;
};
const f1 = (n) => (n === null || n === undefined ? "—" : Number(n).toFixed(1));
const f2 = (n) => (n === null || n === undefined ? "—" : Number(n).toFixed(2));
const int = (n) => (n === null || n === undefined ? "—" : String(Math.round(n)));

/* The slot a move answers, or null for one nothing binds. A move can be bound
 * by several verbs (the down-B pair, the derived dash grab); the EARLIEST slot
 * in genre order is the one a table should file it under. */
function slotOf(mv) {
  let best = null;
  for (const verb of mv.verbs || []) {
    const rank = SLOT_ORDER.get(verb);
    if (rank === undefined) continue;
    if (best === null || rank < SLOT_ORDER.get(best)) best = verb;
  }
  return best;
}

function fighters(gridOnly) {
  return BUNDLE.characters.filter((c) => !gridOnly || c.on_smash_grid);
}
function fighterById(id) {
  return BUNDLE.characters.find((c) => c.id === id) || null;
}

function populateScenarioTargetControls() {
  for (const id of ["fighter-target", "take-target"]) {
    const select = $(`#${id}`);
    if (!select) continue;
    select.replaceChildren(
      el("option", { value: "__mirror__" }, "mirror subject"),
      ...BUNDLE.characters.map((c) => el("option", { value: c.id }, c.display_name || c.id)),
    );
    select.value = state.scenarioTarget;
  }
}

function syncScenarioControls() {
  for (const prefix of ["fighter", "take"]) {
    const target = $(`#${prefix}-target`);
    const behavior = $(`#${prefix}-behavior`);
    const spacing = $(`#${prefix}-spacing`);
    if (target) target.value = state.scenarioTarget;
    if (behavior) behavior.value = state.scenarioBehavior;
    if (spacing) spacing.value = state.scenarioSpacing === null ? "" : String(state.scenarioSpacing);
  }
}

function scenarioInputsChanged(prefix) {
  cancelPlayback();
  const target = $(`#${prefix}-target`);
  const behavior = $(`#${prefix}-behavior`);
  const spacing = $(`#${prefix}-spacing`);
  state.scenarioTarget = (target && target.value) || "__mirror__";
  state.scenarioBehavior = (behavior && behavior.value) || "passive";
  const parsed = spacing && spacing.value !== "" ? Number(spacing.value) : null;
  state.scenarioSpacing = Number.isFinite(parsed) ? parsed : null;
  state.takeFrame = 0;
  state.fighterFrame = 0;
  syncScenarioControls();
  if (state.fighter && state.move) {
    const c = fighterById(state.fighter);
    const m = c && c.moves.find((row) => row.id === state.move);
    if (c && m) renderMoveDetail(c, m);
  }
  if (state.takeFighter && state.takeVerb) drawTake();
}

/* ---------- roster ---------- */
function renderRoster() {
  const q = $("#roster-search").value.trim().toLowerCase();
  const list = fighters(state.gridOnly).filter(
    (c) => !q || c.id.includes(q) || (c.display_name || "").toLowerCase().includes(q)
  );
  const host = $("#roster");
  host.replaceChildren(
    ...list.map((c) =>
      el(
        "div",
        { class: "card", onclick: () => openFighter(c.id) },
        rosterPortrait(c),
        el("div", { class: "card-copy" },
          el("h3", {}, c.display_name || c.id,
             c.on_smash_grid ? el("span", { class: "badge" }, "grid")
                             : el("span", { class: "badge off" }, "off-grid")),
          el("div", { class: "id" }, c.id),
          el(
            "div",
            { class: "facts" },
            el("span", {}, "HP ", el("b", {}, int(c.vitals.max_health))),
            el("span", {}, int(c.moves.length), " moves"),
            c.locomotion ? el("span", {}, "run ", el("b", {}, int(c.locomotion.run_speed))) : null,
            el("span", {}, "provider ", el("b", {}, c.provider || "—"))
          )
        )
      )
    )
  );
  $("#roster-count").textContent = `${list.length} of ${BUNDLE.characters.length} fighters`;
}

/* ---------- fighter ---------- */
function openFighter(id) {
  cancelPlayback();
  state.fighter = id;
  state.move = null;
  showView("fighter");
  $("#fighter-pick").value = id;
  renderFighter();
}

function renderFighter() {
  const c = fighterById(state.fighter);
  if (!c) return;
  $("#fighter-note").textContent =
    `${c.provider || "—"} · ${c.moves.length} moves` +
    (c.on_smash_grid ? " · on the smash grid" : " · not on the smash grid");

  /* body panel */
  const kv = el("dl", { class: "kv" });
  const row = (k, v) => { kv.append(el("dt", {}, k), el("dd", {}, v)); };
  row("Health", int(c.vitals.max_health));
  row("Knockback weight", c.vitals.knockback_weight === null ? "default" : f2(c.vitals.knockback_weight));
  row("Mass", c.vitals.mass === null ? "default" : f2(c.vitals.mass));
  row("Height", c.vitals.canonical_height === null ? "default" : int(c.vitals.canonical_height));
  if (c.locomotion) {
    row("Run speed", int(c.locomotion.run_speed));
    row("Move style", c.locomotion.move_style);
  }
  if (c.movement_tuning) {
    row("Gravity", int(c.movement_tuning.gravity));
    row("Max air speed", int(c.movement_tuning.max_air_speed));
  }
  if (c.abilities) {
    const on = Object.entries(c.abilities).filter(([, v]) => v).map(([k]) => k);
    row("Abilities", on.length ? on.join(", ") : "none");
  }
  if (c.mount) row("Can pilot", (c.mount.pilotable_classes || []).join(", ") || "—");
  if (c.held_item) row("Held item", c.held_item);
  $("#vitals").replaceChildren(kv);
  if (c.description) $("#vitals").append(el("p", { class: "note" }, c.description));

  renderMoveTable(c);
  renderReview();
}

const MOVE_COLUMNS = [
  ["slot", "Slot", (m) => slotOf(m), (m) => SLOT_LABEL.get(slotOf(m)) || "—", "slot"],
  ["name", "Move", (m) => m.id, (m) => m.display_name || m.id, "mono"],
  ["startup", "Startup", (m) => m.derived.startup_f, (m) => f1(m.derived.startup_f)],
  ["active", "Active", (m) => m.derived.active_f, (m) => f1(m.derived.active_f)],
  ["endlag", "Endlag", (m) => m.derived.endlag_f, (m) => f1(m.derived.endlag_f)],
  ["total", "Total", (m) => m.duration_f, (m) => f1(m.duration_f)],
  ["damage", "Dmg", (m) => m.derived.max_damage, (m) => int(m.derived.max_damage)],
  ["charged", "Dmg×", (m) => m.derived.max_damage_charged,
    (m) => (m.smash_charge_mult > 1 ? int(m.derived.max_damage_charged) : "—")],
  ["kb", "KB", (m) => m.derived.max_knockback, (m) => int(m.derived.max_knockback)],
  /* A SEPARATE COLUMN, NOT A BIGGER `Dmg`. Sorting a moveset by damage with
   * shots folded in would rank a projectile against a melee hitbox as though a
   * player could choose between them at the same range. */
  ["shot", "Shot", (m) => m.derived.projectile_damage,
    (m) => (m.derived.projectile_damage === null || m.derived.projectile_damage === undefined
      ? "\u2014"
      : m.derived.projectile_damage_charged
        ? `${int(m.derived.projectile_damage)}\u2192${int(m.derived.projectile_damage_charged)}`
        : int(m.derived.projectile_damage))],
  ["reach", "Reach", (m) => m.derived.reach, (m) => int(m.derived.reach)],
  ["hits", "Boxes", (m) => m.derived.hits, (m) => int(m.derived.hits)],
];

function renderMoveTable(c) {
  const table = $("#moves");
  const col = MOVE_COLUMNS.find((x) => x[0] === state.sort.key) || MOVE_COLUMNS[0];
  const rows = [...c.moves].sort((a, b) => {
    /* Slot order is the genre's, not alphabetical; everything else is numeric
     * or lexical with unbound moves last so a table never opens on them. */
    let av = col[2](a), bv = col[2](b);
    if (col[0] === "slot") {
      av = SLOT_ORDER.has(av) ? SLOT_ORDER.get(av) : 999;
      bv = SLOT_ORDER.has(bv) ? SLOT_ORDER.get(bv) : 999;
    }
    if (av === null || av === undefined) av = -Infinity;
    if (bv === null || bv === undefined) bv = -Infinity;
    const cmp = typeof av === "string" ? av.localeCompare(bv) : av - bv;
    return state.sort.asc ? cmp : -cmp;
  });

  const head = el("tr", {}, ...MOVE_COLUMNS.map(([key, label]) =>
    el("th", {
      class: state.sort.key === key ? `sorted ${state.sort.asc ? "asc" : ""}` : "",
      onclick: () => {
        if (state.sort.key === key) state.sort.asc = !state.sort.asc;
        else state.sort = { key, asc: key === "slot" || key === "name" };
        renderMoveTable(c);
      },
    }, label)
  ));

  const maxes = {};
  for (const [key, , get] of MOVE_COLUMNS) {
    maxes[key] = Math.max(...c.moves.map((m) => Number(get(m)) || 0), 0);
  }

  const body = el("tbody", {}, ...rows.map((m) =>
    el("tr", {
      class: state.move === m.id ? "sel" : "",
      onclick: () => { cancelPlayback(); state.move = m.id; state.fighterFrame = 0; renderMoveTable(c); renderMoveDetail(c, m); renderReview(); },
    }, ...MOVE_COLUMNS.map(([key, , get, fmt, cls]) => {
      const raw = Number(get(m));
      const cell = el("td", { class: cls || "mono bar" }, fmt(m));
      /* A bar behind a number reads faster than the number: the shape of a
       * fighter's kit is visible before any of it is read. */
      if (!cls && Number.isFinite(raw) && maxes[key] > 0) {
        cell.prepend(el("span", { style: `width:${Math.max(0, (raw / maxes[key]) * 100)}%` }));
      }
      return cell;
    }))
  ));
  table.replaceChildren(el("thead", {}, head), body);
}

/* ---------- move detail ---------- */
const WIN_CLASS = {
  startup: "startup", active: "active", recovery: "recovery",
  invuln: "invuln", armor: "armor",
};
function winClass(tag) {
  return WIN_CLASS[tag] || (tag.startsWith("cancelable") ? "cancel" : "recovery");
}

function provenanceBadge(text) {
  return el("span", { class: "provenance" }, text);
}

function windowsText(rows) {
  if (!rows || !rows.length) return "—";
  return rows.map((row) => row.first_tick === row.last_tick
    ? `${row.first_tick}`
    : `${row.first_tick}–${row.last_tick}`).join(", ");
}

function ufdBand(label, windows, totalTicks, frame) {
  return el("div", { class: "ufd-row" },
    el("span", { class: "ufd-label" }, label),
    el("div", { class: "ufd-track" },
      ...(windows || []).map((row) => el("span", {
        class: `ufd-window ${label.toLowerCase().replaceAll(" ", "-")}`,
        title: `${label}: action ticks ${row.first_tick}–${row.last_tick}`,
        style: `left:${(row.first_tick / totalTicks) * 100}%;width:${Math.max(0.8, ((row.last_tick - row.first_tick + 1) / totalTicks) * 100)}%`,
      })),
      el("span", { class: "ufd-playhead", "data-fighter-playhead": "", style: `left:${(frame / totalTicks) * 100}%` })
    )
  );
}

function runtimeMoveDurationTicks(take) {
  const hz = Number((BUNDLE && BUNDLE.sim_hz) || 60);
  for (const frame of take.frames || []) {
    const body = (frame.bodies || []).find((row) => roleOf(row, take) === "subject");
    const move = body && body.move_state;
    if (move && (!take.intended_move || move.id === take.intended_move) && Number.isFinite(Number(move.duration_s))) {
      return Math.max(1, Math.round(Number(move.duration_s) * hz));
    }
  }
  return null;
}

function runtimeExtent(report, take) {
  const candidates = [runtimeMoveDurationTicks(take), 1];
  for (const key of ["startup", "active", "recovery", "invuln", "armor"]) {
    const row = report[key];
    if (row) candidates.push(row.last_tick + 1);
  }
  for (const row of report.live_volume_windows || []) candidates.push(row.last_tick + 1);
  return Math.max(...candidates.filter((x) => Number.isFinite(x)));
}

function updateFighterFrameView(c, m, scenario, take, totalTicks) {
  const lastFrame = Math.max(0, (take.frames || []).length - 1);
  state.fighterFrame = Math.min(Math.max(0, state.fighterFrame), lastFrame);
  const scrub = $("#fighter-runtime-scrub");
  if (scrub) scrub.value = String(state.fighterFrame);
  const currentFrame = (take.frames || [])[state.fighterFrame];
  const currentSubject = currentFrame && (currentFrame.bodies || []).find((body) => roleOf(body, take) === "subject");
  const currentMove = currentSubject && currentSubject.move_state;
  const frameLabel = $("#fighter-runtime-frame");
  if (frameLabel) {
    frameLabel.textContent = `action tick ${state.fighterFrame} / ${lastFrame}` +
      (currentMove && currentMove.phase ? ` · ${currentMove.phase}` : "");
  }
  for (const playhead of document.querySelectorAll("[data-fighter-playhead]")) {
    playhead.style.left = `${(state.fighterFrame / totalTicks) * 100}%`;
  }
  const canvas = $("#fighter-runtime-canvas");
  if (canvas) drawRuntimeDiagnostic(canvas, take, state.fighterFrame, { showArt: true, showHurt: true });
  const img = $("#fighter-engine-render");
  const note = $("#fighter-engine-note");
  const overlay = $("#fighter-engine-overlay");
  if (img && note && overlay) syncEngineRender(take, state.fighterFrame, scenario, { img, note, overlay });
}

function renderMoveDetail(c, m) {
  $("#move-title").textContent = `${m.display_name || m.id} · ${SLOT_LABEL.get(slotOf(m)) || "unbound"}`;
  const host = $("#move-detail");
  const verb = slotOf(m) || (m.verbs || [])[0];
  if (!verb) {
    host.replaceChildren(el("p", { class: "note err" }, "This prepared move has no repertoire verb, so no canonical runtime scenario can drive it."));
    return;
  }
  const scenario = canonicalScenario(c.id, verb);
  const record = evidenceRecord(scenario);

  const controls = el("div", { class: "controls compact" },
    el("button", {
      class: "ghost",
      onclick: () => requestTakeEvidence(scenario, { force: true }).catch(() => {}),
    }, "Regenerate Take"),
    el("button", {
      class: "ghost",
      onclick: async () => {
        const current = evidenceRecord(scenario);
        if (current && current.state === "ready") {
          await requestRenderEvidence(scenario, current.doc.take.frames.length, { force: true, stride: 2 }).catch(() => {});
        }
      },
    }, "Regenerate Render"),
    el("button", {
      class: "act",
      onclick: async () => {
        const doc = await requestTakeEvidence(scenario, { force: true }).catch(() => null);
        if (doc) await requestRenderEvidence(scenario, doc.take.frames.length, { force: true, stride: 2 }).catch(() => {});
      },
    }, "Regenerate All"),
    el("span", { class: "note mono" },
      `${scenario.subject} vs ${scenario.target} · ${scenario.target_behavior} · ` +
      `${scenario.spacing === null ? "default spacing" : `${scenario.spacing}px`}`)
  );

  if (!record) {
    host.replaceChildren(
      controls,
      el("div", { class: "evidence-shell" },
        el("div", { class: "evidence-overlay static" },
          el("span", { class: "spinner" }), el("strong", {}, "Loading runtime evidence…")))
    );
    requestTakeEvidence(scenario).catch(() => {});
    return;
  }
  if (record.state !== "ready") {
    host.replaceChildren(
      controls,
      el("div", { class: "evidence-shell" },
        ["loading", "generating"].includes(record.state)
          ? el("div", { class: "evidence-overlay static" },
              el("span", { class: "spinner" }), el("strong", {}, record.message || "Generating runtime take…"))
          : diagnosticErrorNode(record, () => requestTakeEvidence(scenario, { force: true }).catch(() => {})))
    );
    return;
  }

  const evidence = record.doc;
  const take = evidence.take;
  const report = evidence.report.measurements || evidence.report;
  const lastFrame = Math.max(0, (take.frames || []).length - 1);
  state.fighterFrame = Math.min(state.fighterFrame, lastFrame);
  const totalTicks = runtimeExtent(report, take);

  const cancelText = (m.windows || []).filter((w) => (w.cancel_into || []).length).map((w) =>
    `${f1(w.start_f)}–${f1(w.end_f)} → ${(w.cancel_into_resolved || w.cancel_into || []).join(", ")}`).join(" · ") || "—";
  const resolverOutcomes = (report.consequence_chain || [])
    .filter((link) => link.resolution)
    .map((link) => `${link.resolution}@${link.tick}`);
  const kv = el("dl", { class: "kv runtime-kv" });
  const row = (label, value, provenance) => kv.append(
    el("dt", {}, label),
    el("dd", {}, value, provenance ? provenanceBadge(provenance) : null)
  );
  row("Startup", report.startup ? `${report.startup.ticks} tick(s) · ${report.startup.first_tick}–${report.startup.last_tick}` : "—", "runtime measured");
  row("First active", report.first_active_tick ?? "—", "runtime observation");
  row("Active windows", windowsText(report.live_volume_windows), "runtime observation");
  row("Active gaps", windowsText(report.live_volume_gaps), "derived runtime");
  row("Recovery", report.recovery ? `${report.recovery.ticks} tick(s) · ${report.recovery.first_tick}–${report.recovery.last_tick}` : "—", "runtime measured");
  row("Move duration", runtimeMoveDurationTicks(take) ?? "—", "runtime observation");
  row("Invulnerable", report.invuln ? `${report.invuln.first_tick}–${report.invuln.last_tick}` : "—", "runtime measured");
  row("Travel before active", report.subject_travel_before_active === null ? "—" : `${report.subject_travel_before_active}px`, "derived runtime");
  row("Travel during active", report.subject_travel_during_active === null ? "—" : `${report.subject_travel_during_active}px`, "derived runtime");
  row("Reach bound", report.aabb_reach_bound_px === null ? "—" : `${report.aabb_reach_bound_px}px`, "runtime shape bounds");
  row("Exact target overlap", `${report.target_overlap_ticks || 0} tick(s)`, report.target_overlap_source || "unavailable");
  row("AABB overlap", `${report.aabb_overlap_ticks || 0} tick(s)`, report.aabb_overlap_source || "unavailable");
  row("Contacts", (report.contacts || []).length, "resolver fact");
  row("First contact", report.first_contact_tick ?? "—", "resolver fact");
  row("Resolver outcomes", resolverOutcomes.length ? resolverOutcomes.join(", ") : "—", "causal resolver fact");
  row("Launch speed", report.target_launch_speed === null ? "—" : report.target_launch_speed, "runtime observation");
  row("Spawns", (report.spawns || []).length ? (report.spawns || []).map((x) => `${x.kind}@${x.tick}`).join(", ") : "—", "runtime observation");
  row("Damage", int(m.derived.max_damage), "prepared spec");
  row("Knockback", int(m.derived.max_knockback), "prepared spec");
  row("Cancel opportunities", cancelText, "prepared spec");

  const timeline = el("div", { class: "ufd-timeline" },
    ufdBand("Startup", report.startup ? [report.startup] : [], totalTicks, state.fighterFrame),
    ufdBand("Active", report.live_volume_windows || [], totalTicks, state.fighterFrame),
    ufdBand("Recovery", report.recovery ? [report.recovery] : [], totalTicks, state.fighterFrame),
    ufdBand("Invuln", report.invuln ? [report.invuln] : [], totalTicks, state.fighterFrame)
  );

  const canvas = el("canvas", { id: "fighter-runtime-canvas", class: "hitboxes fighter-runtime-canvas", width: 760, height: 430, "data-height": "430" });
  const scrub = el("input", {
    id: "fighter-runtime-scrub", type: "range", min: "0", max: String(lastFrame), value: String(state.fighterFrame),
    oninput: (event) => {
      cancelPlayback();
      state.fighterFrame = Number(event.target.value);
      updateFighterFrameView(c, m, scenario, take, totalTicks);
    },
  });
  const play = el("button", {
    class: `ghost${state.fighterPlaying ? " on" : ""}`,
    onclick: () => {
      if (state.fighterPlaying) {
        cancelPlayback();
        renderMoveDetail(c, m);
      } else {
        runFighterPlayback(c, m, scenario, take, totalTicks);
        renderMoveDetail(c, m);
      }
    },
  }, state.fighterPlaying ? "Pause" : "Play");
  const currentFrame = (take.frames || [])[state.fighterFrame];
  const currentSubject = currentFrame && (currentFrame.bodies || []).find((body) => roleOf(body, take) === "subject");
  const currentMove = currentSubject && currentSubject.move_state;
  const frameLabel = el("span", { id: "fighter-runtime-frame", class: "note mono" },
    `action tick ${state.fighterFrame} / ${lastFrame}` +
    (currentMove && currentMove.phase ? ` · ${currentMove.phase}` : ""));

  const authored = el("details", { class: "prepared-reference" },
    el("summary", {}, "Prepared authoring reference"),
    el("p", { class: "note" }, "These values come from prepared/exported specification data; they are not the runtime geometry or measured timing above."),
    el("dl", { class: "kv" },
      el("dt", {}, "Authored startup"), el("dd", {}, `${f1(m.derived.startup_f)} f `, provenanceBadge("prepared spec")),
      el("dt", {}, "Authored active"), el("dd", {}, `${f1(m.derived.active_f)} f `, provenanceBadge("prepared spec")),
      el("dt", {}, "Authored endlag"), el("dd", {}, `${f1(m.derived.endlag_f)} f `, provenanceBadge("prepared spec")),
      el("dt", {}, "Authored total"), el("dd", {}, `${f1(m.duration_f)} f `, provenanceBadge("prepared spec")),
      el("dt", {}, "Cancel spec"), el("dd", {}, cancelText)
    )
  );

  const gpuImg = el("img", {
    id: "fighter-engine-render",
    class: "fighter-engine-render",
    alt: "matching real-engine render for this runtime scenario",
    hidden: "",
  });
  const gpuNote = el("p", { id: "fighter-engine-note", class: "note" }, "GPU rendering starts after the runtime take is ready.");
  const gpuOverlay = el("div", { id: "fighter-engine-overlay", class: "evidence-overlay", hidden: "" });
  const geometryPanel = el("div", { class: "fighter-diagnostic" },
    el("h3", {}, "Runtime diagnostic"),
    canvas,
    el("div", { class: "legend" },
      el("span", {}, el("i", { style: "background:var(--active)" }), "attack geometry"),
      el("span", {}, el("i", { style: "background:#49c8d8" }), "effective hurt geometry"),
      el("span", {}, el("i", { style: "background:#47b78a" }), "subject-owned summon"))
  );
  const gpuPanel = el("div", { class: "fighter-gpu evidence-shell" },
    el("h3", {}, "Matching engine render"),
    gpuOverlay,
    gpuImg,
    gpuNote
  );
  const stateBadge = evidence.stale
    ? provenanceBadge(`stale: ${evidence.stale}`)
    : provenanceBadge(evidence.cache_source === "scenario_cache" ? "cached current take" : "runtime take");

  host.replaceChildren(
    controls,
    el("div", { class: "runtime-heading" },
      el("strong", {}, "Runtime frame data"),
      stateBadge),
    evidence.stale ? el("p", { class: "note err" }, `Runtime take is stale: ${evidence.stale}`) : null,
    timeline,
    kv,
    el("div", { class: "controls compact fighter-playback" }, play, scrub, frameLabel),
    el("div", { class: "fighter-evidence-grid" }, geometryPanel, gpuPanel),
    authored
  );
  drawRuntimeDiagnostic(canvas, take, state.fighterFrame, { showArt: true, showHurt: true });
  syncEngineRender(take, state.fighterFrame, scenario, { img: gpuImg, note: gpuNote, overlay: gpuOverlay });
}

/* ---------- reviews ---------- */
async function renderReview() {
  const host = $("#review");
  const c = fighterById(state.fighter);
  if (!c) { host.replaceChildren(); return; }
  const subject = state.move ? `${c.id}/${state.move}` : c.id;

  let existing = null;
  try {
    const res = await fetch(`api/review?subject=${encodeURIComponent(subject)}`);
    if (res.ok) existing = await res.json();
  } catch (_) { /* headless / file:// — the form still works, saving will report */ }

  const score = el("input", { type: "text", size: 4, value: existing?.score ?? "", placeholder: "1–10" });
  const notes = el("textarea", { placeholder: "What is wrong with it, and what would right look like?" },
    existing?.notes ?? "");
  const picked = new Set(existing?.issues ?? []);
  const tagRow = el("div", { class: "tags" }, ...ISSUE_TAGS.map((t) =>
    el("button", {
      class: `ghost ${picked.has(t) ? "on" : ""}`,
      onclick: (e) => {
        if (picked.has(t)) picked.delete(t); else picked.add(t);
        e.target.classList.toggle("on");
      },
    }, t)
  ));
  const status = el("span", { class: "note" }, existing ? `last saved ${existing.updated_at}` : "");

  const save = el("button", {
    class: "act",
    onclick: async () => {
      status.className = "note";
      status.textContent = "saving…";
      try {
        const res = await fetch("api/review", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            subject,
            character: c.id,
            move: state.move,
            score: score.value.trim() === "" ? null : Number(score.value),
            notes: notes.value,
            issues: [...picked],
            cast_generation: BUNDLE.cast_generation,
          }),
        });
        const out = await res.json();
        if (!res.ok) throw new Error(out.error || res.statusText);
        status.className = "note ok";
        status.textContent = `saved to ${out.path}`;
      } catch (err) {
        status.className = "note err";
        status.textContent = `not saved: ${err.message} (run serve_inspector.sh, not file://)`;
      }
    },
  }, "Save feedback");

  host.replaceChildren(
    el("p", { class: "note" }, "Subject: ", el("span", { class: "mono" }, subject)),
    el("div", { class: "controls" }, el("label", { class: "note" }, "Score"), score, save, status),
    tagRow,
    notes
  );
}

/* Draw a hitbox as the shape it ACTUALLY is.
 *
 * ⛔⛔ THE BOX AROUND AN ARC IS NOT THE ARC. Takes recorded every strike as its
 * axis-aligned bounding box, so a rotated box, a disc and a convex sweep all
 * drew as the rectangle that CONTAINS them — measured on this cast, that
 * overstates the strike by a median 1.8x and up to 5.3x.
 *
 * Falls back to the AABB when a take predates the shape field, so an old
 * recording still draws something honest. */
function drawHitboxShape(ctx, h, X, Y, scale) {
  const shape = h.shape;
  ctx.beginPath();
  if (shape && shape.kind === "circle") {
    ctx.arc(X(shape.center[0]), Y(shape.center[1]), shape.radius * scale, 0, Math.PI * 2);
  } else if (shape && shape.kind === "convex" && (shape.points || []).length > 2) {
    shape.points.forEach(([px, py], i) => {
      if (i === 0) ctx.moveTo(X(px), Y(py)); else ctx.lineTo(X(px), Y(py));
    });
    ctx.closePath();
  } else if (shape && shape.kind === "obb") {
    /* Rotation is CCW in screen axes, the convention the engine stores. */
    const [cx, cy] = shape.center;
    const [hx, hy] = shape.half;
    const c = Math.cos(shape.rotation);
    const sn = Math.sin(shape.rotation);
    [[-hx, -hy], [hx, -hy], [hx, hy], [-hx, hy]].forEach(([ox, oy], i) => {
      const wx = cx + ox * c - oy * sn;
      const wy = cy + ox * sn + oy * c;
      if (i === 0) ctx.moveTo(X(wx), Y(wy)); else ctx.lineTo(X(wx), Y(wy));
    });
    ctx.closePath();
  } else {
    ctx.rect(X(h.pos[0] - h.half[0]), Y(h.pos[1] - h.half[1]),
             h.half[0] * 2 * scale, h.half[1] * 2 * scale);
  }
  ctx.fill();
  ctx.stroke();
}

/* ---------- engine takes ---------- */

/* ⭐⭐ THE SEMANTIC ROLE, AND A LEGACY FALLBACK. A v2 take records what every
 * body, strike and shot IS; a v1 take recorded a seat index and a boolean, and
 * an old artifact must still draw. What an old file may contain does not define
 * what a new one may emit — the recorder writes the role. */
function roleOf(row, take) {
  if (row.role) return row.role;
  if (row.subject_owned === true) return "subject_owned";
  if (row.subject_owned === false) return "other";
  if (take && row.seat !== undefined && row.seat !== null) {
    return row.seat === take.seat ? "subject" : "target";
  }
  return "other";
}

const ROLE_COLOR = {
  subject: "#6fb3ff",
  target: "#e8a33d",
  subject_owned: "#47b78a",
  target_owned: "#b7a047",
  other: "#7d8598",
};

const ROLE_LABEL = {
  subject: "SUBJECT",
  target: "TARGET",
  subject_owned: "subject's",
  target_owned: "target's",
  other: null,
};

/* ⛔⛔ THE ROSTER COMES FROM PREPARED CONTENT, NOT FROM THE CACHE. This listed
 * `[...new Set(TAKES.takes.map(t => t.character))]`, so a fighter existed in
 * this view only once somebody had recorded it — "There are 2 fighters now, why
 * not them all?" was not a missing-data question, it was the picker asking the
 * wrong source. The bundle says who EXISTS; the takes say what has been
 * CACHED IN BULK, and a missing bulk entry is a generation state rather than a
 * missing fighter or a dead end. */
function takeRoster() {
  const recorded = new Map();
  ((TAKES && TAKES.takes) || []).forEach((t, i) => {
    if (!recorded.has(t.character)) recorded.set(t.character, []);
    recorded.get(t.character).push(i);
  });
  const rows = ((BUNDLE && BUNDLE.characters) || [])
    .filter((c) => c.on_smash_grid || recorded.has(c.id))
    .map((c) => ({
      id: c.id,
      name: c.display_name || c.id,
      takes: recorded.get(c.id) || [],
    }));
  /* A recording of somebody the current bundle no longer resolves is still
   * evidence, and hiding it would make a stale artifact invisible instead of
   * visible and labelled. */
  for (const [id, takes] of recorded) {
    if (!rows.some((r) => r.id === id)) rows.push({ id, name: `${id} — not in bundle`, takes });
  }
  return rows;
}

/* This fighter's supported moves come from the PREPARED repertoire. Bulk-corpus
 * take indexes are only optional cache hints; selection resolves or generates a
 * scenario-addressed runtime take through `/api/take`. */
function takeSlotsFor(character) {
  const recorded = new Map();
  ((TAKES && TAKES.takes) || []).forEach((t, i) => {
    if (t.character === character) recorded.set(takeVerb(t), i);
  });
  const fighter = fighterById(character);
  const verbs = new Set([...Object.keys((fighter && fighter.verbs) || {}), ...recorded.keys()]);
  return [...verbs]
    .filter(Boolean)
    .sort((a, b) => {
      const ra = SLOT_ORDER.has(a) ? SLOT_ORDER.get(a) : 1e3;
      const rb = SLOT_ORDER.has(b) ? SLOT_ORDER.get(b) : 1e3;
      return ra - rb || a.localeCompare(b);
    })
    .map((verb) => ({
      verb,
      label: SLOT_LABEL.get(verb) || verb,
      take: recorded.has(verb) ? recorded.get(verb) : null,
    }));
}

function renderTakeList() {
  const who = $("#take-fighter");
  const roster = takeRoster();
  if (!roster.length) {
    who.replaceChildren(el("option", {}, "no bundle — run moveset_export"));
    $("#take-pick").replaceChildren(el("option", {}, "—"));
    return;
  }
  /* ⭐ SAY WHAT IS PREPARED AND WHAT IS RECORDED, as two numbers. "2 fighters"
   * invites "why not all of them"; "21 prepared · 2 recorded" answers it, and
   * naming the command makes the answer actionable rather than just honest. */
  const note = $("#take-loaded");
  if (note) {
    const withTakes = roster.filter((r) => r.takes.length);
    const missing = roster.filter((r) => !r.takes.length).map((r) => r.id);
    note.textContent =
      `${roster.length} prepared · ${withTakes.length} recorded` +
      (TAKES ? ` · ${TAKES.takes.length} takes` : " · no takes file");
    note.title = missing.length
      ? `no bulk cache: ${missing.join(", ")}\n\nSelecting a move generates its canonical runtime take on demand.`
      : "every prepared fighter also has a bulk-corpus take";
  }
  /* Follow the fighter the reader was already looking at. Arriving from the
   * Fighter view and being shown somebody else is the tool losing their place. */
  if (!roster.some((r) => r.id === state.takeFighter)) {
    state.takeFighter = roster.some((r) => r.id === state.fighter)
      ? state.fighter
      : roster[0].id;
  }
  who.replaceChildren(
    ...roster.map((r) =>
      el(
        "option",
        { value: r.id, ...(r.id === state.takeFighter ? { selected: "" } : {}) },
        `${r.name}${r.takes.length ? ` · ${r.takes.length} bulk takes` : " · generate on select"}`
      )
    )
  );
  renderTakeOptions();
}

function renderTakeOptions() {
  const pick = $("#take-pick");
  const slots = takeSlotsFor(state.takeFighter);
  if (!slots.length) {
    pick.replaceChildren(el("option", {}, "this fighter binds no moves"));
    state.take = null;
    state.takeVerb = null;
    drawTake();
    return;
  }
  if (!slots.some((s) => s.verb === state.takeVerb)) {
    state.takeVerb = (slots.find((s) => s.take !== null) || slots[0]).verb;
  }
  pick.replaceChildren(
    ...slots.map((s) =>
      el(
        "option",
        { value: s.verb, ...(s.verb === state.takeVerb ? { selected: "" } : {}) },
        `${s.label}${
          s.take === null
            ? " · generate on select"
            : ` · bulk ${TAKES.takes[s.take].frames.length}f`
        }`
      )
    )
  );
  selectVerb(state.takeVerb);
}

/* Show one move of the selected fighter, recorded or not. */
function selectVerb(verb) {
  cancelPlayback();
  state.takeVerb = verb;
  state.take = null; // the interactive scenario cache, not the bulk corpus, is authoritative
  state.takeFrame = 0;
  const scrub = $("#take-scrub");
  scrub.max = "0";
  scrub.value = "0";
  drawTake();
}

function setEvidenceOverlay(selector, stateName, message) {
  const node = typeof selector === "string" ? $(selector) : selector;
  if (!node) return;
  const active = ["loading", "generating", "rendering"].includes(stateName);
  node.hidden = !active;
  node.replaceChildren(
    active ? el("span", { class: "spinner", "aria-hidden": "true" }) : null,
    active ? el("strong", {}, message || stateName) : null
  );
}

function diagnosticErrorNode(record, retry) {
  const detail = record && record.detail;
  return el("div", { class: "diagnostic-error" },
    el("strong", { class: "err" }, record.message || "Diagnostic generation failed"),
    detail ? el("details", {},
      el("summary", {}, "Show output"),
      el("pre", { class: "mono" }, JSON.stringify(detail, null, 2))) : null,
    retry ? el("button", { class: "ghost", onclick: retry }, "Retry") : null);
}

/* One geometry authority for both the Fighter and Engine Takes views. Every
 * shape comes from the runtime CombatObservation carried by a take. */
function drawRuntimeDiagnostic(canvas, take, frameIndex, { showArt = true, showHurt = true } = {}) {
  const frame = take && (take.frames || [])[frameIndex];
  if (!frame) return null;
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 1000;
  const cssH = Number(canvas.dataset.height || 560);
  canvas.width = cssW * dpr;
  canvas.height = cssH * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#0f1116";
  ctx.fillRect(0, 0, cssW, cssH);

  const view = take.view && take.view.length === 4 ? take.view : [-320, -240, 320, 240];
  const scale = Math.min(cssW / (view[2] - view[0]), cssH / (view[3] - view[1]));
  const X = (x) => (x - view[0]) * scale;
  const Y = (y) => (y - view[1]) * scale;

  ctx.fillStyle = "#232733";
  for (const platform of take.platforms || []) {
    ctx.fillRect(
      X(platform[0] - platform[2]),
      Y(platform[1] - platform[3]),
      platform[2] * 2 * scale,
      platform[3] * 2 * scale,
    );
  }

  const cursor = rowCursorsFor(take)[frameIndex];
  for (const body of frame.bodies || []) {
    const role = roleOf(body, take);
    const subject = role === "subject";
    const ticksOnRow = cursor ? cursor.get(body.id || `${body.label}#${body.seat ?? "-"}`) : 0;
    const drew = showArt && drawBodyArt(ctx, body, X, Y, scale, ticksOnRow);
    ctx.strokeStyle = ROLE_COLOR[role] || ROLE_COLOR.other;
    ctx.fillStyle = drew ? "transparent" : subject
      ? "rgba(111,179,255,.16)"
      : "rgba(125,133,152,.12)";
    ctx.lineWidth = subject ? 2 : 1;
    ctx.beginPath();
    ctx.rect(
      X(body.pos[0] - body.half[0]),
      Y(body.pos[1] - body.half[1]),
      body.half[0] * 2 * scale,
      body.half[1] * 2 * scale,
    );
    if (!drew) ctx.fill();
    ctx.stroke();

    if (showHurt) {
      ctx.strokeStyle = "#49c8d8";
      ctx.fillStyle = "rgba(73,200,216,.12)";
      ctx.lineWidth = 1;
      for (const hurt of body.hurtboxes || []) drawHitboxShape(ctx, hurt, X, Y, scale);
      if (body.hurtbox_source === "intangible") {
        ctx.fillStyle = "#49c8d8";
        ctx.font = "10px ui-monospace, monospace";
        ctx.fillText("INTANGIBLE", X(body.pos[0] - body.half[0]), Y(body.pos[1] + body.half[1]) + 12);
      }
    }

    const tag = ROLE_LABEL[role];
    ctx.font = "10px ui-monospace, monospace";
    const top = Y(body.pos[1] - body.half[1]);
    if (tag) {
      ctx.fillStyle = ROLE_COLOR[role] || ROLE_COLOR.other;
      ctx.fillText(tag, X(body.pos[0] - body.half[0]), top - 14);
    }
    if (body.label) {
      ctx.fillStyle = "#98a0b3";
      ctx.fillText(body.label, X(body.pos[0] - body.half[0]), top - 3);
    }
  }

  ctx.lineWidth = 1.5;
  for (const hit of frame.hitboxes || []) {
    const mine = roleOf(hit, take) === "subject_owned";
    ctx.strokeStyle = mine ? "#e2564a" : "rgba(226,86,74,.35)";
    ctx.fillStyle = mine ? "rgba(226,86,74,.22)" : "rgba(226,86,74,.07)";
    drawHitboxShape(ctx, hit, X, Y, scale);
  }

  for (const shot of frame.projectiles || []) {
    const mine = roleOf(shot, take) === "subject_owned";
    ctx.strokeStyle = mine ? "#e8c15a" : "rgba(232,193,90,.35)";
    ctx.fillStyle = mine ? "rgba(232,193,90,.30)" : "rgba(232,193,90,.08)";
    ctx.beginPath();
    ctx.rect(
      X(shot.pos[0] - shot.half[0]),
      Y(shot.pos[1] - shot.half[1]),
      shot.half[0] * 2 * scale,
      shot.half[1] * 2 * scale,
    );
    ctx.fill();
    ctx.stroke();
    if (shot.vel && (shot.vel[0] || shot.vel[1])) {
      ctx.beginPath();
      ctx.moveTo(X(shot.pos[0]), Y(shot.pos[1]));
      ctx.lineTo(X(shot.pos[0] + shot.vel[0] * 0.1), Y(shot.pos[1] + shot.vel[1] * 0.1));
      ctx.stroke();
    }
  }
  return frame;
}

function drawTake() {
  const canvas = $("#take-canvas");
  if (!canvas || !state.takeFighter || !state.takeVerb) return;
  const scenario = canonicalScenario(state.takeFighter, state.takeVerb);
  const record = evidenceRecord(scenario);
  const evidenceState = $("#take-evidence-state");
  if (!record) {
    if (evidenceState) { evidenceState.className = "note"; evidenceState.textContent = "loading cached data"; }
    drawNoTake(canvas, "Loading runtime diagnostic…");
    setEvidenceOverlay("#take-loading", "loading", "Loading runtime evidence…");
    if (state.view === "takes") requestTakeEvidence(scenario).catch(() => {});
    $("#take-frame").textContent = "—";
    $("#take-facts").replaceChildren(el("p", { class: "note" }, "Resolving the canonical scenario…"));
    syncEngineRender(null, 0, scenario);
    return;
  }
  if (record.state !== "ready") {
    const message = record.message || (record.state === "error" ? "Diagnostic generation failed" : "Generating runtime take…");
    if (evidenceState) { evidenceState.className = record.state === "error" ? "note err" : "note"; evidenceState.textContent = record.state === "error" ? "error" : record.state; }
    drawNoTake(canvas, message);
    setEvidenceOverlay("#take-loading", record.state, message);
    $("#take-frame").textContent = "—";
    $("#take-facts").replaceChildren(
      record.state === "error"
        ? diagnosticErrorNode(record, () => requestTakeEvidence(scenario, { force: true }).catch(() => {}))
        : el("p", { class: "note" }, message)
    );
    syncEngineRender(null, 0, scenario);
    return;
  }

  setEvidenceOverlay("#take-loading", "ready", "");
  if (evidenceState) {
    evidenceState.textContent = record.doc.stale
      ? `stale · ${record.doc.stale}`
      : `ready · ${record.doc.cache_source || "runtime"}`;
    evidenceState.className = record.doc.stale ? "note err" : "note ok";
  }
  const take = record.doc.take;
  const scrub = $("#take-scrub");
  const last = Math.max(0, (take.frames || []).length - 1);
  state.takeFrame = Math.min(state.takeFrame, last);
  scrub.max = String(last);
  scrub.value = String(state.takeFrame);
  const frame = drawRuntimeDiagnostic(canvas, take, state.takeFrame, {
    showArt: state.takeArt !== false,
    showHurt: state.takeHurt !== false,
  });
  $("#take-frame").textContent = `${state.takeFrame} / ${last}`;
  if (frame) takeFacts(take, frame);
  syncEngineRender(take, state.takeFrame, scenario);
}

/* Put transient/missing diagnostic state ON the canvas. A blank black rectangle
 * reads as a broken viewer; an explicit state label says what is happening. */
function drawNoTake(canvas, message = "No runtime take is available") {
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 1000;
  const cssH = 560;
  canvas.width = cssW * dpr;
  canvas.height = cssH * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#0f1116";
  ctx.fillRect(0, 0, cssW, cssH);
  ctx.fillStyle = "#7d8598";
  ctx.font = "13px ui-monospace, monospace";
  ctx.fillText(message, 16, cssH / 2);
}

/* The frame's own numbers. Extracted because BOTH draw paths — the engine's
 * rendered picture and the derived sprites — owe the same facts, and a reader
 * must not be able to tell which one they are looking at from the panel going
 * blank. */
function takeFacts(t, frame) {
  const kv = el("dl", { class: "kv" });
  const row = (k, v) => { kv.append(el("dt", {}, k), el("dd", {}, v)); };
  row("Take", t.label);
  row("Fighter", t.character);
  row("Frame", `${state.takeFrame}`);
  row("Move", frame.move || "—");
  row("Grounded", frame.grounded === null ? "—" : frame.grounded ? "yes" : "no");
  row("Position", `(${int(frame.subject_pos?.[0])}, ${int(frame.subject_pos?.[1])})`);
  row("Velocity", `(${int(frame.subject_vel?.[0])}, ${int(frame.subject_vel?.[1])})`);
  /* WHOSE, not how many. The opponent is live and swinging; a count that mixes
   * the two answers a question nobody asked. */
  const mineOnly = (xs) => (xs || []).filter((x) => x.subject_owned !== false).length;
  const theirs = (xs) => (xs || []).length - mineOnly(xs);
  const owned = (xs) => {
    const t = theirs(xs);
    return t ? `${mineOnly(xs)}  (+${t} opponent)` : String(mineOnly(xs));
  };
  row("Live hitboxes", owned(frame.hitboxes));
  row("Projectiles", owned(frame.projectiles));
  row("Bodies", int(frame.bodies.length));
  if (frame.riding) row("Riding", frame.riding);

  /* ⭐⭐ THE MOVE CLOCK, WHICH IS WHAT MAKES A BOX READABLE. "a red box
   * appeared" is not frame data; "0.12s of 0.68, inside the authored Active
   * window" is — and it comes from the runtime's own move state rather than
   * from counting frames in the viewer. */
  const subject = (frame.bodies || []).find((b) => roleOf(b, t) === "subject");
  const move = subject && subject.move_state;
  if (move) {
    row("Phase", move.phase || "between windows");
    row("Move clock", `${move.elapsed_s.toFixed(3)}s of ${move.duration_s.toFixed(3)}s`);
    /* The orientation the move COMMITTED to, beside the body's live facing.
     * Seeing the two disagree is what explains a strike on the far side. */
    if (move.attack_facing !== subject.facing) {
      row("Attack facing", `${move.attack_facing} (body faces ${subject.facing})`);
    }
    row("Landed hit", move.landed_hit ? "yes" : "not yet");
  }
  if (subject) {
    if (subject.hurtbox_source) {
      row("Subject hurtboxes",
          `${(subject.hurtboxes || []).length} · ${subject.hurtbox_source}`);
    }
    if (subject.damage_taken !== undefined) row("Damage taken", int(subject.damage_taken));
    if (subject.hitstun_s > 0) row("Hitstun", `${subject.hitstun_s.toFixed(3)}s`);
    if (subject.hitlag_s > 0) row("Hitlag", `${subject.hitlag_s.toFixed(3)}s`);
  }
  const target = (frame.bodies || []).find((b) => roleOf(b, t) === "target");
  if (target) {
    row("Target", `${target.label || target.id} · ${(target.hurtboxes || []).length} hurtbox(es)`);
    if (target.damage_taken !== undefined) row("Target damage", int(target.damage_taken));
    if (target.hitstun_s > 0) row("Target hitstun", `${target.hitstun_s.toFixed(3)}s`);
  }
  $("#take-facts").replaceChildren(kv);
}

/* Show the engine's rendered frame for wherever the scrubber is.
 *
 * ⛔ A MISMATCHED SEQUENCE IS REFUSED. If the engine played a different move
 * than the verb asked for, showing it here would label one move with another's
 * name — the single worst thing a reference tool can do. The panel says so and
 * the diagnostic canvas beside it carries on. */
function syncEngineRender(take, frameIndex, scenario, elements = null) {
  const img = elements?.img || $("#engine-render");
  const note = elements?.note || $("#engine-render-note");
  const overlay = elements?.overlay || "#render-loading";
  if (!img || !note) return;
  const nothing = (text) => {
    img.removeAttribute("src");
    img.hidden = true;
    note.textContent = text;
  };

  if (!scenario || !scenario.verb) {
    setEvidenceOverlay(overlay, "ready", "");
    return nothing("select a move to render it");
  }
  if (scenario.chain) {
    setEvidenceOverlay(overlay, "ready", "");
    return nothing("GPU rendering is not available for this chain scenario.");
  }
  if (!take || !(take.frames || []).length) {
    setEvidenceOverlay(overlay, "ready", "");
    return nothing("GPU rendering starts after the canonical runtime take is ready.");
  }

  const record = renderedFramesFor(scenario, take.frames.length, { stride: 2 });
  if (!elements) renderStatus(record);
  if (!record || record.state === "rendering") {
    setEvidenceOverlay(overlay, "rendering", "Rendering engine frames…");
    return nothing(`Rendering engine frames… 0 / ${Math.ceil(take.frames.length / 2)}`);
  }
  setEvidenceOverlay(overlay, "ready", "");
  if (record.state !== "ready") {
    const text = record.message || "engine render unavailable";
    note.replaceChildren(
      diagnosticErrorNode(record, () => requestRenderEvidence(
        scenario, take.frames.length, { force: true, stride: 2 }).catch(() => {}))
    );
    img.removeAttribute("src");
    img.hidden = true;
    return;
  }

  const doc = record.doc;
  if (!sameScenario(doc.scenario, scenario)) {
    return nothing("MISMATCH — GPU evidence belongs to a different scenario and was refused.");
  }
  if (doc.outcome === "not_prepared") {
    return nothing(`NOT PREPARED — ${scenario.verb} could not be staged in the requested posture.`);
  }
  if (doc.outcome === "unbound") return nothing(`UNBOUND — ${doc.reason || scenario.verb}`);
  if (doc.mismatch || doc.outcome === "missed") {
    return nothing(`MISMATCH — ${doc.reason || "the engine played another move"}`);
  }

  /* A sampled render is evidence for its own action tick only. With stride 2,
   * tick 17 does not borrow tick 16's picture and tick 149 does not freeze on
   * tick 148. The UI says exactly which ticks have images. */
  const shots = doc.shots || [];
  const index = shots.findIndex((shot) => shot.action_tick === frameIndex);
  if (index < 0) {
    const nearest = shots.reduce((best, shot) => {
      if (!best) return shot;
      return Math.abs(shot.action_tick - frameIndex) < Math.abs(best.action_tick - frameIndex)
        ? shot : best;
    }, null);
    return nothing(
      `No GPU sample for action tick ${frameIndex}` +
      (doc.stride > 1 ? ` · stride ${doc.stride}` : "") +
      (nearest ? ` · nearest sampled tick ${nearest.action_tick}` : "")
    );
  }
  const pick = shots[index];
  const url = (doc.urls || [])[index];
  if (!url) return nothing(`GPU manifest has no image URL for action tick ${frameIndex}`);
  if (img.getAttribute("src") !== url) img.setAttribute("src", url);
  img.hidden = false;
  note.textContent =
    `${doc.renderer || "moveset_render"} · ${pick.file} · action tick ${pick.action_tick}` +
    ` · sim tick ${pick.sim_tick}` +
    (doc.renderer_built ? ` · built ${doc.renderer_built}` : "") +
    (doc.cached_only ? " · CACHED ONLY" : "");
}

/* Which repertoire verb this take drove. The recorder files takes under the
 * verb it pressed, which is exactly what the renderer needs. */
function takeVerb(take) {
  return take.verb || take.label_verb || null;
}

/* ---------- status ---------- */

/* ⭐⭐ THE PAGE EXPLAINS ITSELF. All of this was already answerable on the
 * server and none of it was reachable from the browser: the provenance went to
 * a terminal the person looking at the pictures was not reading. "I can't tell
 * if it is trying to call the tool or not, or if it knows where it is" is a
 * bug in the VIEW, not in the tool it is reporting on. */
async function renderStatusView() {
  const body = $("#status-body");
  body.replaceChildren(el("p", { class: "note" }, "asking the server…"));
  let doc;
  try {
    doc = await (await fetch("/api/status")).json();
  } catch (error) {
    body.replaceChildren(el("p", { class: "note" }, `the server did not answer: ${error}`));
    return;
  }

  const panels = [];
  const panel = (title, rows, note) => {
    const kv = el("dl", { class: "kv" });
    for (const [k, v] of rows) kv.append(el("dt", {}, k), el("dd", {}, v));
    const kids = [el("h2", {}, title), kv];
    if (note) kids.push(el("p", { class: "note" }, note));
    panels.push(el("div", { class: "panel" }, ...kids));
  };

  panel("Where it is", [
    ["Repo", doc.repo],
    ["Sprites", `${doc.sprites_dir}${doc.sprites_dir_exists ? "" : "  (MISSING)"}`],
    ["Renders", doc.renders_dir],
  ], "The sprite directory is the engine's own, served read-only — the page draws exactly what the build would.");

  const b = doc.bundle || {};
  panel("Data", [
    ["Bundle", b.exists ? `${b.fighters} fighters, ${b.sheets} sheets, ${b.schema} (built ${b.built})` : "MISSING — run moveset_export"],
    ["Recording", "cargo run -p ambition_app_tools --bin moveset_takes -- --characters grid"],
    ["Takes", doc.takes && doc.takes.exists
      ? `${doc.takes.takes} takes recorded ${doc.takes.built}` +
        (doc.takes.schema ? ` (${doc.takes.schema})` : "") + " — " +
        `${doc.takes.with_art}/${doc.takes.bodies} bodies with art, ` +
        /* Both halves of the interaction, counted separately: a recording with
         * strikes and no damageable geometry cannot say why an attack missed. */
        `${doc.takes.with_hurtboxes ?? 0}/${doc.takes.bodies} with hurtboxes, ` +
        `${doc.takes.with_role ?? 0}/${doc.takes.bodies} with a role, ` +
        `${doc.takes.with_shape}/${doc.takes.hitboxes} strikes with geometry`
      : "no bulk corpus (interactive scenarios can still generate on demand)"],
    ["Scenario take cache", `${doc.cached_scenarios || 0} cached scenario(s)`],
    ["Cached renders", (doc.cached_renders || []).length ? doc.cached_renders.join(", ") : "none yet"],
  ], [
    b.exists && !b.sheets
      ? "⚠ the bundle carries no sheet table, so Engine Takes can only draw boxes — re-export with a current moveset_export."
      : "",
    /* ⛔ THE STALE-TAKES CASE, said plainly. Rebuilding the binaries does NOT
     * re-record; a recording made before these fields existed leaves the Art
     * button looking broken, which is exactly what it did. */
    doc.takes && doc.takes.stale ? `⚠ ${doc.takes.stale}` : "",
  ].filter(Boolean).join("  "));

  /* ⛔ THE BUILD COMMAND FOR EVERY BINARY, PRESENT OR NOT. Somebody refreshing a
   * two-day-old binary needs the same line as somebody who has none. */
  for (const [name, info] of Object.entries(doc.binaries || {})) {
    panel(name, [
      ["Status", info.found ? `built ${info.built}` : "NOT BUILT"],
      ["Path", info.found ? info.path : info.looked_in.join("  |  ")],
      ["Build", info.build_command],
    ], info.found ? "" : name === "moveset_render"
      ? "Without it, Engine Takes shows the diagnostic canvas alone and says why."
      : name === "moveset_takes"
        ? "Without it, prepared moves remain discoverable but a missing interactive runtime take cannot be generated."
        : "Without it the bundle already on disk is served as-is.");
  }

  body.replaceChildren(el("div", { class: "cols" }, ...panels));
}

/* ---------- shell ---------- */
function showView(name) {
  if (state.view !== name) cancelPlayback();
  state.view = name;
  for (const b of document.querySelectorAll("nav.tabs button")) b.classList.toggle("on", b.dataset.view === name);
  for (const v of document.querySelectorAll(".view")) v.classList.toggle("on", v.id === `view-${name}`);
  if (name === "compare") renderCompare();
  /* Re-pick on ENTRY, so switching fighters elsewhere and coming back here
   * lands on the fighter you were reading rather than wherever you last were. */
  if (name === "takes") renderTakeList();
  if (name === "status") renderStatusView();
}

async function boot() {
  try {
    /* ⛔⛔ CACHE-BUSTED, NOT TRUSTED TO HEADERS. A `no-store` header only helps a
     * browser that has not ALREADY cached the file, and this bundle and the
     * takes beside it were served for a day with no cache directives at all. A
     * stale 5.7MB takes.json is invisible and total: the fighter list shows the
     * characters that recording had, the art shows what it carried, and every
     * one of those is a phantom bug in the tool rather than in the data on disk.
     *
     * ⭐ IT IS A LOCALHOST DEV TOOL READING GENERATED ARTIFACTS. Always-fresh is
     * the only correct policy, and a query parameter enforces it without asking
     * anybody to know about a hard reload. */
    const res = await fetch(`data/moveset_bundle.json?t=${Date.now()}`);
    BUNDLE = await res.json();
  } catch (err) {
    $("#bundle-meta").innerHTML =
      `<span class="err">no bundle — run <span class="mono">cargo run -p ambition_app_tools --bin moveset_export</span></span>`;
    return;
  }
  $("#bundle-meta").textContent =
    `${BUNDLE.characters.length} fighters · ${BUNDLE.smash_grid.length} on the grid · cast generation ${BUNDLE.cast_generation} · ${BUNDLE.sim_hz}Hz`;

  try {
    const res = await fetch(`data/takes/takes.json?t=${Date.now()}`);
    if (res.ok) TAKES = await res.json();
  } catch (_) { /* takes are optional; the static views stand alone */ }

  $("#fighter-pick").replaceChildren(...BUNDLE.characters.map((c) =>
    el("option", { value: c.id }, `${c.display_name || c.id}${c.on_smash_grid ? "" : " (off-grid)"}`)));
  $("#fighter-pick").addEventListener("change", (e) => openFighter(e.target.value));
  populateScenarioTargetControls();
  syncScenarioControls();
  for (const prefix of ["fighter", "take"]) {
    for (const suffix of ["target", "behavior", "spacing"]) {
      $(`#${prefix}-${suffix}`).addEventListener("change", () => scenarioInputsChanged(prefix));
    }
  }

  const bound = new Set(BUNDLE.characters.flatMap((c) => Object.keys(c.verbs)));
  $("#slot-pick").replaceChildren(...SLOTS.filter(([v]) => bound.has(v)).map(([v, label]) =>
    el("option", { value: v, selected: v === state.slot }, label)));
  $("#slot-pick").addEventListener("change", (e) => { state.slot = e.target.value; renderCompare(); });

  $("#roster-search").addEventListener("input", renderRoster);
  $("#grid-only").addEventListener("click", (e) => {
    state.gridOnly = !state.gridOnly;
    e.target.classList.toggle("on", state.gridOnly);
    renderRoster();
  });
  $("#compare-grid-only").addEventListener("click", (e) => {
    state.compareGridOnly = !state.compareGridOnly;
    e.target.classList.toggle("on", state.compareGridOnly);
    renderCompare();
  });
  for (const b of document.querySelectorAll("nav.tabs button")) {
    b.addEventListener("click", () => showView(b.dataset.view));
  }
  $("#take-pick").addEventListener("change", (e) => selectVerb(e.target.value));
  $("#take-fighter").addEventListener("change", (e) => {
    state.takeFighter = e.target.value;
    renderTakeOptions();
  });
  $("#take-scrub").addEventListener("input", (e) => { cancelPlayback(); state.takeFrame = Number(e.target.value); drawTake(); });
  $("#regen-take").addEventListener("click", () => {
    if (!state.takeFighter || !state.takeVerb) return;
    requestTakeEvidence(canonicalScenario(state.takeFighter, state.takeVerb), { force: true }).catch(() => {});
  });
  $("#regen-render").addEventListener("click", () => {
    if (!state.takeFighter || !state.takeVerb) return;
    const scenario = canonicalScenario(state.takeFighter, state.takeVerb);
    const record = evidenceRecord(scenario);
    if (record && record.state === "ready") {
      requestRenderEvidence(scenario, record.doc.take.frames.length, { force: true, stride: 2 }).catch(() => {});
    }
  });
  $("#regen-all").addEventListener("click", async () => {
    if (!state.takeFighter || !state.takeVerb) return;
    const scenario = canonicalScenario(state.takeFighter, state.takeVerb);
    const doc = await requestTakeEvidence(scenario, { force: true }).catch(() => null);
    if (doc) await requestRenderEvidence(scenario, doc.take.frames.length, { force: true, stride: 2 }).catch(() => {});
  });
  /* The art can be turned off. A hitbox that sits behind a big sprite is hard to
   * read, and "where exactly is this volume" is a question the boxes answer
   * better alone — so this view can be either instrument. */
  $("#status-refresh").addEventListener("click", renderStatusView);
  /* The hurtboxes can be turned off for the same reason the art can: two
   * overlapping volume sets on one body is exactly the picture somebody opens
   * this view to check, and exactly the picture that is hardest to read. */
  $("#take-hurt").addEventListener("click", (e) => {
    state.takeHurt = state.takeHurt === false;
    e.target.classList.toggle("on", state.takeHurt !== false);
    drawTake();
  });
  $("#take-art").addEventListener("click", (e) => {
    state.takeArt = state.takeArt === false;
    e.target.classList.toggle("on", state.takeArt !== false);
    drawTake();
  });
  $("#take-play").addEventListener("click", () => {
    if (state.playing) cancelPlayback();
    else runTakePlayback();
  });

  state.fighter = (BUNDLE.characters.find((c) => c.on_smash_grid) || BUNDLE.characters[0])?.id || null;
  renderRoster();
  renderTakeList();
  if (state.fighter) { $("#fighter-pick").value = state.fighter; renderFighter(); }
}

boot();
