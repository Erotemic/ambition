//! Universal brain interface.
//!
//! See `brain/README.md`, `docs/systems/brain-driver.md`, and
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
pub mod smash;
pub mod snapshot;
pub mod state_machine;

// Re-exports are the brain module's public surface. Some action-spec variants
// are not exercised by every current consumer in every build target.
#[allow(unused_imports)]
pub use action_set::{
    held_item_by_id, resolve as resolve_action_requests, ActionRequest, ActionSet, BiteSpec,
    HeldItemSpec, HeldUseBehavior, LungeSpec, MeleeActionSpec, MoveStyleSpec, PunchSpec,
    RangedActionSpec, SlamSpec, SpecialActionSpec, SwipeSpec,
};
#[allow(unused_imports)]
pub use boss_pattern::{
    tick_boss_pattern, BossAttackIntent, BossAttackPattern, BossAttackProfile, BossAttackState,
    BossCapability, BossEncounterPhase, BossMacroState, BossMacroTuning, BossMovementFramePolicy,
    BossMovementProfile, BossPattern, BossPatternCfg, BossPatternContext, BossPatternState,
    BossPatternStep, LiveBossAttack,
};
#[allow(unused_imports)]
pub use player::tick_player_brain;
#[allow(unused_imports)]
pub use smash::{
    BroadMode, CrowdingSignal, DifficultyProfile, ObservationFrame, SmashCfg, SmashState,
    SpecificAction, TerrainAwareness,
};
#[allow(unused_imports)]
pub use snapshot::BrainSnapshot;
#[allow(unused_imports)]
pub use state_machine::{
    tick_state_machine, AuthoredWorldPatrolLane, ChargeCrashCfg, ChargeCrashState, MeleeBruteCfg,
    MeleeBruteState, PatrolCfg, PatrolState, SkirmisherCfg, SkirmisherState, SniperCfg,
    SniperState, StateMachineCfg, WandererCfg, NPC_PATROL_SPEED,
};

#[cfg(test)]
use ambition_engine_core as ae;
use bevy::prelude::*;

/// Per-player slot identifier. Slot `0` is the local primary player;
/// future co-op / split-screen / network players will use slots
/// `1..=N`. Stored as a `u8` so it can fit comfortably in a HUD
/// label, a save key, or a debug overlay glyph.
///
/// `PlayerSlot` is the canonical "which player?" handle for new
/// player-bearing messages and resources. New player-domain message
/// types (heal, damage, respawn, cosmetic, …) SHOULD carry either an
/// `Entity` or a `PlayerSlot` so they don't silently assume the
/// primary player.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerSlot(pub u8);

impl PlayerSlot {
    /// Slot reserved for the local primary player in single-player
    /// builds and for player 1 in future local-multiplayer modes.
    pub const PRIMARY: PlayerSlot = PlayerSlot(0);

    pub fn index(self) -> u8 {
        self.0
    }
}

/// The canonical slot-based controller input model: `PlayerSlot -> ControlFrame`.
///
/// This is the SINGLE source of player control. The body that consumes slot
/// `S`'s frame is whichever entity carries [`Brain::Player`]`(S)` — the home
/// avatar, a possessed NPC, or any future controlled body. Local
/// keyboard/gamepad input populates [`PlayerSlot::PRIMARY`]; co-op /
/// split-screen / netcode will fill higher slots via their own adapters.
///
/// Control authority flows THROUGH the brain: nothing reads "the primary
/// player's input" to decide who acts. The universal-brain path looks up this
/// resource by the ticking brain's slot, so possession is just brain transfer —
/// no input-copy component, no possession-specific override.
#[derive(bevy::ecs::resource::Resource, Clone, Copy, Debug, Default)]
pub struct SlotControls {
    slots: [ambition_engine_core::ControlFrame; Self::MAX_SLOTS],
}

impl SlotControls {
    /// Supported controller slots. Bumped when local multiplayer lands.
    pub const MAX_SLOTS: usize = 4;

    /// This slot's current controller frame (neutral for an unfilled slot).
    pub fn get(&self, slot: PlayerSlot) -> ambition_engine_core::ControlFrame {
        self.slots.get(slot.0 as usize).copied().unwrap_or_default()
    }

    /// Publish a slot's controller frame. Out-of-range slots are ignored.
    pub fn set(&mut self, slot: PlayerSlot, frame: ambition_engine_core::ControlFrame) {
        if let Some(entry) = self.slots.get_mut(slot.0 as usize) {
            *entry = frame;
        }
    }
}

/// Identifies which brain backend drives an actor this tick.
///
/// Brains are dispatched via enum match (not trait objects) to keep
/// the per-tick cost a single switch. New backends extend the enum;
/// future variants will include `Remote`, `Scripted`, and `RlPolicy`.
#[derive(Component, Clone, Debug)]
pub enum Brain {
    /// Human player (or in the future, anything that "presses
    /// inputs"). The slot identifies which `PlayerInputFrame` to
    /// read — for now there's only `PlayerSlot(0)`, but the brain
    /// shape is per-slot ready.
    Player(PlayerSlot),
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

    /// Tick the brain: read the snapshot, mutate any internal state,
    /// and write the abstract intent into `out`. Does not consult the
    /// actor's `ActionSet`; the Smash brain falls back to a peaceful
    /// default. Use [`Brain::tick_with_actions`] when the caller knows
    /// the actor's capabilities and wants the Smash brain to commit
    /// actual attacks.
    pub fn tick(
        &mut self,
        snapshot: &BrainSnapshot,
        out: &mut crate::actor::control::ActorControlFrame,
    ) {
        match self {
            Brain::Player(slot) => player::tick_player_brain(*slot, snapshot, out),
            Brain::StateMachine(cfg) => tick_state_machine(cfg, snapshot, out),
        }
    }

    /// Like [`Brain::tick`] but threads the actor's `ActionSet`. The
    /// Smash brain uses this to gate `MeleeAttack` / `RangedAttack`
    /// emission on the actor's actual melee/ranged capability. Other
    /// brain backends ignore the ActionSet.
    /// `perception` is the body's headless `WorldView` (the world-out port). The
    /// Smash brain consumes it for tactical gates (line-of-fire); other backends
    /// ignore it. Pass `None` from pure-stage tests / callers without perception.
    pub fn tick_with_actions(
        &mut self,
        actions: &action_set::ActionSet,
        snapshot: &BrainSnapshot,
        perception: Option<&crate::perception::WorldView>,
        out: &mut crate::actor::control::ActorControlFrame,
    ) {
        match self {
            Brain::Player(slot) => player::tick_player_brain(*slot, snapshot, out),
            Brain::StateMachine(cfg) => state_machine::tick_state_machine_with_actions(
                cfg, actions, snapshot, perception, out,
            ),
        }
    }

    /// Is this brain currently hostile? Debug tooling / "is this
    /// actor a threat right now" queries use this. Player brains are
    /// treated as "hostile" (they attack when they choose to);
    /// state-machine brains delegate to their cfg.
    pub fn is_hostile(&self) -> bool {
        match self {
            Brain::Player(_) => true,
            Brain::StateMachine(cfg) => cfg.is_hostile(),
        }
    }

    /// True iff this brain backend is the player input translator.
    /// Useful for systems that need to special-case the human-driven
    /// path — e.g. multi-player input routing or HUD focus rules.
    pub fn is_player(&self) -> bool {
        matches!(self, Brain::Player(_))
    }

    /// The PlayerSlot this brain reads input for, if any. `None` for
    /// state-machine brains.
    pub fn player_slot(&self) -> Option<PlayerSlot> {
        match self {
            Brain::Player(slot) => Some(*slot),
            Brain::StateMachine(_) => None,
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
    /// `ambition_runtime::snapshot`, which rewinds the boss's clocks, its step
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
            Brain::Player(_) => "player",
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
    /// (rebuild). `Smash` / `BossPattern` / `Player` compare by variant only:
    /// their tuning is derived (sheet metrics) or their runtime state is exactly
    /// what a rewind restores, so a same-variant match must PRESERVE the
    /// cursor-restored brain rather than rebuild a fresh one.
    pub fn same_authored_configuration(&self, other: &Self) -> bool {
        use StateMachineCfg as C;
        match (self, other) {
            (Brain::Player(a), Brain::Player(b)) => a == b,
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
                // Variant-only for the stateful brains (see the doc note).
                (C::Smash { .. }, C::Smash { .. }) => true,
                (C::BossPattern { .. }, C::BossPattern { .. }) => true,
                _ => false,
            },
            _ => false,
        }
    }
}

/// Sibling component holding the actor's last-tick control frame.
/// The brain-driver system writes into this; the integration stage
/// (collision, cooldowns, effects) reads from it. Made a separate
/// component (rather than a field on `Brain`) so brain swaps don't
/// disturb the frame's value mid-tick.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ActorControl(pub crate::actor::control::ActorControlFrame);

/// Module-local Bevy plugin: registers the universal-brain
/// message channel + counter resource. Use this in place of the
/// raw `app.add_message::<ActorActionMessage>() + init_resource`
/// calls so extraction work (e.g. lifting the brain
/// module into its own crate) is a single `app.add_plugins(...)`
/// change at the call site.
///
/// Scheduling of the per-tick systems (tick_player_brains,
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
        // slot; the body carrying `Brain::Player(slot)` reads its frame.
        app.init_resource::<SlotControls>();
    }
}

impl std::fmt::Display for Brain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Brain::Player(slot) => write!(f, "Player(slot={})", slot.0),
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

    /// True iff this message carries a player projectile tick. The
    /// player projectile EFFECTS consumer filters the action stream
    /// with this predicate to drive its charge state machine.
    pub fn is_player_projectile_tick(&self) -> bool {
        matches!(
            self.request,
            action_set::ActionRequest::PlayerProjectileTick { .. }
        )
    }
}

/// Bevy system: walk every actor entity that has a Brain +
/// ActionSet + ActorControl + gameplay ActorPose and emit one
/// `ActorActionMessage` per resolved action request. Runs after the
/// brain-driver systems (tick_player_brains, update_ecs_actors's
/// runtime tick) so the frame is current.
///
/// The resolver intentionally reads `ActorPose` instead of Bevy
/// `Transform`. Feature sim entities use `CenteredAabb` / `ActorPose` as
/// gameplay truth; rendered child/visual entities own presentation
/// transforms with sprite anchors, scaling, and hierarchy concerns.
pub fn emit_brain_action_messages(
    actors: Query<(
        Entity,
        &ActorControl,
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
/// per charge-capable actor per tick. The player projectile EFFECTS
/// consumer (`update_projectiles` (ambition_actors)) drives its
/// motion-recognition buffer + Fireball charge state machine from
/// this stream instead of reading `PlayerInputFrame` directly.
///
/// Emitted every tick — even on neutral input — because the
/// motion-recognition buffer needs continuous axis samples to detect
/// QCF / half-circle gestures (a "down → down-right → right → press"
/// sequence needs samples from every frame of the rotation, not just
/// the press frame). The consumer cheaply pushes the axis sample
/// into the buffer on idle ticks.
pub fn emit_player_projectile_tick_messages(
    actors: Query<(Entity, &ActorControl, Option<&ChargesProjectiles>)>,
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
                axis: frame.locomotion,
                aim: frame.aim,
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
mod tests;
