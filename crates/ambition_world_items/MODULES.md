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

### ⛔⛔ KNOWN DEFECT, LIVE: this crate's systems lost their simulation phase

Carved out of `actor_monolith::items::pickup` on 2026-09-02 (`69641a83f`). The
carve moved `add_systems` and the ordering BETWEEN the two systems, and left
behind the `configure_sets` that said their set was
`.in_set(Platformer2dSimulationPhaseMonolith::PlayerSimulation)` and
`.after(shared_tangle::lifecycle::BodyCustodySettled)`.

`WorldItemSimulationPlugin` registers the chain `.in_set(GameplayGated)` and
nothing else, so **two facts are missing**: phase membership (`GameplayGated`
does not imply `GameplaySimulationRoot`, so the systems are not authorized as
part of a session's simulation) and the custody ordering edge (a collect can
observe a hand mid-settle).

⚠ **IT COMPILES, IT RUNS, AND BOTH SYSTEMS EXECUTE EVERY FRAME.** Nothing fails,
which is why it survived a 548-test suite and a workspace gate. Found by review,
not by a test.

⇒ The fix is specified in
`docs/planning/engine/actor-monolith-decomposition.md` — a `WorldItemSet
{ Motion, PreCollect, Collect }` in `shared_tangle::schedule`, configured by
THIS crate's plugin end to end, with a guard that asserts phase MEMBERSHIP
rather than that the systems exist. ⛔ Do not repair it by putting the systems
back into `ItemPickupSet`: that set belongs to the PRESSED pickup, and sharing
it was an accident of where the code used to live.

### The split this crate is one half of

`ItemPickupSet` / `items::pickup` keeps the PRESSED pickup — a held weapon taken
with `Attack`. This crate owns the TOUCHED collectible — walked into. The line is
the collect TRIGGER, which is the line the pickup module's own
`AMBITION_REVIEW(discrete_ok)` note had already drawn.
