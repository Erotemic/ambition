//! Combat-phase schedule plugin.
//!
//! EFFECTS-stage brain-action consumers (enemy melee/ranged spawns, boss
//! special-attack spawns), projectile + hitbox + feature-hit resolution,
//! the cut-rope boss-arena tick, and mount/rider link bookkeeping all run
//! here in `Platformer2dSimulationPhaseMonolith::Combat`.
//!
//! Extracted from `app/plugins.rs` (ecs-cleanup-plan #8) so the top-level
//! simulation orchestration reads as a list of named domain plugins rather
//! than one monolithic scheduler.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{
    gameplay_allowed, CombatSet, Platformer2dSimulationPhaseMonolith,
};

/// Schedules the `Platformer2dSimulationPhaseMonolith::Combat` system chain.
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
        //
        // The resolver CARRIES the provider-authored sheets (U1 stage C). Combat
        // may not name that type — its own module doc says the query shape is
        // combat's and the metadata is not — and this crate links both, so this
        // is the seam where the two meet. `refresh_authored_volume_resolver`
        // below keeps the captured copy current.
        app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
        app.insert_resource(authored_volume_resolver_for(&Default::default()));
        app.add_systems(
            bevy::app::Update,
            refresh_authored_volume_resolver.run_if(
                bevy::ecs::schedule::common_conditions::resource_changed::<
                    ambition_sprite_sheet::character::sheets::AuthoredSheets,
                >,
            ),
        );
        // The effect seam: techniques (the shockwave gauntlet, the boss
        // phase-transition slam, …) emit `EffectRequest`; `apply_effects` below
        // drains it. Registered here so the writers never hit an unregistered
        // message.
        app.add_message::<ambition_vfx::EffectRequest>();
        app.add_message::<ambition_platformer2d_actor_monolith::combat::moveset::MoveEventMessage>(
        );
        // One authoritative resolved body-contact fact. The shared hitbox resolver
        // writes it; move confirms and authored on-hit techniques both consume it
        // instead of independently deciding whether the strike connected.
        app.add_message::<ambition_platformer2d_actor_monolith::combat::hitbox::LandedBodyHit>();
        app.add_message::<ambition_platformer2d_actor_monolith::combat::on_hit::OnHitEffectMessage>();
        // **A BODY REACHING ZERO SAYS SO, WHETHER OR NOT A RULESET IS LISTENING.**
        // `apply_player_hit_events` and `apply_actor_hit` both write
        // `BodyKnockedOut`, so the message has to exist wherever they run — not
        // wherever the STOCKS rules happen to be installed. It was registered
        // only in test fixtures, so `mary_o`'s power loop (damage pipeline, no
        // stocks) panicked with "Message not initialized" the moment a knockout
        // landed. A writer whose message is registered by a different plugin is
        // a composition that works until somebody composes differently.
        app.add_message::<ambition_platformer2d_actor_monolith::combat::stocks::BodyKnockedOut>();
        // Programmatic actor-spawn seam: scenario tests and RL/agent scene setup
        // emit `SpawnActorRequest`; `apply_spawn_actor_requests` materializes each
        // actor through the same `spawn_boss` / `spawn_enemy` paths room load uses.
        // Registered (and run) here next to the in-gameplay spawners, but
        // deliberately UNGATED so a scene-setup spawn applies in any `GameMode`.
        app.add_message::<ambition_platformer2d_actor_monolith::features::SpawnActorRequest>();
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::features::apply_spawn_actor_requests
                .in_set(CombatSet::Materialize),
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
                ambition_platformer2d_actor_monolith::features::ecs::attack::tick_body_melee_cooldowns
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
                (
                    ambition_platformer2d_actor_monolith::combat::moveset::resolve_attack_gestures,
                    ambition_platformer2d_actor_monolith::combat::moveset::trigger_moveset_moves,
                )
                    .chain()
                    .run_if(gameplay_allowed),
                // Boss STRIKES trigger the SAME way: when a boss's `active_profile`
                // (mirrored from its pattern this frame) is set, start the boss's
                // move for that profile — a geometry strike's Active-window hit volume
                // OR a content-technique special's per-frame `Effect{key}` sustain.
                // ONE trigger for every boss strike, retiring both `sync_boss_strike_hitboxes`
                // and `dispatch_boss_special` (§A1 — the moveset is the boss's melee system).
                ambition_platformer2d_actor_monolith::features::trigger_boss_attack_moves.run_if(gameplay_allowed),
            )
                .chain()
                .in_set(CombatSet::Trigger),
        );
        app.add_systems(
            sim,
            (
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
                    ambition_platformer2d_actor_monolith::combat::moveset::retire_orphaned_strike_volumes,
                    // ⚠ **BEFORE the advance, deliberately.** A move that landed
                    // this frame is over, and running the advance first would
                    // open its next window — spawning a strike volume for a
                    // move that has already been cancelled by the ground.
                    ambition_platformer2d_actor_monolith::combat::moveset::resolve_aerial_landings,
                    ambition_platformer2d_actor_monolith::combat::moveset::advance_move_playback,
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
                ambition_platformer2d_actor_monolith::combat::moveset::dispatch_move_events.run_if(gameplay_allowed),
                // Melee subsumption read-model (§A1 / §3a): a body whose melee is a
                // moveset `"attack"` move has its `BodyMelee` swing PROJECTED from the
                // live `MovePlayback` here (after `advance_move_playback` set/cleared
                // it this frame), so the actor anim index, telegraph/view index, HUD,
                // and melee tests keep reading the same read-model the flat swing used
                // to publish. Writes no gameplay — the real strike is the move's own
                // hitbox.
                ambition_platformer2d_actor_monolith::combat::moveset::project_moveset_melee_to_body_melee
                    .run_if(gameplay_allowed),
                // Boss strike read-model PROJECTION (E53 Slice B+C): while a boss move
                // is inside its Active window, `BossAttackState`'s active_* fields are
                // DERIVED from the live `MovePlayback` (the move is the authority),
                // mirroring the melee projection above. After `advance_move_playback`
                // so `t` is current; provably equal to the brain's mirror today, it
                // flips WHO owns the strike timing to the shared move runtime.
                ambition_platformer2d_actor_monolith::features::project_boss_attack_state_from_move
                    .run_if(gameplay_allowed),
            )
                .chain()
                .in_set(CombatSet::Playback),
        );
        app.add_systems(
            sim,
            (
                // ── EFFECTS-stage consumers: drain this frame's messages ──
                //
                // EFFECTS-stage consumer: reads ActorActionMessage::Ranged —
                // emitted upstream by `emit_brain_action_messages` (PlayerInput
                // set) for flat-ranged bodies, and by `dispatch_move_events`
                // ABOVE for moveset-ranged bodies — and spawns enemy
                // projectiles, both same-frame. Runs BEFORE the projectile step
                // so projectiles spawned this tick already advance one step
                // this frame, matching the pre-migration latency.
                ambition_platformer2d_actor_monolith::features::spawn_enemy_projectiles_from_brain_actions
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
                    ambition_combat::strike::apply_effects
                        .in_set(ambition_combat::strike::EffectExecutionSet)
                        .run_if(gameplay_allowed),
                    ambition_platformer2d_actor_monolith::features::apply_summon_effects.run_if(gameplay_allowed),
                )
                    .chain(),
                // Phase 3b enemy-pool spawn consumer: drains SpawnProjectile
                // messages emitted by the EFFECTS-stage fire consumers above
                // (apple rain / overfit volley / eye beam / ranged bolts /
                // sentry / meteor / volley) into EnemyProjectileState.bodies
                // BEFORE the step below, so a body spawned this tick advances
                // one step this frame — identical to the old direct push.
                crate::projectile_schedule::apply_enemy_projectile_effects.run_if(gameplay_allowed),
                // ⭐ **WHOSE SHOT THIS IS, FROZEN BEFORE ANYTHING STEPS IT.**
                // The stamp used to be taken lazily on a bolt's first step, which
                // left a window where it existed unstamped — and a firer
                // eliminated inside that window took the answer with them, so the
                // shot spent its life re-asking a body that was gone.
                //
                // ⛔⛔ **TWICE, and the second one is not redundant.** The first
                // draft put this here alone, reasoning that a player bolt
                // materializes after the step below and so first ticks next frame,
                // when this has already run. That is true about STEPPING and false
                // about the window: `take_eliminated_fighters_out_of_play` runs in
                // `CombatSet::Settle`, and `Materialize` is BEFORE `Settle` — so a
                // player bolt fired on the tick its owner is eliminated materializes
                // at the end of this chain, loses its firer later in the same tick,
                // and reaches this system next frame with nothing to read. The
                // window is bounded by the DESPAWN, not by the step.
                //
                // ⛔ not `run_if(gameplay_allowed)` — a bolt that materialized
                // before a pause must not lose its side by being skipped, and
                // stamping is not gameplay progress.
                crate::projectile_schedule::stamp_new_projectile_allegiance,
                // Unified projectile step (player + enemy, faction-routed). Runs
                // AFTER the enemy spawn consumer (so an enemy body spawned this
                // tick advances one step this frame) and BEFORE the player input +
                // spawn below (so a player shot FIRED this frame first ticks next
                // frame — the old asymmetric spawn timing, preserved).
                crate::projectile_schedule::step_projectiles
                    .in_set(crate::projectile_schedule::ProjectileStepSet)
                    .run_if(gameplay_allowed),
                // Player projectile INPUT: charge / Hadouken / fire → SpawnProjectile.
                crate::projectile_schedule::charge_projectile_input,
                // Phase 3b player-pool spawn consumer: materializes player-fired
                // bodies AFTER the step, so the new body first ticks next frame.
                crate::projectile_schedule::apply_player_spawn_projectile_messages,
                // The second stamp: the player bolt that just materialized takes
                // its side HERE, inside `Materialize`, rather than next frame —
                // because its firer can be eliminated later this same tick, in
                // `Settle`. See the note beside the first placement.
                crate::projectile_schedule::stamp_new_projectile_allegiance,
            )
                .chain()
                .in_set(CombatSet::Materialize),
        );
        app.add_systems(
            sim,
            (
                // Hitbox-entity lifecycle for melee strikes (Task A of the
                // actor/brain follow-up plan). `apply_hitbox_damage`
                // resolves overlap → damage event; `tick_and_despawn_hitboxes`
                // advances lifetimes and cleans expired entities.
                // CM4: the attacker's playing move learns its strike CONNECTED from
                // the same `LandedBodyHit` fact that drives authored on-hit effects.
                // Immediately after `apply_hitbox_damage` so this frame's overlaps
                // mark this frame — an OnHit cancel window opens on the connect
                // frame. (Inner tuple: the outer chain is at Bevy's tuple-size
                // ceiling, and these two are one ordered unit anyway.)
                (
                    ambition_platformer2d_actor_monolith::features::apply_hitbox_damage,
                    ambition_platformer2d_actor_monolith::combat::moveset::mark_move_playback_landed_hits,
                )
                    .chain()
                    .run_if(gameplay_allowed),
                // Authored on-hit techniques consume the SAME landed-body fact
                // emitted by `apply_hitbox_damage`; there is no second overlap or
                // relationship pass. `apply_pogo_bounce` then interprets the authored
                // effect against the victim's pogo policy.
                ambition_platformer2d_actor_monolith::combat::on_hit::dispatch_landed_hit_effects.run_if(gameplay_allowed),
                ambition_platformer2d_actor_monolith::combat::on_hit::apply_pogo_bounce.run_if(gameplay_allowed),
                // Genuine WORLD pogo surfaces have no victim entity, so they stay a
                // separate collision-world contact path. ECS bodies are never projected
                // into this world-surface representation.
                ambition_platformer2d_actor_monolith::features::ecs::attack::pogo_moveset_off_world_orbs
                    .run_if(gameplay_allowed),
                ambition_platformer2d_actor_monolith::features::tick_and_despawn_hitboxes,
                // Suppress combat damage during dialog / cutscene / pause: the
                // victim-side `apply_player_hit_events` is already gated this way, so
                // gate the attacker-side application too. Otherwise a body pinned
                // overlapping an actor while a conversation runs keeps registering
                // hits (strikes, FX) on it — the dialog half of the "continuous hit"
                // report. No combat lands in any non-`Playing` mode now.
                ambition_platformer2d_actor_monolith::features::apply_feature_hit_events.run_if(gameplay_allowed),
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
            )
                .chain()
                .in_set(CombatSet::Resolve),
        );
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::features::enforce_mount_rider_link
                .in_set(ambition_platformer2d_actor_monolith::features::MountRiderLinkEnforced)
                .in_set(CombatSet::Settle),
        );

        // **A landed hit shakes the screen** (P4.37). In `Settle` because that
        // is the phase that reads the frame's resolved damage, and in the
        // ENGINE group because the standalone smash binary composes this and
        // not `ambition_app` — the first version of this lived in the app's
        // home-avatar presentation system and so could not fire in the proving
        // ground at all. Body-generic by construction: see the module docs.
        //
        // ⛔ **it is the one system in this schedule that writes non-rollback
        // PRESENTATION state**, so it carries its own authoritative-pass guard
        // as a parameter rather than a `run_if` here — a replayed frame kicking
        // the live camera is a ghost shake, and the guard must survive anyone
        // else registering it. Do not "fix" that by adding a run condition
        // here; read the module's second ⛔⛔ block first.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::features::ecs::shake_camera_on_landed_hits
                .run_if(gameplay_allowed)
                .in_set(CombatSet::Settle),
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
            ambition_platformer2d_actor_monolith::features::ecs::damage_apply::stage_player_victim_hit_events
                .run_if(gameplay_allowed)
                // The PHASE, not the two leaves it used to sit between: this
                // reads the frame's resolved damage, which is what `Settle` is.
                .in_set(CombatSet::Settle)
                .before(ambition_platformer2d_actor_monolith::features::MountRiderLinkEnforced),
        );

        // The FIFO's lifecycle guard: a room boundary voids staged hits from
        // the outgoing population (see the system's docs for the exact leak
        // window). Deliberately NOT gated on `gameplay_allowed` — boundaries
        // happen precisely while gameplay is suspended.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::features::ecs::damage_apply::void_pending_player_hits_at_lifecycle_boundaries
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(Platformer2dSimulationPhaseMonolith::ResetProcessing)
                .before(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );

        // Map the content combat-extension slots into the chain. The app
        // owns this composition (where a domain-local set sits in the
        // global phase); the content plugins own the systems that hang on
        // each slot. Both slots live in `Platformer2dSimulationPhaseMonolith::Combat`.
        //
        // `ContentSpecials` slots in where the inline boss-special block
        // Both slots' PLACEMENT is `configure_platformer2d_simulation_phases`' job now — the phase
        // chain puts `ContentSpecials` inside `Materialize` and `ContentFlavor`
        // between `Resolve` and `Settle`, which is exactly where the two
        // leaf-named edges used to put them. What remains here is the one edge
        // the phase order cannot express: a boss special must reach its content
        // technique BEFORE the effect executors that drain its output, and both
        // live in `Materialize`.
        app.configure_sets(
            sim,
            CombatSet::ContentSpecials.before(ambition_combat::strike::EffectExecutionSet),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::Schedules;

    /// Guards the content combat-extension slots: both must be REGISTERED set
    /// nodes, and both must sit in the combat phase chain.
    ///
    /// If a slot is not a node, the content systems hanging on it still run and
    /// float unordered relative to the projectile/effect executors that drain
    /// their output — a silent spawn-timing regression with no compile error.
    ///
    /// ⚠ The composition matters and this test used to get it wrong. It built
    /// `CombatSchedulePlugin` alone, because that plugin used to configure both
    /// slots itself with leaf-named edges (`.after(spawn_enemy_projectiles…)`,
    /// `.before(enforce_mount_rider_link)`). Placement now belongs to
    /// `configure_platformer2d_simulation_phases`, which owns the phase chain — so a test that adds
    /// only the combat plugin is asserting about a composition no app ships.
    /// Both authorities participate here for the same reason production has both.
    #[test]
    fn content_combat_slots_are_registered_in_the_combat_chain() {
        let mut app = App::new();
        ambition_platformer2d_actor_monolith::schedule::configure_platformer2d_simulation_phases(
            &mut app,
        );
        app.add_plugins(CombatSchedulePlugin);

        let schedules = app.world().resource::<Schedules>();
        let graph = schedules
            .get(Update)
            .expect("Update schedule must exist after the combat schedule is configured")
            .graph();
        for slot in [
            CombatSet::ContentSpecials,
            CombatSet::ContentFlavor,
            // The engine phases the slots hang between. A slot registered into a
            // phase that does not exist is the same silent float, one level up.
            CombatSet::Trigger,
            CombatSet::Playback,
            CombatSet::Materialize,
            CombatSet::Resolve,
            CombatSet::Settle,
        ] {
            assert!(
                graph.system_sets.get_key(slot.intern()).is_some(),
                "{slot:?} must be a registered combat set node. Without it the \
                 systems that hang on it float unordered relative to the \
                 executors that consume their output."
            );
        }
    }
}

/// A resolver closure that carries a snapshot of the authored sheets.
///
/// Cloned rather than borrowed because the resolver outlives any system that
/// could lend it a reference — it is a `Resource` combat reads whenever a strike
/// resolves. Registration happens at plugin build, so the clone is paid once per
/// change and never per hit.
fn authored_volume_resolver_for(
    sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
) -> ambition_platformer2d_actor_monolith::combat::authored_volumes::AuthoredAttackVolumeResolver {
    let sheets = sheets.clone();
    ambition_platformer2d_actor_monolith::combat::authored_volumes::AuthoredAttackVolumeResolver::from_closure(
        move |catalog, sprite_character_id, animation, body_pos, collision, facing, gravity_dir| {
            ambition_character_sprites::authored_attack_volume_resolver(
                &sheets,
                catalog,
                sprite_character_id,
                animation,
                body_pos,
                collision,
                facing,
                gravity_dir,
            )
        },
    )
}

/// Rebuild the resolver when a provider registers a sheet.
///
/// Without this the captured snapshot would be whatever existed at plugin-build
/// time, which is the ordering trap the whole `AuthoredSheets` design avoids
/// elsewhere: a provider added later would resolve volumes from the engine's
/// baked table while resolving everything else from its own sheet.
fn refresh_authored_volume_resolver(
    sheets: bevy::prelude::Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    mut resolver: bevy::prelude::ResMut<
        ambition_platformer2d_actor_monolith::combat::authored_volumes::AuthoredAttackVolumeResolver,
    >,
) {
    *resolver = authored_volume_resolver_for(&sheets);
}
