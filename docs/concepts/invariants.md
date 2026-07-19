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

## Documented elsewhere (pointers)

- **Bevy `Query` iteration order is not stable** — sort by `SimId`/stable key
  wherever order affects outcomes; raw `Entity` ids are NOT stable across GGRS
  rollback entity recreation. `docs/concepts/engine-mental-model.md`, ADR 0023,
  deep-review-2026-07-19 §2.5.
- **`cargo check -p <one_crate>` is not the gate** — `cargo check -p
  ambition_app` is; and the inverse trap is real too (a crate that only
  compiles when co-built siblings unify features in — declare what you use;
  see `ambition_game_shell`'s and `ambition_platformer_primitives`' manifest
  comments). AGENTS.md §Verification.
- **App tests build into ONE `app_it` target** — `cargo test -p ambition_app
  --test app_it -- <module>`. AGENTS.md §Verification, ADR 0025.
- **Time domains are explicit** — timers use `WorldTime::scaled_dt` inside the
  sim; presentation uses `ambition_time::PresentationTime` (under the GGRS host
  `WorldTime.scaled_dt` is the fixed tick — consuming it per rendered frame
  ties animation to refresh rate); never mutate `time_scale` directly, fire
  `ClockScaleRequest`. `docs/concepts/input-and-game-modes.md`, ADR 0011.
- **No pushout, ever** (one exception: portal-close straddle eviction) —
  sweep to TOI; nothing teleports. `docs/planning/vision.md` §8,
  `engine/collision-and-ccd.md`.
- **Feet = the +gravity face of the contact box** (`AabbExt::feet`) — never
  screen-down. `engine/unified-movement-kernel.md`.
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
