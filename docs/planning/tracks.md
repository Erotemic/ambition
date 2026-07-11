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

**State:** windowed presentation and ball dash exist; reusable input and selected
character composition remain unproven.

**Plan:** [`demos/sanic-recovery.md`](demos/sanic-recovery.md)

**Next slice:** add the focused end-to-end input checkpoint test and fix only the
first broken checkpoint. Then extract the selected-character presentation binder
used by both the main app and standalone demos.

**Exit evidence:** two selectable Sanic profiles drive the standard input path
and render their declared art without app-local sprite or keyboard code.

### 4. CC3 enforcement

**State:** diagnostic rig landed; the comprehensive test remains ignored.

**Plan:** [`engine/collision-and-ccd.md`](engine/collision-and-ccd.md) §6

**Next slice:** decide the hard thresholds and make each illegal-state class fail
under a deliberate poison fixture. Only then remove `#[ignore]` or add a separate
non-ignored policy test.

### 5. BD5 boss validator (diagnostic — no active enforcement work)

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

### 6. Super Mary-O completion

**State:** equipment rows and mechanism, camera scroll policy, and flag sequence
exist. The gameplay customer is incomplete.

**Plan:** [`demos/super-mary-o.md`](demos/super-mary-o.md)

**Next slices:** pickup/equip wiring, body-scale collision/render read-fold,
enemies and shell prop, HUD/results, then the headless 1-1 run.

### 7. R6e naming decision

**State:** parked. `player/` has been dissolved; `features/` still names the
actor/prop simulation tree.

**Plan:** [`engine/refactor-chain.md`](engine/refactor-chain.md)

**Decision:** either perform a coherent module-and-type-family rename, or accept
the current name with the module map. A module-only rename is forbidden because
it preserves the misleading `Feature*` vocabulary while adding a second name.

### 8. Large inline-test debt

**State:** the test-policy migration succeeded, but the repository-wide
"no 200-line inline test module" claim is false. The machine inventory currently
finds:

- `crates/ambition_characters/src/equipment.rs`
- `game/ambition_demo_smb1/src/flag.rs`

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
