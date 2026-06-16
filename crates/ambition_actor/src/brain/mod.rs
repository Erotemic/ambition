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
    tick_boss_pattern, BossAttackPattern, BossAttackProfile, BossAttackState, BossEncounterPhase,
    BossMacroState, BossMacroTuning, BossMovementProfile, BossPattern, BossPatternCfg,
    BossPatternContext, BossPatternState, BossPatternStep, CyclePhase,
};
#[allow(unused_imports)]
pub use player::tick_player_brain;
#[allow(unused_imports)]
pub use smash::{
    BroadMode, CrowdingSignal, DifficultyProfile, ObservationFrame, SmashCfg, SmashState,
    SpecificAction, TerrainAwareness,
};
#[allow(unused_imports)]
pub use snapshot::{BrainSnapshot, WallContact};
#[allow(unused_imports)]
pub use state_machine::{
    tick_state_machine, MeleeBruteCfg, MeleeBruteState, PatrolCfg, PatrolState, SharkCfg,
    SharkState, SkirmisherCfg, SkirmisherState, SniperCfg, SniperState, StateMachineCfg,
    WandererCfg, WandererState, NPC_PATROL_SPEED,
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
        cfg.spawn_x = spawn_x;
        cfg.radius = radius;
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
    pub fn tick_with_actions(
        &mut self,
        actions: &action_set::ActionSet,
        snapshot: &BrainSnapshot,
        out: &mut crate::actor::control::ActorControlFrame,
    ) {
        match self {
            Brain::Player(slot) => player::tick_player_brain(*slot, snapshot, out),
            Brain::StateMachine(cfg) => {
                state_machine::tick_state_machine_with_actions(cfg, actions, snapshot, out)
            }
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
                StateMachineCfg::Shark { .. } => "shark",
                StateMachineCfg::BossPattern { .. } => "boss_pattern",
                StateMachineCfg::Smash { .. } => "smash",
                StateMachineCfg::Aerial { .. } => "aerial",
            },
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
    actors: Query<(Entity, &ActorControl, &ActionSet, &crate::actor::ActorPose)>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, action_set, pose) in &actors {
        for request in action_set::resolve(action_set, &control.0, pose.origin()) {
            writer.write(ActorActionMessage {
                actor: entity,
                request,
            });
        }
    }
}

/// Bevy system: emit one `ActorActionMessage::PlayerProjectileTick`
/// per player-brain actor per tick. The player projectile EFFECTS
/// consumer (`update_projectiles` (ambition_sandbox)) drives its
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
    actors: Query<(Entity, &Brain, &ActorControl)>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, brain, control) in &actors {
        if !brain.is_player() {
            continue;
        }
        let frame = &control.0;
        writer.write(ActorActionMessage {
            actor: entity,
            request: action_set::ActionRequest::PlayerProjectileTick {
                axis: frame.desired_vel,
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
/// `RUST_LOG=ambition_sandbox::brain=debug` to see the per-tick
/// resolver output. Useful for EFFECTS-consumer verification
/// without a HUD readout. Not registered by default.
#[allow(dead_code, reason = "diagnostic system; off by default")]
pub fn log_brain_action_messages(mut reader: MessageReader<ActorActionMessage>) {
    for msg in reader.read() {
        bevy::log::debug!(
            target: "ambition_actor::brain",
            "brain action: actor={:?} req={}",
            msg.actor,
            msg.request,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brain_player_is_always_hostile() {
        let b = Brain::Player(PlayerSlot(0));
        assert!(b.is_hostile());
    }

    #[test]
    fn brain_display_contains_label() {
        // Display impl for state-machine brains embeds the label —
        // a future label rename should automatically reflect in
        // Display output. Pin the relationship.
        for template in [
            StateMachineCfg::StandStill,
            StateMachineCfg::Patrol {
                cfg: PatrolCfg::NPC_DEFAULT,
                state: PatrolState::default(),
            },
            StateMachineCfg::Wanderer {
                cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
                state: WandererState::default(),
            },
        ] {
            let b = Brain::StateMachine(template);
            let display = format!("{}", b);
            let label = b.label();
            assert!(
                display.contains(label),
                "Display '{}' should contain label '{}'",
                display,
                label,
            );
        }
    }

    #[test]
    fn emit_brain_action_messages_skips_entities_missing_components() {
        // Resolver queries Brain + ActionSet + ActorControl +
        // ActorPose. Entities missing any one are skipped silently
        // (Bevy query filter). Pins this behavior so a future
        // refactor that loosens the filter doesn't accidentally
        // process partially-spawned entities and panic on the
        // missing fields.
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, emit_brain_action_messages);
        // Entity 1: missing ActionSet.
        let _e1 = app
            .world_mut()
            .spawn((
                Brain::stand_still(),
                ActorControl::default(),
                crate::actor::ActorPose::default(),
            ))
            .id();
        // Entity 2: missing ActorPose.
        let _e2 = app
            .world_mut()
            .spawn((
                Brain::stand_still(),
                ActorControl::default(),
                ActionSet::peaceful(),
            ))
            .id();
        app.update();
        let messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        assert_eq!(
            messages.iter_current_update_messages().count(),
            0,
            "partial entities should produce zero messages",
        );
    }

    #[test]
    fn emit_brain_action_messages_handles_many_actors() {
        // Stress: 50 actors with Brain + ActionSet + ActorPose all
        // wanting to attack this tick. The resolver should emit
        // 50 messages in one update with no panic or quadratic
        // slowdown.
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, emit_brain_action_messages);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        for i in 0..50 {
            app.world_mut().spawn((
                Brain::stand_still(),
                ActorControl(frame),
                actions.clone(),
                crate::actor::ActorPose {
                    center: ae::Vec2::new(i as f32 * 10.0, 0.0),
                    feet: ae::Vec2::new(i as f32 * 10.0, 24.0),
                    facing: 1.0,
                },
            ));
        }
        app.update();
        let messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let count = messages.iter_current_update_messages().count();
        assert_eq!(count, 50, "expected 50 messages, got {count}");
    }

    #[test]
    fn actor_control_default_is_neutral_frame() {
        // ActorControl Default = frame.neutral. Pins the
        // "fresh-spawn ActorControl has zero intent" baseline so
        // the EFFECTS consumer that reads it before any
        // brain tick has run won't spuriously fire actions.
        let ac = ActorControl::default();
        assert_eq!(ac.0, crate::actor::control::ActorControlFrame::neutral());
        assert!(!ac.0.wants_any_action());
    }

    #[test]
    fn brain_plugin_registers_message_and_counter_resource() {
        // Pins the BrainPlugin contract: installs ActorActionMessage
        // + BrainActionCounter resource. A future refactor that
        // splits the plugin or accidentally drops a registration
        // trips this test.
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_plugins(BrainPlugin);
        // Message resource present.
        let _msg = app
            .world()
            .get_resource::<bevy::ecs::message::Messages<ActorActionMessage>>()
            .expect("ActorActionMessage registered");
        // Counter resource present + default-initialized.
        let counter = app
            .world()
            .get_resource::<BrainActionCounter>()
            .expect("BrainActionCounter registered");
        assert_eq!(counter.total, 0);
        assert_eq!(counter.last_frame, 0);
    }

    #[test]
    fn brain_swap_via_commands_replaces_existing_component() {
        // Pins the runtime brain-swap contract — Bevy's
        // commands.entity(e).insert(Brain) replaces the existing
        // Brain component in place rather than producing a
        // duplicate-component panic or silently ignoring the
        // insert. This is the path damage.rs hostile-flip uses.
        use bevy::prelude::*;
        let mut app = App::new();
        let entity = app
            .world_mut()
            .spawn((Brain::stand_still(), ActorControl::default()))
            .id();
        // Initially StandStill.
        let world = app.world();
        let brain = world.get::<Brain>(entity).expect("Brain attached");
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        // Swap to MeleeBrute via the same commands.insert path.
        app.world_mut()
            .entity_mut(entity)
            .insert(Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            }));
        let brain = app
            .world()
            .get::<Brain>(entity)
            .expect("Brain still attached");
        assert!(matches!(
            brain,
            Brain::StateMachine(StateMachineCfg::MeleeBrute { .. })
        ));
    }

    #[test]
    fn brain_tick_survives_100_ticks_for_every_template() {
        // Smoke test: tick each brain template 100 times with a
        // moving target and verify no panic / NaN propagation /
        // state corruption. Pins that the brain dispatch is safe
        // for a sustained game-length tick run, not just one
        // tick.
        let templates: Vec<StateMachineCfg> = vec![
            StateMachineCfg::StandStill,
            StateMachineCfg::Patrol {
                cfg: PatrolCfg::NPC_DEFAULT,
                state: PatrolState::default(),
            },
            StateMachineCfg::Wanderer {
                cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
                state: WandererState::default(),
            },
            StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            },
            StateMachineCfg::Skirmisher {
                cfg: SkirmisherCfg::RANGER_DEFAULT,
                state: SkirmisherState::default(),
            },
            StateMachineCfg::Sniper {
                cfg: SniperCfg::DEFAULT,
                state: SniperState::default(),
            },
        ];
        for template in templates {
            let mut brain = Brain::StateMachine(template);
            for i in 0..100 {
                let mut snap = BrainSnapshot::idle();
                snap.actor_pos = ae::Vec2::new((i as f32) * 0.5, 0.0);
                snap.target_pos = ae::Vec2::new(100.0 + (i as f32) * 0.5, 0.0);
                snap.sim_time = (i as f32) / 60.0;
                snap.dt = 1.0 / 60.0;
                let mut frame = crate::actor::control::ActorControlFrame::neutral();
                brain.tick(&snap, &mut frame);
                // No NaN propagation.
                assert!(frame.desired_vel.x.is_finite());
                assert!(frame.desired_vel.y.is_finite());
                assert!(frame.facing.is_finite());
            }
        }
    }

    #[test]
    fn brain_tick_is_deterministic_given_same_snapshot() {
        // The brain interface is pure(-ish): same brain + same
        // snapshot → same output (modulo internal state mutation).
        // Pin determinism so RL training + trace replay can rely
        // on reproducibility.
        let snap = BrainSnapshot::idle();
        let mut a = Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        });
        let mut b = a.clone();
        let mut frame_a = crate::actor::control::ActorControlFrame::neutral();
        let mut frame_b = crate::actor::control::ActorControlFrame::neutral();
        a.tick(&snap, &mut frame_a);
        b.tick(&snap, &mut frame_b);
        assert_eq!(frame_a, frame_b, "same brain + same snapshot → same frame");
    }

    // Note: `shadow_tick_brain*` helpers + the `CombatTimers` struct
    // were removed when the hostile/boss runtimes became the
    // single-producer-of-intent path; the tests that pinned their
    // behavior went with them.

    #[test]
    fn brain_display_includes_slot_for_player_and_label_for_state_machine() {
        let p = Brain::Player(PlayerSlot(2));
        assert_eq!(format!("{}", p), "Player(slot=2)");

        let sm = Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        });
        assert_eq!(format!("{}", sm), "StateMachine(melee_brute)");

        let stand = Brain::stand_still();
        assert_eq!(format!("{}", stand), "StateMachine(stand_still)");
    }

    #[test]
    fn brain_stand_still_ctor_matches_variant() {
        let b = Brain::stand_still();
        assert!(matches!(
            b,
            Brain::StateMachine(StateMachineCfg::StandStill)
        ));
        assert!(!b.is_hostile());
        assert_eq!(b.label(), "stand_still");
    }

    #[test]
    fn brain_npc_patrol_ctor_inherits_spawn_and_radius() {
        let b = Brain::npc_patrol(120.0, 40.0);
        match &b {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.spawn_x, 120.0);
                assert_eq!(cfg.radius, 40.0);
                assert_eq!(cfg.aggressiveness, 0.0);
            }
            other => panic!("expected Patrol, got {:?}", other),
        }
        assert!(!b.is_hostile());
    }

    #[test]
    fn brain_is_player_predicate_distinguishes_backends() {
        let p = Brain::Player(PlayerSlot(2));
        assert!(p.is_player());
        assert_eq!(p.player_slot(), Some(PlayerSlot(2)));

        let sm = Brain::StateMachine(StateMachineCfg::StandStill);
        assert!(!sm.is_player());
        assert!(sm.player_slot().is_none());
    }

    #[test]
    fn actor_action_message_predicates_match_request_variant() {
        use bevy::prelude::*;
        // Use World::spawn() to get a real Entity since Bevy 0.18
        // removed Entity::from_raw from the public API.
        let mut world = World::new();
        let actor = world.spawn(()).id();
        let m_melee = ActorActionMessage {
            actor,
            request: ActionRequest::Melee {
                spec: MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT),
                origin: ae::Vec2::ZERO,
                facing: 1.0,
                attack_axis: ae::Vec2::ZERO,
            },
        };
        assert!(m_melee.is_melee());
        assert!(!m_melee.is_ranged());
        assert!(!m_melee.is_special());

        let m_special = ActorActionMessage {
            actor,
            request: ActionRequest::Special {
                spec: SpecialActionSpec::BubbleShield,
            },
        };
        assert!(m_special.is_special());
        assert!(!m_special.is_melee());
    }

    #[test]
    fn brain_label_is_per_backend() {
        assert_eq!(Brain::Player(PlayerSlot(0)).label(), "player");
        assert_eq!(
            Brain::StateMachine(StateMachineCfg::StandStill).label(),
            "stand_still"
        );
        assert_eq!(
            Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default()
            })
            .label(),
            "melee_brute"
        );
        assert_eq!(
            Brain::StateMachine(StateMachineCfg::Wanderer {
                cfg: WandererCfg::PUPPY_SLUG_DEFAULT,
                state: WandererState::default()
            })
            .label(),
            "wanderer"
        );
    }

    #[test]
    fn brain_statemachine_delegates_hostility() {
        let peaceful = Brain::StateMachine(StateMachineCfg::Patrol {
            cfg: PatrolCfg::NPC_DEFAULT,
            state: PatrolState::default(),
        });
        assert!(!peaceful.is_hostile());

        let hostile = Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        });
        assert!(hostile.is_hostile());
    }

    #[test]
    fn brain_tick_dispatches_through_enum() {
        // A StandStill brain should produce a neutral frame.
        let mut b = Brain::StateMachine(StateMachineCfg::StandStill);
        let mut out = crate::actor::control::ActorControlFrame::neutral();
        out.melee_pressed = true; // pre-poisoned
        b.tick(&BrainSnapshot::idle(), &mut out);
        assert!(!out.melee_pressed);
    }

    /// observe_brain_action_counter sums per-frame messages into
    /// the resource. Pins the counter system shape — sandbox wiring
    /// or HUD readouts can rely on `last_frame` reflecting the
    /// resolver's per-frame output count.
    #[test]
    fn observe_brain_action_counter_sums_per_frame_messages() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.init_resource::<BrainActionCounter>();
        app.add_systems(
            Update,
            (emit_brain_action_messages, observe_brain_action_counter).chain(),
        );
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        app.world_mut().spawn((
            Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            }),
            ActorControl(frame),
            actions,
            crate::actor::ActorPose::default(),
        ));
        app.update();
        let counter = app.world().resource::<BrainActionCounter>();
        assert_eq!(counter.last_frame, 1);
        assert_eq!(counter.total, 1);
        // Run another tick — counter accumulates.
        app.update();
        let counter = app.world().resource::<BrainActionCounter>();
        assert_eq!(counter.last_frame, 1);
        assert_eq!(counter.total, 2);
    }

    /// emit_brain_action_messages walks every Brain/ActionSet/
    /// ActorControl + ActorPose entity and writes a message per resolved
    /// ActionRequest. Pins that the resolver system, scheduled in
    /// PlayerInput, observes the brain output correctly.
    #[test]
    fn emit_brain_action_messages_writes_one_message_per_request() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, emit_brain_action_messages);
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        frame.facing = 1.0;
        let actions = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                Brain::StateMachine(StateMachineCfg::MeleeBrute {
                    cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                    state: MeleeBruteState::default(),
                }),
                ActorControl(frame),
                actions,
                crate::actor::ActorPose {
                    center: ae::Vec2::new(50.0, 100.0),
                    feet: ae::Vec2::new(50.0, 124.0),
                    facing: 1.0,
                },
            ))
            .id();
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        assert_eq!(received.len(), 1, "expected one Melee message");
        assert_eq!(received[0].actor, entity);
        match received[0].request.clone() {
            ActionRequest::Melee {
                origin,
                facing,
                spec: MeleeActionSpec::Swipe(_),
                ..
            } => {
                assert_eq!(origin, ae::Vec2::new(50.0, 100.0));
                assert_eq!(facing, 1.0);
            }
            other => panic!("expected Melee::Swipe, got {:?}", other),
        }
    }

    /// End-to-end ranged path: a Skirmisher brain ticking past its
    /// fire cooldown inside aggro range produces `frame.fire`, and
    /// the resolver translates that — gated by the actor's ranged
    /// `ActionSet` — into a concrete `ActionRequest::Ranged`. Pins
    /// the seam shark-rider archetypes rely on: without it, the
    /// `ranged: Some(Bolt(...))` row in `enemy_archetypes.ron` is
    /// silently inert. This test was added when the legacy
    /// choreography path was deleted (which previously kept
    /// shark-riders firing even though their `MeleeBrute` brain
    /// only emitted melee intent — the brain template was switched
    /// to `Skirmisher` in the same wave).
    #[test]
    fn skirmisher_brain_resolves_through_action_set_to_ranged_request() {
        // Inside aggro radius, past cooldown.
        let cfg = SkirmisherCfg::RANGER_DEFAULT;
        let mut brain = Brain::StateMachine(StateMachineCfg::Skirmisher {
            cfg,
            state: SkirmisherState::default(),
        });
        let mut snap = BrainSnapshot::idle();
        snap.actor_pos = ae::Vec2::ZERO;
        snap.target_pos = ae::Vec2::new(200.0, 0.0); // inside aggro 320
        snap.sim_time = 5.0; // past fire_cooldown_s 0.8

        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        brain.tick(&snap, &mut frame);
        assert!(
            frame.fire.is_some(),
            "Skirmisher inside aggro + past cooldown must emit fire intent",
        );

        let kit = ActionSet {
            ranged: Some(RangedActionSpec::Bolt {
                speed: 500.0,
                damage: 2,
            }),
            ..Default::default()
        };
        let req = resolve_action_requests(&kit, &frame, snap.actor_pos);
        assert_eq!(req.len(), 1, "exactly one ranged request");
        match req[0].clone() {
            ActionRequest::Ranged { spec, dir, .. } => {
                assert!(
                    matches!(spec, RangedActionSpec::Bolt { .. }),
                    "spec should come from the Bolt kit",
                );
                assert!(dir.x > 0.0, "fire direction should point at target");
            }
            other => panic!("expected ActionRequest::Ranged, got {:?}", other),
        }
    }

    /// End-to-end: a MeleeBrute brain ticks at attack range; its
    /// emitted frame routes through the actor's ActionSet to a
    /// concrete Melee request. Same brain + different ActionSet =
    /// different concrete attack (Swipe vs Lunge). This is the
    /// possession / multi-body invariant: brains are policy,
    /// ActionSets are capability.
    #[test]
    fn melee_brute_brain_resolves_through_action_set() {
        let cfg = MeleeBruteCfg::STRIKER_DEFAULT;
        let mut brain_a = Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg,
            state: MeleeBruteState::default(),
        });
        let mut brain_b = brain_a.clone();
        let mut snap = BrainSnapshot::idle();
        snap.actor_pos = ae::Vec2::ZERO;
        snap.target_pos = ae::Vec2::new(20.0, 0.0); // in attack range

        let mut frame_a = crate::actor::control::ActorControlFrame::neutral();
        let mut frame_b = crate::actor::control::ActorControlFrame::neutral();
        brain_a.tick(&snap, &mut frame_a);
        brain_b.tick(&snap, &mut frame_b);
        assert!(frame_a.melee_pressed);
        assert!(frame_b.melee_pressed);

        let goblin_kit = ActionSet {
            melee: Some(MeleeActionSpec::Swipe(SwipeSpec::STRIKER_DEFAULT)),
            ..Default::default()
        };
        let brute_kit = ActionSet {
            melee: Some(MeleeActionSpec::Lunge(LungeSpec::BRUTE_DEFAULT)),
            ..Default::default()
        };
        let goblin_req = resolve_action_requests(&goblin_kit, &frame_a, snap.actor_pos);
        let brute_req = resolve_action_requests(&brute_kit, &frame_b, snap.actor_pos);
        assert_eq!(goblin_req.len(), 1);
        assert_eq!(brute_req.len(), 1);
        match (goblin_req[0].clone(), brute_req[0].clone()) {
            (
                ActionRequest::Melee {
                    spec: MeleeActionSpec::Swipe(_),
                    ..
                },
                ActionRequest::Melee {
                    spec: MeleeActionSpec::Lunge(_),
                    ..
                },
            ) => {}
            (a, b) => panic!("expected Swipe vs Lunge, got {:?} vs {:?}", a, b),
        }
    }
}
