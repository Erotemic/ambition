# Review at the main machine

Tasks parked here because they need Jon **in person at the main machine**
(in-game reproduction, the LDtk editor GUI, or listening to audio) rather than
remote supervision. Committed so it reaches the main machine via git. Pivoting
to remote-friendly features in the meantime; pick these up when back.

_Last updated: 2026-06-21 (Opus 4.8)_

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

**STATUS — Phase 1 shipped (2026-06-21).** The sprite renderer now emits an
LDtk-consumable visual manifest. New command:
`python -m ambition_sprite2d_renderer ldtk-manifest [--all-sheets]`
→ writes `crates/.../assets/sprites/ldtk_sprite_manifest.json`
(`{tilesets, entity_icons}`), consumed by the existing
`ambition_ldtk_tools … visual-manifest apply-manifest`. `regen_sprites.sh`
now emits + re-applies it. Default is a **curated** map (PlayerStart → the
real `player_robot` sprite, wired into all three worlds — open the editor and
PlayerStart should show the actual player instead of a green gizmo).
`--all-sheets` registers **all 120** sprite sheets as tilesets so any of them
can be assigned in the editor (not committed by default — large `.ldtk`
diff + every sheet PNG is gitignored, so run it locally when you want them).

**To verify at the main machine:** open `sandbox.ldtk` — PlayerStart should
render the real player sprite. If it looks right, decide whether to commit the
`--all-sheets` set too.

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
