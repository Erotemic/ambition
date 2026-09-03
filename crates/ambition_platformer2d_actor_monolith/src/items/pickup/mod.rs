//! The kernel's residue of the pressed held-item domain.
//!
//! The domain — `GroundItem`, `ItemCustody`, the held specs, pickup / use /
//! throw / physics / residency and their tests — is `ambition_held_items`
//! since 2026-09-03 (D33). What stays here is what is the KERNEL's and not the
//! item's: [`ItemPickupSimulationPlugin`], which chains the three
//! `ItemPickupSet` variants and attaches the kernel's own systems to the
//! domain's `HeldItemStep`s; [`restore_custody_to_checkpoint`], which is
//! checkpoint policy over a foreign crate's components; and
//! [`minted_horizon`], whose one kernel reference (`SaveRestored`) is why it
//! did not move.
//!
//! ⛔ NOTHING IS RE-EXPORTED FROM HERE. A `pub use ambition_held_items::…`
//! would keep this module as the discovery path for code it no longer owns;
//! games reach the domain through the facade's `held_items`.

pub mod minted_horizon;

use bevy::prelude::*;

use ambition_characters::brain::{ActionSet, HeldItemSpec};
use ambition_combat::held_items::HeldItem;
use ambition_held_items::{
    equip_held_spec, held_spec_by_id, unequip_held, GroundItem, ItemCustody,
    StashedActionSet, MINTED_ITEM_HALF_EXTENT,
};
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
use ambition_platformer2d_shared_tangle::schedule::{HeldItemStep, ItemPickupSet, SimScheduleExt};

/// The kernel's half of the item schedule: the two variants whose systems it
/// still owns, the three-variant chain, and its own systems that attach to
/// the held-item domain's steps.
///
/// ⛔ `CoreHeldItems` IS NOT CONFIGURED HERE. Its phase nesting and its custody
/// edge belong to `ambition_held_items::HeldItemSimulationPlugin` (D33: a
/// carved domain's plugin configures its own sets end to end). What this
/// plugin adds to that set is the EDGE to its siblings — the three-variant
/// `.chain()` — because only the kernel names all three; and its own systems,
/// which attach `.before`/`.after` a `HeldItemStep` rather than naming a leaf
/// function of a crate it no longer owns.
pub struct ItemPickupSimulationPlugin;

impl Plugin for ItemPickupSimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The inter-variant chain. `CoreHeldItems` is nested in
        // `PlayerSimulation` by its owner; the two siblings are nested here.
        app.configure_sets(
            sim,
            (
                ItemPickupSet::CoreHeldItems,
                ItemPickupSet::ThrownItemEffects,
                ItemPickupSet::WieldedAbilities,
            )
                .chain(),
        );
        app.configure_sets(
            sim,
            (ItemPickupSet::ThrownItemEffects, ItemPickupSet::WieldedAbilities)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        app.init_resource::<crate::shrine::CheckpointResumeProgress>();
        // ⭐ THE KERNEL'S OWN SYSTEMS ATTACH TO A STEP, they are not links of the
        // domain's chain. Each says where it runs in the domain's vocabulary,
        // which is what lets the domain leave this crate without these edges
        // leaving with it. The order they had in the old single chain is the
        // order these edges reproduce, and the guard pins it by shape.
        app.add_systems(
            sim,
            (
                // Held-items, the portal gun, the heal/save shrine, and localized
                // gravity zones are LDtk-authored room entities. The shrine runs
                // before any hand changes this tick.
                crate::shrine::heal_save_shrine_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                // The other half of the shrine: resume at the checkpoint it
                // recorded. Not gated on `gameplay_allowed` — it must land on the
                // FIRST tick a constructed session has a body, and that tick can
                // fall inside a room transition or a loading frame, which is
                // exactly when gameplay is suspended.
                crate::shrine::restore_checkpoint_on_session_start,
            )
                .chain()
                .in_set(ItemPickupSet::CoreHeldItems)
                .before(HeldItemStep::Release),
        );
        app.add_systems(
            sim,
            // A held gun fires between the generic use step and the throw, as
            // it always did.
            crate::abilities::thrown::puppy_slug_gun::fire_puppy_slug_gun_system
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .in_set(ItemPickupSet::CoreHeldItems)
                .after(HeldItemStep::Use)
                .before(HeldItemStep::Throw),
        );
        app.add_systems(
            sim,
            // WHAT THE MATCH DROPS, before the physics that settles it —
            // so an item spawned this tick falls this tick rather than
            // hanging at its point for one frame.
            crate::items::match_spawn::spawn_match_items
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .in_set(ItemPickupSet::CoreHeldItems)
                .after(HeldItemStep::Throw)
                .before(HeldItemStep::Settle),
        );

        // Bombs and gravity grenades run after the held-item throw/physics group.
        app.add_systems(
            sim,
            (
                crate::abilities::ranged::bomb::arm_thrown_bombs
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::bomb::tick_bomb_fuses
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::thrown::gravity_grenade::arm_thrown_gravity_grenades
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::thrown::gravity_grenade::tick_gravity_grenade_fuses
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                ambition_platformer2d_shared_tangle::gravity::tick_temporary_zones
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
            )
                .chain()
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::ThrownItemEffects` (configured above).
                .in_set(ItemPickupSet::ThrownItemEffects),
        );

        // Wielded movement/combat items live in their own group to avoid the
        // chained tuple arity cap in the core held-item group.
        app.add_systems(
            sim,
            (
                crate::abilities::traversal::mark_recall::mark_recall_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::traversal::blink::blink_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::traversal::grapple::grapple_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::shockwave::fire_shockwave_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::volley::fire_volley_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::beam::fire_beam_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::vortex::fire_vortex_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::vortex::update_vortex_wells
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::sentry::fire_sentry_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::sentry::update_sentries
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::traversal::dive::fire_dive_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::abilities::ranged::meteor::fire_meteor_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated),
                crate::ability_cooldown::tick_ability_cooldown,
            )
                .chain()
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::WieldedAbilities` (configured above).
                .in_set(ItemPickupSet::WieldedAbilities),
        );
    }
}

/// Restore held-item custody to the checkpoint baseline in both directions.
///
/// Items held now but absent from the baseline are unequipped; baseline-held items are
/// re-equipped. Authored occurrences can be rematerialized from room records, while
/// runtime-minted instances use the minted-item baseline. `equip_held_spec`/
/// `unequip_held` are used so action-set state stays coherent. Residency and occurrence
/// ledgers are restored by their own projections/owners.
#[allow(clippy::too_many_arguments)]
pub fn restore_custody_to_checkpoint(
    // `SessionCommands`, because materialization SPAWNS. An occurrence
    // rebuilt into a hand is owned by the activation that is restoring, exactly
    // as the room build's would be; a bare `Commands` could only produce a
    // process-resident stranger that outlives the session.
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    mut resets: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint>,
    baseline: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>>,
    // The world's DEFINITIONS, so an identity with no live occurrence behind
    // it can still be turned back into one. Every room, not the neighbours: a
    // body can carry an object any distance before putting it down, so the room
    // holding the record is not reachable by adjacency.
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    // The checkpoint's own DESCRIPTIONS of what the simulation minted, for
    // the occurrences no record in any room can describe. See
    // [`minted_horizon`]; it is the item domain's third arm of the same
    // baseline, and its population is disjoint from the authored one.
    minted: Option<Res<minted_horizon::MintedItemBaseline>>,
    mut items: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut GroundItem,
        &mut ItemCustody,
    )>,
    mut bodies: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ActionSet,
        Option<&HeldItem>,
        Option<&StashedActionSet>,
    )>,
) {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    // Drained unconditionally, like every other reader of this channel.
    let requested = resets.read().count() > 0;
    let Some(baseline) = baseline else {
        return;
    };
    if !requested {
        return;
    }

    // Bodies by identity, so a baseline row can name the hand it belongs to.
    // a `BTreeMap` rather than the query's order: this drives despawns, and
    // Bevy's iteration order is an archetype accident.
    let by_identity: std::collections::BTreeMap<
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        Entity,
    > = bodies
        .iter()
        .map(|(entity, sim_id, _, _, _)| (sim_id.clone(), entity))
        .collect();

    // Collected first: the loop below borrows `bodies` mutably, and an item's
    // decision needs the whole item view.
    let decisions: Vec<(Entity, Option<Entity>, HeldItemSpec)> = items
        .iter()
        .filter_map(|(entity, occurrence, ground, custody)| {
            let wanted = baseline
                .custodian_of(occurrence)
                .and_then(|custodian| by_identity.get(custodian).copied());
            let now = match *custody {
                ItemCustody::Held { holder } => Some(holder),
                ItemCustody::InWorld => None,
            };
            // Agrees with the checkpoint already, including "in nobody's hands
            // then, in nobody's hands now" — which is the overwhelming majority.
            if wanted == now {
                return None;
            }
            Some((entity, wanted, ground.spec.clone()))
        })
        .collect();

    let (reinstate, retract): (Vec<_>, Vec<_>) = decisions
        .into_iter()
        .partition(|(_, wanted, _)| wanted.is_some());

    for (entity, wanted, spec) in retract.into_iter().chain(reinstate) {
        match wanted {
            // ── the checkpoint saw this in a hand; put it back there ──────────
            Some(holder) => {
                let Ok((_, _, mut action_set, _, _)) = bodies.get_mut(holder) else {
                    continue;
                };
                equip_held_spec(&mut commands, holder, &mut action_set, spec);
                if let Ok((_, _, mut ground, mut custody)) = items.get_mut(entity) {
                    *custody = ItemCustody::Held { holder };
                    // A carried item is not in flight — the same zeroing the
                    // pickup does, and for the same fuse-arming reason.
                    ground.vel = Vec2::ZERO;
                }
            }
            // ── acquired after the checkpoint; take it back ───────────────────
            None => {
                if let ItemCustody::Held { holder } = *items
                    .get(entity)
                    .map(|(_, _, _, custody)| custody)
                    .unwrap_or(&ItemCustody::InWorld)
                {
                    // The hand FIRST, while the object is still here to be
                    // identified by. Compared by SPEC ID for the same reason
                    // `return_released_items` does: an equip-swap can leave the
                    // body holding something else entirely, and stripping THAT
                    // hand would take away an item this reset has no claim on.
                    if let Ok((_, _, mut action_set, held, stashed)) = bodies.get_mut(holder) {
                        if held.is_some_and(|held| held.id() == spec.id.as_str()) {
                            unequip_held(&mut commands, holder, &mut action_set, stashed);
                        }
                    }
                }
                // a DESPAWN, and that is the point rather than a shortcut. The
                // identity lives in the record that minted it, so letting the
                // rebuild author it again produces the SAME `SimId` at the
                // AUTHORED position — which is "the key went back on its
                // pedestal". Moving the live entity would need this system to
                // know where the record puts it, a question
                // `RoomOccurrenceOutlook` already owns.
                ambition_platformer2d_shared_tangle::lifecycle::despawn_scoped_entity(
                    &mut commands,
                    entity,
                );
            }
        }
    }

    // ── AND THE ROWS NO OBJECT IN THE WORLD ANSWERS FOR ──────────────────
    //
    // this pass is driven from the BASELINE, not from the world, and that
    // is the whole difference. Everything above starts at a live occurrence
    // and asks whether the checkpoint agrees with where it is — a question that
    // cannot be asked at all about an occurrence whose entity is gone. Those
    // rows are invisible to every query in the engine, so the only place they
    // exist is the baseline, and the only way to find them is to enumerate it.
    //
    // LAST, after the retractions, for the reason the partition above
    // exists: a body has one hand, and the object being taken out of it must
    // leave before the banked one is put back.
    let live: std::collections::BTreeSet<SimId> = items
        .iter()
        .map(|(_, occurrence, _, _)| occurrence.clone())
        .collect();
    let missing: Vec<(SimId, Entity)> = baseline
        .rows()
        .filter(|(occurrence, _)| !live.contains(*occurrence))
        // A hand this world cannot find is not a hand to put anything back
        // into. The row stays in the baseline, so the next death tries again.
        .filter_map(|(occurrence, custodian)| {
            Some((occurrence.clone(), by_identity.get(custodian).copied()?))
        })
        .collect();
    //  the debt belongs to the ROOM BUILD, which already settles exactly this obligation —
    // `outlook.reinstatements` in `features/ecs/spawn`, relocating an authored record to where
    // the ledger says the object lies. A runtime mint falls through it to a warn because no
    // room authors a record for it.
    if missing.is_empty() {
        return;
    }
    // Resolved once, and only when there is something to materialize: a shell
    // host at a non-gameplay route must not author gameplay entities.
    //
    // The authored arm below asks for the world itself.
    let Some(scope) = commands.spawn_scope() else {
        bevy::log::warn!(
            target: "ambition_platformer2d::items",
            "the checkpoint remembers {} carried occurrence(s) this world cannot rebuild: \
             no spawn scope",
            missing.len(),
        );
        return;
    };
    for (occurrence, holder) in missing {
        // Asking the checkpoint FIRST is not a preference between two answers — the two
        // populations are disjoint by construction, because the capture takes only
        // `SpawnOrigin::Dynamic` rows and an authored record can never spell one.
        let described = minted
            .as_deref()
            .and_then(|minted| minted.description_of(&occurrence));
        let rebuilt = match described {
            // ── the simulation minted it: identity + provenance + spec id ─────
            Some(description) => match held_spec_by_id(&description.held_item) {
                Some(spec) => Some((
                    description.origin.clone(),
                    // NO POSITION IS REMEMBERED AND NONE IS NEEDED. The
                    // hand supplies where the object is, and `ground_item_physics`
                    // refuses to step anything not `InWorld`, so this value is
                    // not read while it is carried. It is the honest answer for
                    // the instant before custody applies, exactly as the
                    // authored arm's authored position is.
                    Vec2::ZERO,
                    MINTED_ITEM_HALF_EXTENT,
                    format!("Ground item: {}", description.held_item),
                    spec,
                )),
                None => {
                    // a CONTENT change: the item's spec has been edited out of
                    // the catalog since the checkpoint was taken.
                    bevy::log::warn!(
                        target: "ambition_platformer2d::items",
                        "the checkpoint remembers minted `{occurrence:?}` in a hand as a \
                         `{}`, and no item spec answers to that id any more",
                        description.held_item,
                    );
                    None
                }
            },
            // ── authored: reach the record BY IDENTITY, not by room ───────────
            //
            // The occurrence is resident nowhere, so no room build is coming for
            // it; what rebuilds it is the record that minted it, found wherever
            // in the world that record lives.
            None => match world
                .as_deref()
                .and_then(|world| {
                    crate::construction::authored_occurrence_request(&world.rooms, &occurrence)
                })
                .as_ref()
                .map(|request| (request, &request.parameters))
            {
                Some((
                    request,
                    crate::construction::ActorConstructionParams::GroundItem { spec, held },
                )) => Some((
                    request.origin.clone(),
                    // Where the record puts it. Never read while the object is
                    // in a hand, and the honest answer for the instant before
                    // custody is applied.
                    spec.pos,
                    spec.half_extent,
                    format!("Ground item: {}", spec.name),
                    held.clone(),
                )),
                Some(_) => {
                    // the family that can be CARRIED and the family that can be
                    // materialized are the same one list; a row for anything else
                    // means a producer joined one road and not the other.
                    bevy::log::warn!(
                        target: "ambition_platformer2d::items",
                        "the checkpoint remembers `{occurrence:?}` in a hand, but its record \
                         does not describe something a body can carry",
                    );
                    None
                }
                None => {
                    bevy::log::warn!(
                        target: "ambition_platformer2d::items",
                        "the checkpoint remembers `{occurrence:?}` in a hand, it carries no \
                         minted description, and no room in this world authors a record that \
                         can rebuild it",
                    );
                    None
                }
            },
        };
        let Some((origin, pos, half_extent, name, held)) = rebuilt else {
            continue;
        };
        let Ok((_, _, mut action_set, _, _)) = bodies.get_mut(holder) else {
            continue;
        };
        // the occurrence's OWN `SimId` and provenance, which is what makes
        // this the same occurrence coming back rather than a copy wearing its
        // name. A fresh identity here would be a silent duplication the moment
        // the home room decided the original was still out there — and for a
        // runtime mint, a rebuilt entity with no `SpawnOrigin::Dynamic` would be
        // invisible to the NEXT capture, so the object would survive exactly one
        // death and then become unrecoverable.
        //
        // `InCustodyOf` is NOT written here, for the same reason the arms
        // above do not write it: it is derived from `ItemCustody` by
        // `project_custody_onto_residency`, later in this same tick and two
        // phases before any room sweep reads it.
        commands.spawn_room_in_session(
            scope,
            (
                occurrence.clone(),
                origin,
                Name::new(name),
                GroundItem {
                    spec: held.clone(),
                    pos,
                    vel: Vec2::ZERO,
                    half_extent,
                },
                ItemCustody::Held { holder },
            ),
        );
        equip_held_spec(&mut commands, holder, &mut action_set, held);
    }
}

#[cfg(test)]
/// THE HELD-ITEM CHAIN, BY SHAPE (D33 rule, `actor-monolith-decomposition.md`).
///
/// The kernel's plugin alone, on a bare `App`: the seven `HeldItemStep`s are
/// a chain inside `CoreHeldItems`, `CoreHeldItems` is inside `PlayerSimulation`
/// and after `BodyCustodySettled`, each step holds exactly the systems it
/// should (counted, not named — Bevy hides names without `bevy_ecs/debug`,
/// and a name lookup passes or fails by who else is in the build), and the
/// three foreign systems that USED to be links of one chain attach to a step
/// by an explicit edge. Poison: delete `.in_set(HeldItemStep::Release)` from
/// `return_released_items` and the Release count reads 0; delete the shrine
/// pair's `.before(HeldItemStep::Release)` and the edge count reads 0.
mod held_item_steps {
    //! The kernel's attachments to the carved domain's chain — the domain's
    //! own steps are pinned in `ambition_held_items::schedule_tests`; here the
    //! kernel's plugin is built beside the domain's, because the attachments
    //! name steps the domain configures.
    use ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled;
    use ambition_platformer2d_shared_tangle::schedule::{
        HeldItemStep, ItemPickupSet, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
    };
    use bevy::app::App;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemSet};

    fn set_key<S: SystemSet + Copy + std::fmt::Debug>(graph: &ScheduleGraph, set: S) -> NodeId {
        NodeId::Set(
            graph
                .system_sets
                .get_key(set.intern())
                .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet")),
        )
    }

    fn direct_system_members<S: SystemSet + Copy + std::fmt::Debug>(
        graph: &ScheduleGraph,
        set: S,
    ) -> usize {
        let set_node = set_key(graph, set);
        graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(set_node, NodeId::System(*key))
            })
            .count()
    }

    /// Systems that are direct members of `inside` AND carry a dependency edge
    /// `after -> system -> before` against the named steps (either may be
    /// `None` for "no edge required on that side").
    fn attached_between(
        graph: &ScheduleGraph,
        after: Option<HeldItemStep>,
        before: Option<HeldItemStep>,
    ) -> usize {
        let core = set_key(graph, ItemPickupSet::CoreHeldItems);
        graph
            .systems
            .iter()
            .filter(|(key, _, _)| {
                let node = NodeId::System(*key);
                graph.hierarchy().graph().contains_edge(core, node)
                    && after.is_none_or(|s| {
                        graph.dependency().graph().contains_edge(set_key(graph, s), node)
                    })
                    && before.is_none_or(|s| {
                        graph.dependency().graph().contains_edge(node, set_key(graph, s))
                    })
            })
            .count()
    }

    fn with_graph(f: impl FnOnce(&ScheduleGraph)) {
        let mut app = App::new();
        app.add_plugins((
            ambition_held_items::HeldItemSimulationPlugin,
            super::ItemPickupSimulationPlugin,
        ));
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        f(schedules.get(sim).expect("the sim schedule exists").graph());
    }

    #[test]
    fn held_item_steps_are_a_chain_and_the_attached_systems_sit_where_they_say() {
        with_graph(|graph| {
            let steps = [
                HeldItemStep::Release,
                HeldItemStep::Pickup,
                HeldItemStep::Use,
                HeldItemStep::Throw,
                HeldItemStep::Settle,
                HeldItemStep::Physics,
                HeldItemStep::Residency,
            ];
            let core = set_key(graph, ItemPickupSet::CoreHeldItems);
            for pair in steps.windows(2) {
                assert!(
                    graph
                        .dependency()
                        .graph()
                        .contains_edge(set_key(graph, pair[0]), set_key(graph, pair[1])),
                    "{:?} -> {:?} must be an explicit edge",
                    pair[0],
                    pair[1]
                );
            }
            for step in steps {
                assert!(
                    graph
                        .hierarchy()
                        .graph()
                        .contains_edge(core, set_key(graph, step)),
                    "{step:?} must be inside CoreHeldItems"
                );
            }
            assert!(
                graph.hierarchy().graph().contains_edge(
                    set_key(graph, Platformer2dSimulationPhaseMonolith::PlayerSimulation),
                    core
                ),
                "CoreHeldItems must be inside PlayerSimulation — outside the phase it runs on \
                 a tick the simulation did not advance"
            );
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(set_key(graph, BodyCustodySettled), core),
                "CoreHeldItems must run after custody settles"
            );
            // The domain's own links, one per step, and the residency triple.
            for (step, expected) in [
                (HeldItemStep::Release, 1),
                (HeldItemStep::Pickup, 1),
                (HeldItemStep::Use, 1),
                (HeldItemStep::Throw, 1),
                (HeldItemStep::Settle, 1),
                (HeldItemStep::Physics, 1),
                (HeldItemStep::Residency, 3),
            ] {
                assert_eq!(
                    direct_system_members(graph, step),
                    expected,
                    "{step:?} holds exactly {expected} system(s)"
                );
            }
            // The three attachments that were links of the old single chain:
            // the shrine pair before Release, the gun between Use and Throw, the
            // match spawn between Throw and Settle.
            assert_eq!(
                attached_between(graph, None, Some(HeldItemStep::Release)),
                2,
                "the shrine pair runs before any hand changes"
            );
            assert_eq!(
                attached_between(graph, Some(HeldItemStep::Use), Some(HeldItemStep::Throw)),
                1,
                "the held gun fires between use and throw"
            );
            assert_eq!(
                attached_between(graph, Some(HeldItemStep::Throw), Some(HeldItemStep::Settle)),
                1,
                "the match spawn drops before the physics that settles it"
            );
        });
    }
}
