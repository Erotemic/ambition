# `ambition_boss_encounter` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_boss_encounter** — Boss encounter domain.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`anim`](src/anim.rs) | Boss animation-state derivation from boss-owned runtime state. |
| [`attack_geometry`](src/attack_geometry/mod.rs) | Pure authored attack/body volume math; no ECS access or mutation. |
| [`attack_moveset`](src/attack_moveset.rs) | Boss runtime glue for constructing the shared data-driven attack moveset. |
| [`behavior`](src/behavior.rs) | Boss behavior-profile vocabulary (data-driven). |
| [`catalog`](src/catalog.rs) | App-local composition of provider-authored boss data. |
| [`clusters`](src/clusters.rs) | Authoritative boss ECS components and `BossMut` / `BossRef` views. |
| [`conditions`](src/conditions.rs) | Authored BOSS conditions — "did the player beat this one?" |
| [`ecs`](src/ecs/mod.rs) | Boss encounter-phase projection, brain tick, and body integration systems. |
| [`encounter_entity`](src/encounter_entity.rs) | The ENCOUNTER as a first-class, OPTIONAL entity. |
| [`encounter_script`](src/encounter_script.rs) | Encounter-script EXECUTION + its actor-specific mechanics. |
| [`events`](src/events.rs) | Boss-encounter presentation sink. |
| [`ids`](src/ids.rs) | Boss encounter id helper: `encounter_id_from_name` slugs an authored boss name into a stable id (`"Clockwork Warden"` -> `"clockwork_warden"`). |
| [`pattern`](src/pattern/mod.rs) | THE BOSS PATTERN'S THINKING, which is this domain's own business. |
| [`profile`](src/profile.rs) | Assembled per-boss profile: the content-facing bundle. |
| [`registry`](src/registry.rs) | `BossEncounterRegistry` — the read-only boss DATA CATALOG. |
| [`rewards`](src/rewards.rs) | Boss reward-chest sync — the ECS mirror of "this boss placement is cleared, so its authored `DropChest` reward exists in the room". |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_boss_encounter`. |
| [`roster`](src/roster.rs) | The lib's generic boss-encounter base. |
| [`specs`](src/specs.rs) | App-local boss-encounter spec access. |
| [`sprites`](src/sprites/mod.rs) | Compatibility facade for boss sprite-sheet types. |
| [`systems`](src/systems.rs) | Boss-encounter Bevy systems — the per-frame driver. |

_21 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

Carved out of `ambition_platformer2d_actor_monolith` on 2026-08-17 (D33), 7,635
lines. It depends on nothing above it and **no facade re-export stands in the
monolith** — callers name this crate, or reach it as `ambition_platformer2d::boss_encounter`
through the umbrella (which is what the `game.*-umbrella-only` policies require of
a demo).

**Seams worth knowing.**

* The `Progression` phase chain that drives the per-frame boss tick is registered
  by `ambition_platformer2d_runtime`, not here. This crate owns the CONTENT SLOTS
  in that chain — `ContentEncounterScriptSet`, `ContentEncounterVictorySet`,
  `ContentQuestRewardSet` — so a named game can interleave without the engine chain
  ever naming a content system.
* Player→boss damage ROUTING still lives in the monolith
  (`features::ecs::damage::boss_hit`), and it calls in. That direction is fine; it
  is the reason `features/ecs/bosses/` did NOT have to move for this carve.
* `MountDied` comes from `ambition_platformer2d_shared_tangle::body`, not from the
  monolith's mount coupling that writes it — the message sits below both.
* `test-support` publishes `test_boss_catalog` and `clusters::test_support`. It is
  a dev-only feature because `#[cfg(test)]` does not cross a crate boundary and the
  monolith's boss-adjacent tests build the same shapes. ⛔ never enable it from
  `[dependencies]`.
* `impl SnapshotCursor for BossEncounter` lives in `clusters.rs` because the orphan
  rule puts it there. A field added to what it encodes is a WIRE FORMAT change.
