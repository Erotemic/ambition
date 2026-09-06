---
id: invariants
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-30
related_docs:
  - AGENTS.md
  - docs/concepts/engine-mental-model.md
  - dev/journals/lessons_learned.md
---

# Invariants and traps — the ones that bite

A reference index, not enforcement machinery. Each entry is a rule that has
actually burned an agent in this repo, with where the full story lives. The
first two are documented ONLY here — they were previously discoverable only by
being burned.

## Documented only here

### rustfmt on a `mod.rs` cascades over the whole module tree

`rustfmt --edition 2021 crates/x/src/foo/mod.rs` does not format one file — it
formats every module `foo/` re-exports, producing a huge unrelated diff.
Discipline: format only the files you touched, snapshot `git status` before and
after formatting, and never chain `cargo fmt` into `git add`. Formatting is
advisory, not an acceptance gate (AGENTS.md §Patch discipline).

### Required components silently skip systems

A Bevy system whose query demands `&ComponentX` simply never runs for entities
missing `ComponentX` — no error, no log. When a spawn path forgets one
component of a cluster, every system over that cluster silently ignores the
entity, which presents as "the feature does nothing" rather than a crash. When
adding to a spawn bundle, grep for the cluster's other members' spawn sites;
when a system mysteriously doesn't fire, diff the entity's components against
the query first. (Same failure class: an `Option<Res<T>>` that is `None` in
production because an `insert_resource` was missed — catalog-authority
resources must be non-optional in prod.)

### A surface's art REPEATS; stretching one moves the collision off the picture

Every generated entity prop (`assets/sprites/entities/solid_block.png` and its
kin) carries a **4px fully transparent border**. Stretch one across a block that
is a different shape — Smash's 420×32 stage, Mary-O's 640×32 vault floor — and
the border stretches with it, so the block is solid for ~18–28px past each end of
anything you can see: an invisible floor you stand on and an invisible wall you
hit in mid-air. Since 2026-08-06 the renderer draws every block by REPEATING its
kind's seamless `*_Tile` texture at native scale, whatever the block's
provenance, and the four stretch-only props (`SolidBlock`, `OneWayPlatform`,
`SoftBlinkWall`, `HardBlinkWall`) are deleted so the path cannot come back by
autocomplete. A new `BlockKind` must bring a tile texture or declare itself a
point in `is_point_block_kind` — pinned by
`every_surface_kind_has_a_tile_texture`. If art ever must be stretched again, it
may not have transparent edges, or the collision stops being visible.

## Documented elsewhere (pointers)

- **Bevy `Query` iteration order is not stable** — sort by `SimId`/stable key
  wherever order affects outcomes; raw `Entity` ids are NOT stable across GGRS
  rollback entity recreation. `docs/concepts/engine-mental-model.md`, ADR 0023,
  deep-review-2026-07-19 §2.5.
- **Rollback codec is not rollback correctness** — authoritative state also
  needs the correct entity/lifetime participation, stable semantic identity, and
  deterministic composition when several peers can affect one result. A boot
  census cannot prove runtime-created populations.
  `docs/architecture/engine-architecture.md`, ADR 0027.
- **Gameplay session is not rollback timeline** — rollback health may carry
  across timeline generations only for the same `SessionScopeId`; a different
  gameplay session must not inherit foreign rollback confirmation/health as live
  authority. `docs/architecture/engine-architecture.md`, ADR 0027.
- **`cargo check -p <one_crate>` is not the gate** — `cargo check -p
  ambition_app` is; and the inverse trap is real too (a crate that only
  compiles when co-built siblings unify features in — declare what you use;
  see `ambition_game_shell`'s and `ambition_platformer2d_shared_tangle`' manifest
  comments). AGENTS.md §Verification.
- **App tests build into ONE `app_it` target** — `cargo test -p ambition_app
  --test app_it -- <module>`. AGENTS.md §Verification, ADR 0025.
- **Time domains are explicit** — timers use `WorldTime::scaled_dt` inside the
  sim; presentation uses `ambition_time::PresentationTime` (under the GGRS host
  `WorldTime.scaled_dt` is the fixed tick — consuming it per rendered frame
  ties animation to refresh rate); never mutate `time_scale` directly, fire
  `ClockScaleRequest`. `docs/concepts/input-and-game-modes.md`, ADR 0011.
- **No GEOMETRY-REPAIR pushout** (one exception: portal-close straddle eviction)
  — sweep to TOI; nothing teleports out of an overlap it should never have
  entered. ⚠ **this is a rule about repairing a mistake, not about contact.** Jon,
  2026-08-20: *"The no pushout rule I think is for portals… For bodies I think it
  might be ok. This isn't a hack, it is a game feel feature… It should never be a
  mandatory part of the movement kernel though."* An intentional mechanic may
  constrain, impulse or displace a body against another one — what it may not do
  is become a term every body pays for. `docs/planning/vision.md` §8,
  `docs/concepts/movement-collision.md`, `maintainer-decisions.md` 2026-08-20.
- **Feet = the +gravity face of the contact box** (`AabbExt::feet`) — never
  screen-down. `docs/adr/0024-frame-aware-unified-movement-kernel.md`.
- **ONE BODY, ONE PATH** — before keying anything on player-vs-actor, run the
  bifurcation smell test. AGENTS.md §Core values (the long paragraph).
- **std `HashMap`/`HashSet` iteration is banned in sim** — machine-enforced
  (`tests/ambition_workspace_policy`, ADR 0023); known scanner blind spots are
  listed in deep-review-2026-07-19 §"policy tests already guard".
- **Git-ignored is not missing** — binary asset payloads are present on disk
  but ignored; `ls` before concluding an asset is unavailable. AGENTS.md
  §Current architectural stance.
- **The `.agent` index is machine-local and can be stale** — `agent_query.py`
  now warns when it is behind HEAD; regenerate with
  `python scripts/generate_agent_index.py`. Confirm every generated result in
  source before editing.
