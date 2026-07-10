# FABLE FINAL AUDIT — 2026-07-07 (the last fable pass)

Whole-repo audit after the opus/codex decomposition landing, verified again
2026-07-09 (F9). **Read this before planning structural work — where it
contradicts an older card, it wins.** The F1–F8 prescriptions have all been
executed; what survives below is the RULINGS they established, the lessons
that bind future carves, and the next-phase queue.

---

## F1 — Dep-graph audit: ELEVEN arrows, all closed

The workspace DAG has no cycles (`actors → sim_view` is dev-dep only) and the
big shape was right from the start: `engine_core`/`entity_catalog` dep-free at
the bottom, `characters`/`combat`/`primitives` above `engine_core`, `game/` on
top. All eleven flagged arrows are now burned down or ruled, each ratcheted by
a test in `game/ambition_app/tests/architecture_boundaries.rs`:

- **Moved down:** `GameMode` → `platformer_primitives::schedule`;
  `ControlFrame` → `engine_core` (so reusable character brains no longer dep
  the input adapter); `InventoryUiState` → `ambition_inventory_ui`.
- **Deps deleted:** `ambition_world` → combat/interaction/portal;
  `ambition_render` → `ambition_actors` (the big one); `ambition_host` →
  `ambition_actors`; `ambition_vfx` → `ambition_characters` (vfx owns
  `HitSide`, combat maps at the two edges that need both facts);
  `ambition_asset_manager` → `ambition_sfx` (the adapter was unused —
  deleted, not feature-gated). The `ambition_actors::portal` facade is gone.
- **Two explicit NO-MOVE rulings**, both ratcheted so future cleanups don't
  re-chase them: `ambition_runtime` may name sim/mechanic/model crates because
  it IS the headless composition tier (it must not drift upward into
  app/content/host/render ownership); and `ambition_touch_input → ambition_render`
  is correct because that crate owns the visible touch HUD — it is a
  presentation/input adapter with a legacy name. *A rename/re-home under a
  `presentation/` grouping stays legal but LOW priority.*

## F2 — `ambition_actors`: closed for audit cleanup

The actor DOMAIN (`features` + `player` + `abilities` + `boss_encounter` +
`body_mode`) is legitimately in this crate. Everything else was classified as
misplaced / residual glue / facade and burned down; the ~60 `pub use
ambition_*` re-export sites are gone or documented as real adapter seams
(asset catalog assembly, LDtk encounter loading, Ambition-specific Yarn
bindings, item pickup, concrete schedule installers, map UI hydration).

**Rules this pass established, still binding:**

- **The facade dissolution ratchet:** a facade may be deleted the moment
  `grep -rn "ambition_actors::<mod>"` outside actors returns zero. That
  one-liner is the per-facade exit test.
- **Residual glue splits two ways, one commit each.** Per ADR 0019 the
  plugin/schedule wiring belongs in `ambition_runtime`; actor-DOMAIN reactions
  stay. Never move a glue module wholesale into runtime — that just relocates
  the god-hub.
- **North star for the residual** (fold into `unified-actors.md`): `player/`
  existing as a SIBLING of `features/ecs` is the last structural
  player-centrism. The S5/S6 fighter-unification endgame folds the player's
  special-cased systems into the one actor pipeline; the right long-term shape
  is ONE `actors` tree where player-ness is a brain + a slot, not a directory.
  Do not force this before S5/S6 — **but DO stop adding player-only systems**
  (new work lands body-generic or brain-side).

**Projectiles — why the remaining steppers stay put.** The model lives in
`ambition_projectiles`; three steppers are still actor-woven and must NOT be
forced across the seam until their inputs are plain: charge input reads brain
action messages, `UserSettings`, gravity, and optional player animation facts;
victim routing emits `HitEvent`/heal/SFX/VFX and queries bosses, actors,
breakables, shields, and owner combat; world collision needs the live feature
overlay and the portal-carve snapshot. `ProjectileCollisionWorld` waits on the
world/plain-input follow-up.

> **RE-CHECKED 2026-07-10 (`refactor-chain.md` R4). The ruling held; the prose was
> imprecise.** R3 was the world/plain-input follow-up, so
> `ProjectileCollisionWorld` moved to `ambition_projectiles::collision_world`.
> The other two remain, and their blockers are narrower than written: victim
> routing names exactly THREE `ambition_actors`-owned symbols — `BossConfig` /
> `BossClusterRef` / `BossAnimationFrameSample` (the boss cluster views) and
> `PlayerHealRequested`. Everything else it queries — `CenteredAabb`,
> `BodyOffense`, `BodyDodgeState`, `BodyShieldState`, `FeatureId`,
> `BreakableFeature`, `ActorDisposition`, `FriendlyFire`, `HitEvent`,
> `PlayerEntity`, `FeatureSimEntity`, `GravityCtx`, `LiveProjectile`,
> `GameplayTraceBuffer` — already lives a tier down. Charge input has ONE
> blocker: `player::BodyAnimFacts`. Both discharge at the S5/S6 player fold plus
> a boss-vocabulary settle; neither was forced.

## F3/F4 — compliance + correctness (all closed)

Verified green and still true: the `[W-e]`/`[W-b]` lowering registry has both
the duplicate-registration panic and the unknown-kind hard error, pinned by
`#[should_panic]` tests; `ambition_entity_catalog` deps NOTHING (Tier-0 purity);
`ambition_world` has no LDtk dep and carries an explicit allow-list regression
test; §3.6 GeoId stamping survived the W3 move.

Two REAL regressions were found and fixed in-session, and both are worth
remembering as a class: **the `game/` re-home broke `desktop_asset_root()`**
(a `../ambition_actors/assets` hop that now landed in `game/`, silently falling
back to exe-relative `assets` — "game runs but nothing renders / no music"),
and **the `gameplay_core → ambition_actors` rename broke the music tools'
repo-root probe.** Rename/move fallout hides behind silent fallbacks; audit
every `CARGO_MANIFEST_DIR` hop after a re-home.

The three logged hazards closed later: `ClockResetRequest` routes reset intent
through the one time-control owner (ADR 0010/0011 authority preserved);
deterministic lowest-`PlayerSlot` fallbacks replaced raw Bevy query order at
two sites, tagged `AMBITION_REVIEW(determinism)`; and the `unified_melee` RED
was diagnosed as a **stale read-model assumption in the TEST**, not a sim
regression — `enemy_attacks_player` was always the enemy-AI oracle, while
`unified_melee` is the convergence test and now accepts both swing authorities.

## F5 — elegance directions

Done: the `ambition` umbrella crate (`crates/ambition`) re-exports runtime,
host, render, world, model, and vocabulary crates plus a curated prelude, and
deliberately does NOT dep `ambition_app`/`ambition_content`/kaleidoscope. The
app manifest collapsed to three `ambition*` deps. `game/ambition_demo_sanic`
and `game/ambition_demo_smb1` dep ONLY the umbrella, oracle-ratcheted.

**Still standing:**

3. **At the S5/S6 player-fold, rename `features/` away.** The name is
   pre-decomposition residue. When `player/` folds in, the tree becomes
   `ambition_actors::{bodies, brains, spawn, damage, mount, perception,
   bosses}`. Do not rename before the fold — one churn, not two.
4. **Tests travel with their subject — but check CONTENTS, not filenames.**
   The audit's own hunch was half wrong: `features/conversion_tests.rs` is
   MISNAMED (its content is headless actor movement/collision tests, not LDtk
   conversion) and correctly stays; only the 8 `portal_phase_*` tests actually
   travelled, to `ambition_world::rooms::gate_portal`.
5. **Anti-goal (Jon's tiny-crate skepticism):** the remaining wins are MOVES
   and DELETIONS, not new crates. No new crate without a consumer that exists
   today. The crate count is already at the top of the comfortable range; the
   value now is thinning `ambition_actors` and deleting facades.

## F7 — the lowering seam had three real defects; two rulings, one lesson

1. **RULING — display names live at the RECORD level, never inside schemas.**
   `PlacementRecord` carries `name: String` (serde-default = the id; the
   `PropSpec.name` precedent). Lowered hazards had been labeled by LDtk iid.
   Every future placement family gets display names for free.
2. The inline-motion trap (a legacy inline-`motion` hazard silently becoming
   static) is **superseded**: the F9.2 hazard conversion now lifts inline
   motion to a room-level `KinematicPath` referenced by `path_id`, so no
   legacy-only channel remains.
3. **LESSON, binding on every future carve (add to the D2 template):** the W3
   carve dropped a `cfg(test)` fixture and with it FOUR ruled contract tests.
   `git log --stat` the source module's test files and account for every
   `#[test]` **by name** in the moved crate. A carve that can't run a test must
   MOVE its fixture, not delete the test.

Clean-verified and worth recording as sound: hash-order iteration in sim is
order-insensitive at both live sites; zero `partial_cmp().unwrap()` NaN-sorts;
sim-path `unwrap()`s concentrate in tests; the engine-for-other-games oracle
HOLDS (zero live core→content references — the one `include_str!` is the
sanctioned `cfg(test)` fixture pattern).

**Ops note:** the dev box hit 100% disk mid-audit (`~/ambition-target/debug/
incremental`, 149G of regenerable cache). Consider `CARGO_INCREMENTAL=0` for
full-gate runs, or a periodic `cargo clean` cron, so a full disk doesn't
silently kill a background gate.

---

## F9 — 2026-07-09 verification + the next-phase queue

An INDEPENDENT verification of the executed F1–F6 queue (~40 commits), checked
against manifests and source rather than the execution log: all F1 arrows
closed; world purity real with a ratchet test carrying dissolution
instructions in its failure message; F3.2 (`SweepSample` required on ECS
actors/bosses, `PortalSweepAnchor` retired), F4.3, and F4.4 closed; the E9
exit met. **Gate: 44/44 suites green, zero failures.**

**RULING on the F1.1 route (accepted).** World purity was achieved by making
the family spec types IR-native with actor-side lowering, rather than by
dissolving the typed lists into `PlacementRecord` — a legitimate, arguably
better sequencing, since the dep payoff arrived without waiting for six branch
conversions. But it left a two-channel IR (typed lists + placements) with a
real tax: a dual-emit guard, the F7 name-loss bug class, two spawn paths. The
record-over-schema consolidation therefore continued as IR-internal cleanup.

✅ **THE F9.2 ARC IS CLOSED (2026-07-09).** All six authored-entity families
are placements-only. Interactables, pickups, chests, and breakables were
Tier-0 MOVES into `ambition_entity_catalog::placements` — one pure type each,
no schema/world mirror, because all four are `Vec2`-free. Portals were the
deliberate exception, done as a Tier-0 `PortalSchema` MIRROR (`normal:
[f32;2]`) whose lowering derives the face center from the record's
`aabb.center()`; the runtime-facing `PortalSpec` keeps its `Vec2`. Breakables
had one twist: they enter via the surface-compile pipeline, so the placement
conversion happens in `RoomEmission::from_compiled`. Hazards closed the arc —
`convert_damage_volume` now LIFTS a legacy inline `motion: KinematicPath` to a
synthesized room-level path (`{iid}__inline_motion`) referenced by `path_id`,
behavior-preserving because `HazardRuntime::new_with_paths` resolves it back.

**Result: zero typed per-family `Vec`s on `RoomSpec`, zero typed spawn loops,
and the dual-emit guard DELETED.** `placements` is the sole authored-entity
channel. A future authored family adds ONE `PlacementSchema` variant + one
lowering interpreter; there is no second channel to keep in sync.

### The next-phase queue (in order)

1. **Demo content** — the umbrella's real test and the first BUILD (not
   restructure) item.
   **ADVANCED:** `ambition_demo_sanic` authors a real momentum showcase room
   (`sanic_speedway` — a long solid floor plus a rideable loop as an
   interior-winding `SurfaceChain`) entirely through the `ambition` umbrella,
   with a headless test that composes it and runs the engine's own chain
   validator. A missing re-export would fail to compile there. **The oracle
   held — nothing was missing.**
   **RULING:** the FEEL half — momentum-body tuning to a Sanic movement
   identity, a playable demo binary/app shell, character art — is a
   fundamentally interactive build. It needs playtesting and cannot be
   responsibly completed in a headless autonomous pass. `ambition_demo_smb1` is
   untouched and takes the same shape once its 1-1 geometry is authored.
2. **IR consolidation** — ✅ DONE, arc closed (above).
3. **Projectile remaining steppers** — stay actor-side until their inputs are
   plain (blockers enumerated in F2). Do NOT force this seam.
4. **S5/S6 player fold + the `features/` rename** — deferred by design until
   the unified-actor work is ready. Do not churn the module tree first.
5. **Standing:** no new crates without a present consumer; facade re-adds are
   ratcheted; keep the E9 umbrella narrow (new app/demo/content code imports
   through `ambition::*`, while app-local extensions like kaleidoscope stay
   direct).
