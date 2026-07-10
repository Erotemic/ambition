# The refactor chain — dissolving the adapter shells, then folding the player

**Status:** R1, R2, R3, R5 DONE; R4 re-checked and STOPPED as ruled; R6 IN PROGRESS
(R6a–R6d landed — **`player/` no longer exists**. R6e PARKED with a decision
brief: measured, the `features/` rename is ~1560 sites across 5 crates, not the
722 the plan assumed, and a half-rename would make the tree worse). 2026-07-10.
Six slices, in dependency order.
Each is committable on its own; each states its own exit check.

This doc exists because the 2026-07-10 ledger ruling changed what "finish the
decomposition" means. The residual `ambition_actors` is **64.0k total src lines**
(units: TOTAL, incl. tests) against a projected 31–35k — and the gap is **nine
adapter shells, not one missing carve**. No further crate split is owed, and none
would buy compile time. The residue shrinks by dissolving shells, one technical
precondition at a time. See [`decomposition.md`](decomposition.md) THE LEDGER for
the ruling and the evidence.

**Anchor style (evergreen):** cite `path` + SYMBOL, never line numbers. If a
named symbol has moved, `rg` for it; if it's gone, that's drift — update this doc
in the same commit, don't guess.

---

## Standing rules for this chain

- **Delete, don't bridge.** Pre-release, zero external dependents. A full
  replacement in one commit beats a facade plus a follow-up that never lands.
  Never leave a compatibility re-export "for now".
- **State the units** in any LOC figure you write down. The ledger's numbers are
  total src lines including tests. An earlier pass compared production-only lines
  against a total-lines projection and concluded the opposite of the truth.
- **The gate** (run before every commit; all must be green):
  ```
  cargo test -p ambition_actors --lib
  cargo test -p ambition_engine_core -p ambition_runtime -p ambition_host \
             -p ambition_dialog -p ambition_sim_view -p ambition_combat \
             -p ambition_characters
  cargo test -p ambition_content --features portal
  cargo test -p ambition_content --features ui --test yarn_compile
  cargo test -p ambition_app --features rl_sim
  ```
- **Formatting:** `rustfmt --edition 2021` on the exact files you touched, never
  `cargo fmt`. **Running rustfmt on a `mod.rs` or `lib.rs` formats the whole
  module tree beneath it** — snapshot `git status --porcelain` before and after,
  and revert only files that are newly dirty *and* that you did not edit. (A
  careless revert of that "collateral" once ate a real fix.)
- **Living-plan discipline:** when a slice lands, update this doc and
  `tracks.md` in the SAME commit. When you find drift, fix the doc in the same
  commit as the code that proves it.
- **Do not disagree with fable silently.** Fable's rulings are pre-solved
  designs. If evidence contradicts one, make the case in the doc (vision §7),
  then proceed — but say so, out loud, in the commit message.

---

## R1 — D-C: the mode-scope seam ✅ DONE (2026-07-10)

**Closed the last decomposition artifact.** Vision §5's **scoped game-mode
pattern**: a demo's rules crate gates its systems on an area/room tag, not on
global state, so Ambition hosts several demos' rulesets in one binary.

**What landed.**
- `ambition_world/src/rooms/metadata.rs` → `RoomMetadata::mode: Option<String>`,
  merged first-`Some`-wins like every other string field; `is_empty` accounts for
  it. Authored as the LDtk level string field `mode`
  (`ambition_ldtk_map/src/project.rs` → `LdtkLevel::level_metadata`).
- `ambition_platformer_primitives/src/lifecycle/` → `ModeScopedEntity(String)`
  and `SpawnScopedExt::spawn_mode_scoped`.
- `ambition_runtime/src/mode_scope.rs` → `in_mode(name)`,
  `despawn_departed_mode_entities`, `ModeScopePlugin` (last member of
  `PlatformerEnginePlugins`, `.after(sync_active_room_metadata)` in
  `SandboxSet::Progression`).
- `game/ambition_demo_sanic` → `SANIC_MODE`; `sanic_speedway()` tags its room.

**Two deviations from the pre-solved sketch, both stated out loud (vision §7):**

1. **`ModeScopedEntity` lives in `ambition_platformer_primitives::lifecycle`, not
   in `ambition_runtime`,** with `RoomScopedEntity` / `RunScopedEntity` /
   `PersistentEntity`. It is lifetime-scope VOCABULARY, and anti-god rule 1 sends
   vocabulary DOWN to the crate that owns the domain; it also lets
   `spawn_mode_scoped` join the existing `SpawnScopedExt` verb trait rather than
   forcing a second spawn-helper trait a tier up. Only the SWEEP needs
   `ActiveRoomMetadata`, so only the sweep sits in `ambition_runtime` — the exact
   split `RoomScopedEntity` already uses (marker in primitives, sweep above).
   The sketch's `// ambition_runtime:` comment is satisfied by `in_mode` + the
   plugin; nothing about the design changed.
2. **`in_mode` returns `impl FnMut(Option<Res<ActiveRoomMetadata>>) -> bool +
   Clone`, not `impl Condition`.** That is the signature bevy's own `in_state`
   uses; a bare closure is a `Condition` via `IntoSystem`, and naming `Condition`
   in the return type would force the caller to name its marker generic. `None`
   (no world installed) reads as "in no mode", so a hosted ruleset sleeps rather
   than panicking.

**"Generalize the sweep, don't duplicate it"** resolved to reusing
`lifecycle::despawn_scoped_entity` (which existed with zero callers, documented
as "a runtime-owned verb to grow from") — NOT to folding the mode sweep into
`load_room_geometry`'s room sweep. They are genuinely different lifetimes: a
mode-scoped entity SURVIVES the room transitions inside its own mode, which is
the whole point, and `load_room_geometry`'s loop additionally carries the
transiting body and retires avian physics entities. One sweep could not do both
without a policy argument that means "which scope am I".

**`ambition_runtime` gained a direct `ambition_world` dep** (the space IR is a
tier below the sim). `architecture_boundaries.rs`'s runtime allowlist fired on
it, correctly, and now names it with its reason.

**The rules-crate constructor flag** (`SanicRulesPlugin::hosted()` vs
`::global()`) is NOT built: `SanicDemoContentPlugin` has zero rules today (the
momentum feel is the separate interactive build), so the constructor would be a
facade over nothing. The room already claims the mode, which is what a hosted
ruleset wakes on; the flag lands with sanic's first rule. The pattern is written
out in `mode_scope.rs`'s module docs and pinned by
`ambition_runtime/tests/mode_scope.rs`'s `DemoRulesPlugin` fixture, which is that
constructor flag exactly.

**Exit check — met.** `ambition_runtime/tests/mode_scope.rs`: two hosted rules
plugins coexist; `in_mode("a")` systems do not run while room metadata says `b`;
`ModeScopedEntity("a")` entities are despawned when the active room's mode
changes, while `b`'s survive; a room change WITHIN a mode spares them; a
standalone (ungated) ruleset runs with no mode at all. `demo_shell_smoke.rs`
passes. Plus the E9 oracle in `ambition_demo_sanic`: the seam is reachable
through the `ambition` umbrella alone.

**Found while doing it** (logged in `dev/journals/code_smells.md`): a crate whose
manifest names only `ambition` cannot `#[derive(Resource)]` — bevy's derives
resolve `bevy_ecs` through the consumer's manifest, and a re-export does not
satisfy them. The umbrella's "author a game through this crate alone" claim has
an asterisk.

---

## R2 — E6 teardown ✅ DONE (2026-07-10) — and its premise was WRONG

The teardown landed and the exit check is met. **But this slice's stated payoff
does not exist, and the measurement it demanded is what proved it.** Written down
here rather than quietly dropped.

**What died.** The fused `gnu_ton` boss profile (`boss_profiles.ron` row +
`boss_encounters/gnu_ton.ron` + the `#[cfg(test)] gnu_ton()` alias), its sheet
(`GNU_TON_SHEET` and the `gnu_ton{,_body,_hands}` / `giant_gnu_{body,hands}`
registry rows), and the whole split-layer render — `BossOverlayLayer`,
`BossBodyLayer`, `sync_boss_split_overlay`, `apply_boss_split_body_z`,
`BOSS_SPLIT_BODY_Z`, `BOSS_SPLIT_OVERLAY_Z`, and the `{boss_key}_body` +
`{boss_key}_hands` convention in `upgrade_boss_sprites`.

A two-part boss is now two linked ACTORS (ADR 0020), not one body with a
render-only hands overlay. That is the real win, and it is a correctness win, not
a line-count one: a render layer cannot be hit, possessed, or killed; the giant's
hand limbs can. `GIANT_GNU_SHEET` was a byte-identical clone of the fused sheet,
so the giant mount simply inherited the layout it always described.

**THE MEASUREMENT (units: TOTAL src lines, including tests).**

| Area | net |
|---|---:|
| `game/ambition_content/assets` (the fused profile, its encounter, 5 sheet rows) | **−337** |
| `ambition_render` (the split-layer render) | **−147** |
| `game/ambition_content/src` (two duplicated arena-gate tests collapsed) | −42 |
| `ambition_sprite_sheet` | −29 |
| `game/ambition_app`, `ambition_ldtk_map`, `ambition_sim_view` (test retargets) | +9 |
| **`ambition_actors`** | **+26** |
| **repo-wide** | **−511** |

`ambition_actors/src/boss_encounter/` went **5456 → 5457** total src lines. It did
not shrink. It *grew by one line*, because the retargeted tests carry more
assertions than the ones they replaced.

**Why the premise was wrong.** The ledger's shell table listed `boss_encounter/`
at "6.8k projected out, 5.5k still resident — the BIGGEST shell (E6 deferred
teardown)", and this doc inherited that. It conflated two unrelated things:

1. **The E6 *deferred teardown*** — the fused profile + the split overlay. That is
   a CONTENT and RENDER item. Essentially none of it lived in `boss_encounter/`.
2. **`boss_encounter/`'s 5.5k residency** — which is real, live boss machinery:
   boss attack-GEOMETRY math (`attack_geometry/`, 2.0k, of which 953 lines are the
   sprite-metadata derivation tests), the phase-script runtime
   (`encounter_script.rs`, 563), the encounter entity (374), the boss
   behavior-profile schema + registry (`behavior.rs` 697 + `profile.rs` 196 +
   `specs.rs` 145), and the sim systems (609).

Measured 2026-07-10: `boss_encounter/` reaches into `crate::features` **53 times**
(`BossRef`, `BossConfig`, the cluster views, the spawn machinery). It is the boss
half of the ACTOR domain, woven to it — not an adapter awaiting a move. Its only
`ambition_content` references are three `cfg(test)` fixture `include_str!`s, the
sanctioned pattern.

**Consequence: R2 does NOT unblock R4.** This doc claimed "boss types settle with
R2". They did not: `BossRef` / `BossConfig` / `BossEncounter` all still live in
`ambition_actors`, and the victim-routing stepper still queries them. R4 therefore
loses one of its two claimed unblocks; only R3's `CollisionWorld` move discharges
a real blocker. See R4 below — this makes fable's "do NOT force this seam" more
likely to be the outcome, not less.

**Exit check — met.** `grep -rn 'BossOverlayLayer\|sync_boss_split_overlay'` over
`crates/ game/` returns zero (so do `BossBodyLayer`, `apply_boss_split_body_z`,
`BOSS_SPLIT_*`). GNU-ton still fights: `boss_lifecycle.rs`,
`boss_motion_parity.rs`, `boss_possession_specials.rs`, `boss_contact_iframes.rs`
green, plus the full gate. The ledger table is re-baselined below with units.

**Retarget-before-delete (the stated precondition) found a real behavior fact.**
`boss_possession_specials.rs` spawned the fused `gnu_ton`, which authors no
`possessed_verbs`, so a possessed boss's plain Attack fell back to capability slot
0 (`hand_slam`). The rider DOES author the G5 verb map (`attack` → `hand_sweep`),
so the retargeted test went red until it asserted the authored move. The G5 map
was already live; nothing had ever exercised it through possession. Retargeting
first is what surfaced it.

Two test subjects genuinely moved with the split rather than being renamed: the
per-animation hurtbox metrics and the head-hurtbox alignment guard (TODO #30) both
belong to the `giant_gnu` MOUNT's sheet now — the scholar's own trimmed sheet
authors no body metrics at all, which those tests now pin explicitly.

---

## R3 — the overlay split ✅ DONE (2026-07-10): `CollisionWorld` joined `ambition_world`

**The spike found one dep the analysis had missed, and fixing it properly made
the move cleaner than promised.**

`carve_portal_apertures` called `ambition_portal::pieces::subtract_aabb`. The
residue analysis listed only the three `crate::` touches and never looked at the
`ambition_*` ones — so the move as written would have forced
`ambition_world → ambition_portal`, i.e. the space IR importing a gameplay
MECHANIC. That edge is acyclic (portal names only `engine_core` +
`platformer_primitives`) so it would have compiled, and it would have been wrong:
the world IR is an authored INPUT to the sim, never a peer (decomposition.md's
fault line 2).

`subtract_aabb` is plain rectangle set-difference with **one** consumer outside
its own crate. So it moved DOWN to `ambition_engine_core::geometry` (anti-god
rule 1: vocabulary moves to the crate that owns the domain), along with the
private `aabb_mm` helper, now `geometry::aabb_from_min_max`. Its two tests
travelled with it (F7 test accounting) and a third was added for the
hole-covers-block case. `ambition_portal::pieces` keeps the portal-SPECIFIC part
— how deep and how wide the hole is (`carve_hole`, `CARVE_DEPTH`,
`SURFACE_GRACE`) — and now calls down for the algebra.

**What landed.**
- `ambition_actors/src/world/overlay_rebuild.rs` → **deleted**; it is
  `ambition_world/src/collision.rs`, named for its ONE concern (the composited
  collision world) rather than for the actors-side split it used to be half of.
- `CollisionWorld`, `world_with_sandbox_solids`, `world_with_portal_carves`,
  `world_with_gate_solids_and_carves` now live there, with their six inline tests.
- `MovingPlatformSet` moved out of `ambition_actors/src/lib.rs` down to
  `ambition_world::collision`, beside the `MovingPlatformState` it wraps.
- The `features/` hub's `world_overlay` alias and its four re-exports are
  **gone** (anti-god rule 3). All ~30 consumers import from the owning crate.
- New deps, each with its reason in the manifest: `ambition_world →
  ambition_platformer_primitives` (for `FeatureEcsWorldOverlay`, a content-free
  struct of `Block`s and `Aabb`s); `ambition_sim_view → ambition_world` and
  `ambition_content → ambition_world` (for `MovingPlatformSet`). `ambition_world`
  still uses `bevy_ecs`/`bevy_app` directly — it never took the `bevy` facade.
- `ambition_world`'s own `dependency_tests` allowlist fired on the new dep,
  correctly, and now names it.

**What did NOT move, on purpose.** `world/overlay.rs` — the REBUILD side. It
queries breakables and pogo-target volumes and imports `crate::combat::*`; it is
actor-domain and stays. Only the CONSUMPTION side left.

**Test accounting (F7).** `ambition_actors --lib` 745 → 739 (the six
`collision_world_tests` travelled); `ambition_world` 33 → 39. `ambition_portal`
52 → 50; `ambition_engine_core` +3. Every moved test name exists in its new home.

**LOC (units: TOTAL src lines, including tests).** `ambition_actors/src/world/`
1875 → 1508 (−367); `ambition_actors` overall 63,858 → 63,477 (−381);
`ambition_world` 2897 → 3296 (+399). The residue moved rather than vanished —
which is the point of a carve, and unlike R2 the number moved where the ledger
said it would.

**Exit check — met.** `CollisionWorld` is importable from `ambition_world`;
`ambition_actors` has no `overlay_rebuild` module; the full gate is green. This
was to unblock `ProjectileCollisionWorld` (R4) — and it is now R4's ONLY
discharged blocker, since R2 turned out not to settle boss types.

### The original analysis, for the record

**Unblocked — the precondition fable named has already been met, unnoticed.**

`decomposition.md`'s residue list used to say "`world/overlay{,_rebuild}.rs` join
`ambition_world` once the rebuild's inputs become plain solids." Those are **two
modules with opposite status**:

- `ambition_actors/src/world/overlay.rs` — the REBUILD side. Queries breakables
  and pogo-target volumes; imports `crate::combat::*`. Actor-domain. **Stays.**
- `ambition_actors/src/world/overlay_rebuild.rs` — the CONSUMPTION side. Owns
  `CollisionWorld` (a `SystemParam`), `world_with_sandbox_solids`,
  `world_with_portal_carves`, `carve_portal_apertures`. **Its inputs are already
  plain.**

**Why it's ready (all verified 2026-07-10):**
- `overlay_rebuild.rs` touches `crate::` exactly three times, all for
  `MovingPlatformSet`, `MovingPlatformState`, `world_with_moving_platforms`.
- `MovingPlatformState` and `world_with_moving_platforms` **already live in
  `ambition_world::platforms`**. `ambition_actors/src/world/platforms/mod.rs` is
  a `pub use` facade plus visual systems (`spawn_moving_platform`,
  `sync_moving_platform`) that pull `RoomVisual` / `RoomSet` — those stay.
- `FeatureEcsWorldOverlay` **already lives in
  `ambition_platformer_primitives::feature_overlay`**; `world/overlay.rs`
  re-exports it. Its `portal_carves` field is a plain `Vec<Aabb>`.
- `overlay_rebuild.rs`'s inline tests use only `super::*` and bevy.
- `ambition_platformer_primitives` depends on **nothing but `engine_core`**, so
  adding `ambition_world → ambition_platformer_primitives` is **acyclic**.

**So the only actor-local input is the one-line `MovingPlatformSet` newtype** in
`ambition_actors/src/lib.rs`, which wraps an `ambition_world` type and belongs
there anyway.

**Do this:** (1) spike it — move the file, repoint, see if it compiles, *before*
promising anything; (2) move `MovingPlatformSet` down; (3) add the
`platformer_primitives` dep to `ambition_world`; (4) move `overlay_rebuild.rs`;
(5) repoint consumers. The consumer list (verified) spans `features/ecs/*`,
`abilities/traversal/{blink,dive,grapple}.rs`, `body_mode/mechanics/`,
`items/pickup/`, `projectile/`, `dev/trace/`, `player/body_integration.rs`,
`ambition_runtime/src/projectile_schedule.rs`,
`game/ambition_content/src/portal/carve_adapter.rs`.

**Exit check.** `CollisionWorld` is importable from `ambition_world`;
`ambition_actors` has no `overlay_rebuild` module; the full gate is green. This
unblocks `ProjectileCollisionWorld` (R4).

---

## R4 — projectile steppers ✅ RE-CHECKED (2026-07-10). One moved. Two survive. STOPPED.

**Fable's ruling held.** `ProjectileCollisionWorld` — the one thing F2 named as
"waiting on the world/plain-input follow-up" — came home to
`ambition_projectiles::collision_world`. The other two steppers are still blocked,
so per the standing instruction they stay put and this slice ends here rather
than forcing the seam.

**Moved: `ProjectileCollisionWorld`.** R3 made every input plain, and this is what
that was for. Its three inputs, measured: `Res<RoomGeometry>` (engine_core),
`Res<FeatureEcsWorldOverlay>` (`platformer_primitives`, a content-free struct of
`Block`s and `Aabb`s), and `Query<&PlacedPortal>` (`ambition_portal`) — with its
body calling `ambition_world::collision::world_with_gate_solids_and_carves`.
`ambition_projectiles` already depended on primitives and portal; it gained
`ambition_world`, which is acyclic (the world IR names no projectile).

**Survives: victim routing.** `step_projectiles` names exactly THREE
`ambition_actors`-owned symbols. That is a sharper answer than F2's prose ("queries
bosses, actors, breakables, shields, owner combat") — everything else it touches
already lives a tier down. Measured 2026-07-10:

| Symbol | Home | Discharged by |
|---|---|---|
| `BossConfig`, `BossClusterRef` | `ambition_actors/src/features/ecs/boss_clusters.rs` | the boss-type settle R2 was *supposed* to be |
| `BossAnimationFrameSample` | `ambition_actors/src/boss_encounter/attack_geometry/` | same |
| `PlayerHealRequested` | `ambition_actors/src/player/events.rs` | **R6**, the player fold |

For contrast, these were *already* plain and needed nothing: `CenteredAabb`,
`BodyOffense`, `BodyDodgeState`, `BodyShieldState` (engine_core); `FeatureId`,
`BreakableFeature`, `ActorDisposition`, `FriendlyFire`, `HitEvent`
(ambition_combat); `PlayerEntity`, `FeatureSimEntity`, `GravityCtx`
(platformer_primitives); `LiveProjectile` (ambition_projectiles);
`GameplayTraceBuffer` (ambition_gameplay_trace).

So victim routing is *close* — but its remaining blocker is the boss cluster
views, which R2 was expected to settle and did not (see R2's correction). Forcing
it now would drag `ambition_actors`' boss vocabulary into `ambition_projectiles`,
which is the sideways import anti-god rule 4 forbids. **Stopped.**

**Survives: charge input**, exactly as predicted, and with exactly one blocker:
`crate::player::BodyAnimFacts` (`ambition_actors/src/player/components/`) — fable's
"optional player ANIMATION facts". Every other input is plain (`UserSettings` from
persistence, `ChargesProjectiles`/`ActorActionMessage` from characters,
`PlayerProjectileState` from projectiles itself, `GravityCtx` from primitives).
It folds into **R6**.

**LOC** (units: TOTAL src lines, incl. tests): `ambition_actors/src/projectile/`
1758 → 1719 (−39); `ambition_projectiles` 2182 → 2240 (+58). `projectile/`'s
`crate::features` touches: 14 → 13. A small, honest number for a slice whose
instruction was "move ONLY what is now plain".

### The original analysis, for the record

**Blocked until R2 and R3 land. Fable: "Do NOT force this seam."**

The three actor-woven steppers and their stated blockers
([`fable-final-audit-2026-07-07.md`](fable-final-audit-2026-07-07.md) §F2 —
the list IS there, in the "Projectiles — why the remaining steppers stay put"
paragraph):

| Stepper | Blocker as written | After R2/R3 |
|---|---|---|
| victim routing | queries bosses, actors, breakables, shields, owner combat; emits `HitEvent`/heal/SFX/VFX | **STILL BLOCKED.** R2 was supposed to settle boss types; it did not (see R2's correction) — `BossRef`/`BossConfig`/`BossEncounter` never left `ambition_actors`, and the stepper still queries actors/breakables/shields besides |
| world collision | needs the live feature overlay + the portal-carve snapshot; `ProjectileCollisionWorld` waits on the world follow-up | **R3** is that follow-up |
| charge input | reads brain action messages, `UserSettings`, gravity, optional player ANIMATION facts | still blocked — and it folds into **R6** anyway |

**Anchors.** `ambition_actors/src/projectile/systems.rs` →
`charge_projectile_input`, `step_projectiles`, `try_fire_projectile`;
`ambition_actors/src/projectile/mod.rs`. The model already lives in
`ambition_projectiles`.

**Do this:** after R2 and R3, re-read the blocker paragraph and move ONLY what is
now plain. If a blocker survives, say so in this doc and stop. Charge input is
expected to survive — do not chase it here. **Post-R2 expectation (2026-07-10):
only the world-collision stepper can move, and only if R3 lands.** Two of the
three blockers now survive by measurement, which makes fable's "do NOT force this
seam" the likely honest outcome for the other two.

---

## R5 — the `ControlFrame` allowlist lint ✅ DONE (2026-07-10) (= step 5's Phase C)

**`crates/ambition_runtime/tests/control_frame_lint.rs`.** Written BEFORE the fold,
which is the whole point. Eight tests; the gate on R6 is now armed.

**It found a fifth holder, and the fifth is the only real one.** This doc's own
re-count said four `Res<ControlFrame>` holders in `ambition_actors`. Measured by
the lint: **five**. The extra is
`abilities/traversal/possession.rs::possession_trigger_system`, which no name-grep
found because it is written `Res<ambition_input::ControlFrame>` — the import path
hid it. It is a SIM system, not an input bridge, so **possession is
local-player-only: a second player could never possess anything.** Its own doc
comment already said "the gesture belongs to slot 0"; the invariant was documented
and then forgotten, which is exactly what a lint is for and a paragraph is not.

Nine holders repo-wide over the scanned scope (the sim crates + `ambition_content`,
because a content RULE reading the global frame is as slot-0-only as an engine
system doing it): `engine_core`'s two latch halves, `ambition_actors`' five,
`ambition_content`'s two portal intent bridges. Each carries a `Bridge` category
(`DeviceToFrame` / `Latch` / `FrameToSlot` / `IntentBridge` / `Slot0Gesture`) and a
reason; a test asserts every `Slot0Gesture` reason opens with `MULTIPLAYER TODO`,
so the allowlist doubles as the N1 checklist.

**The lint is BIDIRECTIONAL**, because B3's rot was: an unlisted holder fails, and
an allowlist entry that matches no holder ALSO fails. B3 named
`sync_local_player_input_frame` as a holder; that system reads `Res<SlotControls>`
and never held the frame. Stale in both directions, exactly as suspected — and the
stale direction is the one no ordinary grep lint would ever catch.

**Poison-tested against REAL sources, both directions** (the doc's instruction,
followed literally):
- inject `Res<ambition_input::ControlFrame>` into
  `features/ecs/actors/update.rs` → red, naming file, line, and function;
- put B3's exact wrong claim in the allowlist → red on the STALE branch, quoting
  the history.
Six further poison tests run on synthetic sources, and they **caught two real bugs
in the scanner's first draft**: `MenuControlFrame` is a PREFIX collision (the menu's
frame is a different resource, and the first draft flagged both real menu systems),
and `init_resource::<` contains `resource::<` (so the frame's own owner,
`SimCoreResourcesPlugin::build`, tripped its own lint). A grep lint that cannot fail
is worse than none; mine could not fail *correctly* until it was made to fail.

**Exit check — met.** The lint is green; the allowlist has nine entries, each with a
category and a reason; the poison tests confirm it fails on a violation.
`unified-actors.md` B3's stale sentence is corrected in the same commit, and step 5's
Phase C is marked DONE there.

### The original analysis, for the record

**Unblocked. Small. This is the gate on R6 — write it BEFORE the fold.**

[`unified-actors.md`](unified-actors.md) step 5 said "Phase C (payoff
verification) remains" and never defined it. Defined 2026-07-10: it is this lint.

**Why it's needed, not ceremony.** B3's audit conclusion claims the only
`Res<ControlFrame>` holders inside `ambition_actors` are "the two input-bridge
writers (`populate_control_frame_from_actions`, `sync_local_player_input_frame`)".
Measured: there are **four** —
`schedule/input_systems.rs::populate_control_frame_from_actions`,
`player/input_systems.rs` (two sites, incl. `interaction_input_system`), and
`player/systems.rs::populate_slot_controls` — and `sync_local_player_input_frame`
is **not among them**. Stale in both directions, and nothing guards it:
`architecture_boundaries.rs` asserts only that `ControlFrame` lives in
`engine_core`.

All four still look like input-layer bridges, so the invariant is probably
intact. But it moved once, unnoticed. It is also a **multiplayer bug in waiting**:
the global `ControlFrame` is ONE player's frame, so a body system reading it is
silently slot-0-only.

**Build it like the determinism lints.** `ambition_runtime/tests/determinism_lints.rs`
(ADR 0023) is the template: a grep over the sim crates' non-test sources, an
explicit allowlist with a justifying comment per entry ("device→frame bridge",
"frame→slot bridge"), a failure message naming file, line, and fix, and an
`AMBITION_REVIEW(control_frame)` escape hatch printed by a companion test.

**Poison-test it.** Inject a fake sim reader into a real sim source and confirm
the lint goes red. A grep lint that cannot fail is worse than no lint — the
determinism pass proved this by finding a real `HashSet<Entity>` bug that the
"already true" measurement had missed.

**Exit check.** The lint is green, the allowlist has four entries each with a
reason, and the poison test confirms it fails on a violation. Update
`unified-actors.md` B3's stale sentence in the same commit.

---

## R6 — the player fold (S5/S6) + the `features/` rename — 🟡 IN PROGRESS (2026-07-10)

**Gated on R5 (now green). The big one. A CHECKPOINTED refactor, per the
unified-actors guardrail — each checkpoint lands green on main.**

### ✅ R6a — body vocabulary leaves `player/` (committed)

`crate::actor`'s module doc states the rule: shared body state lands on the actor
vocabulary, never on a `Player*` component, because otherwise `crate::player` is a
universal dependency sink. Two components were breaking it from inside `player/`:

- **`BodyAnimFacts`** — a body's animation signal timers. It was **the single
  biggest reason the sink existed**: 18 non-player modules imported
  `crate::player` solely to name it. Pure vocabulary, no deps → re-homed DOWN to
  `ambition_characters::actor::body`, beside `BodyCombat`/`BodyHealth`/`BodyWallet`
  (whose own doc records the identical `PlayerWallet`→`BodyWallet` re-homing).
- **`BodyMelee`** — already in `ambition_combat`; `player/components` merely
  re-exported it. Deleted; non-player code now reaches it via `crate::actor`.

Measured: the `crate::player` importer sink went **31 → 26** non-player files.

### ✅ R6b — the slot-0 filters: one real fold, ten justified survivors (committed)

**The fold found a bug.** `ability_cooldown::tick_ability_cooldown` filtered
`With<PlayerEntity>, With<PrimaryPlayer>`, but `blink` and `grapple` are already
SUBJECT-GENERIC (they act on `ControlledSubject` and arm the cooldown on *that*
body). So a **possessed actor that blinked could never blink again** — its armed
cooldown was never ticked down. The filter is gone; the system ticks every body;
`a_possessed_body_cooldown_ticks_down_too` pins it. Folding a filter fixed a
correctness bug, exactly as R2's retarget did.

Every surviving sim-side slot-0 filter now uses the NAMED `PrimaryPlayerOnly`
alias and carries a comment saying why (the exit check's second clause). Zero
spelled-out `With<PlayerEntity>, With<PrimaryPlayer>` pairs remain in
`ambition_actors`' non-test sources. Six modules — `gravity/lifecycle`,
`ability_cooldown`, `items/persist`, `items/pickup`, `shrine`,
`features/ecs/damage_apply` — no longer name the player markers at all.

The survivors, and why:

| Site | Why it is slot-0 |
|---|---|
| `possession.rs` `home_q` | the HOME AVATAR is a real concept — the body slot 0 returns to. It is *by definition* not the controlled subject during possession, so nothing else can find it |
| `shrine.rs` | heals the touching body, but also writes a CHECKPOINT — a session fact owned by the local player |
| `items/persist.rs` ×2 | the SAVE FILE is the local player's; `BodyWallet` is body vocabulary but only slot 0's balance round-trips |
| `session/reset/mod.rs` | the reset warps THE player to the start-room spawn |
| `time/time_control/` | bullet-time is a per-PLAYER feel clock (ADR 0010/0011); a second player emits its own intent |
| `dialog/yarn_bindings.rs` | `$player_x`/`$player_y` are authored against the local player — dialogue is told to a human |
| `dev/trace/systems.rs` | the replay trace records one body; per-slot traces are netcode N3 |
| `features/ecs/damage_apply.rs` ×2 | the PLAYER-VICTIM path (hitstop, death banner, safe-position rewind). Actor-vs-actor damage runs the same `HitEvent` stream |
| `gravity/lifecycle.rs` | a flip switch rewrites the ROOM's ambient gravity for everyone, so exactly one body may arm it |
| `world/rooms/systems.rs` | fallback when `ControlledSubject` is not yet resolved (startup) |

One is marked **SLOT-0 SCOPE, NOT BY DESIGN**: `items/pickup/mod.rs`'s
`held_projectile_step`. A held bolt belongs to whichever body picked it up and
should key off `ControlledSubject`. Retargeting a thrown bolt's owner changes hit
attribution — feel — and that never ships blind. Left with the reason in the code.

### ✅ R6c — the control seam leaves `player/` (committed)

`crate::control` now owns the device→slot→body path: `components` (`LocalPlayer`,
`PlayerInputFrame`, `SlotGestures`, `SlotInteractionState`, and the `PlayerSlot`
re-export), `input_systems`, `slots` (the two bridges), `queries`. This is not
player-centrism — it is the wire between a human and a body — and naming it is
most of what "player-ness is a brain and a slot, not a directory" means.

**R5's lint caught this move, which is the entire reason it was written first.**
Three of its nine allowlisted holders changed file. It went red as an
UNLISTED-HOLDER failure and, because it is bidirectional, would have gone red as a
STALE-ENTRY failure too. Not one reason changed — only the homes. A pre-existing
sibling lint (`gameplay_systems_must_not_read_res_time_directly`) fired on the same
move for the same reason. Two guardrails, two true positives, zero surprises.

Measured: the `crate::player` importer sink is **31 → 26 (R6a) → 21 (R6c)**
non-player files. `player/` is 6632 → 5685 total src lines (units: TOTAL, incl.
tests); `control/` is 962.

### ✅ R6d — `player/` is gone (committed)

**Exit-check clause 1 is met: `crates/ambition_actors/src/player/` no longer
exists.** What was in it went where it belongs, and what remained turned out to be
a real concept that had simply never been named:

| Moved | To | Why |
|---|---|---|
| `affordances/` (1971 lines) | **`crate::affordances`** (new, top-level) | a BRIDGE — input × body × world → verb. It is neither the control seam nor the actor sim, which is why it belongs to neither. |
| `movement_fx.rs` | `crate::features` | turns a frame's engine `FrameEvents` into Sfx/Vfx facts for whichever body produced them. |
| `swim.rs`, `ledge_grab.rs` | `crate::features` | thin shims over engine-owned water / ledge state. Measured: they name **zero** `crate::` types. |
| everything else | **`crate::avatar`** (the rename) | it is the HOME AVATAR's. |

**`player/` → `avatar/` is the fold's conclusion, not a cosmetic rename.** R6b
established that the home avatar is a real concept: during possession it is
precisely the body that is NOT the controlled subject, so nothing else can find
it. What is left under that name — the identity bundle, respawn safety, the blink
camera, the starting character, the emitted trail (which filters `PrimaryPlayer`,
correctly), and the tick that integrates the home body — is slot-0's by design.
Its module doc now carries the table above and the sentence that matters: *nothing
here is the ONLY path for anything; the avatar differs from every other actor in
its input frame and its respawn policy, and that is all.*

65 files repointed. Zero errors and zero warnings across
`--workspace --all-targets --features rl_sim`.

### ⏸ R6e — the `features/` rename: PARKED, with a decision brief

**Measured before executing, and the measurement changed the answer.** The plan
called this "a pure mechanical sweep" of 508 internal + 199 external references.
It is not, and doing it as scoped would leave the tree WORSE than it is today.

| Surface | Refs |
|---|---:|
| `crate::features` / `ambition_actors::features` / … | **722** |
| `Feature*`-prefixed IDENTIFIERS (`FeatureId` 165, `FeatureSimEntity` 192, `FeatureEcsWorldOverlay` 80, `FeatureName` 55, `FeatureViewIndex` 40, `FeatureView` 33, `FeatureVisual` 29, …) | **838** |
| Crates that DEFINE a `Feature*` type | **5** (`actors`, `combat`, `platformer_primitives`, `render`, `sim_view`) |

Rename the module alone and you get a module called (say) `actors` or `sim`
containing `FeatureId`, `FeatureSimEntity`, `FeatureVisual`. Today at least the
module and its types agree on the word. **A half-rename creates a new
inconsistency where there is currently only an old one** — and it violates the two
rulings that govern here: unified-actors step 6 ("rename off type-names") and
Jon's standing rule that an id must match its displayed label.

So R6e's real scope is ~1560 sites across five crates, and it turns on a naming
question the rulings do not answer.

#### DECISION BRIEF (for Jon) — what should `features` be called?

The module's own header: *"The enemy / NPC / boss ECS ACTOR SIMULATION — NOT a
feature-toggle layer. Despite the name, 'features' here means in-world entities
(actors plus room props: pickups, chests, switches, breakables, hazards)."* Since
R6d it also holds the body mechanics.

Note there are TWO things to name, and they are different: the MODULE (a
simulation) and the TYPE FAMILY (in-world entities).

| Option | Module | Types | Cost | Trade |
|---|---|---|---|---|
| **A. `sim` + `Entity*`** | `crate::sim`, `ambition::actors::sim::…` | `SimEntityId`, `SimEntity`, … | ~1560 | Reads cleanly outside; `Entity*` risks confusion with bevy's `Entity` unless prefixed `Sim`. |
| **B. `actors` + keep `Feature*`** | `crate::actors` | unchanged | ~722 | Matches vision's "ONE actors tree" literally, but yields `ambition::actors::actors::FeatureId` — redundant AND still mis-typed. **Not recommended.** |
| **C. `entities` + `Entity*`** | `crate::entities` | `EntityId`, … | ~1560 | Truest to the header's own words; collides hard with bevy `Entity` in a Bevy-native codebase. |
| **D. do nothing; fix the DOC** | unchanged | unchanged | 0 | `crates/ambition_actors/MODULES.md` already names `features` as one of three misleading names and says what it is. The cost of the name is now one table row, paid once, for every reader. |

**Recommendation: A, or D.** A is right if the 1560-site sweep is worth a day of
churn on a pre-release engine with zero dependents (it probably is — the cost only
grows). D is right if the answer is "the name is ugly but the map fixes it," which
is a legitimate answer now that a map exists and did not before.

**What is NOT on the table: B, and any half-rename.** Renaming the directory while
leaving `FeatureId` behind buys nothing and costs a new lie.

Parked here per `tracks.md`'s standing rule — *a genuine design ambiguity the
rulings do not cover: do not improvise doctrine, do not block; park the slice,
write the brief, continue with the nearest unambiguous work.* Everything else in
this chain is done.

**Still deferred by prior ruling:** folding the avatar's `ProjectileSpawner`
(cooldown + mana meter + charge state machine) onto `try_fire_ranged`. It changes
FEEL, rides the differential trace, and never ships blind. Same for
`items/pickup/mod.rs`'s `held_projectile_step` (R6b marked it in the code).

**Still deferred by prior ruling:** folding the player's `ProjectileSpawner`
(cooldown + mana meter + charge state machine) onto `try_fire_ranged`. It changes
player FEEL, rides the differential trace, and never ships blind.

### The original analysis, for the record

**Gated on R5. The big one. `player/` is 6.7k total / 4.3k production.**

`player/` existing as a SIBLING of `features/ecs` is the last structural
player-centrism. The endgame is ONE `actors` tree where **player-ness is a brain
and a slot, not a directory**. Fable deferred this "until the unified-actor work
is ready"; step 4 is done, step 5's B1/B2/B3 are done, step 6 (the
`Enemy*`→`Character*` rename) landed, and R5 discharges step 5's Phase C. The
gate opens.

**The fold is not a purge.** ~15 genuine `With<PlayerEntity>` /
`With<PrimaryPlayer>` filters remain in the actors sim (a raw grep says 26; a
third are comments *explaining* a system deliberately has no such filter).
Several are legitimately slot-0-scoped **by design** and must SURVIVE, renamed to
say so: possession's `home_q` (the "home avatar" is a real concept), the shrine's
heal+checkpoint (`shrine.rs`), the wallet save (`items/persist.rs`). Fold
candidates cluster in `player/systems.rs`, `abilities/ability_cooldown.rs`,
`items/pickup/`.

**Explicitly deferred inside this slice, by prior ruling:** folding the player's
`ProjectileSpawner` (cooldown + mana meter + charge state machine) onto
`try_fire_ranged`. It changes player FEEL, rides the differential trace, and
never ships blind. Leave it; note it in this doc when you get there.

**Guardrails** (from `unified-actors.md`): add no new `"player"`-string couplings
or `Player*`-only clusters; body-generic CONSUMERS, not just body-generic state;
this is a checkpointed refactor, not a blind rewrite — the parity harness first.

**Exit check.** `crates/ambition_actors/src/player/` no longer exists as a
sibling of `features/`; every surviving slot-0 filter carries a comment saying
why it is slot-scoped; R5's lint is still green (this is the whole point of
ordering it first); the full gate is green; `player_clone_live.rs`,
`possession_end_to_end.rs`, `unified_melee.rs`, `unified_body_movement.rs`,
`duel_arena.rs` all pass unchanged. Then do the `features/` rename.

---

## Execution record — estimated vs. actual (2026-07-10, opus)

Wall-clock per slice, measured from commit timestamps. "Estimated" is the doc's
own framing before execution; where it made no estimate the column reads by its
adjectives ("small", "the big one", "unblocked, fully pre-solved").

| Slice | Estimated | Actual | What made the difference |
|---|---|---:|---|
| R1 D-C mode-scope seam | "unblocked, fully pre-solved" | **25 min** | It was. The only cost was deciding `ModeScopedEntity`'s crate, and the demo oracle turned up the umbrella's derive-macro asterisk. |
| R2 E6 teardown | "the biggest measurable win in this chain" | **33 min** | Cheap to execute, and the required LOC measurement disproved the premise. Retarget-before-delete found the G5 possessed-verb map. |
| R3 overlay split | "spike it first" | **58 min** | The longest slice, and the spike is why: an unlisted `ambition_portal` dep meant the right move was to send `subtract_aabb` DOWN to `engine_core`, not to give the space IR a mechanic dependency. Plus ~30 consumers, a `--all-targets` feature hole, and a full disk. |
| R4 projectile steppers | "re-check, do NOT force" | **11 min** | Measurement, one 45-line move, and a STOP. The doc's instruction was the whole slice. |
| R5 ControlFrame lint | "an afternoon" | **12 min** | Far cheaper than estimated because `determinism_lints.rs` was a working template. The poison tests cost more than the lint and were worth more. |
| R6a body vocab out | (part of "the big one") | **15 min** | `BodyAnimFacts` had one obvious home and 18 importers. |
| R6b slot-0 filters | (part of "the big one") | **15 min** | The `ability_cooldown` fold turned out to be a live bug. |
| R6c control seam out | (part of "the big one") | **18 min** | The compiler and two lints did the finding; I only had to place things. |
| R6d `player/` dissolves | (part of "the big one") | **31 min** | `affordances/` to its own top-level home, body mechanics to the actor tree, and the remainder correctly renamed `avatar/`. 65 files repointed; compiled first try. |
| R6e `features/` rename | "a pure mechanical sweep" | **PARKED** | Measuring it is what parked it: 1560 sites across 5 crates, not 722, and a half-rename makes the tree worse. Decision brief written. |
| D-B `MODULES.md` | "[sonnet]. Mechanical." | **8 min** | Mechanical, once it was generated from module headers rather than written by hand. |

**Total: ~4h for R1–R3, R5, R6a–R6d, D-B, plus R4's and R6e's ruled stops.** The
slice that ran long (R3) and the two that came in far under (R4, R5) were all cases
where the doc's instruction — *spike it*, *don't force it*, *poison-test it* — was
the load-bearing part, not the code. **Measuring before sweeping stopped two slices
from doing damage** (R2's premise, R6e's scope) and was mandated by the standing
rules for entirely different reasons.

Three findings the plan did not predict, each surfaced by a step the plan
mandated for a different reason:
1. **R2's premise was false.** The exit check's "record the LOC, with units" is
   what proved `boss_encounter/` never was a shell.
2. **`possession_trigger_system` reads the global `ControlFrame`.** R5's lint
   found it; four separate hand-greps (including this doc's) had not.
3. **A possessed body's ability cooldown never expired.** R6b's filter fold found
   it, because `blink`/`grapple` had already been made subject-generic.
4. **The `features/` rename is twice the size the plan thought**, and the half
   version is worse than doing nothing. R6e's "state the units" instinct, applied
   to a reference count instead of a line count.

Three PRE-EXISTING failures were fixed on the way, none of them this chain's: a
`portal_render`-gated content test that had not compiled in weeks (R3), a RED
`check_agent_kb.py` (ADR 0023 missing a required section, D-B), and four dead-code
warnings in `ambition_touch_input`. The workspace now builds `--all-targets
--features rl_sim` with zero errors AND zero warnings.

## What this chain does NOT do

- **No new crates.** The ledger ruling says no further crate split is owed, and
  none buys compile time (≥72 s of the 104 s play loop is the tower ABOVE
  `ambition_actors`). `abilities/` remains a *discretionary* candidate on
  navigability grounds only. Do not carve it as part of this chain.
- **No `features/` split.** Fable: splitting spawn/tick/perceive/damage-routing
  apart would re-fork the actor unification (U1).
- **No module-tree churn before R6.** The `features/` rename rides the fold.
