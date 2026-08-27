//! Universal brain interface.
//!
//! See `brain/README.md`, `docs/systems/actors-brains-and-character-content.md`, and
//! `docs/recipes/extending-brains-and-action-sets.md` for the full navigation map
//! and extension recipe.
//!
//! Every controllable actor carries a [`Brain`]. Each tick the brain reads a
//! [`BrainSnapshot`] and writes intent into
//! [`crate::actor::control::ActorControlFrame`]. Simulation code then consumes
//! that frame uniformly for players, NPCs, enemies, bosses, and future learned
//! or remote policies.
//!
//! Per-entity variety lives in [`ActionSet`]: the brain emits abstract intent
//! such as "melee pressed", and the action set resolves it into the concrete
//! effect for that actor. The resolver emits one [`ActorActionMessage`] per
//! resolved [`action_set::ActionRequest`].

pub mod action_set;
pub mod boss_pattern;
/// The advanced fighter brain (fighter-brain.md): L1's situation classifier today.
pub mod fighter;
pub mod player;
pub mod profile;
pub mod smash;
pub mod snapshot;
pub mod state_machine;

// Re-exports are the brain module's public surface. Some action-spec variants
// are not exercised by every current consumer in every build target.
#[allow(unused_imports)]
pub use action_set::{
    held_item_by_id, held_item_ids, resolve as resolve_action_requests, ActionRequest, ActionSet,
    BiteSpec, HeldItemSpec, HeldUseBehavior, LungeSpec, MeleeActionSpec, MoveStyleSpec,
    ProjectileFlight, PunchSpec, RangedActionSpec, RangedCharge, RangedCommitment, RangedExecution,
    SlamSpec, SpecialActionSpec, SwipeSpec,
};
pub use ambition_entity_catalog::{BrainProfileId, BrainProfileRef};
#[allow(unused_imports)]
pub use boss_pattern::{
    BossAttackIntent, BossAttackPattern, BossAttackProfile, BossAttackState, BossCapability,
    BossEncounterPhase, BossMacroState, BossMacroTuning, BossMovementFramePolicy,
    BossMovementProfile, BossPattern, BossPatternCfg, BossPatternContext, BossPatternState,
    BossPatternStep, LiveBossAttack,
};
#[allow(unused_imports)]
pub use player::tick_player_brain;
pub use profile::BrainProfile;
#[allow(unused_imports)]
// ⛔ THE DATA ONLY. `CrowdingSignal`, `ObservationFrame`, `SpecificAction` and
// `TerrainAwareness` went with the behaviour to `ambition_combat::brain::smash`
// — they are the stages' own vocabulary, not the enum's. What stays is what
// `StateMachineCfg::Smash` names by value and what the snapshot encoder reads.
pub use smash::{BroadMode, DifficultyProfile, SmashCfg, SmashState};
#[allow(unused_imports)]
pub use snapshot::BrainSnapshot;
#[allow(unused_imports)]
pub use state_machine::{
    tick_simple_state_machine, AuthoredWorldPatrolLane, ChargeCrashCfg, ChargeCrashState,
    MeleeBruteCfg, MeleeBruteState, PatrolCfg, PatrolState, SkirmisherCfg, SkirmisherState,
    SniperCfg, SniperState, StateMachineCfg, WandererCfg, NPC_PATROL_SPEED,
};

#[cfg(test)]
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

/// Autonomous policy attached to an actor.
///
/// Driver identity lives in [`crate::control::DrivingParticipant`]; possession
/// does not replace the actor's brain. Enum dispatch keeps per-tick policy
/// selection to a direct match rather than trait-object dispatch.
#[derive(Component, Clone, Debug)]
pub enum Brain {
    /// Pre-canned AI policy template. The variant carries both the
    /// cfg (tuning) and the per-actor runtime state.
    StateMachine(StateMachineCfg),
}

impl Brain {
    /// Construct a `Brain::StateMachine(StandStill)`. Used by spawn
    /// sites that want a no-op AI brain (sandbags, dialogue-only
    /// NPCs).
    pub fn stand_still() -> Self {
        Self::StateMachine(StateMachineCfg::StandStill)
    }

    /// Construct a peaceful NPC patrol brain at the given spawn
    /// position. Convenience wrapper for the spawn-time mapping.
    pub fn npc_patrol(spawn_x: f32, radius: f32) -> Self {
        let mut cfg = PatrolCfg::NPC_DEFAULT;
        cfg.lane = AuthoredWorldPatrolLane::new(spawn_x, radius);
        Self::StateMachine(StateMachineCfg::Patrol {
            cfg,
            state: PatrolState::default(),
        })
    }

    // ⛔⛔ `Brain::tick` AND `Brain::tick_with_actions` ARE GONE, and their
    // absence is the point. They were a match over every variant, so behaviour
    // placement followed the enum: three of the twelve arms are 22k lines of
    // platform-fighter and boss thinking whose destination is a crate ABOVE this
    // one, and a dispatcher living here could never call upward (D168).
    //
    // ⭐ THE SPLIT, not a move. `state_machine::tick_simple_state_machine`
    // answers the nine ordinary NPC arms — this crate's own business — and says
    // so; the composition that owns the whole set dispatches the other three.
    // That is `ambition_platformer2d_actor_monolith::brain_tick`, which already
    // depends on every candidate destination and is in every capability closure
    // anyway.
    //
    // ⚠ measured before doing it: this crate had ZERO production callers of
    // either method. It was an inherent method by habit, not by need.

    /// Is this brain currently hostile? Debug tooling / "is this
    /// actor a threat right now" queries use this. State-machine
    /// brains delegate to their cfg.
    ///
    ///  this answers a question about a POLICY. Whether a body a person is
    /// driving is a threat is a question about the person, and the honest place
    /// to ask it is [`crate::control::DrivingParticipant`] on the body — a driven body's
    /// autonomous policy has no opinion about what its driver is about to do.
    pub fn is_hostile(&self) -> bool {
        match self {
            Brain::StateMachine(cfg) => cfg.is_hostile(),
        }
    }

    /// Read-only access to the actor's `BossPatternState` if this is
    /// a `BossPattern` brain. Returns `None` for every other brain
    /// backend. Convenience for presentation / debug code that needs
    /// the brain's `pattern_timer` clock without match-deconstructing
    /// the variant by hand.
    pub fn boss_pattern_state(&self) -> Option<&boss_pattern::BossPatternState> {
        match self {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => Some(state),
            _ => None,
        }
    }

    /// Mutable access to the actor's `BossPatternState`. For
    /// `ambition_platformer2d_runtime::rollback`, which rewinds the boss's clocks, its step
    /// cursor, and its `rng_seed` — see `SnapshotCursor for Brain`.
    pub fn boss_pattern_state_mut(&mut self) -> Option<&mut boss_pattern::BossPatternState> {
        match self {
            Brain::StateMachine(StateMachineCfg::BossPattern { state, .. }) => Some(state),
            _ => None,
        }
    }

    /// Short label for this brain backend — useful in debug overlays
    /// and trace dumps. Single word per backend.
    pub fn label(&self) -> &'static str {
        match self {
            Brain::StateMachine(cfg) => match cfg {
                StateMachineCfg::StandStill => "stand_still",
                StateMachineCfg::Patrol { .. } => "patrol",
                StateMachineCfg::Wanderer { .. } => "wanderer",
                StateMachineCfg::MeleeBrute { .. } => "melee_brute",
                StateMachineCfg::Skirmisher { .. } => "skirmisher",
                StateMachineCfg::Sniper { .. } => "sniper",
                StateMachineCfg::ChargeCrash { .. } => "charge_crash",
                StateMachineCfg::BossPattern { .. } => "boss_pattern",
                StateMachineCfg::Smash { .. } => "smash",
                StateMachineCfg::Fighter { .. } => "fighter",
                StateMachineCfg::Aerial { .. } => "aerial",
                StateMachineCfg::PlayerDemo { .. } => "player_demo",
            },
        }
    }

    /// Two brains share the same AUTHORED configuration iff they are the same
    /// variant with equal immutable tuning — ignoring mutable runtime state
    /// (patrol/skirmisher cursors, boss/smash clocks and history). This is
    /// finer-grained than [`label`](Self::label): `wanderer_slow` and
    /// `wanderer_fast` both label as `"wanderer"` but differ here.
    ///
    /// Snapshot reconciliation uses it to decide whether a live brain already
    /// matches the brain a restored selection resolves to (leave the ticking
    /// state in place) versus a genuinely different preset in the same family
    /// (rebuild). Every preset-backed variant compares its immutable authored
    /// configuration, ignoring only mutable runtime state (patrol/skirmisher
    /// cursors, boss/smash clocks and history):
    /// - `Smash` compares the full [`SmashCfg`] — two Smash presets differing in
    ///   any authored knob (aggro radius, engage distance, reach, chase/retreat
    ///   speed, difficulty, …) are DISTINCT, so a rewind across such a switch
    ///   rebuilds rather than keeping the future tuning.
    /// - `BossPattern` compares `aggressiveness` + `encounter_id`, which are the
    ///   only authored inputs a `BrainPreset::BossPattern` carries: every other
    ///   `BossPatternCfg` field is DERIVED from `encounter_id` (pattern /
    ///   movement / cycle attacks, via the encounter registry) or captured from
    ///   the live runtime (`spawn` / `combat_size`), so comparing the two
    ///   authored fields is both minimal and complete for the catalog path.
    pub fn same_authored_configuration(&self, other: &Self) -> bool {
        use StateMachineCfg as C;
        match (self, other) {
            (Brain::StateMachine(a), Brain::StateMachine(b)) => match (a, b) {
                (C::StandStill, C::StandStill) => true,
                (C::Patrol { cfg: x, .. }, C::Patrol { cfg: y, .. }) => x == y,
                (C::Wanderer { cfg: x }, C::Wanderer { cfg: y }) => x == y,
                (C::MeleeBrute { cfg: x, .. }, C::MeleeBrute { cfg: y, .. }) => x == y,
                (C::Skirmisher { cfg: x, .. }, C::Skirmisher { cfg: y, .. }) => x == y,
                (C::Sniper { cfg: x, .. }, C::Sniper { cfg: y, .. }) => x == y,
                (C::ChargeCrash { cfg: x, .. }, C::ChargeCrash { cfg: y, .. }) => x == y,
                (C::Aerial { cfg: x, .. }, C::Aerial { cfg: y, .. }) => x == y,
                (C::PlayerDemo { cfg: x, .. }, C::PlayerDemo { cfg: y, .. }) => x == y,
                // The full authored SmashCfg — differing tuning is a different preset.
                (C::Smash { cfg: x, .. }, C::Smash { cfg: y, .. }) => x == y,
                // The authored preset inputs; the rest of the cfg is derived from
                // encounter_id or captured at spawn (see the doc note).
                (C::BossPattern { cfg: x, .. }, C::BossPattern { cfg: y, .. }) => {
                    x.aggressiveness == y.aggressiveness && x.encounter_id == y.encounter_id
                }
                _ => false,
            },
        }
    }
}

/// Module-local Bevy plugin: registers the universal-brain
/// message channel + counter resource. Use this in place of the
/// raw `app.add_message::<ActorActionMessage>() + init_resource`
/// calls so extraction work (e.g. lifting the brain
/// module into its own crate) is a single `app.add_plugins(...)`
/// change at the call site.
///
/// Scheduling of the per-tick systems (tick_controlled_brains,
/// emit_brain_action_messages, observe_brain_action_counter) is
/// still done explicitly in `app/plugins.rs` because they need to
/// chain after sandbox-side input systems — the plugin owns
/// resources, not schedule.
#[derive(Default)]
pub struct BrainPlugin;

impl bevy::app::Plugin for BrainPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_message::<ActorActionMessage>();
        app.init_resource::<BrainActionCounter>();
        // The slot-based controller input model. One entry per participant
        // slot; the body carrying `DrivingParticipant(slot)` reads its frame.
        app.init_resource::<crate::control::SlotControls>();
        //  and the table it is committed FROM. Beside its destination
        // rather than in the host, because a composition that has slots has
        // somewhere for their raw frames to be shaped — the two are one model,
        // and installing them apart is how seat zero ended up with a shaping bus
        // nobody else had.
        app.init_resource::<crate::control::SeatRawFrames>();
    }
}

impl std::fmt::Display for Brain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Brain::StateMachine(_) => write!(f, "StateMachine({})", self.label()),
        }
    }
}

/// Bevy `Message` emitted by the ActionSet resolver — one per
/// concrete action the brain wants this tick. Consumers (combat
/// spawn systems, projectile spawners, special-ability dispatchers)
/// read this to decide what hitboxes / projectiles / FX to spawn.
///
/// Live channel: current consumers include enemy ranged projectiles,
/// enemy melee windup starts, player melee-start gating, GNU-ton
/// apple rain, and Gradient Sentinel boss specials. Pogo and player
/// projectile charge / motion-input handling remain explicit
/// player-specific direct paths.
#[derive(Message, Clone, Debug)]
pub struct ActorActionMessage {
    /// The actor that wants the action.
    pub actor: Entity,
    /// The concrete action request produced by the actor's
    /// ActionSet.
    pub request: action_set::ActionRequest,
}

impl ActorActionMessage {
    /// True iff this message carries a melee request. Cheap
    /// shorthand for `matches!(self.request, ActionRequest::Melee
    /// { .. })`.
    #[allow(dead_code, reason = "filter helper for EFFECTS consumers")]
    pub fn is_melee(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Melee { .. })
    }

    /// True iff this message carries a ranged request.
    pub fn is_ranged(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Ranged { .. })
    }

    /// True iff this message carries a special-ability request.
    #[allow(dead_code, reason = "filter helper for EFFECTS consumers")]
    pub fn is_special(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Special { .. })
    }

    /// True iff this message carries a charge-capable projectile tick. The
    /// body-fire input consumer filters the action stream with this predicate
    /// to drive its charge state machine.
    pub fn is_player_projectile_tick(&self) -> bool {
        matches!(
            self.request,
            action_set::ActionRequest::PlayerProjectileTick { .. }
        )
    }
}

/// Bevy system: walk every actor entity that has a Brain +
/// ActionSet + crate::control::ActorControl + gameplay ActorPose and emit one
/// `ActorActionMessage` per resolved action request. Runs after the
/// brain-driver systems (tick_controlled_brains, update_ecs_actors's
/// runtime tick) so the frame is current.
///
/// The resolver intentionally reads `ActorPose` instead of Bevy
/// `Transform`. Feature sim entities use `CenteredAabb` / `ActorPose` as
/// gameplay truth; rendered child/visual entities own presentation
/// transforms with sprite anchors, scaling, and hierarchy concerns.
pub fn emit_brain_action_messages(
    actors: Query<(
        Entity,
        &crate::control::ActorControl,
        &ActionSet,
        &crate::actor::ActorPose,
        bevy::prelude::Has<MovesetRanged>,
    )>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, action_set, pose, moveset_ranged) in &actors {
        for request in action_set::resolve(action_set, &control.0, pose.origin()) {
            // A body whose ranged shot is a moveset `"ranged"` move fires through the
            // move's timed event (`MoveEventKind::Ranged`), not this flat
            // `frame.fire → Ranged` path — skip the flat emission so it doesn't fire
            // TWICE (the moveset subsumes ranged just as it did melee/specials). The
            // move's fire event re-emits an identical `Ranged` request downstream.
            if moveset_ranged && matches!(request, action_set::ActionRequest::Ranged { .. }) {
                continue;
            }
            writer.write(ActorActionMessage {
                actor: entity,
                request,
            });
        }
    }
}

/// Marker: this body's ranged shot is a data-driven moveset `"ranged"` move (built
/// by `build_actor_moveset` from `ActionSet.ranged`), not the flat
/// `frame.fire → ActionRequest::Ranged` path. `emit_brain_action_messages` skips the
/// flat ranged emission for a body carrying this, so the shot fires once — through
/// the move's timed [`MoveEventKind::Ranged`](ambition_entity_catalog::MoveEventKind)
/// event, which samples live aim and re-emits the same `Ranged` request. The ranged
/// analogue of `MovesetMelee`. `ActionSet.ranged` stays populated (the move dispatch
/// reads the spec + the projectile consumer is unchanged).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MovesetRanged;

/// Capability marker: this actor uses the chargeable-projectile ability — the
/// hold-to-charge / motion-gesture Fireball with its per-frame axis buffer. The
/// projectile-tick stream (`emit_player_projectile_tick_messages`) fires for any
/// actor that carries this, NOT for "the player" — so the mechanic is a per-actor
/// CAPABILITY (pay-for-use, possession-ready), not a property of brain type.
/// Only the player carries it today; a possessed body that adopts the player's
/// kit gets it too. Distinct from an actor's `ActionSet::ranged` slot, which an
/// enemy/boss uses for its OWN (non-chargeable) projectiles.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ChargesProjectiles;

/// Bevy system: emit one `ActorActionMessage::PlayerProjectileTick`
/// per charge-capable actor per tick. The charge-projectile input
/// consumer (`charge_projectile_input` in `ambition_platformer2d_actor_monolith`) drives its
/// motion-recognition buffer + Fireball charge state machine from
/// this stream from the already translated `crate::control::ActorControl` rather than raw slot input.
///
/// Emitted every tick — even on neutral input — because the
/// motion-recognition buffer needs continuous axis samples to detect
/// QCF / half-circle gestures (a "down → down-right → right → press"
/// sequence needs samples from every frame of the rotation, not just
/// the press frame). The consumer cheaply pushes the axis sample
/// into the buffer on idle ticks.
pub fn emit_player_projectile_tick_messages(
    actors: Query<(
        Entity,
        &crate::control::ActorControl,
        Option<&ChargesProjectiles>,
    )>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, charges) in &actors {
        // Capability gate, not an identity gate: emit the charge-tick stream for
        // any actor that carries the chargeable-projectile ability — the player
        // today, a possessed body that adopts the player's kit tomorrow. (Was
        // `brain.is_player()`; bosses/enemies carry a `ranged` ActionSet for their
        // OWN projectiles, so this stays a dedicated opt-in marker, pay-for-use.)
        if charges.is_none() {
            continue;
        }
        let frame = &control.0;
        writer.write(ActorActionMessage {
            actor: entity,
            request: action_set::ActionRequest::PlayerProjectileTick {
                axis: frame.locomotion.vec(),
                aim: frame.aim.vec(),
                press: frame.projectile_pressed,
                held: frame.projectile_held,
                released: frame.projectile_released,
            },
        });
    }
}

/// Resource: per-frame counter of `ActorActionMessage`s observed.
/// EFFECTS consumers uses this to confirm the resolver is
/// actually firing during gameplay before wiring real consumers.
/// HUD / debug tooling can surface it as "brain actions/frame: N".
#[derive(bevy::ecs::resource::Resource, Default, Clone, Copy, Debug)]
pub struct BrainActionCounter {
    /// Total messages observed since last reset (sum across actors).
    pub total: u64,
    /// Messages observed this frame.
    pub last_frame: u32,
}

/// Bevy system: observe the `ActorActionMessage` stream and update
/// the counter. Runs after `emit_brain_action_messages`. Doesn't
/// consume the messages — other readers still see them.
pub fn observe_brain_action_counter(
    mut counter: bevy::ecs::system::ResMut<BrainActionCounter>,
    mut reader: MessageReader<ActorActionMessage>,
) {
    let this_frame = reader.read().count() as u32;
    counter.last_frame = this_frame;
    counter.total = counter.total.wrapping_add(this_frame as u64);
}

/// Bevy system: log each `ActorActionMessage` at debug level using
/// `tracing::debug!`. Gated by the standard tracing filter — set
/// `RUST_LOG=ambition_characters::brain=debug` to see the per-tick
/// resolver output. Useful for EFFECTS-consumer verification
/// without a HUD readout. Not registered by default.
#[allow(dead_code, reason = "diagnostic system; off by default")]
pub fn log_brain_action_messages(mut reader: MessageReader<ActorActionMessage>) {
    for msg in reader.read() {
        bevy::log::debug!(
            target: "ambition_characters::brain",
            "brain action: actor={:?} req={}",
            msg.actor,
            msg.request,
        );
    }
}

#[cfg(test)]
mod slot_gesture_tests;

#[cfg(test)]
mod tests;

/// It is authored vocabulary — `character_archetypes.ron` names these variants — and the content
/// compiler cannot link the actor crate. Generic kit vocabulary: the brain module is the
/// universal-actor abstraction and shouldn't know named enemies, and the runtime brain rebuild
/// (provoke-to-hostile, dismount) must reconstruct a brain from projected data without naming the
/// content archetype enum. Authored per archetype in `character_archetypes.ron` and projected onto
/// [`BrainProfile`] at spawn.
//  `Serialize` because a `BrainProfile` carrying one is now authorable in the
// character catalog, and `CharacterCatalogData` round-trips through serde for
// the content pack. Deserialize alone would have made the new map write-only.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CharacterBrainTemplate {
    /// No motion / no AI — the actor only reacts to events (sandbag's
    /// PunchWeak counter, dialogue-only NPCs that become hostile).
    StandStill,
    /// Surface-walking idle wanderer.
    Wanderer,
    /// Approach-then-strike melee policy. Variety comes from the
    /// per-actor chase_speed / attack_range / aggro_radius in
    /// [`ActorTuning`].
    MeleeBrute,
    /// Strafe-and-fire ranged policy.
    Skirmisher,
    /// Hold position + long-range fire. Like `Skirmisher` but does not
    /// strafe — stationary turret-like enemies.
    Sniper,
    /// Charge-and-crash motion policy: dive at the target, then recover.
    ChargeCrash,
    /// Smash-brawl pipeline: observe → mode → action → difficulty →
    /// emit. See `ambition_characters::brain::smash`.
    Smash,
    /// Lively flyer: an aerial dive-bomber when hostile (stalk → dive →
    /// recover). Shares its code with the peaceful catalog `Aerial` bird via
    /// `StateMachineCfg::Aerial` — hostility is just `aggressiveness > 0`.
    Aerial,
    /// The FB4b fighter brain: L1 classify → L2 options → L3 rollout, on a
    /// human cadence with an APM ceiling and execution noise.
    ///
    /// A match seat travels the archetype path, so a rig reachable only from the catalog was
    /// reachable from everything except a match. Worth stating plainly because the next brain will
    /// need all three too, and nothing currently says so.
    Fighter,
}
