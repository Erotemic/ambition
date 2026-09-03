# `ambition_world_items` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_world_items** — The physical life of a collectible in the world.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`item_motion`](src/item_motion.rs) | Authored pickup motion plans stepped by the engine. |
| [`world_item`](src/world_item.rs) | `WorldItem` — a walk-into collectible that grants EQUIPMENT. |

_2 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._

### ✔ CLOSED: this crate's systems have their simulation phase back

Carved out of `actor_monolith::items::pickup` on 2026-09-02 (`69641a83f`). The
carve moved `add_systems` and the ordering BETWEEN the two systems, and left
behind the `configure_sets` that said their set was
`.in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)` and
`.after(shared_tangle::lifecycle::BodyCustodySettled)` — so **two facts were
missing**: phase membership (`GameplayGated` does not imply
`GameplaySimulationRoot`, so the systems were not authorized as part of a
session's simulation) and the custody ordering edge (a collect could observe a
hand mid-settle).

⚠ **IT COMPILED, IT RAN, AND BOTH SYSTEMS EXECUTED EVERY FRAME.** Nothing
failed, which is why it survived a 548-test suite and a workspace gate. Found by
review, not by a test — and that is the part worth keeping after the fix.

⇒ **Fixed in `9a89bfa20`**, in the shape D33 specifies:
`WorldItemSet { Motion, PreCollect, Collect }` lives in
`shared_tangle::schedule` as VOCABULARY (so a game can order on it without
naming this crate), and `WorldItemSimulationPlugin` does BOTH the
`configure_sets` — the chain, nested in `PlayerSimulation`, each variant
`GameplayGated`, `Motion.after(BodyCustodySettled)` — and the `add_systems`. The
kernel merely composes the plugin. ⛔ The repair deliberately NOT taken was
putting the systems back into `ItemPickupSet`: that set belongs to the PRESSED
pickup, and sharing it was an accident of where the code used to live.

⛔⛔ **THE GUARD IS `simulation_phase_tests` IN `lib.rs`, AND ITS SHAPE IS THE
LESSON.** It asserts phase MEMBERSHIP — the hierarchy edge — never that the
systems exist, because the defect shipped with both systems present and running.
It builds a bare `App` with ONLY this crate's plugin: composing the kernel
beside it would let the kernel's own `configure_sets` supply an edge this crate
failed to declare, and the test would pass on the very defect it exists to
catch.
