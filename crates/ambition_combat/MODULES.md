# `ambition_combat` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat** — Combat helpers and reusable damage volumes.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authored_volumes`](src/authored_volumes.rs) | App-local authored attack-volume resolution. |
| [`banner`](src/banner.rs) | Gameplay banner ticking and deferred-request application. |
| [`breakables`](src/breakables.rs) | Per-frame tick for breakable feature entities: respawn countdown and the stand-to-break collapse trigger. |
| [`components`](src/components/mod.rs) | ECS-native feature components. |
| [`events`](src/events.rs) | Combat-kit message/event vocabulary + small shared value types. |
| [`falling_chest`](src/falling_chest.rs) | Falling-chest physics for ECS reward chests. |
| [`hazard_runtime`](src/hazard_runtime.rs) | `HazardRuntime`: the per-hazard runtime blob (id/name/pos/size, its `DamageVolume`, optional patrol `PathMotion`, and resolve `HitMode`) carried by LDtk-entity hazards. |
| [`hazards`](src/hazards.rs) | Hazard tick: patrol motion, contact damage, and the impact SFX/VFX published to the presentation/audio buses. |
| [`held_items`](src/held_items.rs) | ECS-owned held item capability for actors. |
| [`hitbox`](src/hitbox/mod.rs) | Hitbox-entity lifecycle: spawn → overlap-check → despawn. |
| [`moveset`](src/moveset/mod.rs) | Data-driven move playback — the runtime half of the Smash model. |
| [`on_hit`](src/on_hit.rs) | On-hit techniques — the conditional-hit primitive of the ability model. |
| [`path_motion`](src/path_motion.rs) | `PathMotion`: waypoint-following used by moving hazards/platforms. |
| [`slots`](src/slots.rs) | Anti-clump attack-slot arbitration. |
| [`targeting`](src/targeting.rs) | Per-frame `ActorTarget` selection for non-player actors. |
| [`util`](src/util.rs) | Grab-bag of small feature-side helpers — not a cohesive subsystem. |
| [`variation`](src/variation.rs) | Stable per-actor variation helpers for ECS feature actors. |

_17 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
