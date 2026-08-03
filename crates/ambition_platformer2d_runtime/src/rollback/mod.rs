//! Ambition's integration boundary for `ggrs` + `bevy_ggrs`.
//!
//! GGRS is the sole rollback authority. It owns frame requests, prediction,
//! snapshot history, entity recreation, load ordering, resimulation, confirmed
//! frames, and checksum comparison. Ambition contributes only:
//!
//! - the typed list of authoritative components/resources;
//! - deterministic checksum projections for float-heavy domain values;
//! - exact registration/content identity;
//! - the input bridge and session lifecycle policy.

use bevy::{
    ecs::schedule::{ExecutorKind, LogLevel, ScheduleBuildSettings},
    prelude::*,
};
use bevy_ggrs::{GgrsPlugin, RollbackFrameRate};

pub use bevy_ggrs::{
    AdvanceWorld, AdvanceWorldSystems, ConfirmedFrameCount, GgrsSchedule, LoadWorld,
    LoadWorldSystems, Rollback, RollbackFrameCount, RunGgrsSystems, SaveWorld,
};

/// Ambition-owned work that must run after every `bevy_ggrs` entity/data/map restore.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum AmbitionLoadWorldSet {
    /// Reconcile authored/runtime pairs after all raw `Entity` handles have been remapped.
    Reconcile,
}

mod codec;
mod codecs;
mod domains;
pub mod local_session;
#[cfg(test)]
mod host_invariant_tests;
mod probes;
#[cfg(test)]
mod provenance_tests;
mod registry;
mod session;

pub use codec::*;
pub use codecs::{ensure_sim_id, heal_projectile_owners, mint_spawned_sim_ids};
pub use probes::*;
pub use registry::*;
pub use session::*;

/// Installs the host-independent typed rollback schema used by prepared
/// content identity. Non-GGRS games retain this lightweight registry without
/// installing snapshot history, schedules, checksums, or session machinery.
pub struct AmbitionRollbackSchemaPlugin;

impl Plugin for AmbitionRollbackSchemaPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RollbackRegistry>();
        register_engine_rollback_state(app);
    }
}

/// FORK(ggrs-frame-timing): recover the intra-tick phase from the GGRS
/// driver's own fixed-timestep timing.
///
/// Presentation draws on the render clock while the sim advances on a fixed
/// tick, so a published pose is a step function; drawing it directly makes
/// anything that moves shudder against a smoothly-easing camera. Removing that
/// needs to know how far through the current tick this frame sits, and under
/// GGRS the only truthful source is the driver's own accumulator — the one
/// that decides when to advance. `Time<Fixed>::overstep_fraction()` answers it
/// for the plain fixed host and is unavailable here precisely because GGRS
/// banks its own time.
///
/// `GgrsFrameTiming` publishes that quantity as a supported accessor:
/// `overstep_fraction()` is the accumulator as a fraction of the *actual*
/// timestep the last driver pass used, so it stays correct during run-slow
/// catch-up where that timestep widens — better than dividing by the nominal
/// rate. This compiles against a `bevy_ggrs` fork that backports the accessor
/// onto the v0.21.0 / bevy-0.18 line; the `[patch.crates-io]` entry in the
/// workspace manifest carries the rationale and the condition that retires it.
///
/// A parallel accumulator was considered and rejected: it would agree only
/// while nothing interesting happened, and diverge during run-slow catch-up,
/// stalls, several advances in one frame, and rollback resimulation — exactly
/// when a wrong phase shows most. A presentation clock that lies during a
/// rollback is worse than no smoothing at all.
///
/// Retire when the accessor ships in a released `bevy_ggrs`: drop the
/// `[patch.crates-io]` entry, bump the requirement, and use the released type.
fn sample_ggrs_accumulator_phase(
    timing: Res<bevy_ggrs::GgrsFrameTiming>,
    mut phase: ResMut<ambition_sim_view::PresentationPhase>,
) {
    // `overstep_fraction` reports the accumulator as a fraction of the driver's
    // timestep in `[0, 1)`, and yields 0 before the first driver pass.
    phase.set(timing.overstep_fraction());
}

/// Installs GGRS schedules, snapshot storage, and session/request handling.
pub struct AmbitionRollbackPlugin;

impl Plugin for AmbitionRollbackPlugin {
    fn build(&self, app: &mut App) {
        // ⛔ **THE HOST IS AN AUTHORITY, AND THIS PLUGIN USED TO ASSUME IT.**
        // `register_app_descriptor` ASKS `SimulationHost` before it installs any
        // bevy_ggrs machinery, so a Fixed/render-frame game records the schema
        // and pays for nothing. This plugin asked nothing: it installed GGRS
        // wherever it was added, and the only thing keeping the two halves
        // agreeing was one `if self.host.is_ggrs()` in
        // `PlatformerEnginePlugins::build`.
        //
        // A composition that added this plugin under any other host got a
        // running GGRS session over a world whose registrations were all
        // `RecordedOnly` — every rollback restoring nothing, silently and
        // deterministically. Nothing in the repo does that today; nothing
        // stopped it either.
        //
        // ⭐ and the convention was load-bearing far outside this file: every
        // "that system is in literal `Update`, so it is outside the rollback
        // window" judgement in the schema triage depends on rollback implying
        // `GgrsSchedule`. That reasoning is sound only if this holds, so it is
        // stated here rather than assumed there.
        let host = app.world().get_resource::<crate::SimulationHost>().copied();
        assert_eq!(
            host,
            Some(crate::SimulationHost::Ggrs),
            "AmbitionRollbackPlugin installs GGRS and may only be added to a \
             composition whose simulation host is Ggrs (found {host:?}). Set it \
             with `app.set_simulation_host(SimulationHost::Ggrs)` before adding \
             this plugin — under any other host the rollback registrations are \
             recorded-only, so the session would rewind a world with nothing \
             registered in it."
        );

        app.add_plugins(GgrsPlugin::<AmbitionGgrsConfig>::default())
            .insert_resource(RollbackFrameRate(crate::SIM_TICK_HZ as usize));

        // Publish the rollback host's intra-tick phase for the presented-pose
        // layer. Same set as the fixed-tick sampler `ambition_sim_view` installs
        // for itself — only the clock's hiding place differs.
        //
        // Joining `SamplePhase` is the whole contract: the set is ordered before
        // every resampler by the owning plugin. This used to name ONE resampler
        // with a `.before`, which silently left the feature/actor poses racing
        // the phase they were supposed to be resampled against.
        app.add_systems(
            Update,
            sample_ggrs_accumulator_phase
                .in_set(ambition_sim_view::presented_pose::PresentedPoseStage::SamplePhase),
        );

        // Ambition's gameplay schedule is composed from explicit ordered phase
        // sets, but systems within a phase intentionally rely on deterministic
        // App construction order rather than hundreds of meaningless pairwise
        // edges. GGRS is a managed same-build contract: every peer runs the
        // same plugin graph. Execute that graph serially so conflicting systems
        // cannot race, and disable Bevy's ambiguity diagnostic for this one
        // schedule; the real determinism oracle is SyncTestSession resimulation.
        app.edit_schedule(GgrsSchedule, |schedule| {
            schedule.set_executor_kind(ExecutorKind::SingleThreaded);
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Ignore,
                ..default()
            });
        });

        app.configure_sets(
            bevy_ggrs::LoadWorld,
            AmbitionLoadWorldSet::Reconcile.after(bevy_ggrs::LoadWorldSystems::Mapping),
        )
        .add_systems(
            bevy_ggrs::LoadWorld,
            codecs::reconcile_brain_bindings.in_set(AmbitionLoadWorldSet::Reconcile),
        );

        // ── Per-component restore localization (opt-in) ──
        //
        // Census every registered component's checksum projection at SAVE, and
        // again at LOAD of the same frame, so a divergence NAMES the component
        // instead of naming a frame. Inert unless `RollbackRestoreAudit::enabled`,
        // because censusing every registered type on every save and load is far
        // too expensive to leave on — but installed unconditionally, so a
        // diagnostic session is one resource insert away rather than a rebuild.
        app.add_systems(bevy_ggrs::SaveWorld, probes::record_saved_census);
        app.add_systems(
            bevy_ggrs::LoadWorld,
            probes::compare_restored_census.after(AmbitionLoadWorldSet::Reconcile),
        );
        session::install_session_bridge(app);
    }
}

const ENGINE: &str = "ambition_platformer2d_runtime";

/// The complete engine-owned GGRS rollback registration set. Domain content
/// appends its own entries through [`AmbitionRollbackApp`].
pub fn register_engine_rollback_state(app: &mut App) {
    use ambition_platformer2d_core::body_clusters as bc;
    use AmbitionRollbackApp as _;

    // **DOMAIN ADAPTERS** (Campaign 2). Each owns one gameplay domain's schema;
    // this function aggregates them and stops naming their types. The projectile
    // domain went first — 15 registrations, no reverse dependency, and a state
    // model nothing else writes. `rollback_schema_baseline` is what says the move
    // changed nothing.
    domains::encounter::register(app);
    domains::combat::register(app);
    domains::actors::register(app);
    domains::characters::register(app);
    domains::primitives::register(app);
    domains::vfx::register(app);
    domains::items::register(app);
    domains::portal::register(app);
    domains::projectiles::register(app);
    domains::lifecycle::register(app);
    // Rollback participation. These anchors cover the canonical session root,
    // every simulated body, projectile-only entities, encounter authorities,
    // and any semantic-identity entity that does not fit those families.
    //
    // ⚠ the actor anchors that used to head this chain moved to
    // `domains::actors`; what remains is the primitives-owned set.
    // In-flight strike volumes (moveset melee windows, DamageBox effects,
    // world AOEs). These are Commands-spawned mid-swing with a hit-once
    // set, so they MUST rewind like projectiles: a rollback window that
    // spans the volume's spawn or despawn edge otherwise re-simulates
    // against a fresh empty `HitboxHits` and the same swing hits the same
    // victim twice (the Phase-5 second-hit desync — an armed strike
    // re-staged its player hit on every late resim pass).
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).

    // Canonical live-session root. Authored definitions are immutable and bound
    // by PreparedContentIdentity; only mutable selection/cursor state rewinds.
    //
    // ⚠ the actor-owned members of this group moved to `domains::actors`; the
    // geometry is `ambition_platformer2d_core`'s and stays.
    app.rollback_component_clone::<ambition_platformer2d_core::RoomGeometry>(
        ENGINE,
        "root.geometry",
    );

    // Global authoritative resources.
    app.rollback_resource_canonical::<ambition_time::SimTick>(ENGINE, "resource.sim_tick")
        // **The match activation latch.** (AA2 / AC2)
        //
        // Published from inside the sim schedule on the tick the last seat is
        // filled, and it GATES two behaviours: seating returns early while it
        // exists, and the countdown treats it as proof the match is live. Left
        // unregistered, a rewind across activation restored the fighters — or
        // un-spawned them — and left the latch pointing at a future in which
        // they existed, so seating refused to rebuild the roster it had just
        // lost while the countdown carried on.
        //
        // Correct because `bevy_ggrs` restores ABSENCE as well as value: a
        // rewind to before activation REMOVES this, seating sees no match, and
        // the roster is reconstructed from the same authored inputs.
        //
        // ⚠ **this registration is load-bearing, and there is now a fixture that
        // says so.** Remove it and
        // `rollback_match_activation::a_rewind_across_the_activation_frame_reconstructs_the_same_match`
        // fails on "the restored world was not pre-activation after all", while
        // the two older tests in that file stay green — they cannot reach a
        // pre-activation frame, and their docstring admits it.
        //
        // AA2's lifecycle half is CLOSED as of the seating transaction: seats
        // are resolved and validated before any is built, then constructed in
        // one command flush with this latch, so there is no "between two seats"
        // state a rewind can land in.
        .rollback_resource_canonical::<ambition_time::WorldTime>(ENGINE, "resource.world_time")
        .rollback_resource_canonical::<ambition_platformer2d_world::collision::MovingPlatformSet>(
            ENGINE,
            "resource.moving_platform_set",
        )
        .rollback_resource_clone::<crate::InputStreamRecorder>(
            ENGINE,
            "resource.input_stream_recorder",
        )
        .rollback_resource_canonical::<ambition_time::ClockState>(ENGINE, "resource.clock_state")
        .rollback_resource_clone::<ambition_persistence::save::AmbitionGameSave>(
            ENGINE,
            "resource.sandbox_save",
        )
        .rollback_resource_clone::<ambition_persistence::quest::registry::QuestRegistry>(
            ENGINE,
            "resource.quest_registry",
        )
        // G2b: id → live encounter entity, remapped on every load. A presence
        // probe over a singleton resource sees "still present"; this sees an id
        // pointing at the wrong encounter. Folded in the map's own (sorted) key
        // order, so a permutation between two ids is a difference.
        // G2b: probed through the possessed/home pair's stable identities. A
        // presence probe over a singleton resource sees "still present" and
        // nothing else — and a restore that exchanged the possessed body for the
        // home avatar would invert the whole possession while folding the same
        // census, which is why the ORDER of the pair is folded in.
        // Cross-frame FIFO: produced in `GameplayEffects`, drained in
        // `EncounterSimulation` — which is ordered EARLIER, so the queue is
        // non-empty across a save boundary and a rewind would otherwise replay
        // switch activations the confirmed timeline already applied
        // (deep review 2026-07-19 §2.2).
        // Latent until something mutates them in-session, but a rewind that
        // keeps a predicted faction flip would be a silent desync — registered
        // ahead of the first mutating feature (Phase 5 resource-coverage pass).
        // Cross-frame FIFO: victim-side hits staged in `Combat`, drained by
        // `apply_player_hit_events` in the NEXT frame's `PlayerSimulation` —
        // same shape as `SwitchActivationQueue` above. Found by the Phase-5
        // exit oracle: as a message buffer this was cleared on LoadWorld, so a
        // rewind between the strike and the victim resolver un-hit the player.
        // Checksummed (entity-free projection) so a diverged queue trips the
        // sync test at the staging frame, not a frame later as applied damage.
;

    // Core body state.
    // Provenance and the construction-ownership stamp travel with the entity,
    // so a blob-rebuilt body can still say where it came from — and which room
    // transaction owns it — when nothing around it can. Every planned family
    // carries both since Phase 4; losing the stamp across a rewind would read
    // as OwnershipLost at the next boundary verification.
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).
    app.rollback_component_canonical::<bc::BodyAbilities>(ENGINE, "body.abilities")
        .rollback_component_canonical::<bc::BodyGroundState>(ENGINE, "body.ground")
        .rollback_component_canonical::<bc::BodyWallState>(ENGINE, "body.wall")
        .rollback_component_canonical::<bc::BodyJumpState>(ENGINE, "body.jump")
        .rollback_component_canonical::<bc::BodyDashState>(ENGINE, "body.dash")
        .rollback_component_canonical::<bc::BodyFlightState>(ENGINE, "body.flight")
        .rollback_component_canonical::<bc::BodyBlinkState>(ENGINE, "body.blink")
        .rollback_component_canonical::<bc::BodyDodgeState>(ENGINE, "body.dodge")
        .rollback_component_canonical::<bc::BodyShieldState>(ENGINE, "body.shield")
        .rollback_component_canonical::<bc::BodyOffense>(ENGINE, "body.offense")
        .rollback_component_canonical::<bc::BodyLifetime>(ENGINE, "body.lifetime")
        .rollback_component_canonical::<bc::BodyActionBuffer>(ENGINE, "body.action_buffer")
        .rollback_component_canonical::<bc::BodyBaseSize>(ENGINE, "body.base_size")
        .rollback_component_canonical::<bc::SweepSample>(ENGINE, "body.sweep_sample")
        .rollback_component_canonical::<bc::BodyMana>(ENGINE, "body.mana");
    // In-flight strike volumes — the components on the `entity:hitbox` family
    // (see the require_rollback anchor above). Clone restore + entity mapping;
    // the hit-once sets are dedup truth whose loss re-lands landed hits, and
    // `StrikeVolume` is the owner/window key `retire_orphaned_strike_volumes`
    // reconciles against restored `MovePlayback.live_boxes`.
    // G2b: probed through the OWNER's stable identity, paired with the hitbox's
    // own — the same treatment `ProjectileOwner` has. A strike volume remapped onto
    // the wrong body damages the wrong faction's targets, and a presence count
    // could not tell that from a correct restore.
    // The victims this strike has ALREADY hit. Losing one from the set is a
    // sustained overlap re-hitting a body it already hit, which is exactly the
    // kind of one-frame difference the aggregate reports as a desync with no
    // name attached.
    // G2b: probed through the fired victims' stable identities. A presence
    // count sees the component and nothing of WHO is in the set, so a remap
    // redirecting one victim to the wrong body changes no census — and the
    // visible consequence is a sustained overlap re-firing an on-hit at a body
    // it has already fired at.
    //
    // ⚠ this chain ENDS here: its combat registrations moved to
    // `domains::combat` and the tail it used to flow into is now a separate
    // statement.

    // Actor, combat, and brain state.
    // Armor rows are SPENT by `resolve_body_hit`, so this is mutable combat
    // truth, not authored loadout: without it a rewind re-spends armor that
    // an abandoned future consumed (or keeps armor the confirmed timeline
    // already used up). `WornCharacter` was registered and this was not —
    // an oversight, not a policy split (deep review 2026-07-19 §2.2).
    //
    // ⚠ the character-owned members of this group moved to
    // `domains::characters`; `ActorRoll` is the primitives crate's.
    // A live match's per-body state. Registered together because they are
    // one decision — match activation — landing on a body, and a rewind that
    // kept some and dropped others would produce a fighter that is half in
    // the match (AA2 / AC2, both GPT 5.6 reviews, 2026-07-29).
    // S4 — the stocks loop's own state. A stock count that is NOT rollback
    // state un-spends itself on a rewind: the body comes back and the count
    // does not, so a fighter loses the same stock twice or never loses it at
    // all. Elimination is the same fact one step later, and a rewind that
    // restores a fighter while leaving it eliminated is a body standing in a
    // match nothing will ever let it play.
    // The "already announced" latch for a stocks match's outcome. Registered
    // as STATE rather than left a `Local`, because a rewind across the
    // deciding frame must be able to UN-decide the match — a latch that
    // survives it would swallow the re-announcement on the replay and the
    // ruleset would never hear that the match ended.
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).
    app.rollback_component_canonical::<ambition_platformer2d_core::geometry::CenteredAabb>(
        ENGINE,
        "actor.centered_aabb",
    )
    .rollback_component_canonical::<bc::BodyModeState>(ENGINE, "actor.body_mode")
    .rollback_component_canonical::<bc::BodyLedgeState>(ENGINE, "actor.ledge")
    .rollback_component_canonical::<ambition_platformer2d_core::MotionModel>(
        ENGINE,
        "actor.motion_model",
    )
    .rollback_component_canonical::<bc::BodyComboTrace>(ENGINE, "actor.combo_trace")
    .rollback_component_canonical::<ambition_time::ProperTimeScale>(
        ENGINE,
        "actor.proper_time_scale",
    );

    // Complete rollback entity shapes. The old custom restore engine only
    // patched a narrow state subset and left the remaining components stale.
    // GGRS recreates entities, so every marker, authored/config component, and
    // mutable controller that a recreated actor needs is explicitly stored.
    // The transformation beat's VALUE, not just its participation. The anchor
    // above only installs `bevy_ggrs::Rollback`; without this the beat's
    // `remaining` and — worse — the `was_invulnerable` it borrowed never
    // restore, so a rewind into the middle of a transformation can leave a body
    // permanently untouchable.
    // The REQUEST is state for the same reason. It is necessarily written a
    // frame before it is consumed, and as a message it died in that gap.
    // "A sequence is driving this body." Derived from the sequence that owns it,
    // but derived a phase LATER than it is read — the blanking runs in
    // `PlayerInput` and the death beat that inserts the marker runs in
    // `GameplayEffects` — so a restore that did not carry it would hand the
    // player one live frame in the middle of a scripted sequence.
    // The pose pin the beat and the snake shell both write. Snapshotting the
    // slot itself is owner-agnostic and therefore correct for both: a restore
    // reinstates whatever pin was actually in force. Deriving it from beat state
    // instead would fight the shell for a component it does not own.
    // G2b: a rig IS its slot→limb map, and the map is remapped on every load.
    // A presence count sees "one rig, still here" while the left hand hangs off
    // the right shoulder.
    //
    // The first repair projected `limbs.values()` into the entity-SET census and
    // claimed the slot order came with it. It did not: that census folds targets
    // with a commutative sum, so the two hands trading slots is the same multiset
    // and the same digest — the probe was blind to the one failure the comment
    // named (GPT 5.6, 2026-07-27). The MAP census folds each slot's discriminant
    // against its limb's identity, which is what makes an exchange visible.
    // G2b: which HOST this limb belongs to. Remapped onto the wrong body, the
    // limb station-keeps around a stranger and strikes where that stranger is.
    // G2b: who is riding. A remap that seats the wrong rider locks a body to a
    // mount it never boarded, and the count of occupied slots is unchanged.
    // The UN-GRANTED baseline the live `ActionSet` / `ActorMoveset` are a pure
    // function of (`identity + worn equipment`). Registering the two derived
    // halves and not their base is the `WornEquipment` oversight again: a rewind
    // restored the live kit but left the baseline at whatever an abandoned future
    // derived, so the next `reconcile_equipment_grants` — fired by any armor
    // spend or pickup — recomputed the live kit from the WRONG base and the
    // resimulation stopped matching. That is precisely what
    // `combat_equipment_switch_and_breakable_survive_forced_rollback_identically`
    // caught: it went red when the protagonist's re-rig changed which kit the
    // overlay derives, and stayed red because nothing rewound the base.
    // The MEMO that says the identity baseline above is current for this body.
    //
    // It looks like a cache and is not safe to treat as one. `apply_worn_character_gameplay`
    // re-derives a persona when this record disagrees with the body's worn id or
    // the cast generation — so a rewind that restores an EARLIER `WornCharacter`
    // while leaving this at the abandoned future's id makes the record say
    // "already applied" about a kit the body no longer wears. The derive skips,
    // and the resimulation runs a fighter with somebody else's moves.
    //
    // Exactly the `IdentityKit` oversight one entry up, one level further out:
    // registering a derived value and not the record of what derived it. Found by
    // `every_component_in_a_boss_arena_is_registered_derived_or_waived` within
    // minutes of the component existing (2026-07-29).
    // PROBED over both fields, because a desync here is silent by construction:
    // the wrong baseline does not corrupt a number, it makes the persona derive
    // SKIP — and a presence-only probe would see the component and nothing about
    // which cast it claims. The id is hashed rather than counted so two bodies
    // that swapped identities during a rewound frame do not read as identical.
    // **THE PROJECTION'S OWN MEMO**, and the third time this exact shape has had
    // to be registered rather than assumed derived — after `IdentityKit` and
    // `PersonaBaseline` directly above.
    //
    // `project_prepared_character_definitions` early-exits when this record
    // agrees with the body's worn id and the cast generation, and it also
    // records what it GRANTED so it can retract exactly that. Leave it out of
    // rollback and a rewind restores an earlier `WornCharacter` while the memo
    // still claims the abandoned future's id: the projection skips, and the body
    // resimulates wearing a kit — hurtboxes, movement tuning, sprite-posed body
    // — that its identity no longer asks for. `granted` makes it worse than a
    // stale read, because retraction is driven from it: the wrong record retracts
    // the wrong facts.
    //
    // ⚠ it was ALWAYS unregistered; nothing caught it because no tested room had
    // a body carrying one. Registering the protagonist's own incarnations as
    // characters (2026-07-29) put it on the PLAYER, so it appeared in every room
    // at once and three coverage gates went red together. The component did not
    // become dangerous that day — it became visible.
    //
    // PROBED over the id, the generation AND the grant set, for the reason
    // `PersonaBaseline` is: a desync here is silent by construction. It does not
    // corrupt a number, it makes a derive SKIP, and a presence-only probe would
    // see the component and nothing about what it claims.
    // The quad's placement travels with its size: both are re-derived per pose
    // from the sheet, so restoring one without the other would leave a body
    // drawn at the right scale in the wrong place until the next pose change.
    // The pose→geometry binding itself. Constant per body, but a body the
    // rewind RE-CREATES must come back still bound to its sheet — otherwise it
    // silently reverts to whatever box it was spawned with and never recovers.
    // A body's pose clock ACCUMULATES, and its elapsed value selects which hurtbox
    // keyframe is live -- so a rewind that lost it would resolve a body's damageable
    // silhouette from a different instant than the confirmed timeline did.
    // Authored and immutable at runtime, but bevy_ggrs DESTROYS AND RECREATES
    // rollback entities: unregistered, the doc is simply absent afterwards and the
    // body silently reverts to its sprite-derived compatibility box forever. Same
    // reasoning as `SwitchFeature`.
    // World features that MUTATE during play (deep review 2026-07-19 §2.2).
    // Without these a brick broken in an abandoned future stays broken through
    // the rewind, and the crumble/respawn countdowns resume from predicted
    // values instead of confirmed ones.
    // A chest's PAYLOAD AND STATE, and the marker that says it was opened.
    //
    // `Collected` and `PickupFeature` were both registered; their chest
    // counterparts were not, so a chest opened in an abandoned future kept its
    // reward spent through the rewind — the exact defect the comment above
    // describes for bricks, one feature family over. `Opened` is the same marker
    // class as `PlayerVisual`: bevy_ggrs recreates the entity and an unregistered
    // marker simply does not come back.
    //
    // Found by A19's unswept-population sweep; no room the sweep visited had ever
    // contained a chest.
    // Switch liveness. The `SwitchActivated` MESSAGE is cleared on rollback, but
    // the state that message produced was not rewound — so a switch flipped in an
    // abandoned future stayed on.
    // The switch's authored payload. Immutable at runtime, but bevy_ggrs
    // DESTROYS AND RECREATES rollback entities — anything not registered is
    // simply absent on the recreated entity, so an unregistered authored
    // component silently strips the switch of its identity after a rewind.
    // Same reasoning for the room-visual lifecycle tag: its siblings
    // (`RoomScopedEntity`, `SessionScopedEntity`) are registered, and losing the
    // tag on recreation would leak the entity past its room's teardown.
    //
    // ⚠ this group's character-owned head moved to `domains::characters`.
    // Same reasoning once more, for the tag on the player's body. The portal host
    // asks `With<PlayerVisual>, Without<PortalSceneBody>` to decide what to tag
    // as a portal scene body, so a recreated player that came back without the
    // tag would stop being seen by portal staging entirely.
    // The two pogo CAPABILITY markers, beside the policy and volumes that were
    // already registered. Same reasoning as `PlayerVisual`: bevy_ggrs recreates
    // the entity, and a marker that does not come back silently revokes a
    // capability — `apply_pogo_bounce` gates on `PogoTarget`, and a
    // stand-to-crumble surface's pogo affordance IS `PogoTargetContributor`. A
    // body that stops being bounceable after a rewind is a gameplay divergence
    // that no amount of correct geometry can repair.
    //
    // Found by sweeping rooms nobody had swept before (A19). Their registered
    // siblings sat two lines away this whole time.
    // The ranged sibling, and the pickup/solid-contributor features — found by
    // the combat-calibration-lab coverage sweep (the boot room has no ranged
    // enemy, no pickups, and no breakable, so the boot-room sweep could not
    // see them). Same recreated-entity reasoning as `SwitchFeature` above.
    // The collected latch. Unregistered, a rewind past a collection could not
    // REMOVE it: the resimulated pickup started already-collected, the magnet
    // skipped it (`Without<Collected>`), and its registered `CenteredAabb`
    // froze while the first pass had it moving — the exit oracle's first
    // checksum divergence (combat_calibration_lab, frames 10–12).
    // The mid-toss collection lock (a scattered ring's uncollectible window),
    // registered for the SAME reason `Collected` is: a rewind past the lock's
    // removal must restore it, or the resimulated ring would be collectible a
    // frame early — the magnet/collect guards read it, so it is authoritative.
    // Which sheet a pickup is drawn with. Only a RUNTIME-spawned pickup carries
    // it, and a runtime-spawned pickup is exactly the thing a rewind
    // re-creates — dropping it would leave the resimulated loot invisible while
    // the original was drawn, which is the bug this component exists to fix.
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).
    app.rollback_component_clone::<ambition_platformer2d_core::body_clusters::AbilityBase>(
        ENGINE,
        "body.ability_base",
    )
    // The renderer's runtime-visual DISCOVERY marker. An actor staged outside the
    // authored `RoomSpec` lists (a duel fighter, a runtime-spawned mount) is only
    // given the sprite pipeline because it carries this, so losing it across a
    // restore leaves the actor rendering invisibly for the rest of the session.
    // Same class as `PlayerVisual`, which this instrument caught the same way:
    // presentation, but presentation whose ABSENCE is permanent. Surfaced by the
    // A20 mounted-pair sweep — no swept ROOM stages an actor imperatively.
    // Which provider's bank an entity's cues come out of (G1). For a BODY this is
    // republished every sim tick from its worn character, so it would survive a
    // restore either way — but for a PROJECTILE it is stamped once, at spawn, from a
    // firer that may be dead by the time the bolt lands. Unregistered, bevy_ggrs
    // recreates the bolt without it and the impact reverts to the session's voice
    // for the rest of the shot's life. Same class as `PlayerVisual` and
    // `RuntimeStagedActor`: presentation, but presentation whose ABSENCE is
    // permanent. Probed by value, because the value is the whole fact and a count
    // of "how many entities have a source" says nothing about WHOSE.
    .rollback_component_clone_probed::<ambition_sfx::BodyPresentationSource>(
        ENGINE,
        "presentation.body_source",
        |source| checksum_bytes(source.id().as_str().as_bytes()),
    )
    // The marker that says the per-tick derivation OWNS that source and may retract
    // it. Losing it across a restore would strand a body's source: the derivation
    // stops maintaining what it can no longer recognise as its own.
    .rollback_component_clone::<ambition_sfx::DerivedPresentationSource>(
        ENGINE,
        "presentation.body_source_derived",
    )
    // Portal-gun runtime (deep review 2026-07-19 §2.2). `PortalBody`/`Policy`/
    // `Transit`/`PlacedPortal` were registered but the gun-side state was not,
    // so a rewind could carry a cooldown latch or an in-flight shot in from an
    // abandoned future — permitting or blocking a transit the confirmed
    // timeline never saw.
    // The pickup's ARM TIMER is ticked every sim tick by `arm_portal_pickups`, so a
    // rewind that kept an abandoned future's timer would let the same press that
    // dropped a gun immediately re-grab it — or refuse a grab the confirmed
    // timeline allowed. Surfaced by the coverage sweep only once its population was
    // derived from the rollback vocabulary: a pickup carries neither
    // `FeatureSimEntity` nor `BodyKinematics`.
    .rollback_component_clone::<bevy::prelude::Name>(ENGINE, "entity.name")
    .rollback_component_clone::<bevy::prelude::Transform>(ENGINE, "entity.transform");

    // These values are guaranteed to be republished before any downstream
    // consumer in each GGRS frame, so storing them would duplicate authority.
    // A per-tick MIRROR of the item's own body, not a second authority:
    // `sync_ground_items_to_transitable` overwrites pos/vel/half_extent from the
    // authoritative `GroundItem` (registered state) before portal core reads it, and
    // `sync_transitable_to_ground_items` mirrors the possibly-teleported result
    // straight back. Snapshotting it would give one body two restorable positions.
    //
    // ⚠ this DECLARED-DERIVED group lost its actor-owned head to
    // `domains::actors`; the rest belongs to `ambition_characters`.
    // Recomputed every tick from the authored doc plus the move/pose clocks before
    // anything tests against it, so there is nothing to restore -- and registering
    // it would invite someone to MUTATE it, which is how a hurtbox stops being a
    // pure function of authoritative state (§4.11).
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).
    app.declare_rollback_derived_component::<bevy::prelude::GlobalTransform>(
        ENGINE,
        "derived.global_transform",
        "Bevy transform propagation rebuilds it from Transform and hierarchy",
    )
    // AE6. Derived, not state: `project_combat_rules` rebuilds it in WorldPrep
    // every tick from the match's declaration folded over the world's baseline,
    // both of which outlive any rollback window — the declaration is route
    // lifecycle (`Update`, outside the sim) and the baseline is authored tuning.
    // Registering it as STATE would be the borrow again: a rewind would restore
    // a rules value independently of the declaration that produced it, and the
    // two could then disagree for a frame.
;
    // Scope, projectile, and encounter state.
    // Derived state: one maintenance path, never restore-only repair code.
    // NOTE this justification was wrong until 2026-07-22: it named
    // `ProjectileOwnerId`, which is the firer's raw config id and is EMPTY for
    // every player projectile, so it could not have carried the owner identity
    // for the largest pool in the game. The handle was actually recovered by
    // splitting the projectile's own `SimId` on `/`. It is now recovered from
    // declared provenance, which is what this line always claimed in spirit.
    // ⚠ This was DECLARED DERIVED, on the promise that
    // `heal_projectile_owners` re-resolves it from
    // `SpawnOrigin::Dynamic { parent }`. The promise is not kept: that system's
    // query requires `&SpawnOrigin`, and enemy projectiles carry NONE — measured,
    // `has_origin=false` for every live projectile in the oracle route. So after
    // bevy_ggrs recreated the entity the component was simply gone, the shot's
    // `HitEvent` was emitted with `attacker: None`, and the firer's `ranged` move
    // never learned it connected. That is the equipment oracle's divergence:
    // `MovePlayback.landed_hit` true on three passes and false on the fourth.
    //
    // It is now ordinary rollback state with entity remapping — the same pairing
    // `MovePlayback` uses for its own `live_boxes` handles. A derived declaration
    // is only as good as the system that honours it, and this one names a
    // component the system cannot even see.
    // The boss's SIM-OWNED animation cursor, and the hurtbox sample derived from
    // it. Neither was rollback state, and the coverage sweep never visited a room
    // with a boss in it, so nothing said so. See `rollback_coverage`'s boss-arena
    // sweep, added with this.

    // G2: probed through the OWNER's stable `SimId`, not by counting carriers. The
    // presence probe this used to carry could not tell a correct remap from one that
    // put back the right number of owners and pointed a bolt at the wrong body —
    // which is the failure mode this registration exists to prevent, so the probe
    // was blind to precisely the thing it was added for.
    app    .declare_rollback_derived_component::<ambition_platformer2d_core::body_clusters::BodyEnvironmentContact>(
        ENGINE,
        "derived.body_environment_contact",
        "rewritten every movement step from body geometry and the live world",
    )
    .declare_rollback_derived_component::<ambition_platformer2d_core::BodyMotionFacts>(
        ENGINE,
        "derived.body_motion_facts",
        "republished from MotionModel every movement step",
    )
    .declare_rollback_derived_component::<ambition_sim_view::BodyPoseView>(
        ENGINE,
        "derived.body_pose_view",
        "SimView projection rebuilt every tick",
    )
    .declare_rollback_derived_component::<ambition_sim_view::ProjectileView>(
        ENGINE,
        "derived.projectile_view",
        "SimView projection rebuilt every tick",
    )
    // Frame-derived RESOURCES (Phase 5 resource-coverage pass): each is
    // republished by its ordinary maintenance system before anything reads it,
    // so a rewind that keeps a stale value is overwritten before it matters.
    .declare_rollback_derived_resource::<ambition_platformer2d_core::control_frame::ControlFrame>(
        ENGINE,
        "derived.control_frame",
        "per-tick input frame regenerated from the synchronized input stream",
    )
;

    // Abandoned-future transient ingress must be empty after LoadWorld. Replayed
    // inputs and deterministic systems regenerate the correct messages.
    // S4 — the stocks loop's two messages. Both are written INSIDE the sim
    // schedule, so a rewind that un-happens the KO must un-happen the
    // announcement too: a `BodyKnockedOut` left in the buffer would be re-read on
    // the replay and spend a second stock for one knockout, and a stale
    // `FighterStockSpent` would have a ruleset respawn a fighter that never fell.
    // ⚠ this chain's head moved to a domain adapter (Campaign 2).
    app.clear_message_on_rollback::<ambition_platformer2d_world::rooms::RoomLoaded>(
        ENGINE,
        "message.room_loaded",
    )
    .clear_message_on_rollback::<ambition_platformer2d_world::rooms::RoomTransitionRequested>(
        ENGINE,
        "message.room_transition_requested",
    )
    // A same-tick handshake: the reset processor announces it, and the teardown
    // systems chained after it read it. A cursor GGRS did not rewind would let
    // that teardown fire for a reset the resimulation never committed to — the
    // held items and portals of a session that was, on this timeline, never
    // reset.
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    )
    .clear_message_on_rollback::<ambition_sfx::OwnedSfxMessage>(ENGINE, "message.owned_sfx")
    .clear_message_on_rollback::<ambition_platformer2d_world::rooms::RespawnRoomVisualsRequested>(
        ENGINE,
        "message.respawn_room_visuals",
    )
    // Phase 5 resource-coverage pass: the remaining sim-facing buffers the
    // computed audit surfaced. Same policy as every entry above — empty after
    // LoadWorld, regenerated by replayed inputs.
    .clear_message_on_rollback::<ambition_persistence::quest::QuestAdvanceRequested>(
        ENGINE,
        "message.quest_advance_requested",
    );
}
