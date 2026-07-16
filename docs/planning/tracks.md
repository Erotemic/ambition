# Tracks — current executable queue

This file contains current work only. The canonical HEAD summary is
[`status.md`](status.md). Historical execution records through 2026-07-11 are
archived in
[`docs/archive/reviews/planning-history-2026-07-11/`](../archive/reviews/planning-history-2026-07-11/).

Before changing a status, inspect the source and run the owning exit checks. Do
not copy a prior completion grade forward without re-establishing its evidence.

## Standing verification

The exact test set depends on the touched crates. Structural work normally runs:

```bash
cargo test -p ambition_actors --lib
cargo test -p ambition_engine_core -p ambition_runtime -p ambition_host
cargo test -p ambition_dialog -p ambition_sim_view -p ambition_combat
cargo test -p ambition_characters
cargo test -p ambition_content --features portal
cargo test -p ambition_content --features ui --test yarn_compile
cargo test -p ambition_app --features rl_sim
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

Poison tests land with the enforcement they prove. A diagnostic or ignored test
is not a completion gate.

## Priority queue

### 1. Encounter convergence

**State:** partial foundation, not a unified runtime authority.

**Plan:** [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md)

**Next slice:** define the generic lifecycle and command ingress before adding a
new customer. The slice must prove `Start`, `Signal`, objective success/failure,
and retirement without routing through a boss-specific or wave-specific reducer.

**Exit evidence:** a headless no-actor signal/timer encounter and an ordinary
wave encounter use the same lifecycle reducer; cleanup behavior is selected by
participant ownership/lifetime policy.

### 2. N3.2 exact restore

**State:** open. Existing validation and refusal paths are useful, but exact
rollback across room context and dynamic births is not established.

**Plan:** [`engine/netcode.md`](engine/netcode.md)

**Next slices, in order:**

1. atomic room-context restore;
2. per-spawner reconstruction recipes;
3. standalone preflight of cursor/resolved codecs before mutation;
4. bounded rollback/resimulation proof.

Do not relabel a refusal as exact restore. The acceptance test must restore and
resimulate the difficult window rather than merely reject it.

### 3. Sanic visible/playable recovery

**State:** playable-persona ARCHITECTURE landed (`DONE`): canonical sim-owned
`WornCharacter` identity → gameplay derive + generic `ambition_render` presentation
binder (app + demos), standard host-input path proven headlessly, app duplicate
binder deleted + guarded. ONE identity — dialogue now reads the entity's
`WornCharacter` too (no `StartingCharacter` second authority). Gameplay derivation
is now TOTAL and deterministic: the wear overlay resolves name + kit from identity +
the body's persisted `AbilitySet`, never from prior component state. The
`default_character_id`↔hardcoded-kit coupling is GONE — a new engine-neutral
`PlayableKitSource::{Authored,HostCode}` catalog field decides whose kit a worn body
uses, so the demo genuinely wears Sanic's authored PEACEFUL kit (no melee/ranged/
special) while riding `SurfaceMomentum` (ball dash live), and a `HostCode`
protagonist rebuilds its code kit on re-wear/restore (the old "documented gap" is
closed). `visible` implies `input` so the windowed binary is playable. `OPEN`:
native Sanic sheet provisioning only (demo draws a marked fallback). Per-slice
statuses + test evidence in the plan.

**Plan:** [`demos/sanic-recovery.md`](demos/sanic-recovery.md)

**Next slice:** provision the native Sanic sheet so the windowed demo draws real art.

### 4. Unified swappable movement kernel

**DONE**, including the frame-authority migration (commits `17685105` →
`477700e9`). [ADR 0024](../adr/0024-frame-aware-unified-movement-kernel.md) is
mechanically enforced: ONE frame resolution phase publishes every body's
`ResolvedMotionFrame` (independent basis + accumulated `ForceZone`
contributions) and every consumer reads it; ONE `step_motion` entry plus three
named non-kernel authorities (`transit_body`/`carry_body`/
`constrain_body_pose`); policy-private state lives inside the `MotionModel`
variant with `BodyMotionFacts` as the only outside read surface; support is a
semantic `SupportFact`; the crawler attaches at arbitrary angles through
`SurfaceChain` geometry. Four poison-tested workspace-policy guards.
[`engine/unified-movement-kernel.md`](engine/unified-movement-kernel.md)
documents the invariants, ownership map, and residual debt (portal-transit
orientation source; gravity-resource snapshot registration; device-latch
typing; ball-dash typed op; block↔chain crawl transfer).

**Next slice:** none scheduled; pick up a residual-debt item opportunistically
alongside the next snapshot-ledger or input pass.

### 5. CC3 enforcement

**State:** diagnostic rig landed; the comprehensive test remains ignored.

**Plan:** [`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) §6

**Next slice:** decide the hard thresholds and make each illegal-state class fail
under a deliberate poison fixture. Only then remove `#[ignore]` or add a separate
non-ignored policy test.

### 6. BD5 boss validator (diagnostic — no active enforcement work)

**State:** PARTIAL / DIAGNOSTIC. The validator infrastructure exists and reports
8 errors / 10 warnings as diagnostic findings. By maintainer decision it is
**non-blocking**: not a gate, not a dependency for any other track.

**Plan:** [`engine/boss-design.md`](engine/boss-design.md) §11 (per-slice
DONE/OPEN/BLOCKED).

**No enforcement work is queued.** Do NOT wire the validator into installation and
do NOT drive errors to zero now — several rules are BLOCKED on missing engine
expressivity (boss-feel representations), and the error/warning counts are not a
failure condition. Revisit enforcement only when authoring bosses for feel or
approaching shipment; an install gate is a separate maintainer decision.

### 7. Super Mary-O completion

**State:** equipment rows and mechanism, camera scroll policy, and flag sequence
exist. The gameplay customer is incomplete.

**Plan:** [`demos/super-mary-o.md`](demos/super-mary-o.md)

**Next slices:** pickup/equip wiring, body-scale collision/render read-fold,
enemies and shell prop, HUD/results, then the headless 1-1 run.

### 8. R6e naming decision

**State:** parked. `player/` has been dissolved; `features/` still names the
actor/prop simulation tree.

**Plan:** [`engine/refactor-chain.md`](engine/refactor-chain.md)

**Decision:** either perform a coherent module-and-type-family rename, or accept
the current name with the module map. A module-only rename is forbidden because
it preserves the misleading `Feature*` vocabulary while adding a second name.

### 9. Large inline-test debt

**State:** the test-policy migration succeeded, but the repository-wide
"no 200-line inline test module" claim is false. The machine inventory currently
finds:

- `crates/ambition_characters/src/equipment.rs`
- `crates/ambition_audio/src/catalog.rs`
- `game/ambition_demo_mary_o/src/flag.rs`
- `game/ambition_demo_mary_o/src/lib.rs`

**Plan:** [`test-refactor-plan-2026-07-10.md`](test-refactor-plan-2026-07-10.md)

Extract these modules if the threshold is binding. Otherwise remove the threshold
from project policy rather than declaring it met.

## Open playtest and polish reports

These are observations, not architecture-completion claims:

| Report | Owning area | Required evidence |
|---|---|---|
| Slash VFX can render as a black square | render/sprite source | Reproduce in a real visual run and add a source-selection regression test. |
| DI and smash-charge feel/input seams | combat/input | Jon's feel pass plus an explicit held/released attack signal if partial charge remains desired. |
| Modal body morph still has duplicated presentation machinery | render/character presentation | Replace the bespoke sibling sprite with a body-sheet state only after visual parity is demonstrated. |
| Shrine and glider sprites are broken | sprite publishing/rect metadata | Reproduce against the generated sheet and pin rect/manifest ownership. |
| Portal gun should be an ordinary item | item/portal composition | One item owns one portal pair through the existing spawn capability; no portal-gun engine special case. |
| Build cache can grow excessively | developer tooling | Measure target-dir growth and document an explicit prune workflow before automating deletion. |

## Status-edit checklist

Before writing `DONE`, `LANDED`, `ONE authority`, or `none remain`:

1. Name the exact acceptance test or source invariant.
2. Run it at HEAD.
3. Search for competing owners and consumers, not only the newly added type.
4. Update [`status.md`](status.md) and the owning plan in the same commit.
5. Archive the execution narrative instead of appending it here.

The binding decomposition + completion rules — DONE/OPEN/BLOCKED, caveated
completion is not completion, and no invented status words in executable tables —
are project planning policy and live in [`README.md`](README.md) §"Completion
policy". This queue conforms to them; it does not restate them.
