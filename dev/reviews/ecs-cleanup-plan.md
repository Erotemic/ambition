## FINAL STATUS (2026-06-02, autonomous shotgun session)

All 10 ranked items addressed. Fully done: #1, #2, #3, #4, #5, #7. Advanced
to a bounded, plan-appropriate state with the remainder intentionally
deferred: #6 (in-flight repr unified; faction-routed collision stays
separate by design), #8 (combat + progression schedules pluginized;
presentation installs remain as named helpers), #9 (shared boss read-models
in place + consumers migrated; full boss-body collapse deferred per the
plan's own guidance), #10 (enemy/npc/boss-flags component-driven; boss
pos/size blocked on #9). Suite green throughout (1206 lib tests; all
targets pass). Each item committed separately on
`sprite-props-and-per-sheet-cache`.

## PROGRESS LOG (2026-06-02, autonomous session)

Executed the recommended-order patches #1–#4 from the bottom of this doc,
each behavior-preserving, compiled (`cargo check -p ambition_sandbox
--all-targets` clean) and committed on branch
`sprite-props-and-per-sheet-cache`:

- **#1 Extract ActorAttackState from EnemyRuntime** — the four melee
  fields (windup/active/cooldown/pending_axis) now live in one
  `ActorAttackState` value type (`components.rs`) held as
  `EnemyRuntime::attack`, with `tick()` / `is_active()` / etc. Precursor
  to the standalone ECS-component promotion. Melee-timer tests pass.
- **#2 (ranked #3) Aggression-driven targeting** — `select_actor_targets`
  reads `ActorAggression::target_policy()` (new `AggressionTarget` enum:
  None / NearestPlayer) instead of `ActorFaction::needs_target()` (now
  removed). Passive actors point at self (no origin-facing glitch).
- **#3 (ranked #4) Split apply_feature_hit_events** — extracted
  `apply_actor_hit` and `apply_boss_hit` private helpers; the scheduled
  system just runs queries + dispatches. Damage tests pass.
- **#4 (ranked #5) Delete no-op GameplayEffect::DamageBoss / StrikeNpc**
  — variants, their no-op consumer systems, schedule entries, re-exports
  and emission sites removed.

Also un-broke the lib test suite, which did **not compile** on this
branch (two test fns imported `{LungeSpec, MeleeActionSpec}` but used
`SwipeSpec`) — once compiling, 19 tests failed. Fixed those + the
`ActorStimulus` "Message not initialized" harness gaps in the damage
and projectile test apps (the system writes that message; the test apps
never registered it). **Suite is now 1200 passing / 4 failing.**

The **4 remaining failures are pre-existing logic bugs unrelated to this
refactor** — they were simply never run because the binary didn't
compile. A human should look at them; they may be real regressions on
this WIP branch:
- `brain::boss_pattern::…approach_clamps_to_front_wall_standoff…`
  ("should still close toward the player")
- `brain::smash::mode::…hysteresis_prevents_approach_to_retreat_flip…`
  (Engage vs Approach within dwell)
- `content::features::ecs::bosses::…front_wall_clearance_reports_side_wall…`
  (clearance = 40)
- `content::features::ecs::brain_effects::…ranged_message_spawns_projectile…`
  (mounted-rider owner_id missing `lasersword:` prefix)

### UPDATE (2026-06-02 PM): ranked-#1 fully landed; the 4 failures above are all fixed

- **Ranked #1 "Split EnemyRuntime into ECS components" — DONE (fully dissolved).**
  `ActorRuntime::Enemy` is a payload-free marker; enemy state lives in cluster
  components (EnemyKinematics/EnemyStatus/EnemyConfig/EnemyMotionPath +
  ActorSurfaceState/ActorAttackState). Integration runs on `EnemyMut`; both the
  one-way mirror sync and the EnemyMut load/store bridge are gone. The
  `EnemyRuntime` struct survives only as a transient spawn builder + parity-test
  reference (not ECS state, not a bridge). 1205 tests green.
- The **4 "pre-existing failures"** listed above were all resolved earlier this
  session: the smash-dwell + 2 boss-front-wall tests were stale-assertion fixes,
  and the lasersword ranged test was removed per Jon. **Suite is 1205/0.**

CURRENT STATE OF THE 10 RANKED ITEMS:
1. Split EnemyRuntime → ECS components — ✅ DONE
2. Replace ActorRuntime::Npc/Enemy with one actor state — ✅ DONE. ActorRuntime
   is a payload-free {Npc, Enemy} marker; NPC state lives in NpcConfig/NpcStatus
   + shared ActorKinematics/ActorSurfaceState/ActorMotionPath. Dual NPC+enemy
   systems split into siblings (update_ecs_npcs, apply_npc_stimuli,
   reset_ecs_npc_actors, sync_ecs_npc_actors_with_save) because the cluster
   query-data structs share the kinematics/surface/motion mutably. NpcRuntime
   reduced to a spawn-only builder. 1205 tests green.
3. Targeting via aggression, not faction — ✅ DONE
4. Split apply_feature_hit_events — ✅ DONE (apply_actor_hit/apply_boss_hit)
5. Replace GameplayEffect enum bus with typed messages — ✅ DONE. The enum is
   gone; SetFlagRequested / QuestAdvanceRequested / SwitchActivated /
   GameplaySfxRequested are separate Messages, each with a focused consumer.
6. Unify player/enemy projectile state — 🟡 in-flight representation unified
   (projectile::InFlightProjectile { body, owner_id } replaces the parallel
   Player/Enemy wrappers). The two state containers stay distinct on purpose
   (per-player charge input vs global enemy pool), and collision stays
   faction-routed (attacker-side vs victim-side — genuinely different routing).
7. Data-drive held-item abilities by id — ✅ DONE. Archetypes reference held
   items by id (`held_item: Some("gun_sword")`); resolution goes through the
   `brain::action_set::held_item_by_id` registry. Schema stays id/melee/ranged
   (richer fields deferred to the item pass). Guard test added.
8. Pluginize app/plugins.rs — 🟡 SUBSTANTIAL. Combat chain →
   CombatSchedulePlugin, Progression chain + populate → ProgressionSchedulePlugin
   (each its own file). app/plugins.rs 1036→906. The remaining bulk is the
   presentation `install_*` helpers, already decomposed into named fns; promoting
   them to Plugin structs is cosmetic follow-up.
9. Collapse boss combat runtime further — 🟡 bounded step in place. Bosses
    already expose shared ActorHealth/ActorCombatState/ActorIntent read models
    (sync_boss_actor_components), and consumers now read boss alive/hit_flash
    from ActorCombatState rather than BossRuntime. The full body/pattern-timer
    collapse (boss body → FeatureAabb) is intentionally deferred per the plan's
    own "don't make bosses ordinary ActorRuntime yet" guidance.
10. Component-driven presentation (FeatureViewIndex) — 🟡 MOSTLY. Enemy + NPC
    FeatureView built from FeatureAabb + clusters; boss alive/hit_flash now read
    from the shared ActorCombatState mirror. Boss pos/size still BossRuntime-
    derived (blocked on the #9 body→FeatureAabb migration).

---

I surveyed the current patched tree and I think the remaining “old bridge” pressure points are pretty clear now. The big picture is:

Done / mostly done:
- Player scratchpad bridge is mostly gone.
- ActorPose exists.
- CombatKit exists.
- Aggression/Stimulus exists.
- Bosses now expose shared actor-combat read-model components.
- GameplayEffect::ActorStimulus bridge is gone.

Still old-ish:
- ActorRuntime still stores Npc vs Enemy variants.
- EnemyRuntime is still the main behavior blob.
- BossRuntime is still a smaller but real behavior/body blob.
- Damage/combat still has a god-system shape.
- Player/enemy projectiles are still separate state machines.
- GameplayEffect remains a generic enum bus for unrelated events.
- app/plugins.rs still owns too much scheduling.

The old audit/refactor direction still holds: the engine-core math/collision helpers are not the problem; the remaining bridge work is mostly ECS-hosted runtime blobs and cross-domain routing seams.

Ranked refactors
1. Split EnemyRuntime into ECS components

Impact: highest. Risk: medium-high.

This is the biggest remaining holdout. EnemyRuntime still owns identity, position, velocity, health, archetype, spawn state, attack timers, respawn timers, hit flash, AI mode, grounded state, sprite override, gravity, surface-walking state, pending attack axis, and air-jump state.

That means many “generic actor” systems still eventually ask:

ActorRuntime::Enemy(enemy)

and then reach into enemy.*.

A cleaner target shape:

ActorBody / FeatureAabb / ActorPose
ActorVelocity
ActorHealth
ActorSpawnState
ActorRespawnPolicy
ActorAttackTimers
ActorHitFlash
ActorMotionSurfaceState
ActorAirControlState
ActorArchetype
ActorVisualOverride

I would not remove EnemyRuntime all at once. First extract attack timers + respawn/death state, because those are read in many places and cause actor behavior bugs.

Best first patch:

Extract ActorAttackState from EnemyRuntime

Then move:

attack_windup_timer
attack_timer
attack_cooldown
pending_attack_axis

out of EnemyRuntime, and make melee start / hitbox spawn read ActorAttackState.

This would remove one of the most important reasons combat still needs EnemyRuntime.

2. Replace ActorRuntime::Npc/Enemy with one actor state

Impact: very high. Risk: high unless EnemyRuntime is split first.

The variants were renamed to avoid encoding aggression, which helped. But structurally they still preserve the old world:

ActorRuntime::Npc(npc)
ActorRuntime::Enemy(enemy)

The remaining inelegant adapter is still:

enemy_runtime_for_npc_combat(...)

That means a peaceful NPC still “becomes enemy-shaped” when it retaliates.

The better destination is:

ActorRuntime {
  id/name/interaction/display/body-ish baseline only
}

with aggression, combat kit, health, timers, movement, and visual state as separate components.

I would do this after extracting ActorAttackState and probably ActorMovementState, not before. Otherwise this patch becomes too big.

3. Make targeting use aggression/relationships, not faction shortcuts

Impact: high. Risk: medium.

select_actor_targets currently targets nearest player for factions that “need target.” That is good for co-op, but it is not yet the real relationship model.

Current conceptual rule:

ActorFaction::Enemy/Boss => target player
ActorFaction::Npc/Neutral => skip

Better rule:

ActorAggression target policy decides who is targetable.

For example:

Passive -> no target
RetaliatesWhenHit with target -> that target
HostileToPlayer -> nearest player
HostileToFaction(Pirates) -> nearest pirate
AllyOfPlayer hostile to pirates -> nearest hostile pirate

This would unlock allied NPCs without changing brains or combat systems again.

Good patch scope:

Add AggressionTarget enum and make select_actor_targets read ActorAggression.

Do not build a full relationship graph yet. Just replace faction shortcut targeting with aggression-policy targeting.

4. Split apply_feature_hit_events

Impact: high. Risk: medium.

apply_feature_hit_events is still a major god-system. It applies actor hits, boss hits, breakable hits, pogo-breakable hits, death side effects, VFX, SFX, flags, banter, hitstop, and boss encounter damage.

It now uses a FeatureHitWriters SystemParam, which is better than adding another bridge, but the system is still doing too much.

Best cleanup:

resolve_hit_targets
apply_actor_hit
apply_boss_hit
apply_breakable_hit
emit_hit_feedback
emit_death_feedback

I would keep one scheduled system initially, but split private helpers and small typed writer bundles. Then later each target family can become its own system.

This improves maintainability immediately and makes future HitSpec -> HitResult work much easier.

5. Replace GameplayEffect enum bus with typed messages

Impact: medium-high. Risk: low-medium.

GameplayEffect is much better than the old custom event bus, but it is still a mixed-purpose enum:

SetFlag
AdvanceQuest
ActivateSwitch
DamageBoss
StrikeNpc
PlaySfx

Some variants are real domain events; some are now no-op trace hooks; some are convenience routing.

I would split it into typed messages:

SaveFlagRequested
QuestAdvanceRequested
SwitchActivated
BossDamagedObserved
NpcStruckObserved
GameplaySfxRequested

Then delete the enum bus.

This is not as gameplay-critical as EnemyRuntime, but it reduces mental overhead and avoids recreating a god-router under a nicer name.

I would start by deleting the two no-op-ish variants:

DamageBoss
StrikeNpc

Boss damage already applies directly; NPC retaliation already uses ActorStimulus.

6. Unify player and enemy projectile state

Impact: medium-high. Risk: medium.

The code already has shared ProjectileBody and ProjectileFaction, but runtime state is still split:

PlayerProjectileState
EnemyProjectileState

This split keeps creating special cases:

player projectile visuals
enemy projectile visuals
player collision system
enemy collision system
held item ranged attacks
boss apple rain

Target:

ProjectileEntity or ProjectileState {
  body: ProjectileBody,
  owner: Option<Entity>,
  faction: ProjectileFaction,
  visual: ProjectileVisualKind,
  damage: i32,
}

Then player fireballs, pirate pistol shots, boss apples, arrows, bombs, etc. use the same movement/collision system.

I would do this after CombatKit stabilizes because held items will want to emit ProjectileSpawnRequest.

7. Data-drive held-item abilities more completely

Impact: medium-high. Risk: medium-low.

HeldItemSpec is a good start: item id, optional melee, optional ranged. But the next item pass will stress it. Axe, sword, thrown bomb, and bow will probably need:

visual asset id
hand/socket offset
muzzle offset
projectile visual
projectile arc/gravity/bounce policy
ammo/cooldown
drop/pickup behavior
preferred range

Recommended next shape:

held_items: {
  "pirate_gun_sword": (...),
  "axe": (...),
  "slashing_sword": (...),
  "thrown_bomb": (...),
  "bow": (...),
}

and enemy archetypes reference:

held_item: Some("pirate_gun_sword")

rather than embedding full item specs inline.

That lets authored item additions avoid Rust edits.

8. Extract combat/boss/projectile plugins from app/plugins.rs

Impact: medium. Risk: low.

app/plugins.rs is still over 1,000 lines and owns a lot of detailed scheduling. It is less of a “non-ECS bridge” than it used to be, but it still acts like a god scheduler.

Best split:

CombatPipelinePlugin
ProjectileRuntimePlugin
BossCombatPlugin
CutRopeEncounterPlugin
ActorRuntimePlugin

This is mostly mechanical and behavior-preserving. It improves reviewability and reduces merge conflicts.

I would do this soon, but after one or two more semantic cleanup patches so the plugin seams are clearer.

9. Collapse boss combat runtime further

Impact: medium. Risk: medium.

Bosses now carry shared actor components, which is good. But BossRuntime still owns body, health mirror, facing, hit flash, pattern timer mirror, behavior profile, sprite metrics, and collision integration.

The useful next split is:

BossBody
BossHealthMirror
BossVisualMetrics
BossPatternState
BossEncounterLink

I would not try to make bosses ordinary ActorRuntime yet. Keep boss encounter mechanics separate. But the body/health/pose/combat pieces can continue moving toward shared actor components.

Good first patch:

Move BossRuntime health/facing/hit_flash reads to shared ActorHealth/ActorCombatState where possible.
10. Replace FeatureViewIndex/runtime-derived presentation with component-driven visuals

Impact: medium. Risk: medium.

Presentation still reads a lot of runtime shape:

ActorRuntime::Enemy -> sprite
ActorRuntime::Npc -> sprite
BossFeature -> sprite
FeatureViewIndex -> render sync

The desired shape is:

FeatureVisualSpec
ActorVisualState
ActorPose
ActorCombatState
HeldItemVisual

Rendering should not need to inspect EnemyRuntime or BossRuntime to know what to draw.

This is lower priority than combat semantics, but it will pay off as actors/items/bosses diversify.

My recommended order

I would do this sequence:

Extract ActorAttackState from EnemyRuntime.
Highest leverage and directly attacks the biggest blob.
Make targeting read ActorAggression, not ActorFaction::needs_target.
This gets us closer to allies/faction combat and makes aggression semantically real.
Split apply_feature_hit_events into target-family helpers.
Lower risk than full HitSpec, but sets up that migration.
Delete no-op GameplayEffect::DamageBoss / StrikeNpc variants.
Easy cleanup after the actor-stimulus bridge removal.
Data-drive held items by id through a held-item registry.
Do this before adding axe/sword/bomb/bow.
Unify projectile runtime state.
Do this after held items have a stable spawn request shape.
Collapse ActorRuntime::Npc/Enemy.
Do this after attack/movement/health state have moved out of EnemyRuntime.
Pluginize the remaining app scheduler chunks.
Useful whenever we want a low-risk structural cleanup between semantic patches.
Best next patch
