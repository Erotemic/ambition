//! Universal brain interface — see
//! [`docs/systems/brain-driver.md`](../../../docs/systems/brain-driver.md)
//! for the overview and
//! [`docs/recipes/extending-brains-and-action-sets.md`](../../../docs/recipes/extending-brains-and-action-sets.md)
//! for the extension recipe.
//!
//! Every controllable actor in the sandbox carries a [`Brain`]. The
//! brain reads a [`BrainSnapshot`] each tick and writes intent into
//! `ae::ActorControlFrame`. The simulation half (collision, cooldowns,
//! effects) consumes the frame uniformly — same code path for
//! players, NPCs, enemies, bosses, and (future) RL agents.
//!
//! Per-entity variety lives in an [`ActionSet`] component on the
//! actor entity. The brain emits abstract intent
//! (`melee_pressed = true`); the ActionSet resolves it into the
//! concrete effect (Swipe vs Lunge vs Bite). Two enemies with the
//! same `Brain::StateMachine(MeleeBrute(…))` can look completely
//! different because their ActionSets differ.
//!
//! **Current shape (2026-05-24):** every actor type (player / NPC /
//! enemy / boss) spawns with Brain + ActionSet + ActorControl
//! sibling components. The brain ticks each frame and fills the
//! frame. The [`emit_brain_action_messages`] resolver writes one
//! [`ActorActionMessage`] per resolved [`action_set::ActionRequest`].
//!
//! **What's NOT wired (daytime continuation):** combat / projectile
//! / FX consumers still read from the legacy `EnemyRuntime` /
//! `BossRuntime` / `update_player` paths. The brain output is a
//! parallel shadow. See the daytime EFFECTS-flip procedure in the
//! recipe doc for the migration plan.

pub mod action_set;
pub mod player;
pub mod snapshot;
pub mod state_machine;

// Re-exports are the brain module's public surface — many show
// as "unused" inside the crate today because the EFFECTS-stage
// consumer flip hasn't landed yet (only #[cfg(test)] code in
// other modules reaches some). Allow that so the surface stays
// documented + ready for daytime wiring.
#[allow(unused_imports)]
pub use action_set::{
    resolve as resolve_action_requests, ActionRequest, ActionSet, BiteSpec, LungeSpec,
    MeleeActionSpec, MoveStyleSpec, PunchSpec, RangedActionSpec, SlamSpec, SpecialActionSpec,
    SwipeSpec,
};
#[allow(unused_imports)]
pub use player::{tick_player_brain, tick_player_brain_from_input};
#[allow(unused_imports)]
pub use snapshot::{BrainSnapshot, WallContact};
#[allow(unused_imports)]
pub use state_machine::{
    tick_state_machine, BossPatternCfg, BossPatternState, MeleeBruteCfg, MeleeBruteState,
    PatrolCfg, PatrolState, SkirmisherCfg, SkirmisherState, SniperCfg, SniperState,
    StateMachineCfg, WandererCfg, WandererState,
};

use ambition_engine as ae;
use bevy::prelude::*;

use crate::player::components::PlayerSlot;

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
    /// and write the abstract intent into `out`.
    pub fn tick(&mut self, snapshot: &BrainSnapshot, out: &mut ae::ActorControlFrame) {
        match self {
            Brain::Player(slot) => player::tick_player_brain(*slot, snapshot, out),
            Brain::StateMachine(cfg) => tick_state_machine(cfg, snapshot, out),
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
                StateMachineCfg::BossPattern { .. } => "boss_pattern",
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
pub struct ActorControl(pub ae::ActorControlFrame);

/// Module-local Bevy plugin: registers the universal-brain
/// message channel + counter resource. Use this in place of the
/// raw `app.add_message::<ActorActionMessage>() + init_resource`
/// calls so daytime extraction work (e.g. lifting the brain
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

/// Bevy `Message` emitted by the ActionSet resolver — one per
/// concrete action the brain wants this tick. Consumers (combat
/// spawn systems, projectile spawners, special-ability dispatchers)
/// read this to decide what hitboxes / projectiles / FX to spawn.
///
/// Today only an observation channel — the player's existing
/// combat pipeline still drives hitbox spawns via update_player +
/// the projectile system. Daytime work flips those consumers off
/// the legacy paths and onto this message stream.
#[derive(Message, Clone, Copy, Debug)]
#[allow(dead_code, reason = "fields read by daytime EFFECTS-consumer flip + test code")]
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
    #[allow(dead_code, reason = "filter helper for daytime EFFECTS-flip consumers")]
    pub fn is_melee(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Melee { .. })
    }

    /// True iff this message carries a ranged request.
    #[allow(dead_code, reason = "filter helper for daytime EFFECTS-flip consumers")]
    pub fn is_ranged(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Ranged { .. })
    }

    /// True iff this message carries a special-ability request.
    #[allow(dead_code, reason = "filter helper for daytime EFFECTS-flip consumers")]
    pub fn is_special(&self) -> bool {
        matches!(self.request, action_set::ActionRequest::Special { .. })
    }
}

/// Bevy system: walk every actor entity that has a Brain +
/// ActionSet + ActorControl and emit one `ActorActionMessage` per
/// resolved action request. Runs after the brain-driver systems
/// (tick_player_brains, update_ecs_actors shadow tick) so the
/// frame is current.
pub fn emit_brain_action_messages(
    actors: Query<(Entity, &ActorControl, &ActionSet, &bevy::transform::components::Transform)>,
    mut writer: MessageWriter<ActorActionMessage>,
) {
    for (entity, control, action_set, transform) in &actors {
        let origin = ae::Vec2::new(transform.translation.x, transform.translation.y);
        for request in action_set::resolve(action_set, &control.0, origin) {
            writer.write(ActorActionMessage {
                actor: entity,
                request,
            });
        }
    }
}

/// Resource: per-frame counter of `ActorActionMessage`s observed.
/// Daytime EFFECTS-flip work uses this to confirm the resolver is
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

/// Combat-timer state passed into a shadow tick so the brain's
/// AI evaluator sees correct windup / active / recover / cooldown
/// values. Use `CombatTimers::CLEAR` for actors that don't track
/// attack state (NPCs, sandbags).
#[derive(Clone, Copy, Debug, Default)]
pub struct CombatTimers {
    pub cooldown_remaining: f32,
    pub windup_remaining: f32,
    pub active_remaining: f32,
    pub recover_remaining: f32,
    pub stun_remaining: f32,
}

impl CombatTimers {
    /// No active attack / stun — all zeros. The brain template
    /// reads "can begin attack windup" as true.
    pub const CLEAR: Self = Self {
        cooldown_remaining: 0.0,
        windup_remaining: 0.0,
        active_remaining: 0.0,
        recover_remaining: 0.0,
        stun_remaining: 0.0,
    };
}

/// One-call "tick this brain with a snapshot built from these
/// actor + target positions" helper. Used by every shadow-tick
/// site (`update_ecs_actors` hostile branch, `update_ecs_bosses`)
/// so the snapshot construction lives in one place. Daytime
/// migration tightens this — once a real consumer reads the
/// resulting `ActorControl`, the per-actor brain-driver fills the
/// snapshot's combat-timer / wall-contact fields too.
///
/// Default variant uses `CombatTimers::CLEAR` — see
/// [`shadow_tick_brain_with_timers`] for the variant that passes
/// real attack-timer values.
#[allow(clippy::too_many_arguments, reason = "intentional flat helper; the snapshot it builds is what's deduped")]
pub fn shadow_tick_brain(
    brain: &mut Brain,
    actor_pos: ae::Vec2,
    actor_vel: ae::Vec2,
    actor_facing: f32,
    actor_on_ground: bool,
    alive: bool,
    target_pos: ae::Vec2,
    dt: f32,
) -> ae::ActorControlFrame {
    shadow_tick_brain_with_timers(
        brain,
        actor_pos,
        actor_vel,
        actor_facing,
        actor_on_ground,
        alive,
        target_pos,
        dt,
        CombatTimers::CLEAR,
    )
}

/// Like [`shadow_tick_brain`] but threads real combat timers into
/// the snapshot. Use this from actor systems that track windup /
/// active / recover / cooldown (e.g. the enemy shadow tick — its
/// EnemyRuntime carries those timers and they let the brain
/// correctly emit Telegraph / Attack / Recover modes).
#[allow(clippy::too_many_arguments, reason = "intentional flat helper")]
pub fn shadow_tick_brain_with_timers(
    brain: &mut Brain,
    actor_pos: ae::Vec2,
    actor_vel: ae::Vec2,
    actor_facing: f32,
    actor_on_ground: bool,
    alive: bool,
    target_pos: ae::Vec2,
    dt: f32,
    timers: CombatTimers,
) -> ae::ActorControlFrame {
    let snap = BrainSnapshot {
        actor_pos,
        actor_vel,
        actor_facing,
        actor_on_ground,
        alive,
        target_pos,
        target_alive: true,
        sim_time: 0.0,
        dt,
        attack_cooldown_remaining: timers.cooldown_remaining,
        attack_windup_remaining: timers.windup_remaining,
        attack_active_remaining: timers.active_remaining,
        attack_recover_remaining: timers.recover_remaining,
        stun_remaining: timers.stun_remaining,
        wall_contact: None,
        player_input: None,
    };
    let mut out = ae::ActorControlFrame::neutral();
    brain.tick(&snap, &mut out);
    out
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
    fn shadow_tick_brain_handles_dead_actors_without_emitting_intent() {
        // shadow_tick_brain on a dead actor should emit a neutral
        // frame regardless of brain template — pins the "dead
        // actors don't move or attack" rule across the helper.
        for template in [
            StateMachineCfg::StandStill,
            StateMachineCfg::Patrol {
                cfg: PatrolCfg::NPC_DEFAULT,
                state: PatrolState::default(),
            },
            StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            },
            StateMachineCfg::Skirmisher {
                cfg: SkirmisherCfg::RANGER_DEFAULT,
                state: SkirmisherState::default(),
            },
        ] {
            let mut brain = Brain::StateMachine(template);
            let frame = shadow_tick_brain(
                &mut brain,
                ae::Vec2::ZERO,
                ae::Vec2::ZERO,
                1.0,
                true,
                false, // alive = false
                ae::Vec2::new(20.0, 0.0),
                1.0 / 60.0,
            );
            assert_eq!(frame.desired_vel, ae::Vec2::ZERO, "dead actor should not move");
            assert!(!frame.melee_pressed, "dead actor should not attack");
            assert!(frame.fire.is_none(), "dead actor should not fire");
        }
    }

    #[test]
    fn shadow_tick_with_timers_routes_active_attack_to_brain_mode() {
        // A MeleeBrute brain ticked with an active attack timer
        // should see CharacterAiMode::Attack and emit no fresh
        // melee_pressed (the integration is mid-attack). Pins the
        // CombatTimers → BrainSnapshot threading.
        let mut brain = Brain::StateMachine(StateMachineCfg::MeleeBrute {
            cfg: MeleeBruteCfg::STRIKER_DEFAULT,
            state: MeleeBruteState::default(),
        });
        let timers = CombatTimers {
            cooldown_remaining: 0.5,
            windup_remaining: 0.0,
            active_remaining: 0.05, // mid-swing
            recover_remaining: 0.0,
            stun_remaining: 0.0,
        };
        let frame = shadow_tick_brain_with_timers(
            &mut brain,
            ae::Vec2::ZERO,
            ae::Vec2::ZERO,
            1.0,
            true,
            true,
            ae::Vec2::new(20.0, 0.0),
            1.0 / 60.0,
            timers,
        );
        // Mid-active: brain should NOT re-emit melee_pressed
        // because a swing is already in progress. The integration
        // half tracks the active hitbox.
        assert!(!frame.melee_pressed);
        // And the MeleeBrute state's cached mode should reflect
        // CharacterAiMode::Attack — the engine evaluator returns
        // Attack when active_remaining > 0.
        if let Brain::StateMachine(StateMachineCfg::MeleeBrute { state, .. }) = &brain {
            assert_eq!(state.mode, ae::CharacterAiMode::Attack);
        } else {
            unreachable!();
        }
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
        let mut out = ae::ActorControlFrame::neutral();
        out.melee_pressed = true; // pre-poisoned
        b.tick(&BrainSnapshot::idle(), &mut out);
        assert!(!out.melee_pressed);
    }

    /// observe_brain_action_counter sums per-frame messages into
    /// the resource. Pins the counter system shape — daytime work
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
        let mut frame = ae::ActorControlFrame::neutral();
        frame.melee_pressed = true;
        app.world_mut().spawn((
            Brain::StateMachine(StateMachineCfg::MeleeBrute {
                cfg: MeleeBruteCfg::STRIKER_DEFAULT,
                state: MeleeBruteState::default(),
            }),
            ActorControl(frame),
            actions,
            bevy::transform::components::Transform::IDENTITY,
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
    /// ActorControl entity and writes a message per resolved
    /// ActionRequest. Pins that the resolver system, scheduled in
    /// PlayerInput, observes the brain output correctly.
    #[test]
    fn emit_brain_action_messages_writes_one_message_per_request() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.add_message::<ActorActionMessage>();
        app.add_systems(Update, emit_brain_action_messages);
        let mut frame = ae::ActorControlFrame::neutral();
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
                Transform::from_xyz(50.0, 100.0, 0.0),
            ))
            .id();
        app.update();
        let mut messages = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<ActorActionMessage>>();
        let received: Vec<_> = messages.drain().collect();
        assert_eq!(received.len(), 1, "expected one Melee message");
        assert_eq!(received[0].actor, entity);
        match received[0].request {
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

        let mut frame_a = ae::ActorControlFrame::neutral();
        let mut frame_b = ae::ActorControlFrame::neutral();
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
        match (goblin_req[0], brute_req[0]) {
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
