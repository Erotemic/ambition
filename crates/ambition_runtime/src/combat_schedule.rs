//! Combat-phase schedule plugin.
//!
//! EFFECTS-stage brain-action consumers (enemy melee/ranged spawns, boss
//! special-attack spawns), projectile + hitbox + feature-hit resolution,
//! the cut-rope boss-arena tick, and mount/rider link bookkeeping all run
//! here in `SandboxSet::Combat`.
//!
//! Extracted from `app/plugins.rs` (ecs-cleanup-plan #8) so the top-level
//! simulation orchestration reads as a list of named domain plugins rather
//! than one monolithic scheduler.

use bevy::prelude::*;

use ambition_platformer_primitives::schedule::SimScheduleExt;
use ambition_platformer_primitives::schedule::{gameplay_allowed, CombatSet, SandboxSet};

/// Schedules the `SandboxSet::Combat` system chain.
pub struct CombatSchedulePlugin;

impl Plugin for CombatSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Open, content-owned projectile art registry. Init the empty catalog so
        // the projectile stepper's detonation-FX lookup always has a resource to
        // read; a game's content crate registers each named look into it. The
        // renderer inits it independently for its own art resolution.
        app.init_resource::<ambition_projectiles::ProjectileVisualCatalog>();
        // Open, content-owned motion-technique registry (qcf / hcf / …). Init the
        // empty catalog so the player fire system's gesture lookup always has a
        // resource to read; a game's content crate registers each named gesture.
        app.init_resource::<ambition_projectiles::MotionTechniqueCatalog>();
        // App-local bridge from combat to sprite metadata. Every strike resolves
        // against the same CharacterCatalog resource as spawning and rendering;
        // separate Apps may compose different provider sets safely.
        app.insert_resource(
            ambition_actors::combat::authored_volumes::AuthoredAttackVolumeResolver::new(
                ambition_actors::character_sprites::authored_attack_volume_resolver,
            ),
        );
        // The effect seam: techniques (the shockwave gauntlet, the boss
        // phase-transition slam, …) emit `EffectRequest`; `apply_effects` below
        // drains it. Registered here so the writers never hit an unregistered
        // message.
        app.add_message::<ambition_vfx::EffectRequest>();
        app.add_message::<ambition_actors::combat::moveset::MoveEventMessage>();
        // On-hit techniques (pogo, …): `dispatch_hitbox_on_hit` writes one per
        // landed on-hit volume; the engine `apply_pogo_bounce` + any content
        // technique read it.
        app.add_message::<ambition_actors::combat::on_hit::OnHitEffectMessage>();
        // Programmatic actor-spawn seam: scenario tests and RL/agent scene setup
        // emit `SpawnActorRequest`; `apply_spawn_actor_requests` materializes each
        // actor through the same `spawn_boss` / `spawn_enemy` paths room load uses.
        // Registered (and run) here next to the in-gameplay spawners, but
        // deliberately UNGATED so a scene-setup spawn applies in any `GameMode`.
        app.add_message::<ambition_actors::features::SpawnActorRequest>();
        app.add_systems(
            sim,
            ambition_actors::features::apply_spawn_actor_requests.in_set(SandboxSet::Combat),
        );
        app.add_systems(
            sim,
            (
                // Melee is ONE path: a `"attack"`-verb moveset move (triggered by
                // `trigger_moveset_moves`, advanced by `advance_move_playback`,
                // projected back to `BodyMelee` for the read-model). No flat player
                // or actor melee driver survives. What's left on `BodyMelee` is the
                // cooldown FLOORS — the ranged refire floor (`ranged_cooldown`, I3)
                // and the legacy melee-recovery floor — which this decrements every
                // frame for every body (a ranged body freezes after one shot without
                // it). The strike-spawning `advance_body_melee` / `start_body_melee`
                // are deleted; this is only their surviving cooldown tick.
                ambition_actors::features::ecs::attack::tick_body_melee_cooldowns
                    .run_if(gameplay_allowed),
                // ── Moveset runtime FIRST: produce this frame's action messages ──
                //
                // The move runtime (trigger → advance → dispatch) runs BEFORE the
                // EFFECTS-stage consumers below so a `MoveEventKind::Ranged` /
                // `Effect{key}` fired by a move this frame is consumed THIS frame.
                // The old order put the consumers first, which made every
                // moveset-fired shot cross a frame boundary as an in-flight
                // message — state GGRS clears on `LoadWorld`, so a rollback
                // window landing on the boundary silently swallowed the shot
                // (Phase-5 exit-oracle finding: the striker's `shoot` move fired
                // in the live pass and not in resimulation). Same-frame
                // consumption is the rollback doctrine (deep review §2.2).
                //
                // Data-driven move TRIGGER: a body carrying an `ActorMoveset`
                // repertoire whose control frame presses a verb edge starts the
                // matching move (inserts `MovePlayback`). Before `advance` so a move
                // triggered this tick advances the same frame (fable review §A1,
                // Path B — the production insert the moveset runtime was missing).
                ambition_actors::combat::moveset::trigger_moveset_moves.run_if(gameplay_allowed),
                // Boss STRIKES trigger the SAME way: when a boss's `active_profile`
                // (mirrored from its pattern this frame) is set, start the boss's
                // move for that profile — a geometry strike's Active-window hit volume
                // OR a content-technique special's per-frame `Effect{key}` sustain.
                // ONE trigger for every boss strike, retiring both `sync_boss_strike_hitboxes`
                // and `dispatch_boss_special` (§A1 — the moveset is the boss's melee system).
                ambition_actors::features::trigger_boss_attack_moves.run_if(gameplay_allowed),
                // Data-driven move playback (Smash-model timelines, W9):
                // advances each playing MoveSpec on its OWNER'S proper time,
                // manages window-scoped hit volumes, fires MoveEventMessages.
                // Before apply_hitbox_damage so a window entered this tick
                // resolves its hits this tick.
                // A strike volume's existence is DERIVED from `(owner's move clock,
                // window)`. This enforces that against the world before the clock
                // moves — a no-op every ordinary frame, and the thing that keeps a
                // rollback from stranding the boxes it rewound past.
                (
                    ambition_actors::combat::moveset::retire_orphaned_strike_volumes,
                    ambition_actors::combat::moveset::advance_move_playback,
                )
                    .chain()
                    .run_if(gameplay_allowed),
                // Data-driven move EFFECT dispatch: resolve `MoveEventMessage`s —
                // `Sfx{cue}` → play at the owner; `Effect{key}` → bridge to the SAME
                // `ActorActionMessage::Special` the brain special path emits, so a
                // move fires a content technique with no new plumbing (the seam the
                // boss `Special(key)` profiles reuse). After `advance` so this
                // frame's events dispatch this frame — and before every consumer
                // below, so what it dispatches is also CONSUMED this frame.
                ambition_actors::combat::moveset::dispatch_move_events.run_if(gameplay_allowed),
                // Melee subsumption read-model (§A1 / §3a): a body whose melee is a
                // moveset `"attack"` move has its `BodyMelee` swing PROJECTED from the
                // live `MovePlayback` here (after `advance_move_playback` set/cleared
                // it this frame), so the actor anim index, telegraph/view index, HUD,
                // and melee tests keep reading the same read-model the flat swing used
                // to publish. Writes no gameplay — the real strike is the move's own
                // hitbox.
                ambition_actors::combat::moveset::project_moveset_melee_to_body_melee
                    .run_if(gameplay_allowed),
                // Boss strike read-model PROJECTION (E53 Slice B+C): while a boss move
                // is inside its Active window, `BossAttackState`'s active_* fields are
                // DERIVED from the live `MovePlayback` (the move is the authority),
                // mirroring the melee projection above. After `advance_move_playback`
                // so `t` is current; provably equal to the brain's mirror today, it
                // flips WHO owns the strike timing to the shared move runtime.
                ambition_actors::features::project_boss_attack_state_from_move
                    .run_if(gameplay_allowed),
                // ── EFFECTS-stage consumers: drain this frame's messages ──
                //
                // EFFECTS-stage consumer: reads ActorActionMessage::Ranged —
                // emitted upstream by `emit_brain_action_messages` (PlayerInput
                // set) for flat-ranged bodies, and by `dispatch_move_events`
                // ABOVE for moveset-ranged bodies — and spawns enemy
                // projectiles, both same-frame. Runs BEFORE the projectile step
                // so projectiles spawned this tick already advance one step
                // this frame, matching the pre-migration latency.
                ambition_actors::features::spawn_enemy_projectiles_from_brain_actions
                    .run_if(gameplay_allowed),
                // The 11 per-boss special-attack Techniques (apple rain,
                // eye beam, the Gradient Sentinel barrage family, …) used
                // to sit inline here. They are now content-owned and run
                // in `CombatSet::ContentSpecials`, configured below to slot
                // in at exactly this point — AFTER the enemy-action
                // consumers, BEFORE the effect/projectile executors that
                // drain their `SpawnProjectile`/`EffectRequest` output.
                // Registration lives in
                // `ambition_content::bosses::specials::BossSpecialContentPlugin`.
                // Generic effect executor: drains `EffectRequest` (boss OR
                // player emitted) and makes each effect happen — currently the
                // `DamageBox` AOE (shockwave gauntlet + boss phase-transition
                // slam), faction-tagged at the emitter, resolved by
                // `apply_hitbox_damage` below. Runs at the position the bespoke
                // shockwave consumer used, so spawn timing is unchanged.
                // Box + Summon executors, nested into one chained group (keeps
                // the outer tuple within Bevy's 20-system limit). Summon stays
                // lib-side (the enemy roster) so `apply_effects` is substrate-free;
                // same slot as before, so minion spawn timing is unchanged.
                (
                    ambition_vfx::apply_effects.run_if(gameplay_allowed),
                    ambition_actors::features::apply_summon_effects.run_if(gameplay_allowed),
                )
                    .chain(),
                // Phase 3b enemy-pool spawn consumer: drains SpawnProjectile
                // messages emitted by the EFFECTS-stage fire consumers above
                // (apple rain / overfit volley / eye beam / ranged bolts /
                // sentry / meteor / volley) into EnemyProjectileState.bodies
                // BEFORE the step below, so a body spawned this tick advances
                // one step this frame — identical to the old direct push.
                crate::projectile_schedule::apply_enemy_projectile_effects.run_if(gameplay_allowed),
                // Unified projectile step (player + enemy, faction-routed). Runs
                // AFTER the enemy spawn consumer (so an enemy body spawned this
                // tick advances one step this frame) and BEFORE the player input +
                // spawn below (so a player shot FIRED this frame first ticks next
                // frame — the old asymmetric spawn timing, preserved).
                crate::projectile_schedule::step_projectiles.run_if(gameplay_allowed),
                // Player projectile INPUT: charge / Hadouken / fire → SpawnProjectile.
                crate::projectile_schedule::charge_projectile_input,
                // Phase 3b player-pool spawn consumer: materializes player-fired
                // bodies AFTER the step, so the new body first ticks next frame.
                crate::projectile_schedule::apply_player_spawn_projectile_messages,
                // Hitbox-entity lifecycle for melee strikes (Task A of the
                // actor/brain follow-up plan). `apply_hitbox_damage`
                // resolves overlap → damage event; `tick_and_despawn_hitboxes`
                // advances lifetimes and cleans expired entities.
                // CM4: the attacker's playing move learns its strike CONNECTED
                // (pre-resolved victim events; the volume resolver marks its own).
                // Immediately after `apply_hitbox_damage` so this frame's overlaps
                // mark this frame — an OnHit cancel window opens on the connect
                // frame. (Inner tuple: the outer chain is at Bevy's tuple-size
                // ceiling, and these two are one ordered unit anyway.)
                (
                    ambition_actors::features::apply_hitbox_damage,
                    ambition_actors::combat::moveset::mark_move_playback_landed_hits,
                )
                    .chain()
                    .run_if(gameplay_allowed),
                // On-hit conditional techniques (fable AJ1): while hitboxes are
                // still live, `dispatch_hitbox_on_hit` emits one `OnHitEffectMessage`
                // per damage-valid victim an `on_hit` volume overlaps; the engine
                // `apply_pogo_bounce` technique consumes it same-frame (the chain
                // orders them). Both no-op until a move authors an `on_hit` volume.
                ambition_actors::combat::on_hit::dispatch_hitbox_on_hit.run_if(gameplay_allowed),
                ambition_actors::combat::on_hit::apply_pogo_bounce.run_if(gameplay_allowed),
                // The BLOCK half of the unified pogo: a moveset down-air's pogo
                // hitbox bounces off world `PogoOrb` blocks (the collision-world
                // orbs the flat player pogo used), now that the melee fold routes
                // the down-air through the moveset (fable review R2.5).
                ambition_actors::features::ecs::attack::pogo_moveset_off_world_orbs
                    .run_if(gameplay_allowed),
                ambition_actors::features::tick_and_despawn_hitboxes,
                // Suppress combat damage during dialog / cutscene / pause: the
                // victim-side `apply_player_hit_events` is already gated this way, so
                // gate the attacker-side application too. Otherwise a body pinned
                // overlapping an actor while a conversation runs keeps registering
                // hits (strikes, FX) on it — the dialog half of the "continuous hit"
                // report. No combat lands in any non-`Playing` mode now.
                ambition_actors::features::apply_feature_hit_events.run_if(gameplay_allowed),
                // Cut-rope flavor (rope-cut detection → gate, hazard→visual
                // mirror + impact flavor, prop visuals) used to sit inline
                // here. It is now content-owned and runs in
                // `CombatSet::ContentFlavor`, configured below to slot in at
                // exactly this point — AFTER the feature-hit resolution so
                // it observes this frame's alive-flag transitions, BEFORE
                // the mount/rider bookkeeping. Registration lives in
                // `ambition_content::bosses::AmbitionBossContentPlugin`.
                // Mount/rider link bookkeeping. Runs after damage so
                // it observes the alive flag transition for either
                // side; a dead mount releases its rider (gravity on,
                // solo brain restored) and a dead rider clears the
                // mount's MountSlot back-reference.
                ambition_actors::features::enforce_mount_rider_link,
            )
                .chain()
                .in_set(SandboxSet::Combat),
        );

        // Hand the frame's victim-side hits from the message channel to the
        // rollback-registered FIFO the player resolver (which runs in NEXT
        // frame's PlayerSimulation) drains. Ordered after the attacker-side
        // consumer (i.e. after every writer in this chain) and gated like both
        // hit consumers so paused/dialog frames stage nothing. Registered
        // outside the chain tuple above only because that tuple is at Bevy's
        // arity limit.
        app.add_systems(
            sim,
            ambition_actors::features::ecs::damage_apply::stage_player_victim_hit_events
                .run_if(gameplay_allowed)
                .after(ambition_actors::features::apply_feature_hit_events)
                .before(ambition_actors::features::enforce_mount_rider_link)
                .in_set(SandboxSet::Combat),
        );

        // The FIFO's lifecycle guard: a room boundary voids staged hits from
        // the outgoing population (see the system's docs for the exact leak
        // window). Deliberately NOT gated on `gameplay_allowed` — boundaries
        // happen precisely while gameplay is suspended.
        app.add_systems(
            sim,
            ambition_actors::features::ecs::damage_apply::void_pending_player_hits_at_lifecycle_boundaries
                .in_set(ambition_platformer_primitives::schedule::GameplaySimulationRoot)
                .after(SandboxSet::ResetProcessing)
                .before(SandboxSet::FeatureViewSync),
        );

        // Map the content combat-extension slots into the chain. The app
        // owns this composition (where a domain-local set sits in the
        // global phase); the content plugins own the systems that hang on
        // each slot. Both slots live in `SandboxSet::Combat`.
        //
        // `ContentSpecials` slots in where the inline boss-special block
        // used to be: after the enemy-action consumers, before the
        // effect/projectile executors that drain the specials' output.
        // `ContentFlavor` slots in after feature-hit resolution (so it
        // observes this frame's alive-flag transitions) and before the
        // mount/rider bookkeeping — the cut-rope block's former position.
        app.configure_sets(
            sim,
            (
                CombatSet::ContentSpecials
                    // After the enemy-action consumer — which now runs after
                    // `dispatch_move_events`, so a boss move's `Effect{key}`
                    // Special dispatched this frame reaches its content
                    // technique THIS frame (same-frame doctrine, not next).
                    .after(ambition_actors::features::spawn_enemy_projectiles_from_brain_actions)
                    .before(ambition_vfx::apply_effects)
                    .in_set(SandboxSet::Combat),
                CombatSet::ContentFlavor
                    .after(ambition_actors::features::apply_feature_hit_events)
                    .before(ambition_actors::features::enforce_mount_rider_link)
                    .in_set(SandboxSet::Combat),
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::Schedules;

    /// Guards the content combat-extension slot configuration. Both
    /// `CombatSet` slots must be registered as set nodes in the `Update`
    /// schedule after `CombatSchedulePlugin` builds — that registration IS
    /// the `configure_sets` block above, the seam content boss-specials and
    /// cut-rope flavor hang on. If it is dropped, those content systems
    /// still run but float unordered relative to the projectile/effect
    /// executors that drain their output — a silent spawn-timing
    /// regression with no compile error. (Same graph-introspection pattern
    /// as `presentation_visual_sync_runs_after_feature_view_sync` in
    /// `gameplay_core`.)
    #[test]
    fn content_combat_slots_are_registered_in_the_combat_chain() {
        let mut app = App::new();
        app.add_plugins(CombatSchedulePlugin);

        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(Update)
            .expect("Update schedule must exist after CombatSchedulePlugin")
            .graph();
        for slot in [CombatSet::ContentSpecials, CombatSet::ContentFlavor] {
            assert!(
                graph.system_sets.get_key(slot.intern()).is_some(),
                "{slot:?} must be a registered combat-extension set node \
                 (configured by CombatSchedulePlugin). Without it the \
                 content systems that hang on the slot float unordered."
            );
        }
    }
}
