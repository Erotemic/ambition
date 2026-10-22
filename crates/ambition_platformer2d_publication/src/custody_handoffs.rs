//! What a publication owes the custodian of a hand it emptied.
//!
//! The coordinator's adapter to the item domain: a publication records a
//! handoff when it despawns a predecessor that was in a hand, and drains or
//! discards its own handoffs when it publishes or refuses. What a hand holds
//! and how it is stripped are the item domain's (`HeldItem`, `GroundItem`,
//! `ambition_combat::hand`); the record lives on the publication, so only
//! that publication can create, drain or discard it.

use bevy::prelude::*;

use ambition_characters::brain::ActionSet;
use ambition_combat::held_items::HeldItem;
use ambition_held_items::GroundItem;

// ── A10 MODEL A: custody handoffs ────────────────────────────────────────────

/// What publication must tell the custodian once it has despawned a predecessor
/// that was in a hand.
///
/// ⛔⛤ **THE TWO FACTS THE CUSTODIAN NEEDED FROM THE LIVE ENTITY.**
/// `restore_custody_to_checkpoint` strips a hand by comparing the held item's
/// SPEC ID before removing it — because an equip-swap can leave the body holding
/// something else entirely, and stripping THAT hand would take away an item the
/// reset has no claim on. Both facts are knowable at DECLARATION time, while the
/// predecessor is still standing, which is the whole of why publication can now
/// despawn it (Model A) instead of leaving it for the custodian (Model B).
///
/// ⚠ THE CUSTODIAN IS RECORDED BY `SimId`, NOT BY `Entity`. A death
/// reconstruction can rebuild the body between the declaration and the drain,
/// and an `Entity` recorded across that is a dangling answer of the kind this
/// campaign spent the day removing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustodyHandoff {
    /// The occurrence publication despawned.
    pub occurrence: ambition_platformer2d_shared_tangle::sim_id::SimId,
    /// Whose hand it was in.
    pub custodian: ambition_platformer2d_shared_tangle::sim_id::SimId,
    /// Which item, so the drain strips the right hand.
    pub spec_id: String,
}

/// Handoffs a publication owes the custodian, between its despawn and the drain.
///
/// ⛔⛤ **A COMPONENT ON THE EXACT `RoomPublication`, NEVER A RESOURCE — REVIEW
/// FINDING 1, 2026-09-15.** This was a process-global `Vec` for about an hour,
/// and that is incompatible with A10.1/A10.2: two independent publications can
/// coexist, so `P`'s drain consumed `Q`'s handoffs and `P`'s refusal erased them.
/// The bad interleaving is concrete —
///
/// ```text
/// P records HP;  Q records HQ
/// P refuses   -> a global discard clears HP and HQ
/// Q publishes -> Q despawns its predecessor, no HQ remains
///             -> the holder keeps a HeldItem naming a dead entity
/// ```
///
/// — and the reverse ordering drains a hand before its own publication has even
/// happened. It may be unreachable in a one-active-room runtime; A10.2 exists
/// precisely so that assumption is not load-bearing.
///
/// ⇒ Handoffs sit beside the baseline, the staged world, the effects and the
/// verdict: **only P can create, drain or discard P's.**
#[derive(bevy::prelude::Component, Default, Clone, Debug, PartialEq)]
pub struct CustodyHandoffs(Vec<CustodyHandoff>);

/// Record what the custodian will need, if this occurrence is in a hand.
///
/// Returns `true` when the handoff is recorded ON `publication`, which is the
/// caller's licence to declare `DepartureAuthority::Publication` instead of
/// `Custodian`. `false` keeps Model B for that row — an honest fallback rather
/// than a guess, because a hand this world cannot resolve is one the drain could
/// not strip either.
pub fn record_custody_handoff(
    world: &mut World,
    publication: crate::PublicationHandle,
    occurrence: &ambition_platformer2d_shared_tangle::sim_id::SimId,
    entity: Entity,
) -> bool {
    let Some(custody) =
        world.get::<ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>(entity)
    else {
        return false;
    };
    let custodian_entity = custody.custodian;
    let Some(custodian) = world
        .get::<ambition_platformer2d_shared_tangle::sim_id::SimId>(custodian_entity)
        .cloned()
    else {
        return false;
    };
    let Some(spec_id) = world
        .get::<GroundItem>(entity)
        .map(|ground| ground.spec.id.clone())
    else {
        return false;
    };
    let Ok(mut receipt) = world.get_entity_mut(publication.entity()) else {
        // No receipt, no owner for the handoff: keep Model B for this row.
        return false;
    };
    let mut handoffs = receipt
        .get::<CustodyHandoffs>()
        .cloned()
        .unwrap_or_default();
    handoffs.0.push(CustodyHandoff {
        occurrence: occurrence.clone(),
        custodian,
        spec_id,
    });
    receipt.insert(handoffs);
    true
}

/// Strip every hand THIS publication's despawns left holding a dead entity.
///
/// ⭐ THE SAME SPEC-ID COMPARISON `restore_custody_to_checkpoint` MAKES, and for
/// the same reason: an equip-swap can leave the body holding something else, and
/// stripping that hand would take away an item this publication has no claim on.
pub fn apply_custody_handoffs(
    world: &mut World,
    publication: crate::PublicationHandle,
) -> usize {
    let Ok(mut receipt) = world.get_entity_mut(publication.entity()) else {
        return 0;
    };
    let Some(handoffs) = receipt.take::<CustodyHandoffs>() else {
        return 0;
    };
    let handoffs = handoffs.0;
    if handoffs.is_empty() {
        return 0;
    }
    let mut bodies: std::collections::BTreeMap<
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        Entity,
    > = std::collections::BTreeMap::new();
    let mut query = world.query::<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &ActionSet,
    )>();
    for (entity, sim_id, _) in query.iter(world) {
        bodies.insert(sim_id.clone(), entity);
    }
    let mut stripped = 0;
    for handoff in handoffs {
        let Some(holder) = bodies.get(&handoff.custodian).copied() else {
            // The hand is gone with its body; the occurrence it held is already
            // despawned.
            continue;
        };
        let holds_it = world
            .get::<HeldItem>(holder)
            .is_some_and(|held| held.id() == handoff.spec_id);
        if !holds_it {
            continue;
        }
        ambition_combat::hand::refold_in_world_with_held(world, holder, None);
        world.entity_mut(holder).remove::<HeldItem>();
        stripped += 1;
    }
    stripped
}

/// Forget the handoffs THIS publication recorded: it despawned nothing, so the
/// hands they describe are still correct.
///
/// ⛔ Another publication's handoffs are untouched — the defect this shape exists
/// to make unexpressible.
pub fn discard_custody_handoffs(
    world: &mut World,
    publication: crate::PublicationHandle,
) {
    if let Ok(mut receipt) = world.get_entity_mut(publication.entity()) {
        receipt.remove::<CustodyHandoffs>();
    }
}

#[cfg(test)]
mod custody_handoff_tests {
    use super::*;
    use ambition_characters::brain::HeldItemSpec;
    use ambition_held_items::ItemCustody;
    use ambition_platformer2d_shared_tangle::lifecycle::{CustodyDurability, InCustodyOf};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// A body that can hold something, and the thing it holds.
    fn a_hand_holding(world: &mut World, body: &str, item: &str) -> (Entity, Entity) {
        let holder = world
            .spawn((
                SimId::placement(body),
                ActionSet::default(),
                HeldItem::new(HeldItemSpec {
                    id: item.to_string(),
                    ..HeldItemSpec::default()
                }),
            ))
            .id();
        let occurrence = world
            .spawn((
                SimId::placement(item),
                GroundItem::at_rest(
                    HeldItemSpec {
                        id: item.to_string(),
                        ..HeldItemSpec::default()
                    },
                    Vec2::ZERO,
                    Vec2::ONE,
                ),
                ItemCustody::Held { holder },
                InCustodyOf {
                    custodian: holder,
                    durability: CustodyDurability::Restored,
                },
            ))
            .id();
        (holder, occurrence)
    }

    fn a_publication(world: &mut World) -> crate::PublicationHandle {
        crate::PublicationHandle(world.spawn_empty().id())
    }

    /// ⛔⛤ **REVIEW FINDING 1: ONE PUBLICATION'S HANDOFFS ARE ITS OWN.**
    ///
    /// These lived in a process-global `Vec` for about an hour, which is
    /// incompatible with A10.2 — two independent publications can coexist, so
    /// `P`'s drain consumed `Q`'s handoffs and `P`'s refusal erased them. The
    /// interleaving below is the one that loses a hand: P refuses, Q publishes,
    /// and Q's holder is left with a `HeldItem` naming an entity Q despawned.
    #[test]
    fn a_refused_publication_does_not_erase_another_publications_handoffs() {
        let mut world = World::new();
        let (p_holder, p_item) = a_hand_holding(&mut world, "p_body", "p_item");
        let (q_holder, q_item) = a_hand_holding(&mut world, "q_body", "q_item");
        let p = a_publication(&mut world);
        let q = a_publication(&mut world);

        assert!(record_custody_handoff(
            &mut world,
            p,
            &SimId::placement("p_item"),
            p_item
        ));
        assert!(record_custody_handoff(
            &mut world,
            q,
            &SimId::placement("q_item"),
            q_item
        ));

        // P refuses. Only P's handoff goes.
        discard_custody_handoffs(&mut world, p);
        assert_eq!(
            world.get::<CustodyHandoffs>(p.entity()),
            None,
            "P's refusal did not discard P's own handoffs"
        );
        assert!(
            world.get::<CustodyHandoffs>(q.entity()).is_some(),
            "⛔ P'S REFUSAL ERASED Q'S HANDOFFS. Q will despawn its predecessor \
             and nothing will strip the hand that was holding it"
        );

        // Q publishes: its predecessor is gone and its hand is reconciled.
        world.entity_mut(q_item).despawn();
        assert_eq!(apply_custody_handoffs(&mut world, q), 1);
        assert!(
            world.get::<HeldItem>(q_holder).is_none(),
            "Q's holder kept a `HeldItem` naming the entity Q despawned"
        );
        // ⭐ AND P'S HAND IS UNTOUCHED: P published nothing, so its holder is
        // still correctly holding its item.
        assert!(
            world.get::<HeldItem>(p_holder).is_some(),
            "⛔ Q'S DRAIN STRIPPED P'S HAND"
        );
    }

    /// The inverse ordering: P publishes while Q is still pending.
    #[test]
    fn a_publication_drains_only_its_own_handoffs() {
        let mut world = World::new();
        let (p_holder, p_item) = a_hand_holding(&mut world, "p_body", "p_item");
        let (q_holder, q_item) = a_hand_holding(&mut world, "q_body", "q_item");
        let p = a_publication(&mut world);
        let q = a_publication(&mut world);
        assert!(record_custody_handoff(
            &mut world,
            p,
            &SimId::placement("p_item"),
            p_item
        ));
        assert!(record_custody_handoff(
            &mut world,
            q,
            &SimId::placement("q_item"),
            q_item
        ));

        world.entity_mut(p_item).despawn();
        assert_eq!(apply_custody_handoffs(&mut world, p), 1);
        assert!(world.get::<HeldItem>(p_holder).is_none());
        // Q is still pending: its handoff and its hand are both intact.
        assert!(
            world.get::<CustodyHandoffs>(q.entity()).is_some(),
            "⛔ P'S DRAIN CONSUMED Q'S HANDOFFS"
        );
        assert!(
            world.get::<HeldItem>(q_holder).is_some(),
            "⛔ P'S DRAIN STRIPPED Q'S HAND BEFORE Q PUBLISHED"
        );
        let _ = q_item;
    }
}
