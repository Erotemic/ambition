# Universal brain interface

**Status:** live, but not finished. The original 2026-05 design plan has been condensed into this current-state page.

## Current contract

Every controllable entity is moving toward the same pipeline:

```text
Brain -> ActionSet / ActorControl -> ActorActionMessage -> gameplay consumers
```

The live code has sibling components for brain/action/control state, and bosses are actors rather than a separate special-case class. The player ECS migration is complete: the old `ae::Player`, `PlayerMovementAuthority`, and `PlayerBody` aggregate paths are gone.

## Why this exists

The project used to split behavior by entity class: player, NPC, enemy, and boss each had their own control vocabulary and update path. That made “play as a goblin,” RL control, remote co-op, and content-authored behavior harder than they needed to be.

The desired shape is that entity identity chooses a brain backend, not a bespoke simulation loop.

## Brain backends

| Backend | Intended use |
|---|---|
| `Player` | Human/controller input. |
| `Remote` | Networked or replayed control frames. |
| `RlPolicy` | Batched inference / training. |
| `StateMachine` | NPC/enemy-style AI. |
| `BossPattern` | Generic boss-pattern driver with named boss data above it. |
| `Scripted` | Cutscene or authored input tracks. |

Not every backend is equally implemented today. Treat this table as the stable design vocabulary, not proof that every row is production-ready.

## Current cleanup targets

- Keep player movement on `ActorControl` instead of reintroducing player-only control seams.
- Keep melee/projectile/boss consumers on `ActorActionMessage` where possible.
- Centralize enemy brain construction and variation policy.
- Finish any remaining player-specific pogo/target-surface duplication by sharing target-surface policy.
- Add `RlPolicy` only when there is a concrete training/inference consumer; do not add speculative FFI.

## Rules for future work

- Add behavior variants in Rust when they need new code paths.
- Put stable knobs and named content in data/content layers.
- Avoid `dyn Brain` in hot paths; enum dispatch and backend batching are easier to profile and serialize.
- Overlap old and new consumers briefly when changing authority, then delete the old path in the same branch.

## Current references

- `docs/systems/brain-driver.md`
- `docs/recipes/extending-brains-and-action-sets.md`
- `docs/adr/0016-actor-unification.md`
- `crates/ambition_actor/src/brain/`
- `crates/ambition_actor/src/actor/control.rs`
