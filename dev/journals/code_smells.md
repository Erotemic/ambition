# Code smell backlog

Running log of smells noticed *opportunistically* while doing other work (Jon's
standing instruction, 2026-06-10). The rule: while focused on a big task, don't chase
smells — append them here so they aren't forgotten, and revisit later. Only fix inline
when the fix is very clear AND carries no risk of slowing the main task.

Append-only during runs; triage/prune during cleanup passes (move fixed items to the
Resolved section, condensed to a one-liner with the verdict/commit).

Entry format:

```
## YYYY-MM-DD <short title>
- **Where:** file:line (or module)
- **Smell:** what's wrong, one or two sentences
- **Noticed while:** the task being worked
- **Suggested fix / size:** sketch + rough effort (S/M/L)
```

---

## Open

## 2026-06-21 Dead `landed`/`killed` scaffold in `advance_attack` (+ possibly-broken pogo-off-enemy)
- **Where:** crates/ambition_app/src/app/world_flow/attack.rs ~316-347 (`advance_attack`)
- **Smell:** `let landed = false; let killed = false;` are hardcoded (synchronous hit resolution moved to the ECS damage queue), so every block gated on them is dead: the connect-sound `SfxMessage::Hit` (line ~321) AND — more worryingly — the pogo-impulse-on-landing block (`if landed && abilities.pogo && spec.can_pogo ...`, ~329-347). Pogo off the *orb* is a separate live path (~260-285); pogo off a *landed enemy hit* via this block can never fire. Either it's genuinely broken (needs migrating to the ECS damage queue like the connect sound was) or it's residue to delete. Found while answering "is there an attack-connect sound?" — answer: yes, the generic `SfxMessage::Hit` from `features/ecs/damage/mod.rs:307`, NOT this dead site; there is no *distinct* hit-confirm cue.
- **Noticed while:** investigating the mockingbird "swing doesn't fire / doesn't register while overlapping the boss" bug.
- **Suggested fix / size:** S to delete the dead connect-sound lines; M to decide+fix the pogo-off-enemy path (verify in-game whether enemy pogo-bounce currently works, then either migrate to the ECS queue or remove). Don't blind-delete — the pogo block may be a latent bug, not just dead code.

## 2026-06-13 Docs reference deleted RON-based levels
- **Where:** docs that predate the LDtk-only world source (ADR 0009's "Consequences" implies RON-world-authoring docs remain unswept). NOTE: `check_doc_links.py` passes (links resolve); this is about stale *prose* describing removed RON room/world authoring, not broken links.
- **Smell:** RON-shaped room/world levels were fully removed (LDtk is the only world source), but some docs still describe them as if extant. Jon's standing rule: a doc describing something that no longer exists is a smell.
- **Suggested fix / size:** S — grep docs for "RON room|world|manifest|level" prose, archive or rewrite.

## 2026-06-10 FeatureVisualKind::Sandbag variant in the generic kit
- **Where:** crates/ambition_gameplay_core/src/mechanics/combat/events.rs (FeatureVisualKind)
- **Smell:** a named-ish variant in kit vocabulary (excluded from the combat-kit guard word list).
- **Suggested fix / size:** S — rename to TrainingDummy, BUT it touches LDtk/content mapping, so do it with `ambition_ldtk_tools`, not a blind rename.

## 2026-06-10 Special-attack EFFECTS consumers are half-vocabulary (post de-name)
- **Where:** crates/ambition_gameplay_core/src/features/ecs/brain_effects.rs (spawn_gnu_apple_rain_*, spawn_overfit_volley_*, LockOnBeam/PitTrap/RotatingCross/MinionCascade consumers); SpecialActionSpec docs in ambition_characters/src/brain/action_set.rs
- **Smell:** the BossAttackProfile de-name is honest at the key/schedule/geometry/param layers, but the consumer impls still bake content (apple art identity, gnu-named fns, "GNU-ton boss:" spec docs).
- **Suggested fix / size:** M — lift baked constants + projectile-art identity into RON spec fields; rename consumers to the vocabulary. The active target of the Technique/Effects framework design (2026-06-13).

## 2026-06-15 Gravity-inversion residual design questions
Found via the headless `gravity_symmetry.rs` harness. The input-frame gates (crouch,
drop-through, attack-pogo, fast-fall, possession, ladder-jump, ledge, patrol wall-stop)
are now gravity-relative. These four remain world-Y-locked — each is a DESIGN question
(should it be gravity-relative?), cheaply verifiable by adding a symmetry case:
- **Directional attack hitbox offset** — `ambition_combat/src/lib.rs:446` (`view.pos + spec.hitbox_offset`): down/up/forward offsets are world-locked, so directional attacks are screen-relative.
- **`ground_gap_below_feet`** — `ambition_app/src/app/world_flow.rs:63` probes world-down for landing feedback.
- **Thrown ground-item physics** — `ambition_gameplay_core/src/items/pickup/mod.rs:169` (`GROUND_ITEM_GRAVITY`): thrown items fall world-down regardless of the gravity field.
- **Player knockback** — `apply_player_hit_events` builds `editable_tuning.as_engine()` without `apply_gravity_dir`; UNTESTED under a flip.

## 2026-06-21 Sprite-renderer path helpers duplicated + generated dir scattered
- **Where:** `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/cli.py` (`package_dir`/`repo_root`/`sandbox_sprites_dir`/`generated_dir`) vs the now-deleted `paths.py`.
- `paths.py` (`package_root`/`tool_root`/`repo_root`/`generated_root`/`sandbox_sprites_dir`) was the *better-factored* version — repo_root searches upward for `crates/`+`tools/` instead of hardcoding `parents[3]` — but it was **orphaned** (zero importers). Deleted it as dead code 2026-06-21; cli.py keeps its working copies.
- **TODO:** extract cli.py's path helpers into `registry/paths.py` (one home, the upward-search impl) and have cli import them. NOT done in the org pass because `cli.generated_dir(name)` = `DEFAULT_ASSET_DIR / name` is *semantically different* from `paths.generated_root()` = `tool_root()/generated` — they're not 1:1, so the dedup needs care, and the pixel parity harness does not assert output *paths* (pytest's draw_all/install tests do, partially).
- Related: generated output lands in **three** dirs — `generated/`, `targets/generated/`, and the tool-root `generated/` (all gitignored now, so a consistency smell, not a git-hygiene problem). Pick one canonical generated root when doing the path dedup.

## Resolved

- **2026-06-17 Patrol wall-stop read screen-vel.x** — under sideways gravity the patrol "reverse facing" detection watched the zeroed gravity axis and never fired (enemy ground into the wall). Now watches the gravity-perpendicular side velocity in both grounded integrators. `5c29c4a9`; pinned by `patrol_enemy_reverses_facing_at_a_wall_under_sideways_gravity`.
- **2026-06-17 Vestigial `PlayerPlatformRideState`** — write-only after riding became emergent; removed across the chain. `2a5aafde`.
- **2026-06-15 Dual inventory bags** — `ItemKind`/`PlayerInventory` deleted, collapsed onto `OwnedItems`/`Item`; dialogue can now grant any of the 24 items.
- **2026-06-15 Boss sprite assets** — 7 named `GameAssets` fields + per-boss loaders collapsed to one `boss_sprites: HashMap<&str, _>` + a data table; renderer names no boss. Replay bit-identical.
- **2026-06-10 ItemKind/Item enum split** — dual-bag half resolved (above); the 24-row `Item` enum INTENTIONALLY kept (type-level equip/ability wiring; narrow closed enum preferred over a wide registry). Won't-do-by-analysis.
- **2026-06-10 BossAttackProfile brain enum** — LEAVE the enum: the named melee variants are SHARED attack-shape vocabulary across many bosses; content-specific specials route through `Special(String)` (consumers already in `ambition_content::bosses::specials`). Won't-do-by-analysis.
- **2026-06-10 audio/music runtime extraction** — music-cue catalog moved to `ambition_content::music`; the ~89L of remaining `audio/runtime.rs` is thin game-glue, the generic half is already `ambition_audio`. No crate to extract.
- **2026-06-10 check_doc_links red** — links now resolve (`check_doc_links.py` passes). Residual suggestion: add it to CI to prevent future drift.
- **2026-06-15 Gravity-relative input frame** — crouch / drop-through / attack-pogo / fast-fall / possession / ladder-jump / ledge route their vertical "descend" gate through `ae::movement::gravity_descend`. One-way landing already gravity-symmetric.
- **2026-06-14 Cube edge-button (page-turn) duplicated per face** — extracted one `edge_button_nav(...)` consumed by both callers; the only per-face difference is the `EdgeInward` param.
- **2026-06-13 EnemyConfig.archetype tuning hub** — `EnemyArchetype` enum deleted; roster lifted to `ambition_content`, enemies resolve by brain-key against an installed `EnemyRoster`. Guarded by `architecture_boundaries_enemy_config_is_archetype_free`.
- **2026-06-15 audio/mod.rs `use bevy::prelude::*` "unused"** — load-bearing false positive (child modules re-glob it via `use super::*`). DO NOT remove.
- **2026-06-21 Alpha-clobber audit surface (sprite renderer)** — drawing a translucent fill straight onto an RGBA image with `ImageDraw.Draw(img)` *replaces* the destination alpha (clobbers underlying content) instead of blending; correct path is a scratch layer + `Image.alpha_composite` (the "gnu_ton rule"). Flagged by Jon (recurring agent mistake; likely latent bugs exist). Added the canonical primitive `core/draw.overlay_draw` (+ `composite_polygon`), pinned by `tests/test_core_overlay.py`. TODO: (a) unify the 3 existing scratch-layer copies onto `overlay_draw` — `generic_explosions._overlay_draw` DONE (delegates to core, parity-clean, since core uses the same `"RGBA"` scratch mode); `skeleton.composite_polygon` uses PLAIN `Draw` (not `"RGBA"`) so its unification would shift overlapping-translucent pixels → needs a parity-checked bless; rigdoc painter still TODO. (b) audit the ~139 plain `ImageDraw.Draw(img)` sites for translucent-over-content clobbers — **the pixel parity harness CANNOT catch these** (they render consistently wrong, so there's no before/after drift). Needs eyeball/heuristic, not the harness.
