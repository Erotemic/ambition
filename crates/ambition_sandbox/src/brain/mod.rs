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
}
