# NPC / Enemy → One Actor: Unification Execution Brief

_Status: ✅ COMPLETE 2026-06-22 (Opus 4.8) • Owner: Jon Crall_

## ✅ Completion note (Opus 4.8, 2026-06-22)

All 7 phases executed and committed to `main`. **There is no longer a notion of
an NPC vs an enemy** — every actor is one ECS cluster driven by a `Brain`, and
"enemy" is the runtime `ActorDisposition::Hostile` state, not a class.

What landed (one commit per phase):
1. `ActorInteraction` — dialogue is a shared actor capability, not an NPC trait.
2. Provoke accumulator → `ActorAggression.strikes`; hostility → `ActorDisposition`.
3+4. **The cluster merge.** `npc_clusters.rs` deleted; NPCs spawn through the
   unified `ActorClusterSeed::new_peaceful_npc`; one `update_ecs_actors` ticks
   every actor; one damage/reset/save/stimulus path each. Provoke is **in place**
   (`provoke_actor_in_place`) — no entity churn, keeps sprite (the balloon bug
   class is structurally gone).
5. `ActorRuntime` enum **deleted**; `FeatureVisualKind` + all gates derive from
   `ActorDisposition`.
6. Relational targeting seam: `FactionRelations` resource + a non-player-centric
   `select_actor_targets` (an actor hunts whoever its faction is hostile to).
7. Cluster types renamed `Enemy*` → `Actor*` (`ActorConfig`/`ActorStatus`/
   `ActorMut`/`ActorClusterSeed`/`ActorTuning`/`actor_clusters.rs`).

Headless gate met: `cargo build -p ambition_app` + `cargo test -p
ambition_gameplay_core` (947) + `-p ambition_render` (26) + the
`architecture_boundaries` guard (30) all green.

**What Jon must verify in-game** (cannot be checked headless — see §6):
- Peaceful NPC still patrols / stands / **flies (parrot)** / **talks** (dialogue
  opens; idle barks fire).
- Striking an NPC past the threshold flips it hostile **in place** (same sprite,
  no balloon, no teleport) and it then attacks; hit/hostile barks fire.
- Enemies still aggro / chase / attack as before; pirates keep their gun-sword.
- Save/reload: a provoked NPC stays hostile.
- **Known behavior change to confirm acceptable:** a same-room *reset* no longer
  reverts a provoked NPC to peaceful (it respawns at its spawn but stays hostile —
  the in-place peaceful-revert is a noted follow-up, see Phase 3+4).
- Boss path untouched (bosses are their own cluster — out of scope).

Follow-ups noted inline: in-place peaceful-revert-on-reset; renaming the still-
"enemy"-named spawn bundles + `ecs_enemy_*` render helpers (out of Phase 7 scope);
making player-targeting itself relational so a future stealth system can fully
hide the player.

---

_Original brief (Authored 2026-06-22, Opus 4.8) follows._

This is a **self-contained execution brief**: a fresh agent should be able to run
it end-to-end in one focused session against the codebase, with no other context.
Read it fully before touching code. The gate for every commit is **it compiles +
`cargo test` is green**; runtime behavior (combat, dialogue, patrols, stealth)
**cannot be verified headless** — Jon verifies that after, so commit compiling
checkpoints and KEEP MOVING (do not pause for mid-way verification).

---

## 1. The vision (Jon's words, paraphrased — this is the spec)

> There should be **no notion of an NPC**. Everyone is just an **actor controlled
> by a brain**. Picture Skyrim: you sneak into a room, the actors go about their
> business and are aggressive toward whoever they're *normally* aggressive toward.
> When you step out of the shadows they **decide whether to be aggressive toward
> you** — maybe you have a bounty, maybe you killed their family, maybe you're a
> nice guy. **The reason doesn't matter; the architecture must support it.** NPCs
> and enemies are **equally capable of combat, dialogue, and everything else**.
> The *only* thing that makes an enemy an enemy is that **it wants to kill you**.
>
> **Big future win to set up for:** the brain may later be driven by an **AI
> agent**. Plan for that — it's the payoff that makes the world interesting.

Three hard requirements fall out of this:

1. **No actor "type".** Delete the `NPC` vs `Enemy` distinction. One actor,
   parameterized by data (brain + disposition + capabilities + optional dialogue).
2. **Hostility is relational, not player-centric.** An actor can be hostile to
   *any* target (another actor, a faction), and its hostility toward the *player*
   is a runtime decision, not a spawn-time type. "Enemy" = "currently wants to
   kill the player," a state, not a class.
3. **The brain is the one behavior interface.** Everything an actor does flows
   through its `Brain` (snapshot → `ActorControlFrame`). Keep that boundary clean
   so a future `Brain::Agent` (LLM/agent-driven) slots in as just another brain.

### What "archetype" vs "catalog" means (the thing we're killing)

Today an actor's brain is built two different ways depending on which authoring
door it came through — **this duplication is the smell, and unifying it is the
core of this work:**

- **Archetype path (enemies):** an `EnemyBrain` enum value →
  `spec_for_brain()` → an `EnemyArchetypeSpec` (read from
  `enemy_archetypes.ron`) carrying tuning + brain-construction inputs +
  combat capabilities. Used by LDtk `EnemySpawn`s.
- **Catalog path (NPCs):** a `character_id` string → the character catalog →
  `default_brain_for_character_id()` builds a `Brain` directly; tuning is
  implicit/peaceful. Used by LDtk `NpcSpawn`s.

**Both produce the same runtime trio: a `Brain` component (behavior), an
`ActionSet` (capabilities), and tuning.** They are two data sources for one
concept. The end state: **one actor spawn path** that takes a unified
"actor spec" (whatever authoring door it came from) and produces one cluster.
The two DATA TABLES (`enemy_archetypes.ron` + character catalog) can keep
existing as sources, but they must feed **one resolver → one cluster → one tick**.

---

## 2. Target architecture (end state)

**One actor.** Every combat/dialogue/world participant (was-NPC, was-enemy,
future allies, summons) is the same entity shape:

| Concern | Component (mostly already exist) |
|---|---|
| Identity | `ActorIdentity { id, name }` |
| Body | `BodyKinematics`, `ActorSurfaceState`, `ActorMotionPath` |
| Behavior | `Brain` (the one authority) + `ActorControl` (its output frame) |
| Capabilities | `ActionSet` + `CombatKit` + `CombatCapabilities` |
| Tuning | `ActorTuning` (rename of `EnemyTuning`) |
| Liveness/combat state | `ActorStatus` (rename of `EnemyStatus`) + `ActorHealth`/`ActorCombatState`/`ActorIntent`/`ActorCooldowns` read-models |
| Config | `ActorConfig` (rename/merge of `EnemyConfig`; carries id/name/spawn/tuning/reconstruction info) |
| **Relations** | `ActorFaction` + `ActorDisposition` + `ActorAggression { mode, target }` + `ActorTarget` (ALL already exist — this is the Skyrim seam) |
| Combat attack state | `ActorAttackState` |
| **Dialogue (optional)** | `ActorInteraction { interactable }` (NEW shared component; only talkable actors have it) |
| **Sprite render size (optional)** | `ActorRenderSize` (DONE — already shared) |

**Deleted:** `ActorRuntime` enum, `NpcConfig`, `NpcStatus`, `NpcMut`,
`NpcClusterScratch`, `NpcClusterQueryData`, `update_ecs_npcs`,
`apply_npc_stimuli`, `reset_ecs_npc_actors`, `sync_ecs_npc_actors_with_save`,
`HostileNpcConversionPlan` / `make_entity_enemy` / `enemy_cluster_for_hostile_npc`
(the cluster-swap migration), `npc_component_snapshot`.

**One tick.** `update_ecs_actors` ticks every actor. Peaceful actors simply don't
attack — already gated by `tuning` (`attacks_player`, `body_contact_damage`) and
disposition; the slot-board / crowding / body-contact passes filter on
`disposition == Hostile` (or `aggression`), not on a type tag. The brain drives
patrol/idle/chase uniformly (the integration spine is ALREADY shared — see the
"Non-player-centric run FULLY DONE" memory). The two ticks exist today *only*
because two QueryDatas both `&mut BodyKinematics`; one cluster ⇒ one query ⇒ the
conflict is gone by construction.

**Hostility is a state transition, not a respawn.** "Provoke" (player crosses a
threshold, or a relationship flips) = set `ActorDisposition::Hostile`, set
`ActorAggression.target`, and **swap the `Brain` + `ActionSet` components in
place**. No cluster swap, no entity churn. This is what fixes the whole class of
"NPC turns into a different thing" bugs (e.g. the render-size balloon already
fixed by moving render size to a shared component).

**Relational targeting (the Skyrim behavior).** `select_actor_targets` currently
picks "nearest `ActorFaction::Player`." Generalize it: an actor targets the
nearest entity its **faction relations** mark hostile, with per-actor overrides
(`ActorAggression`). This is what makes "aggressive to who they're normally
aggressive toward" + "decides about *you* when you reveal yourself" work without
player-centrism. (Full faction-relations is Phase 6 / forward-looking — the
collapse Phases 1–5 must not regress today's "hostile actors chase the player.")

**Agent-pluggable brain (future-proofing, do not break it).** Keep `Brain`'s
contract exactly `tick(&BrainSnapshot, &mut ActorControlFrame)` — snapshot in,
control frame out, no ECS/world access inside. A future `Brain::Agent` (an
LLM/agent policy) becomes another variant/impl producing the same control frame.
**Do not** let the unification leak ECS queries or world state into the brain
boundary; the whole point is that any brain (scripted state machine OR agent)
plugs into the same actor.

---

## 3. Current state & blast radius (file map)

All paths under `crates/` unless noted. This is the exhaustive map; the executor
should re-grep to catch drift but should not need to rediscover structure.

### The split is expressed by:
- `ActorRuntime { Npc, Enemy }` — `ambition_gameplay_core/src/features/ecs/actors/mod.rs:69` (has `.disposition()`).
- NPC cluster — `ambition_gameplay_core/src/features/ecs/npc_clusters.rs`
  (`NpcConfig`, `NpcStatus`, `NpcMut`, `NpcClusterScratch`, `NpcClusterQueryData`).
- Enemy cluster — `ambition_gameplay_core/src/features/ecs/enemy_clusters.rs`
  (`EnemyConfig`, `EnemyStatus`, `EnemyMut`, `EnemyClusterSeed`,
  `EnemyClusterQueryData` — the richer superset; **make THIS the unified cluster**).
- NPC behaviors — `ambition_gameplay_core/src/features/npcs.rs` (`NpcMut` impl:
  `tick_via_brain`, `integrate_velocity`/`_aerial`, `build_brain`, barks,
  dialogue helpers). The integration is already the shared spine; only
  `build_brain` (catalog) is NPC-specific.
- Two ticks — `ambition_gameplay_core/src/features/ecs/actors/update.rs`:
  `update_ecs_actors` (enemy/hostile, ~49–484) vs `update_ecs_npcs` (~495–618);
  plus `tick_npc_idle_barks` (~828) and `sync_actor_poses_from_feature_aabbs`.
- Conversion — `ambition_gameplay_core/src/features/ecs/actors/conversion.rs`
  (`HostileNpcConversionPlan`, `make_entity_enemy`, `enemy_cluster_for_hostile_npc`,
  `hostile_enemy_brain_for_npc`, `npc_component_snapshot`).
- Aggression trigger — `ambition_gameplay_core/src/features/ecs/aggression.rs`
  (`apply_npc_stimuli` reads `npc.status.strikes` vs threshold → conversion;
  `apply_actor_stimuli` for enemies).
- Spawn — `ambition_gameplay_core/src/features/ecs/spawn_actors.rs`
  (`spawn_interactable` routes `InteractionKind::Npc` → `NpcActorSpawnPlan::peaceful`;
  enemy spawn via `EnemyClusterSeed`).
- Reset — `ambition_gameplay_core/src/features/ecs/reset.rs`
  (`reset_ecs_npc_actors` separate from enemy reset, again only due to the borrow split).
- Save — `ambition_gameplay_core/src/features/ecs/save_sync.rs`
  (`sync_ecs_npc_actors_with_save` separate from `sync_ecs_actors_with_save`;
  provoked-NPC load rebuilds via conversion).
- Read-model build — `ambition_gameplay_core/src/features/ecs/view_index.rs:175–247`
  (matches `ActorRuntime` → `FeatureVisualKind::{Npc,Enemy,TrainingDummy}` + flash source).
- Anim/name lookups — `ambition_gameplay_core/src/features/ecs/anim_helpers.rs`
  (`ecs_npc_name`, `ecs_npc_anim_state` vs `ecs_enemy_name`/`ecs_enemy_anim_state`;
  `ActorSpriteData` tuple has both `Option<&NpcConfig>` and enemy fields).
- Interact — `ambition_gameplay_core/src/features/ecs/interact.rs:66`
  (`matches!(actor, ActorRuntime::Npc)` gate for dialogue).
- Damage — `ambition_gameplay_core/src/features/ecs/damage/{mod,actor_hit}.rs`
  (separate NPC vs enemy hit handlers; NPC path increments `strikes`).
- Mount — `ambition_gameplay_core/src/features/ecs/mount/mod.rs` (enemy-only mount/rider gates).
- Target volumes — `ambition_gameplay_core/src/features/ecs/target_volumes.rs:31` (NPC vs enemy branch).
- Brain effects — `ambition_gameplay_core/src/features/ecs/brain_effects.rs` (skip-if-NPC gates for melee/ranged).
- Render — `ambition_render/src/rendering/`: `actors/mod.rs` (`upgrade_npc_sprites` /
  `upgrade_enemy_sprites`), `features.rs`, `primitives.rs` (Npc=blue/z2, Enemy=red/z1),
  `world.rs:645–664` (dialogue UI only for `::Npc`), `hit_flash.rs`, `deep_dream.rs`,
  `pirate_weapon.rs`.
- Player affordances — `ambition_player/src/affordances/interactable_proximity.rs:67`
  (talk highlight only for `::Npc`).
- Asset resolvers — `ambition_assets/resolvers.rs` (`InteractionKind::Npc` → `NpcTerminal` sprite).
- Debug overlay — `ambition_app/src/dev/debug_overlay/gizmos.rs` (npc/enemy colors + labels — already label-aware from recent work).
- Content — `ambition_content/src/bosses/cut_rope/victory.rs` (post-boss NPC spawn).
- LDtk → `InteractionKind::Npc` — `ambition_world/ldtk_world/conversion/entity_converters.rs:149`.
- Tests constructing `ActorRuntime::Npc/Enemy`, `NpcConfig`, `NpcStatus` literally:
  `features/ecs/tests.rs`, `actors/tests.rs`, `aggression.rs` (test mod),
  `damage/tests.rs`, `mount/tests.rs`, `conversion`/`conversion_tests`.

### `FeatureVisualKind` is derived from `ActorRuntime` (view_index.rs)
After the collapse it must be derived from **disposition/faction** (e.g.
`Hostile` or hostile-faction → `Enemy` kind; talkable/peaceful → `Npc` kind;
sandbag tuning → `TrainingDummy`). Rendering keeps the `Npc/Enemy` *visual* kinds
(z-order, dialogue bubble, sprite path) — they just stop being a type and become
a function of state.

---

## 4. Execution plan (phased; each phase compiles + ships green)

Work the phases in order. After EACH phase: `cargo build -p ambition_app` +
`cargo test -p ambition_gameplay_core` green, then commit (`Co-Authored-By` the
executing model). The cluster merge (Phase 3) is the atomic one; Phases 1–2 are
prep that compiles independently.

> **Naming:** make `EnemyConfig`/`EnemyStatus`/`EnemyMut`/`EnemyClusterSeed`/
> `EnemyClusterQueryData`/`EnemyTuning` the unified types and **rename** them to
> `Actor*` as the final phase (Phase 7) once nothing NPC-specific remains. Until
> then, leaving the `Enemy*` names is fine — keep moving, rename last.

### Phase 0 — DONE (baseline)
- `ActorRenderSize` shared component (sprite render size survives disposition flip). ✅
- This brief. ✅

### Phase 1 — Extract NPC dialogue into a shared `ActorInteraction` component ✅ DONE (Opus 4.8)
_Component added; dialogue/bark helpers decoupled from `NpcConfig` (take interactable+name+id+status pieces); interact + proximity-highlight gate off `ActorInteraction` presence + `ActorDisposition::Peaceful`; spawn (lib + cut_rope victory) inserts it. Build + 949 tests green._
The NPC's only truly-unique DATA is its dialogue/interaction + patrol/talk radii.
- Add `ActorInteraction { interactable: crate::interaction::Interactable, talk_radius: f32 }`
  in `combat/components/actors.rs` (re-export via `features::`).
- Move `npcs.rs` dialogue/bark helpers to take `&ActorInteraction` (or the
  `Interactable`) instead of `&NpcConfig`. They already read only
  `interactable.kind` + `name` + status — thread those explicitly.
- The interact system (`interact.rs`), proximity highlight
  (`interactable_proximity.rs`), dialogue UI (`world.rs`) read `ActorInteraction`
  presence instead of `ActorRuntime::Npc`.
- NPC spawn inserts `ActorInteraction`. **Keep `NpcConfig` for now** (still holds
  id/name/spawn/patrol) — this phase only lifts the dialogue data out so an
  enemy *could* also be talkable. Compiles green.

### Phase 2 — Collapse status + provoke onto shared components ✅ DONE (Opus 4.8)
_Provoke accumulator `strikes` moved to `ActorAggression.strikes`; `hostile` dropped from `NpcStatus` (it was a mirror of the already-synced `ActorDisposition`). `NpcStatus` is now just `{ ai_mode, hit_flash }`. Readers updated: aggression threshold, damage increment/threshold/bark (via `NpcHitTarget.aggression`), idle-barks (disposition gate), reset (clears `aggression.strikes`/`target`). **Deviation from brief:** NPCs do NOT yet adopt `EnemyStatus` here — doing so while the damage path is still split would create a write-write `EnemyStatus` conflict in `apply_feature_hit_events` (it queries `Option<EnemyClusterQueryData>`). NPCs adopt `EnemyStatus` in Phase 3 where the damage path also collapses to one cluster. Build + 949 tests green._
- Give NPCs an `EnemyStatus` (alive=true, health=`Health::new(1)`,
  respawn_timer=0, plus ai_mode/hit_flash) instead of `NpcStatus`.
- Move the provoke counter (`NpcStatus.strikes`) into `ActorAggression` (add
  `strikes: i32` there — it already models `RetaliatesWhenHit { strike_threshold }`,
  so the accumulator belongs with it). `hostile` becomes `ActorDisposition::Hostile`
  (already a component). Delete `NpcStatus`.
- Update readers: `aggression.rs` (threshold check), `damage/actor_hit.rs`
  (increment strikes), `npcs.rs` barks (read strikes from aggression),
  `view_index.rs`/`anim_helpers.rs` (hit_flash now from `EnemyStatus`), reset, save.
- Compiles green. (Two ticks still exist — config not merged yet.)

### Phase 3 + 4 — Merge the cluster + in-place provoke ✅ DONE (Opus 4.8)
_Done together: once NPCs carry the unified enemy cluster, the old cluster-swap
provoke is impossible, so the in-place flip (Phase 4) lands with the merge.
**What changed:** NPCs spawn through `EnemyClusterSeed::new_peaceful_npc` (peaceful
tuning: `attacks_player=false`, zero aggro, `max_run_speed=NPC_PATROL_SPEED`,
`health=1`, aerial from catalog body-kind) + a catalog `Brain` (`npc_brain_from_catalog`)
+ peaceful `ActionSet` + `ActorInteraction` + `ActorRenderSize`. `npc_clusters.rs`
(`NpcConfig`/`NpcStatus`/`NpcMut`/`NpcClusterScratch`/`NpcClusterQueryData`) **deleted**.
`update_ecs_npcs` deleted — `update_ecs_actors` ticks every actor (peaceful no-op the
combat passes via tuning; brain drives patrol/idle/fly). `apply_npc_stimuli` folded into
`apply_actor_stimuli`; `reset_ecs_npc_actors` into `reset_ecs_room_features`;
`sync_ecs_npc_actors_with_save` into `sync_ecs_actors_with_save`. Provoke
(`provoke_actor_in_place`, in `actors/conversion.rs`) re-resolves the hostile
archetype, overwrites the cluster config in place, swaps `Brain`+`ActionSet`, flips
`ActorDisposition`/`ActorRuntime` — no entity churn, keeps sprite + `ActorRenderSize`.
Damage handler branches on `ActorDisposition` (peaceful→strikes/bark, hostile→damage).
`sync_actor_components_from_enemy`→`sync_actor_components_from_cluster` (disposition-aware,
no longer writes disposition). `EnemyMut::update` faces the brain frame regardless of
`attacks_player` (so peaceful patrollers face their walk direction). Render
(`hit_flash`/`features`/`deep_dream`) + app schedules + content victory NPC updated.
**Deferred:** same-room reset does NOT yet revert a provoked NPC to peaceful (old
behavior preserved: it resets to spawn but stays hostile) — the in-place revert is a
small follow-up. Build (`ambition_app`) + 945 gameplay_core + 26 render + 30
architecture_boundaries tests green._

### Phase 3 — Merge the cluster (the atomic one)
- Make `NpcConfig`'s remaining fields part of `EnemyConfig` OR a tiny companion:
  - `id`, `name`, `spawn` → already on `EnemyConfig` (`ActorSpawnState`).
  - `patrol_radius`, `talk_radius`, `aerial` → fold into `ActorInteraction`
    (talk_radius already there) + the brain (patrol bounds already encode radius)
    + `surface.gravity_scale` (aerial already drives this). Net: these stop being
    config fields.
- **Spawn NPCs through `EnemyClusterSeed`** with a **peaceful actor spec**:
  - Build a peaceful `EnemyArchetypeSpec`-equivalent (attacks_player=false,
    body_contact_damage=false, low/zero aggro, `max_run_speed = NPC_PATROL_SPEED`,
    health=1, the right `is_aerial` from the catalog body kind).
  - Set the `Brain` component from the catalog (`build_brain` logic moves here —
    it produces a `Brain` directly; keep it). The `EnemyConfig.brain`/`brain_spec`
    reconstruction fields: for catalog actors, store enough to rebuild the
    peaceful brain (or mark "brain is authoritative, no archetype reconstruction"
    — see Phase 4). Attach `ActorInteraction` for talkable ones.
- Delete `NpcConfig`, `NpcMut`, `NpcClusterScratch`, `NpcClusterQueryData`,
  `npc_clusters.rs`. Move the still-needed `NpcMut` integration (`integrate_*`,
  `tick_via_brain`) — but note `EnemyMut::update` already integrates via the
  shared spine, so most of `npcs.rs` integration is redundant; keep only what the
  peaceful path needs (it should reduce to "tick brain → `EnemyMut::update`").
- Delete `update_ecs_npcs`; `update_ecs_actors` now ticks all actors. Peaceful
  actors: the slot-board/body-contact passes filter on hostility (disposition or
  `tuning.attacks_player`), so they no-op; the brain drives patrol/idle. Verify
  the enemy tick's brain path handles StandStill/Patrol/Aerial peaceful brains
  (it already runs arbitrary `Brain`s).
- Delete `apply_npc_stimuli`, `reset_ecs_npc_actors`,
  `sync_ecs_npc_actors_with_save` — fold into the enemy equivalents (now one
  query). Update their schedule registrations.
- This is the big compile-error chase. Drive it to green.

### Phase 4 — Replace the migration with an in-place provoke
- Delete `HostileNpcConversionPlan` / `make_entity_enemy` /
  `enemy_cluster_for_hostile_npc`. Provoke (in `apply_actor_stimuli`, the merged
  stimulus handler) = on threshold/relationship flip: set
  `ActorDisposition::Hostile`, set `ActorAggression.target = attacker`, swap the
  `Brain` + `ActionSet` to the hostile variant (reuse
  `hostile_enemy_brain_for_npc`'s mapping logic, now "pick a combat brain for this
  actor"), bump `tuning.attacks_player = true`. No component remove/insert of the
  cluster, no entity churn.
- The provoked actor keeps its sprite (`ActorRenderSize` + sprite override already
  shared) — the balloon bug class is structurally gone.

### Phase 5 — Delete `ActorRuntime`; derive visual kind from state ✅ DONE (Opus 4.8)
_The `ActorRuntime { Npc, Enemy }` enum is **deleted**. It was a pure mirror of
`ActorDisposition` (spawn/provoke always set them together), so every
`matches!(actor, ActorRuntime::Enemy)` became `disposition.is_hostile()` and
`::Npc` became `disposition.is_peaceful()` — a faithful 1:1 swap. `FeatureVisualKind`
is now a function of state in `view_index`: `is_sandbag → TrainingDummy`, else
`hostile → Enemy`, else `Npc` (a provoked NPC turns red automatically).
`provoke_actor_in_place` drops the runtime flip (just flips disposition) and now
also restores the hostile archetype's HP pool. Converted: update (Pass-1 slot
gate, pose-sync), aggression/save_sync (provoke), spawn, target_volumes,
brain_effects, mount, view_index, anim_helpers (`ActorSpriteData` drops the tag;
helpers read the cluster directly). Cross-crate: render (deep_dream / pirate_weapon
/ features), app debug overlay (color/label by disposition), content victory NPC.
Stale `ActorRuntime` comments cleaned up. Build + 945 + 26 + 30 tests green._
- Remove the `ActorRuntime` enum. Everywhere that matched it:
  - `view_index.rs`: `FeatureVisualKind` from disposition/faction/tuning
    (hostile or hostile-faction → `Enemy`; talkable/peaceful → `Npc`; sandbag →
    `TrainingDummy`).
  - render `features.rs`/`primitives.rs`/`world.rs`, debug overlay, player
    affordances, interact, anim_helpers, mount, target_volumes, brain_effects,
    damage: replace `matches!(actor, ActorRuntime::X)` with the disposition /
    faction / `ActorInteraction`-presence equivalent.
  - `anim_helpers.rs`: collapse `ecs_npc_name`/`ecs_npc_anim_state` +
    `ecs_enemy_*` into one `ecs_actor_*` reading the unified cluster +
    `ActorInteraction`.
- Update all tests that constructed `ActorRuntime::Npc/Enemy` / `NpcConfig` /
  `NpcStatus` literally.

### Phase 6 — Relational, non-player-centric targeting ✅ DONE (Opus 4.8)
_Added `FactionRelations` (a `Resource`: `hostile[from][to]` matrix +
`set_hostile`/`set_mutual_hostile`/`is_hostile`) — the seam stealth/bounty/grudge/
alliance systems write to. `select_actor_targets` is now relational: a non-passive
actor's candidate pool = the player baseline (so hostile enemies + retaliating NPCs
keep chasing/facing the player — nothing regresses) PLUS any non-player actor its
faction is relationally hostile to; nearest wins. Default matrix is all-false, so
behavior is byte-identical today; flipping a relation makes an actor hunt another
faction's actors with no player involved (verified by two new tests). Registered
via `WorldPrepSchedulePlugin`. **Scope note (per brief):** this is the seam only —
the stealth/bounty systems that WRITE to it are future work, and the player
baseline is still carried by `ActorAggression` (a future richer pass can make
player-targeting itself relational so stealth can fully hide you). Build + 947
gameplay_core tests green._

### Phase 6 — Relational, non-player-centric targeting (the Skyrim payoff)
This is the part Jon most cares about conceptually; it's the smallest *code* but
the biggest *capability*. Can be its own session if Phase 5 ran long.
- Add a `FactionRelations` model: per-faction stance (Hostile/Neutral/Friendly)
  toward other factions, + the `ActorAggression.target`/mode per-actor override.
  Seed it so today's behavior is preserved (hostile-faction actors target the
  player; peaceful don't).
- Generalize `select_actor_targets`: an actor's target = nearest entity whose
  relationship to it is Hostile (could be another actor, not just the player).
  Keep "player is the usual target" as the default relation so nothing regresses.
- This is where "they're aggressive to who they're normally aggressive toward,
  and decide about you when you reveal yourself" lives: stealth/reveal flips the
  player's relation to a faction; a bounty/grudge is a per-actor or per-faction
  relation override. **Don't build the stealth/bounty systems here** — just make
  targeting *relational* so those systems have a seam to write to.

### Phase 7 — Rename `Enemy*` → `Actor*` ✅ DONE (Opus 4.8)
_Mechanical rename of the unified cluster types now that nothing NPC-specific
remains: `EnemyConfig`→`ActorConfig`, `EnemyStatus`→`ActorStatus`,
`EnemyMut`→`ActorMut`, `EnemyClusterSeed`→`ActorClusterSeed`,
`EnemyClusterQueryData(Item)`→`ActorClusterQueryData(Item)`,
`EnemyTuning`→`ActorTuning`, `as_enemy_mut`→`as_actor_mut`,
`enemy_clusters.rs`→`actor_clusters.rs`. Precise word-disjoint replacement across
39 files; the archetype/roster DATA names stay (`EnemyArchetypeSpec`,
`enemy_archetypes.ron`, `EnemyBrain`, `EnemyRoster`, `EnemyRespawnPolicy`,
`EnemyBrainSpec/Template`). Deliberately left for a follow-up (out of brief
scope): `EnemyActorBundle`/`EnemyActorSpawnPlan`, `enemy_component_snapshot`,
`ecs_enemy_*` render helpers — all still "enemy"-named but used by the unified
path. Build + 947 + 26 + 30 tests green._
Pure mechanical rename once nothing NPC-specific remains: `EnemyConfig`→`ActorConfig`,
`EnemyStatus`→`ActorStatus`, `EnemyMut`→`ActorMut`, `EnemyClusterSeed`→`ActorClusterSeed`,
`EnemyTuning`→`ActorTuning`, `EnemyClusterQueryData`→`ActorClusterQueryData`,
`enemy_clusters.rs`→`actor_clusters.rs`, `update_ecs_actors` stays. Keep
`EnemyArchetypeSpec`/`enemy_archetypes.ron` as a DATA source name if desired, or
rename to `actor_archetypes` — author's call. Update the
`architecture_boundaries` guard tests if any name them.

---

## 5. Brain unification (kill archetype-vs-catalog)

The end state: **one resolver, `Brain` component authoritative.**

- The per-tick authority is ALREADY the `Brain` component (`brain.tick(snapshot,
  &mut frame)`), built at spawn from either door. Keep it. **Do not** route
  behavior through the config's archetype/catalog fields at tick time — they're
  spawn/reconstruction-only.
- Spawn: one `spawn_actor(seed)` builds the cluster + `Brain` + `ActionSet` +
  tuning. The two authoring doors (LDtk `EnemySpawn` via archetype data, LDtk
  `NpcSpawn` via catalog data) each produce a `seed`; the catalog and
  `enemy_archetypes.ron` are just two tables feeding it. A later content pass MAY
  merge them into one "actor table," but that is NOT required for this refactor —
  unifying the RUNTIME (one cluster/tick/provoke) is.
- Reconstruction (provoke/dismount) needs "what combat brain should this actor
  use when it turns hostile." Today `hostile_enemy_brain_for_npc` infers it from
  id/name/dialogue; enemies carry `EnemyBrain` for it. Unify as a single
  `fn hostile_brain_for(actor) -> (Brain, ActionSet)` that works for any actor
  (it already mostly does — it's string-matching the id/name).

**Agent brain hook (do not break):** keep `BrainSnapshot` as the *only* input and
`ActorControlFrame` as the *only* output of `Brain::tick`. A future
`Brain::Agent { policy }` produces the same frame from the same snapshot. The
unification must not add ECS/world access inside the brain boundary — if Phase 3
is tempted to reach into the world from the brain to make peaceful actors work,
that's a smell: put it in the tick (the EnemyMut/ActorMut update), not the brain.

---

## 6. Verification (Jon runs after; agent cannot)

Headless gate (agent must pass before declaring done):
- `cargo build -p ambition_app` green; `cargo test -p ambition_gameplay_core`
  green; `cargo test -p ambition_render` green.
- Keep/extend tests: the aggression-threshold test
  (`npc_flips_hostile_once_strikes_reach_the_threshold`) becomes a disposition-flip
  test; add a test that a provoked actor keeps its entity id + sprite (no churn);
  add a relational-target test (Phase 6).

Jon's in-game checklist (the things that break silently):
- A peaceful NPC still patrols / stands / flies (parrot) / talks (dialogue opens).
- Striking an NPC past the threshold flips it hostile **in place** (same sprite,
  no balloon, no teleport), and it then attacks.
- Idle barks still fire (parrot cove); hit/hostile barks still fire.
- Enemies still aggro/chase/attack the player as before.
- Save/reload: a provoked NPC stays hostile; a reset room restores peaceful.
- Boss path untouched (bosses are their own cluster — out of scope here).

---

## 7. Risks & gotchas

- **The cluster merge is atomic** — Phases 1–2 compile independently, but Phase 3
  produces no intermediate green state until the whole NPC cluster is gone. Budget
  for a long compile-error chase; don't commit a half-merged tree.
- **Peaceful actors through the enemy tick:** the enemy tick does slot-board,
  crowding, body-contact, hitbox spawn. Confirm each is gated on hostility/tuning
  so peaceful actors no-op (they mostly are: body-contact checks
  `tuning.body_contact_damage`; slot requests filter alive hostiles). The brain
  drives movement; `EnemyMut::update` already runs arbitrary brains.
- **Patrol motion path:** NPC patrol uses `ActorMotionPath` + a Patrol brain;
  enemies use the same components. Ensure the peaceful Patrol brain + motion path
  survive the merge (don't drop `ActorMotionPath` from the unified seed).
- **Aerial/floating (parrot):** `surface.gravity_scale <= 0` selects the floating
  integrator. Preserve that — set `gravity_scale` from the catalog body kind at
  spawn (already done in `npc_clusters::new_with_paths`).
- **`FeatureVisualKind` mapping** must keep dialogue bubbles + talk highlights for
  peaceful/talkable actors (key off `ActorInteraction` presence, not a type).
- **Save format:** `sync_ecs_*_with_save` — keep the persisted flag keys stable
  (`npc_<id>_hostile`) so existing saves still resolve, or migrate deliberately.
- **`architecture_boundaries.rs` guard tests** may assert module/type names —
  update them in the rename phase.
- **Replay/behavior may change** — that's accepted (Jon verifies). The only hard
  gate is compiles + tests green.

---

## 8. Commit discipline

- One commit per phase (or per green sub-step within Phase 3). Message: what +
  why; sign `Co-Authored-By: <executing model> <noreply@anthropic.com>`.
- Never `git add -A` (the tree carries dev junk + a concurrent agent works in
  `tools/ambition_music_renderer/`); stage explicit paths.
- No backticks in `git commit -m` strings (they trigger shell substitution).
- Update this doc's phase checkboxes as you land each phase, and write a short
  completion note (what changed, what Jon must verify) at the top when done.

---

## 9. Why this is the right shape (for the reviewer)

The collapse isn't just de-duplication — it's the precondition for the game Jon
wants: a world of actors whose hostility is **relational and dynamic** (stealth,
bounty, grudges, alliances) and whose brains can later be **agent-driven**. As
long as "enemy" is a spawn-time *type*, none of that is expressible. After this,
"enemy" is just "an actor currently hostile to you," every actor can fight, talk,
and reason, and dropping in an AI-agent brain is one new `Brain` variant — not a
new entity kind.
