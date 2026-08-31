"use strict";
/* Ambition moveset balance inspector.
 *
 * Reads the bundle `moveset_export` writes and answers the questions a balance
 * pass asks: what does this move cost, what does it buy, and is it in line with
 * the rest of the cast. Nothing here re-derives frame data — the exporter did
 * that against the composed host, and a second implementation would be a second
 * thing to keep true.
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
function renderedFramesFor(character, verb, scenario = {}) {
  /* ⛔⛔ KEYED ON CHARACTER **AND VERB**. This asked for a character alone, and
   * the endpoint photographed a fighter STANDING — so every move of a fighter
   * shared one cache entry of somebody doing nothing. A move renderer that
   * ignores which move was selected is the bug this whole campaign was about.
   *
   * ⛔⛔ AND ON THE SCENARIO. This panel sits BESIDE the diagnostic canvas, and a
   * render staged from across the stage next to a take recorded at 40px is two
   * different fights presented as one. The take's own target and spacing travel
   * with the request. */
  if (!character || !verb) return null;
  const params = new URLSearchParams({ character, verb, frames: "24", stride: "2" });
  if (scenario.target && scenario.target !== character) params.set("target", scenario.target);
  if (scenario.spacing !== null && scenario.spacing !== undefined) {
    params.set("spacing", String(scenario.spacing));
  }
  /* ⛔⛔ AND WHETHER THE OPPONENT FIGHTS BACK. The renderer defaults a missing
   * behaviour to PASSIVE, so a take recorded against a live CPU opponent was
   * shown beside a render of a target standing still — the same class of "two
   * fights presented as one" the target and spacing above exist to prevent. */
  if (scenario.behavior) params.set("target_behavior", String(scenario.behavior));
  const key = `${character}/${verb}/${params.get("target") || ""}/` +
    `${params.get("spacing") || ""}/${params.get("target_behavior") || ""}`;
  const have = RENDERS.get(key);
  if (have !== undefined) return have;
  RENDERS.set(key, null);
  fetch(`/api/render?${params}`)
    .then((r) => r.json())
    .then((doc) => {
      if (!doc || !doc.available || !doc.urls || !doc.urls.length) {
        /* Remember the refusal so the page does not re-ask on every redraw. */
        /* ⛔ THE COMPOSITE KEY, like every other write here. This stored the
         * refusal under `character` alone while lookups use `character/verb`,
         * so a failure was never found again and the page re-asked the endpoint
         * on every redraw — a failed GPU render re-spawning the renderer once a
         * frame. */
        RENDERS.set(key, {
          available: false,
          reason: doc && doc.reason,
          hint: doc && doc.hint,
        });
        renderStatus(key);
        /* ⛔ AND REPAINT. The panel is showing "rendering…" and the answer has
         * arrived; without this it keeps saying that until something else
         * happens to redraw. */
        if (state.view === "takes") drawTake();
        return;
      }
      const images = doc.urls.map((u) => {
        const img = new Image();
        img.src = u;
        img.addEventListener("load", () => { if (state.view === "takes") drawTake(); });
        return img;
      });
      RENDERS.set(key, {
        ...doc,
        available: true,
        images,
        stride: doc.stride,
        renderer: doc.renderer,
        built: doc.renderer_built,
      });
      renderStatus(key);
      /* ⛔⛔ REPAINT ON THE MANIFEST, NOT ONLY ON AN IMAGE `load`. A cached image
       * can be `complete` before its listener is attached, so the load event
       * never fires and the panel keeps saying "rendering…" with every frame
       * already in the page. The manifest arriving IS the moment there is
       * something to draw. */
      if (state.view === "takes") drawTake();
    })
    .catch((error) => {
      RENDERS.set(key, { available: false, reason: String(error) });
      renderStatus(key);
      if (state.view === "takes") drawTake();
    });
  return null;
}

/* Say WHICH picture is on screen. A view that silently swaps between engine
 * frames and a CPU approximation is a view whose fidelity nobody can trust. */
function renderStatus(key) {
  const node = $("#take-source");
  if (!node) return;
  /* ⛔⛔ A RECORDING WITH NO ART MAKES THE ART BUTTON LOOK BROKEN. Pressing it
   * toggles between sprites and boxes, and with nothing to toggle TO the page
   * simply redraws the same picture — which reads as a dead control rather than
   * as missing data. Say so where the button is. */
  if (TAKES && !takesCarryArt()) {
    node.textContent = "sprites: none in this recording — re-run moveset_takes";
    node.title = "cargo run -p ambition_app_tools --bin moveset_takes -- --characters <id>";
    return;
  }
  const have = RENDERS.get(key);
  if (!have) { node.textContent = "sprites: derived (asking the engine…)"; return; }
  /* WHICH BINARY DREW THIS, AND WHEN IT WAS BUILT. Nothing in this tool builds,
   * so that stamp is the only thing separating a current picture from one taken
   * before an hour of engine changes. On the unavailable path, the build command
   * is the useful half — a reason without a remedy is just a complaint. */
  /* ⛔ AVAILABLE IS NOT THE SAME AS SHOWN. A mismatched or unbound render is a
   * perfectly available manifest that the panel REFUSES to display, and this
   * said "rendered by the engine" beside a panel saying UNBOUND. */
  const refused = have.available && (have.mismatch || have.outcome === "unbound"
    || have.outcome === "missed" || have.outcome === "not_prepared");
  node.textContent = refused
    ? `sprites: derived — the engine render is ${have.outcome || "a mismatch"} for this verb`
    : have.available
      ? `sprites: rendered by the engine${have.built ? ` (moveset_render built ${have.built})` : ""}`
      : `sprites: derived — ${have.reason || "engine render unavailable"}`;
  node.title = refused ? (have.reason || "") : have.available ? (have.renderer || "") : (have.hint || "");
}

const SHEETS = new Map();
function sheetImage(key) {
  if (SHEETS.has(key)) return SHEETS.get(key);
  const meta = BUNDLE && BUNDLE.sheets && BUNDLE.sheets[key];
  if (!meta) { SHEETS.set(key, null); return null; }
  const pages = (meta.images && meta.images.length ? meta.images : [meta.image]).map((name) => {
    const img = new Image();
    img.src = `/art/${name}`;
    /* A redraw when the bytes land, or the first frame a sheet appears on stays
     * a box until something else happens to repaint. */
    img.addEventListener("load", () => { if (state.view === "takes") drawTake(); });
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
  /* Which MOVE is selected — a verb from prepared content, which exists whether
   * or not a recording of it does. `take` is the cached recording for it, or
   * null: the fighter and the move come from the bundle, the frames from the
   * cache. */
  takeVerb: null,
  /* Whether the cyan damageable volumes are drawn. */
  takeHurt: true,
  takeFrame: 0,
  playing: false,
  /* ⛔⛔ WHICH VIEW IS ON SCREEN, and it was READ IN TWO PLACES AND WRITTEN IN
   * NONE. Both the sprite-sheet loader and the engine-render loader redraw with
   * `if (state.view === "takes") drawTake()`, and against a field nothing ever
   * assigned that condition is false forever — so an image arriving after the
   * last draw NEVER repainted. The engine panel sat on "rendering special_up…"
   * with all 24 PNGs already loaded in the page, and a sheet that finished late
   * left boxes where its art should have been. Nothing but a browser could find
   * this: every endpoint was correct and every file was served. */
  view: "fighter",
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
  );
  $("#roster-count").textContent = `${list.length} of ${BUNDLE.characters.length} fighters`;
}

/* ---------- fighter ---------- */
function openFighter(id) {
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
      onclick: () => { state.move = m.id; renderMoveTable(c); renderMoveDetail(c, m); renderReview(); },
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

function renderMoveDetail(c, m) {
  $("#move-title").textContent = `${m.display_name || m.id} · ${SLOT_LABEL.get(slotOf(m)) || "unbound"}`;
  const host = $("#move-detail");
  const total = Math.max(m.duration_s, 0.0001);

  const bar = el("div", { class: "timeline" },
    ...m.windows.map((w) =>
      el("div", {
        class: `win ${winClass(w.tag)}`,
        title: `${w.tag} ${f1(w.start_f)}–${f1(w.end_f)}f` +
               (w.cancel_into.length
                 ? ` → ${w.cancel_into.join(", ")}` +
                   ((w.cancel_into_resolved || []).length
                     ? ` = ${w.cancel_into_resolved.join(", ")}`
                     : "")
                 : "") +
               (w.motion_scale !== 1 ? ` · motion ×${f2(w.motion_scale)}` : ""),
        style: `left:${(w.start_s / total) * 100}%;width:${((w.end_s - w.start_s) / total) * 100}%`,
      })
    ),
    ...m.events.map((e) =>
      el("div", {
        class: "ev",
        title: `${f1(e.at_f)}f ${e.kind}${e.detail ? " " + e.detail : ""}`,
        style: `left:${(e.at_s / total) * 100}%`,
      })
    )
  );

  const ticks = [];
  for (let f = 0; f <= m.duration_f; f += 5) {
    ticks.push(el("span", { style: `left:${(f / m.duration_f) * 100}%` }, String(f)));
  }

  const kv = el("dl", { class: "kv" });
  const row = (k, v) => { kv.append(el("dt", {}, k), el("dd", {}, v)); };
  row("Clip", m.clip);
  row("Startup", `${f1(m.derived.startup_f)} f`);
  row("Active", `${f1(m.derived.active_f)} f`);
  row("Endlag", `${f1(m.derived.endlag_f)} f`);
  row("Total", `${f1(m.duration_f)} f`);
  row("Damage", `${int(m.derived.max_damage)}${m.derived.sum_damage !== m.derived.max_damage ? ` (${m.derived.sum_damage} all hits)` : ""}`);
  if (m.smash_charge_mult > 1) row("Charged", `×${f2(m.smash_charge_mult)} → ${int(m.derived.max_damage_charged)}`);
  if (m.charge) row("Holds", `${f2(m.charge.max_hold_s)}s on ${m.charge.gesture}`);
  row("Knockback", int(m.derived.max_knockback));
  row("Reach", `${int(m.derived.reach)} × ${int(m.derived.vertical_reach)} px`);
  row("Posture", m.gates.grounded === null ? "either" : m.gates.grounded ? "grounded" : "airborne");
  if (m.gates.recovery !== "none") row("Recovery", m.gates.recovery.replace(/_/g, " "));
  if (m.gates.forbidden_while_held) row("While held", "forbidden");
  if (m.gates.roots_steering) row("Steering", "rooted");
  if (m.landing_lag_s !== null) row("Landing lag", `${f1(m.landing_lag_s * BUNDLE.sim_hz)} f`);
  if (m.autocancel_after_s !== null) row("Autocancel", `${f1(m.autocancel_after_s * BUNDLE.sim_hz)} f`);
  if (m.start_impulse) row("Start impulse", `(${int(m.start_impulse[0])}, ${int(m.start_impulse[1])})`);
  if (m.repeat) row("Loops", `${f2(m.repeat.from_s)}–${f2(m.repeat.to_s)}s, max ${f2(m.repeat.max_s)}s`);
  /* THE SHOT, AS ITS OWN OFFENCE. A pure ranged attack has no body hitbox, so
   * every melee row above it reads 0 — and "Fires: the body's ranged action"
   * left a move that hits for 14 looking harmless. The numbers are reported
   * BESIDE the body's, never folded into them: a projectile is not a melee
   * hitbox, and a balance view that conflated them would be lying about reach,
   * about trades, and about what a shield is for. */
  if (m.derived.fires_projectile) {
    const d = m.derived;
    row("Fires", d.projectile_source === "equipped"
      ? "an equipped weapon"
      : "the body's ranged action");
    if (d.fire_f !== null && d.fire_f !== undefined) row("Fire frame", `${f1(d.fire_f)} f`);
    if (d.projectile_damage !== null && d.projectile_damage !== undefined) {
      row("Shot damage", d.projectile_damage_charged
        ? `${int(d.projectile_damage)} → ${int(d.projectile_damage_charged)} charged`
        : int(d.projectile_damage));
    }
    if (d.projectile_speed !== null && d.projectile_speed !== undefined) {
      row("Shot speed", d.projectile_speed_charged
        ? `${int(d.projectile_speed)} → ${int(d.projectile_speed_charged)} px/s charged`
        : `${int(d.projectile_speed)} px/s`);
    }
    if (d.projectile_size_charged) row("Shot size", `×${f2(d.projectile_size_charged)} charged`);
  }

  /* ⭐⭐ THE AUTHORED CANCEL GRAPH, RESOLVED. `["attack", "smash",
   * "any_attack"]` is what somebody wrote; the question a reader has is WHICH
   * MOVES that is, and the answer is this character's own repertoire.
   *
   * ⛔ THE EXPORTER RESOLVES IT, NOT THIS FILE. `MovesetContract::cancel_targets`
   * matches on the same verb-class names the trigger road matches on; teaching
   * the browser that vocabulary would be a second copy of it, and two copies
   * that must agree are one copy plus a bug. */
  const cancels = (m.windows || []).filter((w) => (w.cancel_into || []).length);
  if (cancels.length) {
    for (const w of cancels) {
      const when = w.tag.split(":")[1] || "always";
      const resolved = w.cancel_into_resolved || [];
      row(
        `Cancels (${when})`,
        `${f1(w.start_f)}–${f1(w.end_f)}f → ` +
          (resolved.length
            ? `${resolved.join(", ")}   [authored: ${w.cancel_into.join(", ")}]`
            : /* A rule that resolves to nothing names moves this fighter does
               * not have — worth seeing rather than hiding. */
              `${w.cancel_into.join(", ")} — resolves to NO move this fighter has`)
      );
    }
  }

  const canvas = el("canvas", { class: "hitboxes", width: 420, height: 300 });

  const events = m.events.length
    ? el("div", { class: "note", style: "margin-top:8px" },
        "Events: ",
        m.events.map((e) => `${f1(e.at_f)}f ${e.kind}${e.detail ? ` ${e.detail}` : ""}`).join(" · "))
    : null;

  host.replaceChildren(
    bar,
    el("div", { class: "ruler" }, ...ticks),
    el("div", { class: "legend" },
      el("span", {}, el("i", { style: "background:var(--startup)" }), "startup"),
      el("span", {}, el("i", { style: "background:var(--active)" }), "active"),
      el("span", {}, el("i", { style: "background:var(--recovery)" }), "recovery"),
      el("span", {}, el("i", { style: "background:var(--invuln)" }), "invuln"),
      el("span", {}, el("i", { style: "background:var(--armor)" }), "armor"),
      el("span", {}, el("i", { style: "background:var(--cancel)" }), "cancelable")),
    kv,
    canvas,
    events
  );
  drawHitboxes(canvas, c, m);
}

/* Body-local hitboxes, drawn against the fighter's own silhouette.
 *
 * ⛔ `+y` IS GRAVITY-DOWN in every authored offset, which is the opposite of a
 * canvas's own y only in sign convention — the catalog's `+y` and the canvas's
 * `+y` both point down the screen, so no flip is applied. A flip here would put
 * every up-tilt under the fighter's feet. */
function drawHitboxes(canvas, c, m) {
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 420;
  canvas.width = cssW * dpr;
  canvas.height = 300 * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, cssW, 300);

  const volumes = m.windows.flatMap((w) => w.volumes.map((v) => ({ v, w })));
  /* The fighter's own box. Nothing authors an explicit body today, so this is
   * the genre's standing silhouette scaled by the authored height when there is
   * one — a reference to judge reach against, labelled as such rather than
   * presented as measured geometry. */
  const bodyH = c.vitals.canonical_height || 64;
  const bodyHalf = [bodyH * 0.28, bodyH * 0.5];

  let maxX = bodyHalf[0], maxY = bodyHalf[1];
  for (const { v } of volumes) {
    maxX = Math.max(maxX, Math.abs(v.offset[0]) + v.half_extents[0]);
    maxY = Math.max(maxY, Math.abs(v.offset[1]) + v.half_extents[1]);
  }
  const pad = 16;
  const scale = Math.min((cssW / 2 - pad) / (maxX || 1), (300 / 2 - pad) / (maxY || 1));
  const ox = cssW / 2, oy = 150;
  const X = (x) => ox + x * scale;
  const Y = (y) => oy + y * scale;

  /* ground line at the fighter's feet */
  ctx.strokeStyle = "#2a3040";
  ctx.beginPath();
  ctx.moveTo(0, Y(bodyHalf[1]));
  ctx.lineTo(cssW, Y(bodyHalf[1]));
  ctx.stroke();

  ctx.strokeStyle = "#6fb3ff";
  ctx.fillStyle = "rgba(111,179,255,.10)";
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.rect(X(-bodyHalf[0]), Y(-bodyHalf[1]), bodyHalf[0] * 2 * scale, bodyHalf[1] * 2 * scale);
  ctx.fill();
  ctx.stroke();

  /* facing arrow, so "+x = facing" is visible rather than remembered */
  ctx.strokeStyle = "#98a0b3";
  ctx.beginPath();
  ctx.moveTo(X(0), Y(bodyHalf[1] + 10));
  ctx.lineTo(X(maxX * 0.35), Y(bodyHalf[1] + 10));
  ctx.lineTo(X(maxX * 0.28), Y(bodyHalf[1] + 6));
  ctx.stroke();

  for (const { v, w } of volumes) {
    const active = w.tag === "active";
    ctx.strokeStyle = active ? "#e2564a" : "#8a5f5a";
    ctx.fillStyle = active ? "rgba(226,86,74,.22)" : "rgba(138,95,90,.12)";
    ctx.lineWidth = active ? 1.5 : 1;
    if (v.radius !== null && v.radius !== undefined) {
      ctx.beginPath();
      ctx.arc(X(v.offset[0]), Y(v.offset[1]), v.radius * scale, 0, Math.PI * 2);
      ctx.fill(); ctx.stroke();
    } else {
      const x = X(v.offset[0] - v.half_extents[0]);
      const y = Y(v.offset[1] - v.half_extents[1]);
      ctx.beginPath();
      ctx.rect(x, y, v.half_extents[0] * 2 * scale, v.half_extents[1] * 2 * scale);
      ctx.fill(); ctx.stroke();
    }
    /* the launch angle this box commits to */
    if (v.launch_dir) {
      const len = 26;
      const n = Math.hypot(v.launch_dir[0], v.launch_dir[1]) || 1;
      ctx.strokeStyle = "#e6c14a";
      ctx.beginPath();
      ctx.moveTo(X(v.offset[0]), Y(v.offset[1]));
      ctx.lineTo(X(v.offset[0]) + (v.launch_dir[0] / n) * len, Y(v.offset[1]) + (v.launch_dir[1] / n) * len);
      ctx.stroke();
    }
    ctx.fillStyle = "#e6e9f0";
    ctx.font = "11px ui-monospace, monospace";
    ctx.fillText(`${v.damage}`, X(v.offset[0]) + 3, Y(v.offset[1]) - 3);
  }

  ctx.fillStyle = "#98a0b3";
  ctx.font = "10px ui-monospace, monospace";
  ctx.fillText(`${Math.round(1 / scale * 10) / 10} px/unit · body is a ${Math.round(bodyH)}px reference`, 6, 292);
}

/* ---------- compare ---------- */
function median(xs) {
  const v = xs.filter((x) => Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return null;
  const mid = v.length >> 1;
  return v.length % 2 ? v[mid] : (v[mid - 1] + v[mid]) / 2;
}

const COMPARE_COLUMNS = [
  ["fighter", "Fighter", (r) => r.c.display_name || r.c.id, null],
  ["move", "Move", (r) => r.m.display_name || r.m.id, null],
  ["startup", "Startup f", (r) => r.m.derived.startup_f, f1],
  ["active", "Active f", (r) => r.m.derived.active_f, f1],
  ["endlag", "Endlag f", (r) => r.m.derived.endlag_f, f1],
  ["total", "Total f", (r) => r.m.duration_f, f1],
  ["damage", "Dmg", (r) => r.m.derived.max_damage, int],
  ["charged", "Dmg×", (r) => r.m.derived.max_damage_charged, int],
  ["kb", "KB", (r) => r.m.derived.max_knockback, int],
  /* The compare view's whole job is "is this slot out of line", and a roster
   * whose ranged fighters all read 0 damage answers that wrong for every one
   * of them. */
  ["shot", "Shot", (r) => r.m.derived.projectile_damage, int],
  ["shotf", "Fire f", (r) => r.m.derived.fire_f, f1],
  ["growth", "Growth", (r) => {
    const gs = r.m.windows.flatMap((w) => w.volumes.map((v) => v.knockback_growth))
      .filter((g) => g !== null && g !== undefined);
    return gs.length ? Math.max(...gs) : null;
  }, f2],
  ["reach", "Reach", (r) => r.m.derived.reach, int],
  ["hp", "HP", (r) => r.c.vitals.max_health, int],
];

function renderCompare() {
  const rows = [];
  for (const c of fighters(state.compareGridOnly)) {
    const moveId = c.verbs[state.slot];
    if (!moveId) continue;
    const m = c.moves.find((x) => x.id === moveId);
    if (m) rows.push({ c, m });
  }

  /* Flag a cell against the roster's own middle for this slot. A median, not a
   * mean: one 5x outlier drags a mean until nothing else reads as unusual, and
   * finding that outlier is the whole point of the view. */
  const stats = {};
  for (const [key, , get] of COMPARE_COLUMNS) {
    /* ⛔⛔ FILTER ABSENCE BEFORE CONVERTING, because `Number(null)` is 0 and
     * `Number.isFinite(0)` is true. A row the table draws as an em dash was
     * contributing a ZERO to this median, pulling it down and making genuinely
     * small values read as ordinary. Projectile-only moves currently have a
     * null startup, so this is not hypothetical (GPT 5.6, 2026-08-27). */
    const vals = rows
      .map(get)
      .filter((v) => v !== null && v !== undefined)
      .map(Number)
      .filter(Number.isFinite);
    const med = median(vals);
    const spread = med === null ? null
      : median(vals.map((v) => Math.abs(v - med))) || (med * 0.15) || 1;
    stats[key] = { med, spread, max: Math.max(...vals, 0) };
  }

  const col = COMPARE_COLUMNS.find((x) => x[0] === state.compareSort.key) || COMPARE_COLUMNS[0];
  rows.sort((a, b) => {
    let av = col[2](a), bv = col[2](b);
    if (av === null || av === undefined) av = -Infinity;
    if (bv === null || bv === undefined) bv = -Infinity;
    const cmp = typeof av === "string" ? av.localeCompare(bv) : av - bv;
    return state.compareSort.asc ? cmp : -cmp;
  });

  const head = el("tr", {}, ...COMPARE_COLUMNS.map(([key, label]) =>
    el("th", {
      class: state.compareSort.key === key ? `sorted ${state.compareSort.asc ? "asc" : ""}` : "",
      onclick: () => {
        if (state.compareSort.key === key) state.compareSort.asc = !state.compareSort.asc;
        else state.compareSort = { key, asc: key === "fighter" || key === "move" };
        renderCompare();
      },
    }, label)
  ));

  const body = el("tbody", {}, ...rows.map((r) =>
    el("tr", { onclick: () => { openFighter(r.c.id); state.move = r.m.id; renderFighter(); renderMoveDetail(r.c, r.m); } },
      ...COMPARE_COLUMNS.map(([key, , get, fmt]) => {
        const raw = get(r);
        /* Absent is ABSENT, not zero — see the median above. Without this a
         * cell showing an em dash still took a hot/cold class and drew a
         * zero-width bar, both computed from a value it does not have. */
        const num = raw === null || raw === undefined ? NaN : Number(raw);
        const s = stats[key];
        let cls = fmt ? "mono bar" : "";
        if (fmt && s && s.med !== null && Number.isFinite(num) && s.spread > 0) {
          const z = (num - s.med) / s.spread;
          if (z > 2) cls += " hot";
          else if (z < -2) cls += " cold";
        }
        const cell = el("td", { class: cls }, fmt ? fmt(raw) : String(raw));
        if (fmt && s && s.max > 0 && Number.isFinite(num)) {
          cell.prepend(el("span", { style: `width:${Math.max(0, (num / s.max) * 100)}%` }));
        }
        return cell;
      }))
  ));
  $("#compare").replaceChildren(el("thead", {}, head), body);
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
 * RECORDED, and a missing recording is missing evidence rather than a missing
 * fighter. */
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

/* This fighter's supported moves, from the PREPARED repertoire, each with the
 * recording the cache holds for it — or none.
 *
 * ⭐ A MOVE WITH NO TAKE IS STILL SELECTABLE. The engine render is produced on
 * demand per character+verb and needs no recording at all, so an unrecorded
 * fighter is inspectable the moment it is prepared. */
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
      ? `not recorded: ${missing.join(", ")}\n\n` +
        "cargo run -p ambition_app_tools --bin moveset_takes -- --characters grid"
      : "every prepared fighter is recorded";
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
        `${r.name}${r.takes.length ? ` · ${r.takes.length} takes` : " · not recorded"}`
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
            ? " · not recorded"
            : ` (${TAKES.takes[s.take].frames.length}f)`
        }`
      )
    )
  );
  selectVerb(state.takeVerb);
}

/* Show one move of the selected fighter, recorded or not. */
function selectVerb(verb) {
  state.takeVerb = verb;
  const slot = takeSlotsFor(state.takeFighter).find((s) => s.verb === verb);
  state.take = slot && slot.take !== null ? slot.take : null;
  state.takeFrame = 0;
  const scrub = $("#take-scrub");
  const frames = state.take === null ? 0 : TAKES.takes[state.take].frames.length;
  scrub.max = String(Math.max(frames - 1, 0));
  scrub.value = "0";
  drawTake();
}

function drawTake() {
  const canvas = $("#take-canvas");
  if (!canvas) return;
  const t = state.take === null || !TAKES ? null : TAKES.takes[state.take];
  const frame = t ? t.frames[state.takeFrame] : null;
  /* ⭐ AN UNRECORDED MOVE IS STILL A MOVE. The diagnostic canvas needs a take;
   * the engine render does not, so the panel beside it can still photograph
   * this move on demand and the reader is told which half is missing. */
  if (!frame) {
    drawNoTake(canvas);
    syncEngineRender({ character: state.takeFighter, verb: state.takeVerb }, 0);
    $("#take-frame").textContent = "—";
    $("#take-facts").replaceChildren(
      el("p", { class: "note" },
        `no recording for ${state.takeFighter} · ${state.takeVerb || "—"}. `,
        el("span", { class: "mono" },
          `cargo run -p ambition_app_tools --bin moveset_takes -- --characters ${state.takeFighter}`))
    );
    return;
  }
  const ctx = canvas.getContext("2d");
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 1000;
  const cssH = 560;
  canvas.width = cssW * dpr; canvas.height = cssH * dpr;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = "#0f1116";
  ctx.fillRect(0, 0, cssW, cssH);

  /* The take carries the stage rectangle it was recorded in, so the view is the
   * same across every take rather than a per-frame autoscale that makes a
   * fighter look stationary while the world slides. */
  /* ⛔ A TAKE WITH NO VIEW STILL DRAWS. `view[2]` on an absent rectangle throws,
   * and a throw here kills the playback timer — the failure `check_draw_path`
   * exists for. The recorder's own fallback for a take it could not measure is
   * this same rectangle. */
  const view = t.view && t.view.length === 4 ? t.view : [-320, -240, 320, 240];
  const scale = Math.min(cssW / (view[2] - view[0]), cssH / (view[3] - view[1]));
  const X = (x) => (x - view[0]) * scale;
  const Y = (y) => (y - view[1]) * scale;

  /* ⛔⛔ THE ENGINE RENDER IS NOT COMPOSITED HERE, AND THE ATTEMPT WAS WRONG.
   * `/api/render` photographs a fighter standing in `hall_of_characters`: a
   * whole-room shot, in the CAMERA's coordinate space, of a scene that is not
   * this take. Drawing it as the canvas background and then overlaying hitboxes
   * computed in the TAKE's world coordinates put two unrelated spaces on one
   * picture — so the strike landed nowhere near the fighter and the whole thing
   * read as "a room with a box on it". A view that draws a wrong picture is
   * worse than one that draws none, because it is believed.
   *
   * ⭐ THE RENDER COMES BACK when it is driven PER MOVE and reports the camera
   * transform it was taken with, which are the two things that would let a
   * hitbox be placed on it. Until then the derived sprites below are drawn in
   * the take's own space, which is at least self-consistent. */

  /* ⭐⭐ THE ENGINE'S OWN PICTURE, IN ITS OWN PANEL. It is NOT composited onto
   * this canvas: the render is a whole-scene shot in the CAMERA's space and the
   * boxes below are in the TAKE's world space, and drawing one over the other
   * put a strike nowhere near its fighter — the thing Jon saw as "a room with a
   * hitbox drawn randomly on it". Side by side gives the real art AND accurate
   * diagnostics without conflating two coordinate systems.
   *
   * ⭐ SYNCHRONISED BY `action_tick`, not by absolute `sim_tick`. The recorded
   * take and the GPU run are separate sessions with no shared origin; what they
   * share is how far into the EXERCISE each frame is. */
  syncEngineRender(t, state.takeFrame);

  /* platforms */
  ctx.fillStyle = "#232733";
  for (const p of t.platforms || []) {
    ctx.fillRect(X(p[0] - p[2]), Y(p[1] - p[3]), p[2] * 2 * scale, p[3] * 2 * scale);
  }

  for (const b of frame.bodies) {
    const role = roleOf(b, t);
    const subject = role === "subject";
    /* ART FIRST, then the box over it. The box is a diagnostic and has to stay
     * legible on top of the sprite; drawing it under would hide the very
     * alignment somebody opened this view to check. */
    const cursor = rowCursorsFor(t)[state.takeFrame];
    const ticksOnRow = cursor ? cursor.get(b.id || `${b.label}#${b.seat ?? "-"}`) : 0;
    const drew = state.takeArt !== false && drawBodyArt(ctx, b, X, Y, scale, ticksOnRow);
    ctx.strokeStyle = ROLE_COLOR[role] || ROLE_COLOR.other;
    /* An unfilled box once the art is under it: a translucent wash over a sprite
     * is a tint on the character, which is a lie about how it looks in game. */
    ctx.fillStyle = drew ? "transparent" : subject ? "rgba(111,179,255,.16)" : "rgba(125,133,152,.12)";
    ctx.lineWidth = subject ? 2 : 1;
    ctx.beginPath();
    ctx.rect(X(b.pos[0] - b.half[0]), Y(b.pos[1] - b.half[1]), b.half[0] * 2 * scale, b.half[1] * 2 * scale);
    if (!drew) ctx.fill();
    ctx.stroke();

    /* ⭐⭐ DAMAGEABLE GEOMETRY, WHICH IS HALF THE INTERACTION. An attack volume
     * drawn alone cannot say whether apparent contact is real: the box may be
     * passing through a frame in which this body is intangible, or through a
     * silhouette much narrower than the sprite. Cyan, and from the same runtime
     * view the production overlay draws. */
    if (state.takeHurt !== false) {
      ctx.strokeStyle = "#49c8d8";
      ctx.fillStyle = "rgba(73,200,216,.12)";
      ctx.lineWidth = 1;
      for (const hurt of b.hurtboxes || []) drawHitboxShape(ctx, hurt, X, Y, scale);
      /* ⛔ AN EMPTY LIST IS A DECISION, NOT A GAP — and it is invisible unless
       * the view says so. `intangible` is a body nothing can hit this frame. */
      if (b.hurtbox_source === "intangible") {
        ctx.fillStyle = "#49c8d8";
        ctx.font = "10px ui-monospace, monospace";
        ctx.fillText("INTANGIBLE", X(b.pos[0] - b.half[0]), Y(b.pos[1] + b.half[1]) + 12);
      }
    }

    /* ⭐⭐ THE ROLE, IN WORDS, ON THE PICTURE. The scenario may seat one
     * character twice; a colour cannot tell them apart and a seat index is a
     * convention the reader has to be taught. */
    const tag = ROLE_LABEL[role];
    ctx.font = "10px ui-monospace, monospace";
    const top = Y(b.pos[1] - b.half[1]);
    if (tag) {
      ctx.fillStyle = ROLE_COLOR[role] || ROLE_COLOR.other;
      ctx.fillText(tag, X(b.pos[0] - b.half[0]), top - 14);
    }
    if (b.label) {
      ctx.fillStyle = "#98a0b3";
      ctx.fillText(b.label, X(b.pos[0] - b.half[0]), top - 3);
    }
  }

  /* Hit volumes. The SUBJECT's are solid red; the opponent's are dimmed — the
   * take deliberately runs a live CPU, so a box on screen is not necessarily
   * the move's, and drawing them identically is what let the recorder's counts
   * be misread for so long. */
  ctx.lineWidth = 1.5;
  for (const h of frame.hitboxes || []) {
    const mine = roleOf(h, t) === "subject_owned";
    ctx.strokeStyle = mine ? "#e2564a" : "rgba(226,86,74,.35)";
    ctx.fillStyle = mine ? "rgba(226,86,74,.22)" : "rgba(226,86,74,.07)";
    drawHitboxShape(ctx, h, X, Y, scale);
  }

  /* ⛔⛔ PROJECTILES WERE RECORDED AND NEVER DRAWN. The take carries
   * `frame.projectiles` and the inspector's own documentation promises "the
   * fighter, its live hitboxes, its projectiles, and anything its move spawned"
   * — so a ranged move played back as a fighter standing still doing nothing
   * (GPT 5.6, 2026-08-27). A shot is drawn as its body plus a velocity whisker,
   * because where it is going is the half a still frame cannot show. */
  for (const s of frame.projectiles || []) {
    const mine = roleOf(s, t) === "subject_owned";
    ctx.strokeStyle = mine ? "#e8c15a" : "rgba(232,193,90,.35)";
    ctx.fillStyle = mine ? "rgba(232,193,90,.30)" : "rgba(232,193,90,.08)";
    ctx.beginPath();
    ctx.rect(X(s.pos[0] - s.half[0]), Y(s.pos[1] - s.half[1]), s.half[0] * 2 * scale, s.half[1] * 2 * scale);
    ctx.fill(); ctx.stroke();
    if (s.vel && (s.vel[0] || s.vel[1])) {
      /* A tenth of a second of travel: long enough to read direction, short
       * enough not to leave the shot behind on a fast one. */
      ctx.beginPath();
      ctx.moveTo(X(s.pos[0]), Y(s.pos[1]));
      ctx.lineTo(X(s.pos[0] + s.vel[0] * 0.1), Y(s.pos[1] + s.vel[1] * 0.1));
      ctx.stroke();
    }
  }

  $("#take-frame").textContent = `${state.takeFrame} / ${t.frames.length - 1}`;
  takeFacts(t, frame);
}

/* Say that there is no recording, ON the canvas. A blank black rectangle reads
 * as a broken viewer; a sentence reads as a missing artifact. */
function drawNoTake(canvas) {
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
  ctx.fillText("no recorded take for this move — the engine render panel still works",
               16, cssH / 2);
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
function syncEngineRender(take, frameIndex) {
  const img = $("#engine-render");
  const note = $("#engine-render-note");
  if (!img || !note) return;
  /* An image with no `src` is a BROKEN IMAGE ICON in every browser, which reads
   * as "this failed to load" rather than "there is nothing to show yet". */
  const nothing = (text) => { img.removeAttribute("src"); img.hidden = true; note.textContent = text; };
  const verb = takeVerb(take);
  if (!verb) return nothing("this take names no verb, so there is nothing to render");
  /* The scenario the TAKE was recorded in, so both panels show one fight. */
  const doc = renderedFramesFor(take.character, verb, {
    target: take.target,
    spacing: take.requested_spacing,
    behavior: take.target_behavior,
  });
  if (!doc) return nothing(`rendering ${verb}…`);
  if (!doc.available) {
    return nothing(`engine render unavailable — ${doc.reason || "no renderer"}` +
      (doc.hint ? ` · ${doc.hint}` : ""));
  }
  /* ⛔⛔ FOUR WAYS A RENDER CAN FAIL TO BE THIS MOVE, and showing the pictures
   * for any of them labels one move with another's name — the single worst thing
   * a reference tool can do. `outcome` is the renderer's own word for which one
   * it was; `mismatch` is kept for a manifest recorded before it existed. */
  if (doc.outcome === "not_prepared") {
    return nothing(
      `NOT PREPARED — the posture ${verb} needs could not be established, so the ` +
      `engine answered a different button. Showing the diagnostic take only.`);
  }
  if (doc.outcome === "unbound") {
    return nothing(`UNBOUND — ${doc.reason || `${take.character} binds no move to ${verb}`}`);
  }
  if (doc.mismatch || doc.outcome === "missed") {
    return nothing(`MISMATCH — ${doc.reason || "the engine played another move"}`);
  }
  /* Nearest shot at or before this action tick: the take records every tick and
   * the render samples by stride, so most take frames have no exact shot. */
  const shots = doc.shots || [];
  let pick = shots[0];
  for (const shot of shots) {
    if (shot.action_tick <= frameIndex) pick = shot; else break;
  }
  if (!pick) return nothing("this render took no pictures");
  const url = (doc.urls || [])[shots.indexOf(pick)];
  if (url && img.getAttribute("src") !== url) img.setAttribute("src", url);
  img.hidden = false;
  note.textContent =
    `${doc.renderer || "moveset_render"} · ${pick.file} · action tick ${pick.action_tick}` +
    ` · sim tick ${pick.sim_tick}` +
    (doc.renderer_built ? ` · built ${doc.renderer_built}` : "") +
    /* ⛔ A RUN THAT NEVER REACHED THE RELEASE photographed a charge that never
     * paid out, and a viewer scrubbing its last frame would read that as the
     * whole move. */
    (doc.release_reached === false
      ? ` · ⚠ stopped at action tick ${doc.last_action_tick} of ${doc.hold_ticks}, before the release`
      : "") +
    (doc.cached_only ? " · ⚠ CACHED, no renderer on this machine" : "");
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
      : "none yet — run moveset_takes"],
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
        ? "Without it there are no recorded takes to look at."
        : "Without it the bundle already on disk is served as-is.");
  }

  body.replaceChildren(el("div", { class: "cols" }, ...panels));
}

/* ---------- shell ---------- */
function showView(name) {
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
  $("#take-scrub").addEventListener("input", (e) => { state.takeFrame = Number(e.target.value); drawTake(); });
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
  $("#take-play").addEventListener("click", (e) => {
    state.playing = !state.playing;
    e.target.classList.toggle("on", state.playing);
    const step = () => {
      if (!state.playing || !TAKES || state.take === null) return;
      const t = TAKES.takes[state.take];
      state.takeFrame = (state.takeFrame + 1) % t.frames.length;
      $("#take-scrub").value = String(state.takeFrame);
      drawTake();
      setTimeout(step, 1000 / 30);
    };
    step();
  });

  state.fighter = (BUNDLE.characters.find((c) => c.on_smash_grid) || BUNDLE.characters[0])?.id || null;
  renderRoster();
  renderTakeList();
  if (state.fighter) { $("#fighter-pick").value = state.fighter; renderFighter(); }
}

boot();
