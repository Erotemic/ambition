# 0022: Respawn policy — dead stays dead by default; respawning is authored

## Status

Accepted; IMPLEMENTED (2026-07-06). Decided by Jon (2026-07-05 triage of the
infinite-respawn defect; Q29 triage line confirmed 2026-07-06).

## Context

Killed NPCs re-instantiated alive on every room re-entry. The spawn/despawn
machinery was sound; the defect was POLICY spread across three carriers:
a derived `EnemyRespawnPolicy` whose default (`OnRoomReenter`) made the kill
hook write no death flag, a separate `respawn_in_place_seconds` timer path
for sandbags, and a `save_sync` liveness read that a killed-but-unprovoked
peaceful NPC fell through entirely.

## Decision

1. **One authored enum, one carrier.** `RespawnPolicy { DeadStaysDead
   (default), OnRest, OnRoomReenter, InPlace(seconds) }` is authored per
   archetype row (`respawn:` in `character_archetypes.ron`), carried ONCE on
   `ActorTuning.respawn`, and matched by both death paths (the kill hook's
   flag write and the in-place revive tick). The old derived helper, the
   separate timer field, and the caps duplicates are deleted.
2. **The default is `DeadStaysDead`** — the intuitively-correct rule for a
   unique actor in a persistent world ("Morrowind rules": killing a
   questline-critical NPC is allowed; the narrative consequence is a
   per-GAME choice). The engine makes the correct default the easy one;
   respawning is an AUTHOR'S opt-in (`OnRoomReenter` = the Mob choice).
3. **Policy is ultimately a property of the PLACEMENT.** NPC placements are
   unique by construction: the peaceful spawn plan pins `DeadStaysDead`
   regardless of the mob-tier policy of the combat archetype the NPC borrows
   when provoked. (A future EnemySpawn LDtk field may override a single
   placement the same way.)
4. **Liveness on load is universal.** `sync_ecs_actors_with_save` applies
   the persisted death flag to EVERY persistent actor, provoked or not —
   the fall-through branch shape is gone, and the missing test now exists.

## Current implications for agents

Author `respawn:` on archetype rows: trash mobs `OnRoomReenter`, mini-boss
presences `OnRest`, training sandbags `InPlace(secs)`; leave unique/named
actors on the default. Never reintroduce a parallel respawn mechanism —
grow this enum (it is the seam a future wave/`Mob` system extends).
`encounter:*` ids keep their own state machine; boss defeat persists via
the `bosses` save vector, not these flags.
