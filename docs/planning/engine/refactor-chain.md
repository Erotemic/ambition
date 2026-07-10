# The refactor chain — dissolving the adapter shells, then folding the player

**Status:** R1 DONE (2026-07-10). Six slices, in dependency order.
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

## R3 — the overlay split: `CollisionWorld` joins `ambition_world`

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

## R4 — projectile steppers: re-check, don't force

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

## R5 — the `ControlFrame` allowlist lint (= step 5's Phase C)

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

## R6 — the player fold (S5/S6) + the `features/` rename

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

## What this chain does NOT do

- **No new crates.** The ledger ruling says no further crate split is owed, and
  none buys compile time (≥72 s of the 104 s play loop is the tower ABOVE
  `ambition_actors`). `abilities/` remains a *discretionary* candidate on
  navigability grounds only. Do not carve it as part of this chain.
- **No `features/` split.** Fable: splitting spawn/tick/perceive/damage-routing
  apart would re-fork the actor unification (U1).
- **No module-tree churn before R6.** The `features/` rename rides the fold.
