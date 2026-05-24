//! Universal brain interface — see `docs/planning/universal-brain-interface.md`.
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
//! Chunk 2 (this module's introduction) is a *parallel shape*: the
//! brain components are wired into the type system but no actor
//! uses them yet. NPCs/enemies/bosses still run through their
//! existing runtimes. Chunk 3 migrates NpcRuntime; later chunks
//! follow.

pub mod action_set;
pub mod player;
pub mod snapshot;
pub mod state_machine;

pub use action_set::{
    resolve as resolve_action_requests, ActionRequest, ActionSet, BiteSpec, LungeSpec,
    MeleeActionSpec, MoveStyleSpec, PunchSpec, RangedActionSpec, SlamSpec, SpecialActionSpec,
    SwipeSpec,
};
pub use player::{tick_player_brain, tick_player_brain_from_input};
pub use snapshot::{BrainSnapshot, WallContact};
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
}

/// Sibling component holding the actor's last-tick control frame.
/// The brain-driver system writes into this; the integration stage
/// (collision, cooldowns, effects) reads from it. Made a separate
/// component (rather than a field on `Brain`) so brain swaps don't
/// disturb the frame's value mid-tick.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ActorControl(pub ae::ActorControlFrame);

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
pub struct ActorActionMessage {
    /// The actor that wants the action.
    pub actor: Entity,
    /// The concrete action request produced by the actor's
    /// ActionSet.
    pub request: action_set::ActionRequest,
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

/// One-call "tick this brain with a snapshot built from these
/// actor + target positions" helper. Used by every shadow-tick
/// site (`update_ecs_actors` hostile branch, `update_ecs_bosses`)
/// so the snapshot construction lives in one place. Daytime
/// migration tightens this — once a real consumer reads the
/// resulting `ActorControl`, the per-actor brain-driver fills the
/// snapshot's combat-timer / wall-contact fields too.
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
        attack_cooldown_remaining: 0.0,
        attack_windup_remaining: 0.0,
        attack_active_remaining: 0.0,
        attack_recover_remaining: 0.0,
        stun_remaining: 0.0,
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
