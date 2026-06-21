# Review at the main machine

Tasks parked here because they need Jon **in person at the main machine**
(in-game reproduction, the LDtk editor GUI, or listening to audio) rather than
remote supervision. Committed so it reaches the main machine via git. Pivoting
to remote-friendly features in the meantime; pick these up when back.

_Last updated: 2026-06-21 (Opus 4.8)_

---

## 0. Latest pass (2026-06-21 cont.) — RUN `./regen_sprites.sh` FIRST

Three things landed; the first explains why your sprite changes "looked the
same":

- **Publishing bug fixed (the big one).** Rigged characters (Emmy/`noether` and
  any rig-editor `.rig.json`) were listed in regen's expected-files but **never
  actually published** by it, and the caches ignored `.rig.json` — so rig edits
  silently never reached the crate (Emmy's sheet was weeks stale). Fixed: regen
  now discovers + publishes every rigged doc and hashes `.rig.json` in both
  caches. **Action: run `./regen_sprites.sh`** — Emmy should then be taller
  (collision_scale 2.0), hi-res, and her hairpin rigid (no bobbing). Verified
  the publish produces those; you just never had it.
  - *Publishing-elegance Q:* the contained fix is done. A bigger win would be a
    `cargo`/watch hook that auto-republishes changed content, but that's a
    separate refactor — say the word.

- **LDtk confusing sprites removed.** Every entity except `PlayerStart` is now a
  plain colored **region box** (renderMode Rectangle) instead of a gizmo/sprite
  stretched over its bounds — that stretching is what made zones look like
  geometry, and Tile mode hid the box outline (why the Solid square was
  invisible; the outline is back now). New tool: `visual-manifest
  clear-entity-icons`. The generic spawners (boss/enemy/npc) got no sprite (they
  couldn't say *which* one). **Verify:** regions readable, Solid visible. If
  Solid's fill is still faint, I'll bump its opacity (one tweak). Door→door
  sprite + per-enemy art are the noted follow-ups.

- **Slash effect now respects the reference frame.** It orients to the hitbox
  (gameplay authority, already gravity-relative): down-tilt = horizontal forward
  poke, down-air = downward sweeping arc, all rotating with C4 gravity. Pinned
  by tests (render rotation under 4 dirs; combat strike-direction under 4
  gravities). **Verify in-game / symmetry room.** Handedness (crescent curl on
  a left-facing swing) and size are the likely tweaks — flag and I'll adjust.
  Decision made per your "can't decide": **hitbox stays the authority, effect
  orients to it** (no hitbox change was needed — the offsets already pointed
  right).

---

## 1. Mockingbird attack swing-gate — capture an F8 trace ⏳ (do this)

**Why it's machine-bound:** needs you to reproduce the bug live and press F8.

**The bug:** standing/flying inside the mockingbird body, pressing attack
produces **no swing sound and no hit** (works fine when disjoint). You confirmed
you're *not* taking contact damage inside the body, so it isn't hitstun.

**What I found:** I traced the whole attack pipeline (input → `melee_pressed`
passthrough → `resolve()` → `emit_brain_action_messages` → `start_attack`).
There is **no position / boss-overlap / fly-state dependency** that should gate
the swing. The only gates are `hitstun<=0` (ruled out), `abilities.attack` (on),
and `attack.is_none()`. So the cause isn't visible statically — it needs a trace
from your real repro.

**What to do:**
1. Reproduce (fly into the boss, press attack, observe no swing).
2. Press **F8** within ~4 s (the ring buffer holds ~240 frames).
3. Send me the dump's `## Frames` block from `debug_traces/`.

**What it'll tell us:** each frame line now carries
`atk[p=.. s=.. abil=.. hs=.. inv=..]` — `p`=attack pressed, `s`=swing live
(`attacking`), `abil`=attack ability on, `hs`=hitstun, `inv`=invuln. A frame
with `p=true` where `s` never flips `true` (with `hs=0`, `abil=true`) pinpoints
whether it's a stuck `attacking`, the ability flipping off, or a logic bug in
the start gate. (Instrumentation landed in commits `a6460e3b`, `42e58ee6`.)

---

## 2. LDtk tiles & sprites not rendering nicely in the editor 🎨

**Why it's machine-bound:** needs the LDtk editor GUI to see what's actually
wrong and to verify a fix.

**STATUS — shipped (2026-06-21).** The sprite renderer now emits an
LDtk-consumable visual manifest and the worlds use it. New command:
`python -m ambition_sprite2d_renderer ldtk-manifest [--all-sheets]`
→ writes `crates/.../assets/sprites/ldtk_sprite_manifest.json`
(`{tilesets, entity_icons}`), consumed by the existing
`ambition_ldtk_tools … visual-manifest apply-manifest`. `regen_sprites.sh`
emits + re-applies it.

The committed/default set is **curated** — every registered tileset is
actually used by an entity def (no orphans):
- `PlayerStart` → `player_robot`
- `NpcSpawn` → `merchant_prototype`
- `EnemySpawn` → `goblin`
- `BossSpawn` → `gnu_ton_boss`

wired into all three worlds. Open the editor and those entities should show
real sprites instead of gizmos. `--all-sheets` registers **all 120** sheets
as tilesets so any of them can be assigned by hand (not committed by default:
~6.8k-line `.ldtk` diff + every sheet PNG is gitignored + 116 would be
unused — run it locally when you want the full tileset browser).

**To verify at the main machine:** open `sandbox.ldtk` — PlayerStart /
NpcSpawn / EnemySpawn / BossSpawn should render real sprites. Decide whether
to (a) commit `--all-sheets` too, and/or (b) pick different representatives
(it's a one-line-each dict, `DEFAULT_ENTITY_SPRITE_MAP` in
`tools/ambition_sprite2d_renderer/.../ldtk_manifest.py`).

**Phase 2 (the real richness — needs an LDtk-schema decision):** the generic
spawners (`EnemySpawn` / `NpcSpawn` / `BossSpawn`) are 1:many, so a single
representative would mislead. Proper per-instance editor visuals mean adding a
field (an enum whose values carry each character's `tileRect`, or a Tile/enum
ref) so each *placed* spawn shows its actual character. That's a schema +
tooling task to design in the editor — left for review.

**Symptom (your words):** tiles and sprites don't render nicely in the LDtk
editor; we may need to emit JSON at generation time that LDtk can consume.

**What already exists (so we build on it, not from scratch):**
- `editor_icons.png` + an `EditorIcons` tileset with per-entity `tileRect`
  wiring (entity *icons* — commits `cccb04e2`, `e5d15def`). This is the *entity*
  side and is at least partly working.
- `tools/ambition_ldtk_tools/.../edit/tilesets.py` — registers tileset defs
  (`defs.tilesets[]` = source PNG + grid metadata). See **ADR 0015 (LDtk
  tileset rendering)** for the plan this was meant to unblock.
- `.../edit/assets.py` — points entity defs at tiles in registered tilesets.
- `.../edit/visual_manifest.py` — **the key one.** Its docstring says it is
  *designed* to consume "a small, stable manifest shape that can be generated
  from today's fixtures and later from whatever RON/YAML metadata the sprite
  refactor emits." That is exactly your "emit JSON LDtk can use" idea — the
  consumer half exists; the **generator half (sprite/tile renderer emitting that
  manifest) is not wired up.**

**Working hypothesis:** the sprite/tile generator
(`tools/ambition_sprite2d_renderer`) produces PNG atlases but does **not** emit
the LDtk-consumable visual manifest (tileset grid size / spacing / padding +
per-sprite tile rects). So tilesets are registered by hand or not at all, and
LDtk's uniform-grid tile slicing doesn't line up with our packed atlases →
sprites/tiles look wrong in the editor.

**To review/decide when back:**
1. Open the `.ldtk` in the editor and pin down *what* is broken — Tiles layers,
   entity sprites, both? Wrong cell/offset, clipped, or just missing?
2. Check: are the tilesets registered (`defs.tilesets[]`), do the referenced
   PNGs exist on disk, and do their dimensions match the `.ldtk`'s recorded
   tileset sizes? (mismatch = slicing garbage).
3. Decide the manifest schema the sprite generator should emit (grid size,
   spacing, padding, per-sprite rect) so `visual_manifest.py` can consume it.
4. Wire the generator to emit it; regen via `regen_sprites.sh` (keep a fresh
   clone working — regen-invariant). Then re-open the editor to verify.

**Constraints to respect:** never hand-edit `sandbox.ldtk` JSON — go through
`ambition_ldtk_tools` (add a subcommand if one's missing). Sheets/atlases are
gitignored (regenerated); commit Python + the `.ldtk` wiring only.

---

## 3. Player spawn placement — feet sink into the floor (in-game verify) 🔎

**Why it's machine-bound:** needs in-game verification I can't do headless.

**Status:** deferred + flagged earlier. When spawning from a `PlayerStart`
(`--start-room` / respawn), the player's feet land ~17–31 px *into* the floor
(its AABB is top-aligned to the entity). **Door entry is correct**
(`gap_below_feet≈2 px`), so it's a Rust spawn-placement convention issue
affecting all rooms via `--start-room`/respawn, not a one-room LDtk tweak. The
trace warm-up gate already silences the harmless 1-frame dump it caused.
Low urgency; worth a proper fix once you can eyeball it. Say the word and I'll
take it as its own pass.

---

## 4. Sprite resolution / in-game size knobs (rig path done; fleet rollout pending) 🔭

**Why partly machine-bound:** the *crispness* and *Emmy's height vs the player*
only read true in the running game.

**Done (2026-06-21):** rigged characters can now specify their own in-game
tuning + render resolution, instead of inheriting the `DEFAULT_TUNING`
(collision_scale 1.5) fallback:
- A rig doc may carry `"sprite_tuning": {"collision_scale": …, …}` — emitted to
  the sheet RON and used by the runtime for **in-game display size**
  (height = `collision * collision_scale`). This is the knob for "make X
  taller/bigger" without touching gameplay collision.
- `"frame": {"render_scale": N}` multiplies the texture's pixel resolution
  (geometry stays in base-frame units). The in-game **size is unchanged** —
  `sprite_render_size` derives height from collision and only takes *aspect*
  from the frame — so this is pure anti-pixelation: more native pixels under
  the same display quad.
- Emmy (`noether`) uses both: `collision_scale 2.0` (taller than the short
  robot) + `render_scale 2` (crisp).

**Verify at the main machine:** is Emmy now clearly taller than the player, and
not pixelated? `collision_scale` is a single number in `gen_noether_rig.py`
(`sprite_tuning`) — nudge if she reads too tall/short. (I calibrated 2.0
blind; the player is ~`48 * its collision_scale * 1.16` tall.)

**Fleet rollout — DONE (2026-06-21).** `RenderConfig.render_scale` (default
**2**) now flows through `authoring/sheet.py` (`build_spritesheet`), so every
toon/adapter sheet renders at 2× native resolution. The "128-base" generators
(`robot_side`/`goblin_side`/`boss_side`) were drawing at an absolute scale
that ignored the canvas size — fixed to scale-to-frame-width like the toon
generator (`S = float(scale) * size[0] / 128`), identical at the 128 default,
only activating at render_scale>1. Verified headless: player_robot 128→256,
goblin 121→239, boss 128→256, absurd_general 90→176 — all ~2× with aspect
preserved; 21 slow-render pipeline tests pass at the new default.

Non-breaking: in-game display size is collision-driven and takes only aspect
from the frame, so this is pure anti-pixelation. Sheets are gitignored, so this
landed as a **code-only change** — the fleet re-renders crisp on the next
`./regen_sprites.sh`.

**Verify at the main machine:** run `./regen_sprites.sh`, then confirm in game
that NPCs/enemies/player are noticeably sharper. If something is *still* soft,
it's being displayed even larger than 2× covers — bump the default to 3 in
`RenderConfig`, or set `render_scale: 3` on that character's config. (Cost:
disk + render time scale with the square of render_scale.)

Per-character in-game *size* tuning for the toon path already exists too
(`sheet_tuning` in a character's YAML → RON), so retiring the hardcoded Rust
`SheetTuning` consts in favor of data-driven tuning is available whenever you
want it.

---

## 5. Player slash effect — hooked up; tune look/size/placement in game 🗡️

**Why machine-bound:** the effect's on-screen size, position vs the strike,
and timing only read true in the running game.

**Done (2026-06-21):** the generated `robot_slash` sheet (was never referenced
in Rust — the player only ever spawned a yellow debug hitbox box) is now wired
to player attacks. `start_attack` emits `VfxMessage::Slash`, picking the row
from the attack intent: **Side/Up** energy-arc crescents for most swings, the
tapered **Down** lance/poke for down-tilt & pogo (the "different one" — already
generated). One shared effect; each attack can graduate to a bespoke one later.

**Verify at the main machine:** do attacks now show the arc/poke (not a yellow
box)? Is it the right size/position? Knobs:
- size: `slash_effect_size` in `attack.rs` (currently 2× the hitbox's max dim).
- position: spawned at the hitbox center — could anchor at the sheet's `origin`
  (hand/pivot) anchor instead for a more rooted swing.
- timing: fires once at swing start; uses scaled time (slows in bullet-time).

**Refactor that came with it (your call paid off):** the shrine's private
record→atlas / row-lookup helpers were lifted into a shared
`rendering::sheet_atlas`; the slash lives in its own `rendering::slash_visuals`
next to `shrine_visuals`, both built on it. The old `SlashPreview` debug box is
retired (if you want a toggleable hitbox overlay back, that's a small debug
gizmo, separate from the effect).
