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
        app.add_message::<ambition_combat::moveset::MoveEventMessage>();
        // One authoritative resolved body-contact fact. The shared hitbox resolver
        // writes it; move confirms and authored on-hit techniques both consume it
        // instead of independently deciding whether the strike connected.
        app.add_message::<ambition_combat::hitbox::LandedBodyHit>();
        app.add_message::<ambition_combat::on_hit::OnHitEffectMessage>();
        // A BODY REACHING ZERO SAYS SO, WHETHER OR NOT A RULESET IS LISTENING.
        // `apply_player_hit_events` and `apply_actor_hit` both write `BodyKnockedOut`, so the
        // message has to exist wherever they run — not wherever the STOCKS rules happen to be
        // installed. A writer whose message is registered by a different plugin is a composition
        // that works until somebody composes differently.
        app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
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
                // Data-driven move TRIGGER: a body carrying an `ActorMoveset` repertoire whose
                // control frame presses a verb edge starts the matching move (inserts
                // `MovePlayback`).
                (
                    // ⛔ FIRST IN THE COMBAT PHASE, so a body that left play on
                    // any road last tick has no move before this tick's trigger
                    // or playback can touch it. See
                    // `end_moves_for_bodies_out_of_play` for why it is an
                    // invariant re-established here rather than a line in each
                    // death road.
                    ambition_combat::death_rules::end_moves_for_bodies_out_of_play,
                    ambition_combat::moveset::resolve_attack_gestures,
                    // Input leniency sits BETWEEN interpretation and the action
                    // authority, and only there: it decays the body's combat
                    // verb windows, arms them from this tick's resolved edges,
                    // and re-proposes an unspent press. The trigger below is the
                    // one seam that can accept — and therefore spend — one.
                    ambition_combat::moveset::buffer_combat_action_presses,
                    ambition_combat::moveset::trigger_moveset_moves,
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
                // Before apply_hitbox_damage so a window entered this tick resolves its hits
                // this tick. A strike volume's existence is DERIVED from `(owner's move clock,
                // window)`. This enforces that against the world before the clock moves — a
                // no-op every ordinary frame, and the thing that keeps a rollback from
                // stranding the boxes it rewound past.
                (
                    ambition_combat::moveset::retire_orphaned_strike_volumes,
                    // BEFORE the advance, deliberately. A move that landed
                    // this frame is over, and running the advance first would
                    // open its next window — spawning a strike volume for a
                    // move that has already been cancelled by the ground.
                    ambition_combat::moveset::resolve_aerial_landings,
                    // AFTER the landing that charges it, so a body that landed
                    // and left the ground in the same frame is charged and then
                    // released rather than released and then charged — which
                    // would leave it paying lag in mid-air, the one state the
                    // rule exists to prevent.
                    ambition_combat::moveset::edge_cancel_landing_recovery,
                    // BEFORE the advance: a stow is a decision about the charge
                    // as it stands, and running the clock first would bank a
                    // shot one tick fuller than the one the player put away.
                    ambition_combat::moveset::stow_a_stored_charge_on_guard,
                    ambition_combat::moveset::advance_move_playback,
                    // Right behind the clock that moves them: a move's authored
                    // Invuln / Armor windows are republished onto the two facts
                    // the rest of combat already reads, so eligibility keeps ONE
                    // authority and nothing downstream learns to read timelines.
                    ambition_combat::moveset::project_move_defense_windows,
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
                // ⭐ THE OTHER HALF OF THE SPECIAL TURN, after the trigger that
                // opens its window — a flick on the same tick as the press is
                // the press, not a B-reverse.
                ambition_combat::moveset::apply_special_turn_flicks.run_if(gameplay_allowed),
                // ⛔ BEFORE `dispatch_move_events`, and that ordering is the
                // whole mechanic: the move's `Ranged` event is dispatched there
                // and `spawn_projectiles_from_brain_actions` routes the shot by
                // WHAT IS IN THE HAND. A brandish that landed after would fire
                // the fighter's bare-handed shot on the frame it drew the gun.
                ambition_combat::held_items::brandish_the_playing_move_s_weapon
                    .run_if(gameplay_allowed),
                ambition_combat::moveset::dispatch_move_events.run_if(gameplay_allowed),
                // Writes no gameplay — the real strike is the move's own hitbox.
                ambition_combat::moveset::project_moveset_melee_to_body_melee
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
                // EFFECTS-stage consumer: reads ActorActionMessage::Ranged — emitted upstream
                // by `emit_brain_action_messages` (PlayerInput set) for flat-ranged bodies, and
                // by `dispatch_move_events` ABOVE for moveset-ranged bodies — and emits open
                // projectile requests.
                ambition_platformer2d_actor_monolith::features::spawn_projectiles_from_brain_actions
                    .run_if(gameplay_allowed),
                // EFFECTS-stage consumer, beside the shot spawner and for the same
                // reason: a move's timed technique is dispatched as an
                // `ActorActionMessage` above, and a teleport that ran a phase
                // later would move the body after the frame it was authored for.
                ambition_platformer2d_actor_monolith::abilities::traversal::teleport::apply_authored_teleports
                    .run_if(gameplay_allowed),
                // EFFECTS-stage consumer, beside the teleport and for the same
                // reason: a health change authored on a move's timeline must
                // land on the frame the move named. It sits AHEAD of
                // `apply_effects` so a fighter who paid for a move is already
                // poorer when this frame's hits resolve — a price that settled
                // afterwards would let her be launched at the percent she had
                // before she bought the tempo.
                ambition_combat::vitality::apply_authored_vitality.run_if(gameplay_allowed),
                // EFFECTS-stage consumer, beside the teleport and for the same
                // reason. ⛔ AND IT MUST NOT BE ORDERED AGAINST THE TELEPORT:
                // the two never act on one body on one frame (a move authors
                // one technique or the other), so a `.chain()` here would be a
                // constraint stating a relationship that does not exist.
                ambition_platformer2d_actor_monolith::abilities::traversal::trapdoor::apply_authored_trapdoors
                    .run_if(gameplay_allowed),
                (
                    ambition_combat::strike::apply_effects
                        .in_set(ambition_combat::strike::EffectExecutionSet)
                        .run_if(gameplay_allowed),
                    ambition_platformer2d_actor_monolith::features::apply_summon_effects.run_if(gameplay_allowed),
                )
                    .chain(),
                // Immediate projectile materializer: actor/item/boss fire above
                // writes the projectile domain's own request and explicitly asks
                // to begin on THIS tick. Materialize before the step so the
                // historical first-tick timing remains unchanged without routing
                // authoritative spawning through the VFX effect enum.
                crate::projectile_schedule::materialize_projectiles_for_this_tick
                    .run_if(gameplay_allowed),
                // TWICE, and the second one is not redundant. The first
                // draft put this here alone, reasoning that a player bolt
                // materializes after the step below and so first ticks next frame,
                // when this has already run. That is true about STEPPING and false
                // about the window: `take_eliminated_fighters_out_of_play` runs in
                // `CombatSet::Settle`, and `Materialize` is BEFORE `Settle` — so a
                // delayed bolt fired on the tick its owner is eliminated materializes
                // at the end of this chain, loses its firer later in the same tick,
                // and reaches this system next frame with nothing to read. The
                // window is bounded by the DESPAWN, not by the step.
                //
                // not `run_if(gameplay_allowed)` — a bolt that materialized
                // before a pause must not lose its side by being skipped, and
                // stamping is not gameplay progress.
                crate::projectile_schedule::stamp_new_projectile_allegiance,
                // Unified projectile step (player + enemy, faction-routed).
                crate::projectile_schedule::step_projectiles
                    .in_set(crate::projectile_schedule::ProjectileStepSet)
                    .run_if(gameplay_allowed),
                // Named body-fire INPUT: charge / Hadouken / fire → ProjectileSpawnRequest.
                crate::projectile_schedule::charge_projectile_input,
                // Delayed projectile materializer: the charged/named body-fire
                // road explicitly asks to begin NEXT tick. It runs after the step,
                // preserving the existing fire latency through the same entity
                // constructor used by immediate actor/item/boss shots.
                crate::projectile_schedule::materialize_projectiles_for_next_tick,
                // The second stamp: the delayed bolt that just materialized takes
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
                    // ⭐ TWO ATTACKS MEETING IS RESOLVED FIRST, and it has to be:
                    // the trade must be known for BOTH volumes before EITHER
                    // asks about a victim, or whichever one the query yields
                    // first lands before anybody knows it was cancelled.
                    ambition_combat::clank::arbitrate_attack_clanks,
                    // …and what the trade costs, immediately after, so the
                    // moves it ends are gone before the damage sweep looks at
                    // anything they own.
                    ambition_combat::clank::rebound_from_clanks,
                    ambition_platformer2d_actor_monolith::features::apply_hitbox_damage,
                    ambition_combat::moveset::mark_move_playback_landed_hits,
                )
                    .chain()
                    .run_if(gameplay_allowed),
                ambition_combat::on_hit::dispatch_landed_hit_effects.run_if(gameplay_allowed),
                ambition_combat::on_hit::apply_pogo_bounce.run_if(gameplay_allowed),
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
                // It is now content-owned and runs in `CombatSet::ContentFlavor`, configured below
                // to slot in at exactly this point — AFTER the feature-hit resolution so it
                // observes this frame's alive-flag transitions, BEFORE the mount/rider bookkeeping.
                // Registration lives in `ambition_content::bosses::AmbitionBossContentPlugin`.
                // Mount/rider link bookkeeping. Runs after damage so it observes the alive flag
                // transition for either side; a dead mount releases its rider (gravity on, solo
                // brain restored) and a dead rider clears the mount's MountSlot back-reference.
            )
                .chain()
                .in_set(CombatSet::Resolve),
        );
        // ⭐ TWO SYSTEMS, CHAINED, AND THE CHAIN IS THE POINT. The link enforcer
        // announces a dissolution with `MountDied`; the rebuild answers it. They
        // are chained rather than merely both in `Settle` because the message is
        // an IN-TICK channel — the reader must run after this frame's write, and
        // the rebuilt brain must be inserted before the dismounted body is next
        // simulated. Chaining puts both facts in one line instead of an ordering
        // that happens to hold.
        app.add_message::<ambition_mount::DismountRequested>();
        app.add_message::<ambition_mount::RiderDismounted>();
        // A summon that asked to be ridden and was refused. Written by the
        // construction road inside its exclusive command, read by whichever
        // ruleset decides what an unclaimed mount is for.
        app.add_message::<ambition_mount::RideRefused>();
        app.add_systems(
            sim,
            (
                ambition_mount::enforce_mount_rider_link,
                ambition_platformer2d_actor_monolith::features::rebuild_dismounted_rider_brains,
            )
                .chain()
                .in_set(ambition_mount::MountRiderLinkEnforced)
                .in_set(CombatSet::Settle),
        );
        // LEAVING THE SADDLE VOLUNTARILY — the twin of the enforcer above,
        // which owns leaving it because somebody DIED.
        //
        // ⛔ CHAINED, AND AFTER the enforcer. A lease that runs out on the same
        // tick a mount dies must find the death already handled: the enforcer
        // keeps `RidingOn` attached on purpose so a reset can re-mount, and a
        // dismount request landing first would remove the link it is relying on.
        // The other order is silent — the rider ends up correctly off either
        // way, and the mount's slot does not.
        //
        // ⚠ the tick and the apply are chained for the ordinary in-tick-channel
        // reason: the request is written and consumed in one frame, so a reader
        // scheduled merely "in the same set" would read it a frame late and put
        // riders down one tick after their lease expired.
        app.add_systems(
            sim,
            (
                ambition_mount::tick_ride_leases,
                ambition_mount::apply_dismount_requests,
            )
                .chain()
                .after(ambition_mount::MountRiderLinkEnforced)
                .in_set(CombatSet::Settle),
        );
        // GETTING ON — the counterpart to the two systems above, and the half a
        // summon no longer does for itself.
        //
        // ⛔ BEFORE the lease tick, not after. A ride that boards on this tick
        // gets its full lease: the alternative spends a frame of a five-second
        // clock before the rider is even welded, which is invisible and wrong in
        // exactly the way clocks usually are.
        app.add_systems(
            sim,
            ambition_mount::board_reserved_mounts
                .before(ambition_mount::tick_ride_leases)
                .after(ambition_mount::MountRiderLinkEnforced)
                .in_set(CombatSet::Settle),
        );

        // it is the one system in this schedule that writes non-rollback
        // PRESENTATION state, so it carries its own authoritative-pass guard
        // as a parameter rather than a `run_if` here — a replayed frame kicking
        // the live camera is a ghost shake, and the guard must survive anyone
        // else registering it. Do not "fix" that by adding a run condition
        // here; read the module's second block first.
        app.add_systems(
            sim,
            ambition_combat::hit_camera_shake::shake_camera_on_landed_hits
                .run_if(gameplay_allowed)
                .in_set(CombatSet::Settle),
        );

        // The MATCH's impact freeze. ⭐ ONE system — the hold is an absolute
        // expiry tick, so there is nothing to decay and nothing to hand back.
        //
        // ⛔ IT READS `ResolvedBodyHit`, NOT the landed hits the shake above
        // reads. The shake is presentation and geometry is enough for it; the
        // freeze needs the hit's RESULT, and the two roads that produce one
        // resolve on different frames. See the system's own note.
        app.init_resource::<ambition_combat::impact_hitstop::ImpactHitstop>();
        app.add_systems(
            sim,
            ambition_combat::impact_hitstop::request_impact_hitstop_on_resolved_hits
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
            ambition_damage::stage_player_victim_hit_events
                .run_if(gameplay_allowed)
                .in_set(CombatSet::Settle)
                .before(ambition_mount::MountRiderLinkEnforced),
        );

        // The FIFO's lifecycle guard: a room boundary voids staged hits from
        // the outgoing population (see the system's docs for the exact leak
        // window). Deliberately NOT gated on `gameplay_allowed` — boundaries
        // happen precisely while gameplay is suspended.
        app.add_systems(
            sim,
            ambition_damage::void_pending_player_hits_at_lifecycle_boundaries
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(Platformer2dSimulationPhaseMonolith::ResetProcessing)
                .before(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );

        // Map the content combat-extension slots into the chain. The app
        // owns this composition (where a domain-local set sits in the
        // global phase); the content plugins own the systems that hang on
        // each slot. Both slots live in `Platformer2dSimulationPhaseMonolith::Combat`.
        //
        // What remains here is the one edge the phase order cannot express: a boss special must
        // reach its content technique BEFORE the effect executors that drain its output, and both
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
    /// Placement now belongs to `configure_platformer2d_simulation_phases`, which owns the phase
    /// chain — so a test that adds only the combat plugin is asserting about a composition no app
    /// ships. Both authorities participate here for the same reason production has both.
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
) -> ambition_combat::authored_volumes::AuthoredAttackVolumeResolver {
    let sheets = sheets.clone();
    ambition_combat::authored_volumes::AuthoredAttackVolumeResolver::from_closure(
        move |catalog, sprite_character_id, animation, collision, clip_elapsed| {
            ambition_character_sprites::authored_attack_volume_resolver(
                &sheets,
                catalog,
                sprite_character_id,
                animation,
                collision,
                clip_elapsed,
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
        ambition_combat::authored_volumes::AuthoredAttackVolumeResolver,
    >,
) {
    *resolver = authored_volume_resolver_for(&sheets);
}
