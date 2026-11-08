//! THE BRAIN DISPATCH, and it lives here because this is the only crate that can
//! see every destination.
//!
//! ⛔⛔ IT USED TO BE `Brain::tick` IN `ambition_characters`, one match over all
//! twelve variants — which pinned behaviour placement to the enum. Three of the
//! arms (`Fighter`, `Smash`, `BossPattern`) are 5,399 lines whose destination is a
//! crate ABOVE the floor, and a dispatcher in the floor crate can never call
//! upward. D168 sat blocked on exactly that.
//!
//! ⭐⭐ THE SPLIT, NOT A MOVE. `ambition_characters` keeps
//! `tick_simple_state_machine` for the nine ordinary NPC arms — its own business,
//! implemented in its own file — and reports whether it answered. This function
//! is the outer match, and it dispatches the three big arms to wherever they end
//! up living. Today they are all still in `ambition_characters`; the point is
//! that moving one is now an edit to THIS file plus a `Cargo.toml`, instead of a
//! design problem.
//!
//! ⭐ AND THE MONOLITH COSTS NOTHING TO PUT IT IN. It already depends on
//! `ambition_characters`, `ambition_combat` and `ambition_boss_encounter`, and
//! the capability-footprint baseline already contains all four — so no closure
//! grows. Measured under D168 before choosing this home.

use ambition_boss_encounter::pattern::tick_boss_pattern_via_state_machine;
use ambition_characters::brain::action_set::ActionSet;
use ambition_characters::brain::state_machine::tick_simple_state_machine;
use ambition_characters::brain::{Brain, BrainSnapshot, StateMachineCfg};
use ambition_characters::perception::WorldView;
use std::borrow::Cow;

/// Tick a brain without the actor's `ActionSet` — the Smash arm falls back to a
/// peaceful default, which is what a caller that does not know the body's
/// capabilities is entitled to.
pub fn tick_brain(
    brain: &mut Brain,
    snapshot: &BrainSnapshot,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    tick_brain_with_actions(brain, &ActionSet::peaceful(), snapshot, None, out);
}

/// Tick a brain, threading the body's capabilities and its world view.
/// Borrowing callers retain their existing semantics. A fighter that buffers
/// the view must clone it because the caller retains ownership.
pub fn tick_brain_with_actions(
    brain: &mut Brain,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    perception: Option<&WorldView>,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    tick_brain_with_view(brain, actions, snapshot, perception.map(Cow::Borrowed), out);
}

/// The production actor update already owns its freshly constructed view.
/// Transfer it into a fighter's perception buffer without cloning its actor,
/// projectile, terrain, portal and string vectors. Non-fighter brains still
/// receive a borrowed view through the same dispatch.
///
/// PERF REVIEW (commit c62122dfe5c05cb66a0c0d130433250976dee2df):
/// The owned-view change was measured against the earlier borrowed-view path
/// in four-CPU Smash captures. `tick_actor_brains` was ~1.012 ms before and
/// ~1.064 ms after; the combat/render schedules also became slower in that
/// capture, so the comparison does NOT establish either a win or a causal
/// regression for this copy removal. A future agent should A/B the two paths
/// with matched match state and profiling overhead, inspecting allocations,
/// `Cow::into_owned`, and delayed-perception history. Keep or revert the
/// owned path based on that evidence, not on the assumption that fewer clones
/// necessarily improves frame time.
pub fn tick_brain_with_actions_owned(
    brain: &mut Brain,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    perception: Option<WorldView>,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    tick_brain_with_view(brain, actions, snapshot, perception.map(Cow::Owned), out);
}

fn tick_brain_with_view(
    brain: &mut Brain,
    actions: &ActionSet,
    snapshot: &BrainSnapshot,
    perception: Option<Cow<'_, WorldView>>,
    out: &mut ambition_characters::actor::control::ActorControlFrame,
) {
    let Brain::StateMachine(sm) = brain;
    // The nine ordinary arms answer for themselves — including the dead-actor
    // case, so there is exactly one place that decides what a corpse emits.
    if tick_simple_state_machine(sm, snapshot, out) {
        return;
    }
    // ⚠ EXHAUSTIVE over what the simple dispatcher declined. A new variant that
    // `tick_simple_state_machine` does not answer is a compile error here rather
    // than a body that silently stops thinking.
    match sm {
        StateMachineCfg::BossPattern { cfg, state } => {
            tick_boss_pattern_via_state_machine(cfg, state, snapshot, out)
        }
        StateMachineCfg::Smash { cfg, state } => ambition_combat::brain::smash::tick_smash(
            cfg, state, actions, snapshot, perception.as_deref(), out,
        ),
        StateMachineCfg::Fighter { cfg, state } => {
            ambition_combat::brain::fighter::tick_fighter_owned(
                cfg,
                state,
                snapshot,
                perception.map(Cow::into_owned),
                out,
            )
        }
        StateMachineCfg::StandStill
        | StateMachineCfg::Patrol { .. }
        | StateMachineCfg::Wanderer { .. }
        | StateMachineCfg::MeleeBrute { .. }
        | StateMachineCfg::Skirmisher { .. }
        | StateMachineCfg::Sniper { .. }
        | StateMachineCfg::ChargeCrash { .. }
        | StateMachineCfg::Aerial { .. }
        | StateMachineCfg::Roam { .. } => {
            unreachable!("the simple dispatcher answers these and returns true")
        }
    }
}
