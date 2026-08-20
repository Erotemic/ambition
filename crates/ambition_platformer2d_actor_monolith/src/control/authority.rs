//! **WHO DRIVES THIS BODY** — control authority as its own fact.
//!
//! ⭐⭐ **`Brain::Player(slot)` was carrying TWO meanings and only one of them is
//! a brain.** *"A participant drives this body"* is not an AI backend; it sits in
//! the same enum as `Wanderer` and `BossPattern` because that enum was the only
//! place to say it. Every exhaustive match over `Brain` therefore has an arm for
//! a thing that is not a policy, and — the expensive half — **possession has to
//! MOVE the variant** to change who is driving, which destroys the target's own
//! policy and forces `PossessionState` to stash it in `restore_brain`.
//!
//! ⇒ [`ControlAuthority`] is that fact on its own. It is **DERIVED**, not
//! written at the possess site, for the reason `InCustodyOf` is: a component
//! reprojected every tick from state that IS in the snapshot needs no snapshot
//! entry of its own, and writing it at a decision site would create a population
//! nothing re-derives — a rewind past the possession would drop it with nothing
//! to put it back.
//!
//! ⭐ **the two inputs, and neither is privileged.** A body carrying
//! `Brain::Player(slot)` has that slot's authority; a live possession REDIRECTS
//! the primary slot's authority onto the driven body. Redirect rather than move:
//! the home avatar keeps its player brain and the target keeps its own policy, so
//! releasing needs nothing put back.
//!
//! ⚠ **this slice does not delete `Brain::Player`.** 194 sites name it across 14
//! crates, and the review's instruction was explicit — *"evidence-driven carve;
//! do not redesign the brain stack at once."* What lands here is the SEAM: one
//! component that answers *who drives*, one arbiter that reads it, and a
//! possession that stops swapping policies around to say something it can now say
//! directly.

use bevy::prelude::*;

use ambition_characters::brain::{Brain, PlayerSlot};

use crate::abilities::traversal::possession::PossessionState;

/// **The participant slot driving this body**, this tick.
///
/// ⛔ **not authored and not written by hand** — [`project_control_authority`]
/// owns it, and anything that wants to change who is driving changes one of that
/// system's INPUTS. A second writer is the fork this type exists to prevent.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlAuthority(pub PlayerSlot);

/// Re-derive, every tick, which participant drives which body.
///
/// ⭐ **possession is a REDIRECT, not a move.** The primary slot's authority goes
/// to `PossessionState::possessed` while a possession is live and to the body
/// wearing `Brain::Player(PRIMARY)` otherwise. Nothing is stashed, because
/// nothing was taken away.
///
/// ⚠ **compared before writing**, like every other derive on this road: an
/// unconditional insert marks the component changed every tick of a possession,
/// and change ticks do not rewind.
pub fn project_control_authority(
    mut commands: Commands,
    state: Res<PossessionState>,
    brains: Query<(Entity, &Brain)>,
    existing: Query<()>,
    held: Query<(Entity, &ControlAuthority)>,
) {
    use std::collections::BTreeMap;

    // ⚠ a `BTreeMap` rather than the query's order: this decides component
    // writes a control arbiter reads, and Bevy's iteration order is an archetype
    // accident.
    let mut wanted: BTreeMap<Entity, PlayerSlot> = BTreeMap::new();
    for (entity, brain) in &brains {
        if let Some(slot) = brain.player_slot() {
            wanted.insert(entity, slot);
        }
    }
    // The redirect. Only the PRIMARY slot possesses — see `possession_trigger_system`.
    if let Some(possessed) = state.possessed {
        if existing.get(possessed).is_ok() {
            wanted.retain(|_, slot| *slot != PlayerSlot::PRIMARY);
            wanted.insert(possessed, PlayerSlot::PRIMARY);
        }
    }

    for (entity, authority) in &held {
        if wanted.get(&entity) != Some(&authority.0) {
            commands.entity(entity).remove::<ControlAuthority>();
        }
    }
    for (entity, slot) in wanted {
        if held.get(entity).map(|(_, a)| a.0) != Ok(slot) {
            commands.entity(entity).try_insert(ControlAuthority(slot));
        }
    }
}
