//! Item schedule integration and checkpoint custody restoration.
//!
//! Held-item transitions belong to `ambition_held_items`. This module orders the
//! item capabilities and restores custody from the admitted checkpoint data.

pub mod minted_horizon;
pub use minted_horizon::{GrantSource, PickupGranted, RewardGrant, RewardGrantsSinceCheckpoint};

use bevy::prelude::*;

use ambition_characters::brain::HeldItemSpec;
use ambition_combat::hand::RepertoireQuery;
use ambition_held_items::{
    equip_held_spec, held_spec_by_id, unequip_held, GroundItem, ItemCustody,
    MINTED_ITEM_HALF_EXTENT,
};
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
use ambition_platformer2d_shared_tangle::schedule::{HeldItemStep, ItemPickupSet, SimScheduleExt};

/// The kernel's half of the item schedule, which after two carves is exactly
/// ONE thing plus its own attachments: the three-variant chain, and the kernel
/// systems that hang off the held-item domain's steps.
///
/// ⛔ IT CONFIGURES NO VARIANT AT ALL NOW, and that is the finished shape rather
/// than an omission. `CoreHeldItems` belongs to
/// `ambition_held_items::HeldItemSimulationPlugin`; `ThrownItemEffects` and
/// `WieldedAbilities` belong to `ambition_abilities::AbilitySimulationPlugin`
/// since the abilities carve (D33, 2026-09-03). A carved domain configures its
/// own sets end to end, and none of these three is the kernel's.
///
/// ⭐ WHAT THE KERNEL KEEPS IS THE EDGE NOBODY ELSE CAN DECLARE. The
/// three-variant `.chain()` orders sets owned by two OTHER crates, so neither
/// owner can name both sides — `ambition_held_items` must not name
/// `ambition_abilities`'s sets to order against them, and vice versa. The
/// kernel depends on both, so it is the one crate that can, and D33's second
/// sentence puts exactly this edge here: a set two crates order on is
/// `shared_tangle` vocabulary configured by exactly one owner.
///
/// Its own systems attach `.before`/`.after` a `HeldItemStep` rather than
/// naming a leaf function of a crate it no longer owns.
pub struct ItemPickupSimulationPlugin;

impl Plugin for ItemPickupSimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The inter-variant chain, and nothing else. All three variants are
        // nested in `PlayerSimulation` by their owners.
        app.configure_sets(
            sim,
            (
                ItemPickupSet::CoreHeldItems,
                ItemPickupSet::ThrownItemEffects,
                ItemPickupSet::WieldedAbilities,
            )
                .chain(),
        );
        // ⛔ THE KERNEL'S SYSTEMS RUN IN `CoreHeldItems`, SO THE KERNEL NESTS IT.
        // The held-items domain nests the set in `PlayerSimulation` as well (and
        // adds its custody edge); this is the same membership, stated by the
        // plugin whose systems depend on it. Without it a composition that
        // leaves held-use out has a set no phase owns, and the shrine, the gun
        // and the match spawn run on every schedule pass with none of the
        // phase's gating: MEASURED, `fire_puppy_slug_gun_system` failed
        // parameter validation on the first tick of a collection-only host.
        app.configure_sets(
            sim,
            ItemPickupSet::CoreHeldItems
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::PlayerSimulation),
        );
        // ⭐ THE KERNEL'S OWN SYSTEMS ATTACH TO A STEP, they are not links of the
        // domain's chain. Each says where it runs in the domain's vocabulary,
        // which is what lets the domain leave this crate without these edges
        // leaving with it. The order they had in the old single chain is the
        // order these edges reproduce, and the guard pins it by shape.
        // ⛔ THE OTHER HALF OF THE SHRINE IS NOT HERE ANY MORE, 2026-09-08 (A1b).
        // The startup resume and its progress state were initialized and
        // installed from this plugin, which made a composition's ability to
        // resume its session depend on it having held items.
        // `SessionCheckpointHorizonPlugin` installs
        // `restore_checkpoint_on_session_start` now — keeping this edge and this
        // set, so the move changed no order.
        //
        // ⚠ The progress resource itself did not survive the move: A1c/5 deleted
        // `CheckpointResumeProgress`'s two per-generation latches in favour of
        // `SessionStartupResume`, which names the admitted operation it is
        // waiting on instead of keeping a second completion mechanism beside the
        // reset road's.
        app.add_systems(
            sim,
            // Held-items, the portal gun, the heal/save shrine, and localized
            // gravity zones are LDtk-authored room entities. The shrine runs
            // before any hand changes this tick.
            ambition_shrine::heal_save_shrine_system
                .in_set(ambition_shrine::ShrineRest)
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .in_set(ItemPickupSet::CoreHeldItems)
                .before(HeldItemStep::Release),
        );
        app.add_systems(
            sim,
            // WHAT THE MATCH DROPS, before the physics that settles it —
            // so an item spawned this tick falls this tick rather than
            // hanging at its point for one frame.
            crate::match_spawn::spawn_match_items
                .run_if(bevy::prelude::resource_exists::<ambition_match::ActiveMatch>)
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .in_set(ItemPickupSet::CoreHeldItems)
                .after(HeldItemStep::Throw)
                .before(HeldItemStep::Settle),
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
///
/// ⛔⛔ IT READS THE ADMITTED OPERATION, NOT THE RAW REQUEST, and of the three
/// domains that used to read `ResetToCheckpoint` directly this is the one whose
/// refused-reset behaviour was worst. It takes an object out of the hand because
/// the banked custody relation did not have it there; what puts the object back
/// on its pedestal is the ROOM RECONSTRUCTION the reset asked for — and a
/// refusal is precisely the case where that never happens. The two halves of one
/// restore ran on opposite sides of an admission neither consulted, and the
/// object survived in neither. Measured 2026-09-08.
#[allow(clippy::too_many_arguments)]
pub fn restore_custody_to_checkpoint(
    // `SessionCommands`, because materialization SPAWNS. An occurrence
    // rebuilt into a hand is owned by the activation that is restoring, exactly
    // as the room build's would be; a bare `Commands` could only produce a
    // process-resident stranger that outlives the session.
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    // The lifecycle layer's installed inputs carry the custody relation; the
    // item domain's carry the recipes for the mints those rows name. Both are
    // present only while the commit executor runs this schedule.
    inputs: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CheckpointRestoreInputs>>,
    item_inputs: Option<Res<minted_horizon::ItemCheckpointRestoreInputs>>,
    // The world's DEFINITIONS, so an identity with no live occurrence behind
    // it can still be turned back into one. Every room, not the neighbours: a
    // body can carry an object any distance before putting it down, so the room
    // holding the record is not reachable by adjacency.
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
    >,
    mut items: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut GroundItem,
        &mut ItemCustody,
    )>,
    mut bodies: Query<(
        Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        RepertoireQuery,
    )>,
    // The live room of each holder, for an object built again into its hand.
    stamps: Query<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
) {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    let Some(inputs) = inputs else {
        return;
    };
    let baseline = &inputs.custody;
    // The checkpoint's own DESCRIPTIONS of what the simulation minted, for the
    // occurrences no record in any room can describe: the item domain's third
    // arm of the same baseline, whose population is disjoint from the authored
    // one.
    let minted = item_inputs.as_deref().map(|inputs| &inputs.minted);

    // Bodies by identity, so a baseline row can name the hand it belongs to.
    // a `BTreeMap` rather than the query's order: this drives despawns, and
    // Bevy's iteration order is an archetype accident.
    let by_identity: std::collections::BTreeMap<
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        Entity,
    > = bodies
        .iter()
        .map(|(entity, sim_id, _)| (sim_id.clone(), entity))
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
                let Ok((_, _, mut repertoire)) = bodies.get_mut(holder) else {
                    continue;
                };
                equip_held_spec(&mut commands, holder, &mut repertoire, spec);
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
                    if let Ok((_, _, mut repertoire)) = bodies.get_mut(holder) {
                        if repertoire.held.is_some_and(|held| held.id() == spec.id.as_str()) {
                            unequip_held(&mut commands, holder, &mut repertoire);
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
        let described = minted.and_then(|minted| minted.description_of(&occurrence));
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
                    ambition_platformer2d_construction::authored_occurrence_request(&world.rooms, &occurrence)
                })
                .as_ref()
                .map(|request| (request, &request.parameters))
            {
                Some((
                    request,
                    ambition_platformer2d_construction::ActorConstructionParams::GroundItem { spec, held },
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
        let Ok((_, _, mut repertoire)) = bodies.get_mut(holder) else {
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
        // `project_custody_onto_residency`, which the item domain chains
        // after this system in `CheckpointDomainApply`, so the restore's
        // verification reads it.
        let mut rebuilt = commands.spawn_room_in_session(
            scope,
            (
                occurrence.clone(),
                origin,
                Name::new(name),
                GroundItem::at_rest(held.clone(), pos, half_extent),
                ItemCustody::Held { holder },
            ),
        );
        // The object is in its holder's live room. A carried object gets that
        // stamp from the crossing that moves its holder, but this object did not
        // exist during the restore's crossing. Without a stamp it leaves with
        // any departing room (`InRoomInstance::leaves_with`), and the restore's
        // own crossing retires the room it replaces on the next tick.
        if let Ok(stamp) = stamps.get(holder) {
            rebuilt.insert(*stamp);
        }
        equip_held_spec(&mut commands, holder, &mut repertoire, held);
    }
}

