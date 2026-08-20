//! **WHO IS CARRYING WHOM, for BODIES** — the one owner of
//! [`InCustodyOf`](ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf)
//! on everything that is not an item.
//!
//! ⭐⭐ **THIS IS A LAW, NOT AN ABILITY, and it lived in one until 2026-08-20.**
//! It grew up inside `abilities/traversal/possession.rs` because possession was
//! the first — and for a while the only — reason a body stopped being resident
//! in its room. It is not the only one now: the rule closes transitively over
//! mounts and limbs to any depth, and a carry, a vehicle, scripted transport or
//! a room-capable capture would each be another ROOT. Leaving it in the
//! possession ability meant every one of those had to modify possession in order
//! to participate, which is feature-centric ownership of a body-generic fact.
//!
//! ⇒ **possession supplies one INPUT here; it does not own the law.** The roots
//! are read at the top of [`project_body_custody`] and the closure below them is
//! shared. Adding a second root is a few lines *in this file*, beside the first,
//! rather than an edit to somebody's ability.
//!
//! ⛔ **deliberately concrete and typed.** There is no registry, no erased
//! callback and no generic attachment graph: the engine has exactly two
//! attachment relations (`RidingOn`, `Limb`) and one root (`PossessionState`),
//! and a framework for three facts is harder to read than the three facts. When a
//! third relation arrives it joins the `edges` list; when the list stops being
//! legible, that is the evidence for abstracting it.
//!
//! ⭐ **the schedule reads
//! [`BodyCustodySettled`](ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
//! not this module.** The item road's residency projection depends on this
//! having run, and it says so by ordering against the set the system carries —
//! so this move cost no reader an edit.

use bevy::prelude::*;

use crate::abilities::traversal::possession::PossessionState;

/// **Re-derive, every tick, which bodies are in whose custody.**
///
/// The ROOTS are read first (today: [`PossessionState`] — possession is custody
/// of a body, and it is one reason a body travels rather than the definition of
/// one), and everything after them is the shared closure.
///
/// ⭐ **possession is custody of a body**, so it uses the same vocabulary a
/// carried object does — `InCustodyOf`, whose own doc says *"the LIFETIME is
/// unchanged, and that is deliberate"* and names *"a possessed actor"* among the
/// custodians. Possession used to express the same fact by SWAPPING the body's
/// lifetime (room scope out, session scope in), which hid it from every query
/// that requires the room scope — including the occurrence ledger's, so a home
/// room re-authored a SECOND copy of the body being driven.
///
/// ⭐⭐ **AND THE RULE IS TRANSITIVE, WHICH IS WHY THIS IS ONE SYSTEM AND NOT
/// TWO** — and, since 2026-08-20, why it is not an ability's either.** A mount is in its RIDER's custody exactly while that rider is itself
/// travelling, so a piloted mount rides through a door with its pilot while an
/// AI-piloted one stays room furniture. ⛔ it was two systems for one afternoon
/// and they FOUGHT: the mount's projection granted the marker in `WorldPrep` and
/// this one retracted it in `PlayerSimulation` on the same tick, because
/// `InCustodyOf` has no field saying who granted it and no structural
/// discriminator separates the populations — every actor carries
/// `TemporaryControl`, and a mount carries `MountSlot` whether ridden or not.
/// ⇒ **one component, one owner**: the whole non-item body population is decided
/// here, in one pass, and the retraction cannot disagree with the grant.
///
/// ⛔⛔ **IT IS A DERIVE AND NOT A FOLLOW-UP CALL, for a rollback reason.**
/// `InCustodyOf` is registered as a DERIVED component on the strength of one
/// sentence — *"room residency reprojected from `ItemCustody` every tick"* — and
/// that sentence is what excuses it from the snapshot. A possessed body has no
/// `ItemCustody`; writing the marker at the possess site would create a
/// population nothing reprojects, and a rewind past the possession would drop it
/// with nothing to put it back. Reading `PossessionState`, which IS rollback
/// state, keeps the excuse true.
///
/// ⚠ **the retraction arm is scoped by `Without<GroundItem>`**, because the item
/// domain owns the marker on objects and reprojects it from its own authority.
///
/// ⚠ **compared before writing**, like its item sibling: an unconditional insert
/// would mark the component changed on every tick of a possession, and change
/// ticks do not rewind.
pub fn project_body_custody(
    mut commands: Commands,
    state: Res<PossessionState>,
    riders: Query<(Entity, &crate::features::RidingOn)>,
    // ⭐ **the third edge kind, and the reason this closes a CLOSURE rather than
    // walking a fixed depth.** A boss rides a mount and that mount has hands:
    // rider → mount → limbs is three links, and `gnu_ton_arena` authors exactly
    // that. Measured before the fix: the rider and mount crossed into
    // `hall_of_bosses` and the mount arrived HANDLESS.
    limbs: Query<(Entity, &crate::features::Limb)>,
    // ⛔ **`RoomScopedEntity`, NOT `RoomResident`, and the difference is a TICK.**
    // `RoomResident` excludes anything wearing `InCustodyOf` — the very marker
    // this system writes — so reading it here makes the rule depend on its own
    // previous output. The chain then converges one tick per link: releasing a
    // rider left its mount in custody for a frame, because the rider's own
    // retraction had not landed yet. Asking whether the rider is room-SCOPED is
    // the same question with none of the feedback: a room-scoped rider travels
    // only if THIS pass says so, and a session-scoped one always travels.
    room_scoped: Query<(), With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>>,
    // ⛔ existence, kept apart from residency: a mount that DIED leaves
    // `RidingOn` dangling by design ("keeping the link record lets the same-room
    // reset path re-mount the rider"), and a dead entity must not be read as
    // "travelling".
    existing: Query<()>,
    held: Query<
        (
            Entity,
            &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf,
        ),
        Without<crate::items::pickup::GroundItem>,
    >,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
    use std::collections::BTreeMap;

    // WHO SHOULD BE IN WHOSE CUSTODY THIS TICK.
    //
    // ⚠ a `BTreeMap` rather than the query's order: this decides component
    // writes that reach a room sweep, and Bevy's iteration order is an archetype
    // accident.
    let mut wanted: BTreeMap<Entity, Entity> = BTreeMap::new();
    if let Some((possessed, home)) = state.possessed.zip(state.home) {
        if existing.get(possessed).is_ok() && existing.get(home).is_ok() {
            wanted.insert(possessed, home);
        }
    }
    // ⭐⭐ **EVERYTHING ATTACHED TO A TRAVELLER TRAVELS, TO ANY DEPTH.** The
    // attachments are edges `(attachment → anchor)`; an attachment travels when
    // its anchor does, and an anchor travels when it is already in this pass's
    // set or has no room scope at all (the session-scoped home avatar).
    //
    // ⛔ **a FIXPOINT and not an ordered pass, because the depth is content's to
    // choose.** `gnu_ton_arena` authors a boss riding a mount that has hands —
    // three links — and an ordered pass encodes the depth it happened to be
    // written for. Iterating until nothing changes cannot be wrong about a chain
    // somebody authors later. Bounded by the edge count, so it terminates
    // whatever the content says; a cycle simply stops adding.
    //
    // ⚠ **`CapturedBy` is deliberately NOT an edge here.** A captive is attached
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
