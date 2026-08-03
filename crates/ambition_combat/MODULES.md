# `ambition_combat` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat** — Combat helpers and reusable damage volumes.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`archetype_spec`](src/archetype_spec.rs) | The enemy-ARCHETYPE authoring vocabulary — the type `character_archetypes.ron` is deserialized into. |
| [`authored_volumes`](src/authored_volumes.rs) | App-local authored attack-volume resolution. |
| [`banner`](src/banner.rs) | Gameplay banner ticking and deferred-request application. |
| [`breakables`](src/breakables.rs) | Per-frame tick for breakable feature entities: respawn countdown and the stand-to-break collapse trigger. |
| [`causal`](src/causal.rs) | This crate's causal facts: **why did a body lose a stock, get eliminated, or end the match?** |
| [`components`](src/components/mod.rs) | ECS-native feature components. |
| [`content_schema`](src/content_schema.rs) | The combat capability's authored-content SCHEMA registration. |
| [`events`](src/events.rs) | Combat-kit message/event vocabulary + small shared value types. |
| [`falling_chest`](src/falling_chest.rs) | Falling-chest physics for ECS reward chests. |
| [`hazard_runtime`](src/hazard_runtime.rs) | `HazardRuntime`: the per-hazard runtime blob (id/name/pos/size, its `DamageVolume`, optional patrol `PathMotion`, and resolve `HitMode`) carried by LDtk-entity hazards. |
| [`hazards`](src/hazards.rs) | Hazard tick: patrol motion and contact damage. |
| [`held_items`](src/held_items.rs) | ECS-owned held item capability for actors. |
| [`hitbox`](src/hitbox/mod.rs) | Hitbox-entity lifecycle: spawn → overlap-check → despawn. |
| [`moveset`](src/moveset/mod.rs) | Data-driven move playback — the runtime half of the Smash model. |
| [`on_hit`](src/on_hit.rs) | On-hit techniques — the conditional-hit primitive of the ability model. |
| [`path_motion`](src/path_motion.rs) | `PathMotion`: waypoint-following used by moving hazards/platforms. |
| [`rules`](src/rules.rs) | **The combat rules a match plays under — resolved, not borrowed.** (AE6) |
| [`slots`](src/slots.rs) | Anti-clump attack-slot arbitration. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`stocks`](src/stocks.rs) | **Stocks: the loop a KO'd fighter actually goes round.** (S4 part 1) |
| [`targeting`](src/targeting.rs) | Per-frame `ActorTarget` selection for non-player actors. |
| [`util`](src/util.rs) | Grab-bag of small feature-side helpers — not a cohesive subsystem. |
| [`variation`](src/variation.rs) | Stable per-actor variation helpers for ECS feature actors. |

_23 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
