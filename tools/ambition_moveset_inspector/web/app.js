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
let state = {
  fighter: null,
  move: null,
  slot: "smash_forward",
  gridOnly: true,
  compareGridOnly: true,
  sort: { key: "slot", asc: true },
  compareSort: { key: "fighter", asc: true },
  take: null,
  takeFrame: 0,
  playing: false,
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
               (w.cancel_into.length ? ` → ${w.cancel_into.join(", ")}` : "") +
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
  if (m.derived.fires_projectile) row("Fires", "the body's ranged action");

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

/* ---------- engine takes ---------- */
function renderTakeList() {
  const pick = $("#take-pick");
  if (!TAKES || !TAKES.takes.length) {
    pick.replaceChildren(el("option", {}, "no takes recorded"));
    return;
  }
  pick.replaceChildren(...TAKES.takes.map((t, i) =>
    el("option", { value: String(i) }, `${t.character} · ${t.label} (${t.frames.length}f)`)));
  state.take = 0;
  loadTake(0);
}

function loadTake(i) {
  state.take = i;
  state.takeFrame = 0;
  const t = TAKES.takes[i];
  $("#take-scrub").max = String(t.frames.length - 1);
  $("#take-scrub").value = "0";
  drawTake();
}

function drawTake() {
  if (!TAKES || state.take === null) return;
  const t = TAKES.takes[state.take];
  const frame = t.frames[state.takeFrame];
  if (!frame) return;
  const canvas = $("#take-canvas");
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
  const view = t.view;
  const scale = Math.min(cssW / (view[2] - view[0]), cssH / (view[3] - view[1]));
  const X = (x) => (x - view[0]) * scale;
  const Y = (y) => (y - view[1]) * scale;

  /* platforms */
  ctx.fillStyle = "#232733";
  for (const p of t.platforms || []) {
    ctx.fillRect(X(p[0] - p[2]), Y(p[1] - p[3]), p[2] * 2 * scale, p[3] * 2 * scale);
  }

  for (const b of frame.bodies) {
    const subject = b.seat === t.seat;
    ctx.strokeStyle = subject ? "#6fb3ff" : b.kind === "summon" ? "#47b78a" : "#7d8598";
    ctx.fillStyle = subject ? "rgba(111,179,255,.16)" : "rgba(125,133,152,.12)";
    ctx.lineWidth = subject ? 2 : 1;
    ctx.beginPath();
    ctx.rect(X(b.pos[0] - b.half[0]), Y(b.pos[1] - b.half[1]), b.half[0] * 2 * scale, b.half[1] * 2 * scale);
    ctx.fill(); ctx.stroke();
    if (b.label) {
      ctx.fillStyle = "#98a0b3";
      ctx.font = "10px ui-monospace, monospace";
      ctx.fillText(b.label, X(b.pos[0] - b.half[0]), Y(b.pos[1] - b.half[1]) - 3);
    }
  }

  /* Hit volumes. The SUBJECT's are solid red; the opponent's are dimmed — the
   * take deliberately runs a live CPU, so a box on screen is not necessarily
   * the move's, and drawing them identically is what let the recorder's counts
   * be misread for so long. */
  ctx.lineWidth = 1.5;
  for (const h of frame.hitboxes || []) {
    const mine = h.subject_owned !== false;
    ctx.strokeStyle = mine ? "#e2564a" : "rgba(226,86,74,.35)";
    ctx.fillStyle = mine ? "rgba(226,86,74,.22)" : "rgba(226,86,74,.07)";
    ctx.beginPath();
    ctx.rect(X(h.pos[0] - h.half[0]), Y(h.pos[1] - h.half[1]), h.half[0] * 2 * scale, h.half[1] * 2 * scale);
    ctx.fill(); ctx.stroke();
  }

  /* ⛔⛔ PROJECTILES WERE RECORDED AND NEVER DRAWN. The take carries
   * `frame.projectiles` and the inspector's own documentation promises "the
   * fighter, its live hitboxes, its projectiles, and anything its move spawned"
   * — so a ranged move played back as a fighter standing still doing nothing
   * (GPT 5.6, 2026-08-27). A shot is drawn as its body plus a velocity whisker,
   * because where it is going is the half a still frame cannot show. */
  for (const s of frame.projectiles || []) {
    const mine = s.subject_owned !== false;
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
  $("#take-facts").replaceChildren(kv);
}

/* ---------- shell ---------- */
function showView(name) {
  for (const b of document.querySelectorAll("nav.tabs button")) b.classList.toggle("on", b.dataset.view === name);
  for (const v of document.querySelectorAll(".view")) v.classList.toggle("on", v.id === `view-${name}`);
  if (name === "compare") renderCompare();
  if (name === "takes") drawTake();
}

async function boot() {
  try {
    const res = await fetch("data/moveset_bundle.json");
    BUNDLE = await res.json();
  } catch (err) {
    $("#bundle-meta").innerHTML =
      `<span class="err">no bundle — run <span class="mono">cargo run -p ambition_app_tools --bin moveset_export</span></span>`;
    return;
  }
  $("#bundle-meta").textContent =
    `${BUNDLE.characters.length} fighters · ${BUNDLE.smash_grid.length} on the grid · cast generation ${BUNDLE.cast_generation} · ${BUNDLE.sim_hz}Hz`;

  try {
    const res = await fetch("data/takes/takes.json");
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
  $("#take-pick").addEventListener("change", (e) => loadTake(Number(e.target.value)));
  $("#take-scrub").addEventListener("input", (e) => { state.takeFrame = Number(e.target.value); drawTake(); });
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
