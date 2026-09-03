//! Projects body custody from authoritative roots and attachment relations.
//!
//! This module owns `InCustodyOf` for non-item bodies. Possession supplies one
//! custody root; mounts and limbs propagate custody transitively. The schedule
//! exposes completion through `BodyCustodySettled`.

use ambition_characters::actor::limb::Limb;
use bevy::prelude::*;

use crate::abilities::traversal::possession::PossessionState;

/// Re-derive non-item body custody each tick from rollback-authoritative roots.
///
/// Possession is custody, and custody is transitive: a ridden mount follows a
/// rider that is itself traveling, while an independently controlled mount remains
/// room-scoped. This system is the sole owner of `InCustodyOf` for non-item bodies.
///
/// `InCustodyOf` remains derived rollback state because it is reconstructed from
/// registered roots such as `PossessionState`. `GroundItem` is excluded because
/// the item domain owns its custody projection. Writes are change-checked so
/// reconciliation does not create unrewound change-tick churn.
pub fn project_body_custody(
    mut commands: Commands,
    state: Res<PossessionState>,
    riders: Query<(Entity, &ambition_mount::RidingOn)>,
    limbs: Query<(Entity, &Limb)>,
    // `RoomScopedEntity`, NOT `RoomResident`, and the difference is a TICK. `RoomResident`
    // excludes anything wearing `InCustodyOf` — the very marker this system writes — so reading it
    // here makes the rule depend on its own previous output. Asking whether the rider is
    // room-SCOPED is the same question with none of the feedback: a room-scoped rider travels only
    // if THIS pass says so, and a session-scoped one always travels.
    room_scoped: Query<(), With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>>,
    // existence, kept apart from residency: a mount that DIED leaves
    // `RidingOn` dangling by design ("keeping the link record lets the same-room
    // reset path re-mount the rider"), and a dead entity must not be read as
    // "travelling".
    existing: Query<()>,
    held: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf,
        ),
        Without<ambition_held_items::GroundItem>,
    >,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
    use std::collections::BTreeMap;

    // WHO SHOULD BE IN WHOSE CUSTODY THIS TICK.
    //
    // a `BTreeMap` rather than the query's order: this decides component
    // writes that reach a room sweep, and Bevy's iteration order is an archetype
    // accident.
    let mut wanted: BTreeMap<Entity, Entity> = BTreeMap::new();
    if let Some((possessed, home)) = state.possessed.zip(state.home) {
        if existing.get(possessed).is_ok() && existing.get(home).is_ok() {
            wanted.insert(possessed, home);
        }
    }
    // EVERYTHING ATTACHED TO A TRAVELLER TRAVELS, TO ANY DEPTH. The
    // attachments are edges `(attachment → anchor)`; an attachment travels when
    // its anchor does, and an anchor travels when it is already in this pass's
    // set or has no room scope at all (the session-scoped home avatar).
    //
    // a FIXPOINT and not an ordered pass, because the depth is content's to
    // choose. `gnu_ton_arena` authors a boss riding a mount that has hands —
    // three links — and an ordered pass encodes the depth it happened to be
    // written for. Iterating until nothing changes cannot be wrong about a chain
    // somebody authors later. Bounded by the edge count, so it terminates
    // whatever the content says; a cycle simply stops adding.
    //
    // `CapturedBy` is deliberately NOT an edge here. A captive is attached
    // to its captor by exactly this rule, but no composition can express a
    // captor carrying one through a door — capture is the platform fighter's,
    // and a versus stage has no room changes. Adding the edge would be a rule
    // for a state nothing can reach, which is how a carry list grows entries
    // nobody can test.
    let edges: Vec<(Entity, Entity)> = riders
        .iter()
        .map(|(rider, riding)| (riding.mount, rider))
        .chain(limbs.iter().map(|(limb, attached)| (limb, attached.of)))
        .filter(|(attachment, anchor)| {
            existing.get(*attachment).is_ok() && existing.get(*anchor).is_ok()
        })
        .collect();
    loop {
        let mut grew = false;
        for (attachment, anchor) in &edges {
            let anchor_travels = wanted.contains_key(anchor) || room_scoped.get(*anchor).is_err();
            if anchor_travels && wanted.insert(*attachment, *anchor).is_none() {
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }

    for (entity, custody) in &held {
        if wanted.get(&entity) != Some(&custody.0) {
            commands.entity(entity).remove::<InCustodyOf>();
        }
    }
    for (subject, custodian) in wanted {
        if held.get(subject).map(|(_, custody)| custody.0) != Ok(custodian) {
            commands.entity(subject).try_insert(InCustodyOf(custodian));
        }
    }
}
