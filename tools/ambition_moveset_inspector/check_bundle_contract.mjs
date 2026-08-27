/* Does the bundle carry every field the UI reads?
 *
 * ⛔⛔ THE FAILURE THIS EXISTS FOR IS SILENT. `app.js` renders a missing field as
 * "—", which is also what it renders for a legitimately absent value, so a
 * renamed key in `moveset_export.rs` produces a page full of dashes that looks
 * like content rather than a break. Nothing in the browser says otherwise and
 * there is no browser in CI.
 *
 *   node tools/ambition_moveset_inspector/check_bundle_contract.mjs
 *
 * ⛔ IT ASSERTS PRESENCE, NOT VALUE. `null` is a real answer everywhere here
 * (an unauthored knockback weight, a move with no active window); `undefined`
 * is the key not existing, and that is the only thing this fails on.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const bundlePath = process.argv[2] || join(here, "data", "moveset_bundle.json");

let bundle;
try {
  bundle = JSON.parse(readFileSync(bundlePath, "utf8"));
} catch (err) {
  console.error(`[bundle-contract] cannot read ${bundlePath}: ${err.message}`);
  console.error("  run: cargo run -p ambition_app_tools --bin moveset_export");
  process.exit(2);
}

const problems = [];
const need = (obj, path, keys) => {
  for (const k of keys) {
    if (obj === null || obj === undefined) { problems.push(`${path} is absent`); return; }
    if (!(k in obj)) problems.push(`${path}.${k} is missing`);
  }
};

need(bundle, "bundle", ["schema", "sim_hz", "cast_generation", "smash_grid", "characters"]);
if (bundle.schema !== "ambition.moveset_inspector.v2") {
  problems.push(`schema is ${bundle.schema}; the UI reads ambition.moveset_inspector.v2`);
}
if (!Array.isArray(bundle.characters) || bundle.characters.length === 0) {
  problems.push("no characters in the bundle");
}

let moveCount = 0;
let boundCount = 0;
for (const c of bundle.characters ?? []) {
  const at = `character[${c.id}]`;
  need(c, at, [
    "id", "display_name", "provider", "on_smash_grid", "description",
    "vitals", "locomotion", "movement_tuning", "abilities", "mount",
    "held_item", "body", "verbs", "moves",
  ]);
  need(c.vitals, `${at}.vitals`, ["max_health", "mass", "knockback_weight", "canonical_height"]);
  for (const m of c.moves ?? []) {
    moveCount += 1;
    const mAt = `${at}.move[${m.id}]`;
    need(m, mAt, [
      "id", "display_name", "verbs", "clip", "duration_s", "duration_f",
      "gates", "start_impulse", "smash_charge_mult", "charge",
      "landing_lag_s", "autocancel_after_s", "repeat", "windows", "events", "derived",
    ]);
    need(m.gates, `${mAt}.gates`,
      ["grounded", "recovery", "forbidden_while_held", "roots_steering"]);
    need(m.derived, `${mAt}.derived`, [
      "startup_s", "startup_f", "active_s", "active_f", "endlag_s", "endlag_f",
      "max_damage", "sum_damage", "max_knockback", "reach", "vertical_reach",
      "hits", "fires_projectile", "max_damage_charged",
    ]);
    if ((m.verbs ?? []).length) boundCount += 1;
    for (const w of m.windows ?? []) {
      need(w, `${mAt}.window`, [
        "tag", "cancel_into", "start_s", "end_s", "start_f", "end_f",
        "motion_scale", "sustain_effect", "volumes",
      ]);
      for (const v of w.volumes ?? []) {
        need(v, `${mAt}.volume`, [
          "offset", "half_extents", "radius", "damage", "knockback",
          "knockback_growth", "launch_dir", "reaction", "on_hit", "vfx", "hit_sfx",
        ]);
      }
    }
    for (const e of m.events ?? []) need(e, `${mAt}.event`, ["at_s", "at_f", "kind", "detail"]);
  }
}

/* A grid fighter whose verbs table binds nothing would render an empty compare
 * view that looks like "no outliers" rather than "no data". */
const gridWithoutVerbs = (bundle.characters ?? [])
  .filter((c) => c.on_smash_grid && Object.keys(c.verbs ?? {}).length === 0)
  .map((c) => c.id);
if (gridWithoutVerbs.length) {
  problems.push(`on the grid with no bound verbs: ${gridWithoutVerbs.join(", ")}`);
}

/* ⭐⭐ THE ART TABLE, because the viewer now BLITS from it. Every field below is
 * dereferenced while drawing a frame; a sheet missing one draws nothing and the
 * page silently degrades to the boxes it had before anybody asked for art. */
let sheetCount = 0;
let rectCount = 0;
for (const [key, sheet] of Object.entries(bundle.sheets ?? {})) {
  sheetCount += 1;
  need(sheet, `sheet ${key}`, [
    "image", "images", "frame_width", "frame_height",
    "body_pixel_bbox", "feet_pixel", "rows",
  ]);
  /* ⛔ THE TWO THE PLACEMENT DEPENDS ON. `feet_pixel` is the horizontal origin
   * and `body_pixel_bbox` is the scale; a sheet with neither can only be drawn
   * centred on its frame, which is the defect the engine's own anchor had. */
  if (!sheet.body_pixel_bbox || !sheet.feet_pixel) {
    problems.push(`sheet ${key}: no body rectangle or feet pixel, so its art cannot be placed`);
    continue;
  }
  if (!(sheet.rows ?? []).length) {
    problems.push(`sheet ${key}: no rows, so no frame can be drawn`);
  }
  for (const row of sheet.rows ?? []) {
    need(row, `sheet ${key} row`, ["animation", "row_index", "frame_count", "page", "rects"]);
    for (const r of row.rects ?? []) {
      rectCount += 1;
      if (!Array.isArray(r) || r.length !== 7) {
        problems.push(
          `sheet ${key} row ${row.animation}: a rect is not [x,y,w,h,page,off_x,off_y]`
        );
        break;
      }
    }
    /* A row that declares more frames than it packs would index past its own
     * rect list on the last frame of every playthrough of that animation. */
    if ((row.rects ?? []).length && row.rects.length < row.frame_count) {
      problems.push(
        `sheet ${key} row ${row.animation}: declares ${row.frame_count} frames and packs ${row.rects.length}`
      );
    }
  }
}

/* ⛔ A GRID FIGHTER WHOSE SHEET IS NOT IN THE TABLE draws as a box forever, and
 * that is the difference between "the art is off" and "there is no art". Named
 * rather than failed: a build with gitignored regen output legitimately has
 * fewer sheets than the cast. */
const gridWithoutArt = (bundle.characters ?? [])
  .filter((c) => c.on_smash_grid)
  .filter((c) => {
    const base = String(c.spritesheet ?? "")
      .split("/").pop().replace(/\.png$/, "").replace(/_spritesheet$/, "");
    return !base || !(bundle.sheets ?? {})[base];
  })
  .map((c) => c.id);
if (gridWithoutArt.length) {
  console.warn(
    `[bundle-contract] WARN — on the grid with no baked sheet (will draw as boxes): ${gridWithoutArt.join(", ")}`
  );
}

/* THE TWO RANGED CASES, BY NAME. A firing move draws its shot from one of two
 * places and the export used to report only the first, so the grid's one charge
 * shot exported as a move with no damage. Both are pinned because they are
 * different code paths that produce the same field. */
const RANGED_CASES = [
  { character: "npc_pirate_admiral", move: "run_out_the_guns", source: "equipped", charges: false },
  { character: "projectile_polygon", move: "polygon_projectile_charge_shot", source: "body", charges: true },
];
for (const want of RANGED_CASES) {
  const c = (bundle.characters ?? []).find((x) => x.id === want.character);
  if (!c) { problems.push(`ranged case: no character ${want.character}`); continue; }
  const m = (c.moves ?? []).find((x) => x.id === want.move);
  if (!m) { problems.push(`ranged case: ${want.character} has no move ${want.move}`); continue; }
  const d = m.derived ?? {};
  if (!d.fires_projectile) {
    problems.push(`${want.move}: does not fire, so its shot fields prove nothing`);
    continue;
  }
  if (d.projectile_source !== want.source) {
    problems.push(
      `${want.move}: shot came from '${d.projectile_source}', expected '${want.source}'`
    );
  }
  /* A firing move with a null shot is the exact defect this pins: the move
   * visibly fires and the viewer has nothing to show for it. */
  if (!(d.projectile_damage > 0) || !(d.projectile_speed > 0)) {
    problems.push(
      `${want.move}: fires but reports damage=${d.projectile_damage} speed=${d.projectile_speed}`
    );
  }
  if (want.charges) {
    /* A charge that pays nothing is a charge nobody would hold. */
    if (!(d.projectile_damage_charged > d.projectile_damage)) {
      problems.push(
        `${want.move}: charges, but full damage ${d.projectile_damage_charged} does not beat its tap ${d.projectile_damage}`
      );
    }
  } else if (d.projectile_damage_charged !== null && d.projectile_damage_charged !== undefined) {
    problems.push(`${want.move}: does not charge, yet reports a charged damage`);
  }
}

const unique = [...new Set(problems)];
if (unique.length) {
  console.error(`[bundle-contract] FAIL — ${unique.length} problem(s):`);
  for (const p of unique.slice(0, 40)) console.error(`  ${p}`);
  process.exit(1);
}
console.log(
  `[bundle-contract] PASS — ${bundle.characters.length} fighters, ` +
  `${moveCount} moves (${boundCount} bound to a verb), ` +
  `${sheetCount} sheets / ${rectCount} frame rects, schema ${bundle.schema}`
);
