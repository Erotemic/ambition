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
if (bundle.schema !== "ambition.moveset_inspector.v1") {
  problems.push(`schema is ${bundle.schema}; the UI reads ambition.moveset_inspector.v1`);
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

const unique = [...new Set(problems)];
if (unique.length) {
  console.error(`[bundle-contract] FAIL — ${unique.length} problem(s):`);
  for (const p of unique.slice(0, 40)) console.error(`  ${p}`);
  process.exit(1);
}
console.log(
  `[bundle-contract] PASS — ${bundle.characters.length} fighters, ` +
  `${moveCount} moves (${boundCount} bound to a verb), schema ${bundle.schema}`
);
