//! Pick up / throw held items (vertical slice).
//!
//! A `GroundItem` sits in the world; an empty-handed player presses `Attack`
//! while overlapping it to pick it up — the item's `HeldItemSpec` is overlaid
//! onto the player's `ActionSet` (so e.g. the axe grants its swing) and a
//! `HeldItem` component is attached. `Shield + Attack` throws the held item
//! back onto the ground ahead of the player, restoring the player's original
//! action set.
//!
//! One held item at a time; `Attack` picks up / uses and `Shield + Attack`
//! throws.

pub mod conditions;
pub mod minted_horizon;

use bevy::prelude::*;

use crate::actor::BodyKinematics;
use crate::features::HeldItem;
use crate::platformer_runtime::prelude::SpawnScopedExt;
use ambition_characters::brain::{
    ActionSet, HeldItemSpec, HeldUseBehavior, MeleeActionSpec, SwipeSpec,
};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::lifecycle::SpawnSessionScopedExt;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
#[cfg(feature = "portal")]
use ambition_portal2d::PortalGun;

/// Public schedule labels for held-item and ground-item simulation.
///
/// Other modules should order against these sets rather than concrete system
/// functions. That keeps cross-subsystem dependencies stable while item pickup
/// continues moving out of `app/plugins.rs`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum ItemPickupSet {
    /// Held-item pickup/use/throw plus ground-item physics.
    CoreHeldItems,
    /// Bombs, gravity grenades, and other effects armed by thrown items.
    ThrownItemEffects,
    /// Wielded movement/combat abilities and ability cooldown maintenance.
    WieldedAbilities,
}

/// Module-local plugin for held-item, pickup, thrown-item, and wielded-item
/// simulation systems.
///
/// The app installs this plugin, but the item module owns the registration and
/// ordering details for item behavior.
pub struct ItemPickupSimulationPlugin;

impl Plugin for ItemPickupSimulationPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // THE ITEM DOMAIN PUBLISHES ITS OWN QUESTION. Registered from the
        // domain's own plugin and naming no other domain — which is the whole
        // acceptance for the condition contract: a provider that had to be
        // listed somewhere central would not be a provider, it would be a case
        // in somebody else's match.
        {
            use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
            app.publish_condition(conditions::is_held_descriptor(), conditions::is_held);
        }
        // Durable room state, and the only leg of it that has a producer.
        // Inserted here because this is where the producer is registered; every
        // consumer takes it as an `Option`, so a composition without this plugin
        // remembers nothing and authors every room from its records — which is
        // exactly what it did before the ledger existed.
        app.init_resource::<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>();
        app.configure_sets(
            sim,
            (
                ItemPickupSet::CoreHeldItems,
                ItemPickupSet::ThrownItemEffects,
                ItemPickupSet::WieldedAbilities,
            )
                .chain()
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );
        // Both chains were internally ordered, internally correct, and siblings under
        // `PlayerSimulation` with nothing between them.
        app.configure_sets(
            sim,
            ItemPickupSet::CoreHeldItems
                .after(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
        );

        app.add_systems(
            sim,
            (
                // Held-items, the portal gun, the heal/save shrine, and localized
                // gravity zones are LDtk-authored room entities.
                crate::shrine::heal_save_shrine_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // The other half of the shrine: resume at the checkpoint it
                // recorded. Not gated on `gameplay_allowed` — it must land on the
                // FIRST tick a constructed session has a body, and that tick can
                // fall inside a room transition or a loading frame, which is
                // exactly when gameplay is suspended.
                crate::shrine::restore_checkpoint_on_session_start,
                // BEFORE the pickup, and that placement is load-bearing —
                // see [`return_released_items`]. It reads a hand that has already
                // settled, so it can never mistake an item the pickup below took
                // this very tick for one nobody is holding.
                return_released_items
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                pickup_held_item_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // Pickups that MOVE, stepped before the collect below so a
                // pickup is collected where it IS this tick — a fast one would
                // otherwise stay collectable from a box it has already left.
                crate::items::item_motion::step_item_motion
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // Touch-to-collect equipment pickups (mushroom / flower). A
                // sibling collect trigger to the pressed held-item pickup above.
                crate::items::world_item::collect_world_items
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                fire_held_ranged_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                held_projectile_step
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::thrown::puppy_slug_gun::fire_puppy_slug_gun_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                throw_held_item_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // WHAT THE MATCH DROPS, before the physics that settles it —
                // so an item spawned this tick falls this tick rather than
                // hanging at its point for one frame.
                crate::items::match_spawn::spawn_match_items
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // ⭐ SUPPORT IS RE-VALIDATED BEFORE THE STEP, so an item whose
                // platform left falls THIS tick rather than hanging for one —
                // the same reason the spawn above runs before the physics — and
                // an item whose platform MOVED goes with it.
                carry_or_wake_settled_items
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                ground_item_physics
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // RESIDENCY FOLLOWS CUSTODY. Last in the chain, so it sees
                // the custody this tick actually settled on — the release derive
                // above ran first, the pickup and the throw wrote directly. And
                // deliberately UNGATED: a room transition suspends gameplay
                // between the crossing and the commit, which is precisely the
                // window in which the room sweep reads residency.
                project_custody_onto_residency,
                // WHERE A CARRIED OCCURRENCE CAME TO REST. Strictly
                // between residency and the custody projection below, and both
                // edges are load-bearing — see this system's own doc. It turns
                // the outgoing `InCustody` row of an object that was just put
                // down into the `Placed` row that survives the room's unload;
                // the custody projection below then finds nothing to retract.
                record_placed_ground_items,
                // AND WHAT THE WORLD REMEMBERS ABOUT IT. Immediately
                // after residency, because it reads residency: an occurrence in
                // somebody's custody is alive and is not the room's to rebuild,
                // so the room that authored it must not mint a second one
                // behind the same `SimId::placement(..)`.
                //
                // registered from here and written NOWHERE near here: the
                // system is generic lifecycle vocabulary (it queries
                // `InCustodyOf`, which knows nothing about items) and this is
                // simply the chain whose last link produces its input. do not
                // read that as "the ledger is about items" — the next thing to
                // put a row in it will not be one.
                ambition_platformer2d_shared_tangle::lifecycle::project_custody_onto_authored_occurrences,
            )
                .chain()
                // `ItemPickupSet::CoreHeldItems` is configured
                // `.in_set(PlayerSimulation)` above, so the parent placement is
                // already implied — a direct `.in_set(PlayerSimulation)` here
                // would be a redundant hierarchy edge.
                .in_set(ItemPickupSet::CoreHeldItems),
        );

        // Portal-gun ground pickups: arm the LDtk-authored pickup here; the
        // Ambition inventory grant (`pickup_portal_gun_system`) is registered
        // by the content layer (`AmbitionPortalAdaptersPlugin`), ordered
        // `.after(arm_portal_pickups)` inside this same set so the chain edge
        // is preserved without this generic module naming content.
        #[cfg(feature = "portal")]
        app.add_systems(
            sim,
            ambition_portal2d::arm_portal_pickups
                .in_set(ambition_portal2d::PortalPickupArming)
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::CoreHeldItems` (configured above).
                .in_set(ItemPickupSet::CoreHeldItems),
        );

        // Bombs and gravity grenades run after the held-item throw/physics group.
        app.add_systems(
            sim,
            (
                crate::abilities::ranged::bomb::arm_thrown_bombs
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::bomb::tick_bomb_fuses
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::thrown::gravity_grenade::arm_thrown_gravity_grenades
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::thrown::gravity_grenade::tick_gravity_grenade_fuses
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                ambition_platformer2d_shared_tangle::gravity::tick_temporary_zones
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
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
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::traversal::blink::blink_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::traversal::grapple::grapple_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::shockwave::fire_shockwave_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::volley::fire_volley_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::beam::fire_beam_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::vortex::fire_vortex_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::vortex::update_vortex_wells
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::sentry::fire_sentry_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::sentry::update_sentries
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::traversal::dive::fire_dive_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::abilities::ranged::meteor::fire_meteor_system
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                crate::ability_cooldown::tick_ability_cooldown,
            )
                .chain()
                // Parent `PlayerSimulation` already implied via
                // `ItemPickupSet::WieldedAbilities` (configured above).
                .in_set(ItemPickupSet::WieldedAbilities),
        );
    }
}

const PICKUP_HALF: f32 = 18.0;
const THROW_AHEAD: f32 = 48.0;

/// The body a runtime-minted instance gets. A mint has no authored record to
/// take a size from, so this is a property of the MINT SITE rather than of the
/// instance — which is why it is not part of the instance's durable description
/// and why the checkpoint restore rebuilds one with the same constant rather
/// than remembering a copy per object.
pub(crate) const MINTED_ITEM_HALF_EXTENT: Vec2 = Vec2::splat(PICKUP_HALF);

/// Custody of one persistent physical item instance.
///
/// Instance items stay alive across `world -> held -> world`; custody determines
/// whether they are resident in a room and simulated there. Currency/health remain
/// quantity pickups, and consumable `WorldItem`s still end their lifetime on use.
/// Inventory storage is count-based, so it does not preserve an instance identity.
///
/// This is rollback state because restoring the wrong custody can duplicate an item
/// between a holder and the world. The holder entity is remapped on rollback load.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemCustody {
    /// Lying in the world: drawn, simulated by [`ground_item_physics`], and
    /// collectible.
    #[default]
    InWorld,
    /// Carried by `holder`. The entity stays alive with all of its identity;
    /// it simply stops being a thing in the world.
    Held { holder: Entity },
}

impl ItemCustody {
    /// True while the item is a thing in the world — the condition every
    /// world-facing reader (physics, pickup, the drawn view, the harness's
    /// observation) checks before it touches an item.
    pub fn in_world(&self) -> bool {
        matches!(self, Self::InWorld)
    }

    /// True iff `body` is the one carrying this item. How the throw finds the
    /// object a hand is holding without the hand having to remember an entity.
    pub fn held_by(&self, body: Entity) -> bool {
        matches!(self, Self::Held { holder } if *holder == body)
    }
}

impl bevy::ecs::entity::MapEntities for ItemCustody {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        if let Self::Held { holder } = self {
            *holder = mapper.get_mapped(*holder);
        }
    }
}

/// A held item resting in the world, pick-up-able with `Attack` when the
/// player is empty-handed. Thrown items carry a `vel` and arc under gravity
/// until they settle on a surface (`vel == ZERO` means resting).
///
/// A required component makes "every ground item has a custody" a property of the TYPE, so a
/// ninth spawn site cannot omit it and cannot default to a state that reads as "not in the
/// world".
#[derive(Component, Clone, Debug)]
#[require(ItemCustody)]
pub struct GroundItem {
    pub spec: HeldItemSpec,
    pub pos: Vec2,
    pub vel: Vec2,
    pub half_extent: Vec2,
}

const GROUND_ITEM_GRAVITY: f32 = 1400.0;
const THROW_SPEED_X: f32 = 320.0;
const THROW_SPEED_UP: f32 = 260.0;

/// THIS ITEM IS AT REST ON SOMETHING and does not need stepping.
///
/// ⭐⭐ AN EXPLICIT FACT, because the alternative was asking `vel == ZERO` to
/// mean two states at once — *supported and sleeping* AND *a free body with no
/// initial impulse* — and the second one arrives the moment anything places an
/// item without throwing it. The match spawner and the Z-drop both do exactly
/// that, each with a comment saying gravity would take over; it did not, and they
/// hung in the air.
///
/// ⛔⛔ AND SIMPLY DELETING THE EARLY-OUT DOES NOT WORK — measured twice.
/// Stepping every item drops the whole AUTHORED population out of the world: an
/// authored placement is put down at rest and is not necessarily standing on
/// collision geometry the penetration predicate can see, so a room rebuild came
/// back with ZERO ground items where it had fifteen. Authored placements carry
/// this marker; things somebody threw, dropped or spawned do not.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, Default)]
pub struct SettledItem;

/// Wake a settled item whose support has gone away.
///
/// ⛔⛔ SETTLING WAS PERMANENT. `ground_item_physics` skips `Without<SettledItem>`
/// and the marker was only removed by CUSTODY transitions (released from a hand,
/// thrown), so an item that landed on a MOVING PLATFORM — which the composited
/// collision world lets it do, deliberately — stayed fixed in WORLD SPACE when
/// the platform moved on. A platform that disappears leaves the same hovering
/// item.
///
/// ⭐ THE PROBE IS THE ONE `ground_item_physics` ALREADY USES, against the same
/// composited world, so "what counts as support" has one definition rather than
/// two that can disagree.
///
/// ⭐⭐ AND A SUPPORTED ONE RIDES, which is the half this used to say it had not
/// built. It reached for support IDENTITY and a local offset — and neither is
/// needed, because `Block::velocity` is the block's own PER-FRAME DISPLACEMENT
/// and its doc already says the sweep carries *"any body resting on the block"*
/// by it, *"uniform across every body, with no per-actor wiring"*. The fact was
/// at the site: the probe below already finds the block.
///
/// ⛔ THE PROBE ASKS WHERE THE BLOCK WAS. It runs after the platform has moved,
/// so testing against its NEW footprint would drop an item off any platform
/// moving faster than the probe's one-pixel band. `translated(-velocity)` is the
/// same correction `ledge_grab::runtime` makes, and it is the identity for
/// static geometry.
///
/// ⛔ AND `velocity` IS NOT DRAG. A conveyor belt has zero displacement and a
/// non-zero drag, and its own doc says so — an item on a belt stays put, which
/// is right: nothing is carrying it.
pub fn carry_or_wake_settled_items(
    mut commands: Commands,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut settled: Query<(Entity, &mut GroundItem, &ItemCustody), With<SettledItem>>,
) {
    let Some(world) = world.solids() else {
        return;
    };
    for (entity, mut item, custody) in &mut settled {
        if !custody.in_world() {
            continue;
        }
        // A thin band just past the item's own footprint: what it would rest ON.
        let probe = ae::Aabb::new(
            item.pos + Vec2::new(0.0, 1.0),
            item.half_extent + Vec2::new(0.0, 1.0),
        );
        let support = world.blocks.iter().find(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            ) && probe.strict_intersects(block.aabb.translated(-block.velocity))
        });
        match support {
            None => {
                commands.entity(entity).remove::<SettledItem>();
            }
            Some(block) => {
                if block.velocity != Vec2::ZERO {
                    item.pos += block.velocity;
                }
            }
        }
    }
}

/// Integrate in-world ground items under gravity and settle them when they'd
/// enter a solid / one-way surface. [`SettledItem`] items are skipped.
pub fn ground_item_physics(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    mut commands: Commands,
    mut grounds: Query<(Entity, &mut GroundItem, &ItemCustody), Without<SettledItem>>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Thrown / dropped items settle on the composited collision world, so a
    // moving platform / ECS solid catches them the same as authored geometry.
    let Some(world) = world.solids() else {
        return;
    };
    // Thrown / dropped items are free bodies that integrate through the shared
    // world-forces seam. Gravity is resolved per item by position, so an item
    // thrown into a gravity column falls the column's way (localized).
    for (entity, mut item, custody) in &mut grounds {
        // A carried item has no independent motion — it is not in the world to
        // fall through. Checked on custody rather than inferred from `vel ==
        // ZERO`, because "resting" and "in a hand" are different states that
        // happen to share a velocity, and only one of them may be stepped.
        if !custody.in_world() {
            continue;
        }
        // Free bodies resolve gravity by the body-overlap rule, not the center
        // point (ADR 0024) — a zone grabs an item the item TOUCHES.
        let local = ambition_platformer2d_shared_tangle::gravity::GravityField {
            dir: gravity.dir_for(ae::Aabb::new(item.pos, item.half_extent)),
        };
        ambition_platformer2d_shared_tangle::gravity::apply_world_forces(&mut item.vel, GROUND_ITEM_GRAVITY, &local, dt);
        let next = item.pos + item.vel * dt;
        let next_aabb = ae::Aabb::new(next, item.half_extent);
        let blocked = world.blocks.iter().any(|block| {
            matches!(
                block.kind,
                ae::BlockKind::Solid | ae::BlockKind::OneWay | ae::BlockKind::BlinkWall { .. }
            ) && next_aabb.strict_intersects(block.aabb)
        });
        // Out of the world rectangle on ANY side (not just world-down) — so an
        // item that flies off the side under a gravity flip parks too.
        let outside_world = next.y > world.size.y + 200.0
            || next.y < -200.0
            || next.x > world.size.x + 200.0
            || next.x < -200.0;
        if blocked || outside_world {
            // Settle in place (simple — no slide), and SAY SO: the marker is
            // what stops this item being stepped again, replacing the
            // `vel == ZERO` reading that could not tell rest from release.
            item.vel = Vec2::ZERO;
            commands.entity(entity).try_insert(SettledItem);
        } else {
            item.pos = next;
        }
    }
}

/// Return an item to world custody when its holder no longer holds that item spec.
///
/// This runs before pickup command application so it reads settled hand state and cannot
/// immediately undo a same-tick pickup. The entity/`SimId` is preserved; release places
/// the item at the holder and clears velocity. Despawned holders are left to the actor
/// death/drop policy to avoid creating a duplicate drop.
pub fn return_released_items(
    mut commands: Commands,
    // The hand, and where the body is standing. `Option<&HeldItem>` rather than
    // `Has<..>`: an equip-SWAP leaves the body holding a DIFFERENT item, so
    // "there is a hand" is not the question — "is this object the thing in it"
    // is, and only the spec id can answer that.
    holders: Query<(&BodyKinematics, Option<&HeldItem>)>,
    mut carried: Query<(Entity, &mut GroundItem, &mut ItemCustody)>,
) {
    for (entity, mut item, mut custody) in &mut carried {
        let ItemCustody::Held { holder } = *custody else {
            continue;
        };
        // A holder that no longer exists is the death-drop resolver's question,
        // not this one's — see the above.
        let Ok((kin, held)) = holders.get(holder) else {
            continue;
        };
        // Still the object in that hand — nothing to do. Compared by SPEC ID
        // rather than by "is there a hand at all", because an equip-SWAP leaves
        // the body holding a DIFFERENT item and orphans the old one just as
        // thoroughly as a Stow does.
        if held.is_some_and(|held| held.id() == item.spec.id.as_str()) {
            continue;
        }
        // The body let go: the object lands where that body is standing.
        item.pos = kin.pos;
        // A released object is at rest, not mid-throw: `arm_thrown_bombs` reads
        // "moving" as "thrown", and a bomb stowed from the menu must not arm.
        item.vel = Vec2::ZERO;
        *custody = ItemCustody::InWorld;
        // Released from a hand at whatever height that body is at: this object
        // is back under gravity until it lands on something.
        commands.entity(entity).remove::<SettledItem>();
    }
}

/// Derive room residency from item custody.
///
/// An item held by a travelling/nonresident holder follows that holder via
/// `InCustodyOf`; an item held by a room-resident fixture remains resident; an
/// `InWorld` item is resident in the active room. This projection is recomputed each
/// tick from rollback-authoritative `ItemCustody`, including transition frames, so it
/// needs no independent rollback state.
pub fn project_custody_onto_residency(
    mut commands: Commands,
    items: Query<
        (
            Entity,
            &ItemCustody,
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf>,
        ),
        With<GroundItem>,
    >,
    // Residency follows the holder's actual `RoomResident` status, not merely
    // room scope. This makes nested custody transitive. A missing holder is
    // treated as resident so an orphaned item is retired with the room.
    // TODO(item-orphan): release custody when a holder despawns.
    existing: Query<()>,
    residents: Query<(), ambition_platformer2d_shared_tangle::lifecycle::RoomResident>,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
    for (entity, custody, suspended) in &items {
        let holder = match *custody {
            ItemCustody::InWorld => None,
            ItemCustody::Held { holder } => {
                // A holder that is GONE: the object is an orphan and stays
                // resident, so the room it is in still retires it. A holder that
                // exists and is a ROOM RESIDENT — a room fixture's hand — keeps
                // the object resident too. Anything else is a holder that
                // travels: the session-scoped home avatar, or a room-scoped body
                // whose own residency is suspended by custody.
                if existing.get(holder).is_err() || residents.get(holder).is_ok() {
                    None
                } else {
                    Some(holder)
                }
            }
        };
        match (holder, suspended) {
            // Already says what it should say.
            (Some(holder), Some(InCustodyOf(current))) if *current == holder => {}
            (Some(holder), _) => {
                commands.entity(entity).insert(InCustodyOf(holder));
            }
            (None, Some(_)) => {
                commands.entity(entity).remove::<InCustodyOf>();
            }
            (None, None) => {}
        }
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
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_world::rooms::RoomSet>,
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
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
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
                equip_held_spec(
                    &mut commands,
                    holder,
                    &mut action_set,
                    spec,
                    owned.as_deref_mut(),
                );
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
                            unequip_held(
                                &mut commands,
                                holder,
                                &mut action_set,
                                stashed,
                                owned.as_deref_mut(),
                            );
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
        equip_held_spec(
            &mut commands,
            holder,
            &mut action_set,
            held,
            owned.as_deref_mut(),
        );
    }
}

/// WHERE A CARRIED OCCURRENCE CAME TO REST — the second producer of the
/// whereabouts ledger, and the first one that outlives the thing it describes.
///
/// Custody answers *"in a hand"*, which suppresses re-authoring for exactly as
/// long as the hand exists. An object PUT DOWN is in nobody's custody, so
/// custody can say nothing about it — and the room it was dropped in destroys it
/// on unload, after which the world's only memory of where it was is this row.
///
/// it tracks only occurrences the ledger ALREADY remembers. The condition
/// is `remembers(sim_id)`, which on the tick a hand empties is still the
/// outgoing `InCustody` row — so the population is exactly "things somebody
/// carried", never "every object in the room". A producer that recorded every
/// authored occurrence's position would be the universal instance registry this
/// ledger exists to not be, and would rewrite an enemy's row on every step it
/// took.
///
/// item-domain producer, generic vocabulary. The ledger's rows are
/// occurrence-generic; a producer must read a POSITION, and there is no generic
/// position for a simulated occurrence — so the producers live with the families
/// that have one. Room transition still knows nothing about items.
pub fn record_placed_ground_items(
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ambition_platformer2d_world::rooms::RoomSet>,
    >,
    items: Query<
        (
            &ambition_platformer2d_shared_tangle::sim_id::SimId,
            &GroundItem,
            &ItemCustody,
        ),
        With<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
    >,
    occurrences: Option<
        ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>,
    >,
) {
    let (Some(room_set), Some(mut occurrences)) = (room_set, occurrences) else {
        return;
    };
    let room = &room_set.active_spec().id;
    // BTreeMap, not the query's order. This value reaches a construction
    // plan; an archetype-ordered read here is a determinism bug that reproduces
    // perfectly on one machine.
    let mut placed: std::collections::BTreeMap<
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        Vec2,
    > = std::collections::BTreeMap::new();
    for (sim_id, ground, custody) in &items {
        if !custody.in_world() {
            continue;
        }
        // AN OCCURRENCE COMES TO REST HERE ONLY IF IT WAS IN A HAND, OR
        // WAS ALREADY RESTING HERE — and that is an invariant, not a filter.
        //
        // an object cannot change rooms without being carried. Every legitimate relocation
        // passes through custody: picked up in A (the row becomes `InCustody`), carried, put
        // down in B (this system writes `Placed { B }`). It is a stale duplicate the world
        // should not be holding, and believing it would be the ledger taking dictation from the
        // very duplication it exists to prevent.
        //
        // it refuses rather than repairs, deliberately.
        let comes_to_rest_here = match occurrences.whereabouts(sim_id) {
            Some(
                ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::InCustody,
            ) => true,
            Some(
                ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Placed {
                    room: recorded_room,
                    ..
                },
            ) => recorded_room == room,
            // No row: not something anybody carried, so not this producer's
            // population at all. Terminal: an ended occurrence does not come back
            // by being observed lying somewhere.
            None
            | Some(
                ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Consumed,
            ) => false,
        };
        if !comes_to_rest_here {
            continue;
        }
        placed.insert(sim_id.clone(), ground.pos);
    }
    if placed.is_empty() {
        return;
    }
    let unchanged = placed.iter().all(|(sim_id, at)| {
        matches!(
            occurrences.whereabouts(sim_id),
            Some(ambition_platformer2d_shared_tangle::lifecycle::OccurrenceWhereabouts::Placed {
                room: recorded_room,
                at: recorded_at,
            }) if recorded_room == room && recorded_at == at
        )
    });
    if unchanged {
        return;
    }
    occurrences.republish_placements(room, placed);
}

/// The player's pre-pickup `ActionSet`, restored when the held item is thrown.
#[derive(Component, Clone)]
pub struct StashedActionSet(pub ActionSet);

/// Authored axe held item: a keep-on-use heavy melee swing (placeholder tuning).
pub fn axe_spec() -> HeldItemSpec {
    HeldItemSpec {
        id: "axe".into(),
        melee: Some(MeleeActionSpec::Swipe(SwipeSpec {
            windup_s: 0.22,
            active_s: 0.12,
            recover_s: 0.30,
            damage: 3,
            reach_px: 64.0,
        })),
        ranged: None,
        // Has a melee verb → Auto keeps it on use (swing, don't throw).
        use_behavior: HeldUseBehavior::Auto,
    }
}

/// Authored javelin held item: a *pure throwable* (no melee/ranged verb), so
/// using it (`Attack` while holding) throws it — the `ThrowOnUse` behavior.
pub fn javelin_spec() -> HeldItemSpec {
    HeldItemSpec {
        id: "javelin".into(),
        melee: None,
        ranged: None,
        // The canonical thrown item: using it (plain Attack) throws it.
        use_behavior: HeldUseBehavior::ThrowOnUse,
    }
}

/// The laser gun-sword as a *player* held item — the same authored `gun_sword`
/// the pirates carry (`ambition_characters::brain::held_item_by_id`). Picking it up replaces
/// the player's melee swing with the item's *ranged* verb, so `Attack` fires a
/// laser bolt instead of swinging — the unification the pirates will share once
/// their dedicated sniper mode is dropped (see TODO).
pub fn gunsword_spec() -> HeldItemSpec {
    ambition_characters::brain::held_item_by_id("gun_sword")
        .expect("gun_sword is a built-in held item")
}

/// Resolve a catalog [`crate::items::Item`]'s held-item spec, for equipping from
/// a non-pickup source (the inventory menu). The three wired weapons each have a
/// spec; everything else returns `None`.
pub fn held_spec_for_item(item: crate::items::Item) -> Option<HeldItemSpec> {
    use crate::items::Item;
    match item {
        Item::Axe => Some(axe_spec()),
        Item::Javelin => Some(javelin_spec()),
        Item::GunSword => Some(gunsword_spec()),
        _ => item
            .held_item_id()
            .and_then(ambition_characters::brain::held_item_by_id),
    }
}

/// Resolve a held item's authored spec id back to its spec — the reverse of
/// [`HeldItemSpec::id`], and the item domain's answer to "what is this thing".
///
/// the reverse direction is what makes a durable description possible. A
/// checkpoint that wants to rebuild a runtime-minted instance stores the id and
/// nothing else (see
/// [`minted_horizon`](crate::items::pickup::minted_horizon)); storing the
/// resolved spec would put a second authority for *what a javelin is* inside a
/// snapshot, so the id has to be resolvable from outside.
///
/// both registries, in that order, because there are two. The three wired
/// weapons are built here by [`held_spec_for_item`] and are NOT rows in
/// `ambition_characters`'s table; the pirates' `gun_sword_heavy` is a row there
/// and has no catalog slot. Consulting one alone silently loses half the items.
pub fn held_spec_by_id(id: &str) -> Option<HeldItemSpec> {
    crate::items::Item::from_held_item_id(id)
        .and_then(held_spec_for_item)
        .or_else(|| ambition_characters::brain::held_item_by_id(id))
}

/// TAKE custody of a held item — one operation, both ends.
///
/// Stash the current action set, overlay the item's verbs, attach [`HeldItem`],
/// and name the catalog slot this body is now holding. Every way a body comes to
/// hold a weapon calls this: the world pickup ([`pickup_held_item_system`]) and
/// the inventory menu. There is no second place that writes half of it.
///
/// Handing the catalog to the transfer is what makes the two ends move together or not at all.
/// `None` means "this body has no catalog behind it" (a headless fixture, a game with no
/// inventory) — never "skip the bookkeeping".
///
/// `grant` belongs to the sites that confer a quantity with no object — `<<give_item>>`, the
/// shop, ability drops.
pub fn equip_held_spec(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    spec: HeldItemSpec,
    owned: Option<&mut crate::items::OwnedItems>,
) {
    // Resolved BEFORE either end is written, so the catalog can never be told
    // about a slot the body did not end up with. An id with no catalog row (the
    // pirates' `gun_sword_heavy`) equips normally and claims no slot.
    let slot = crate::items::Item::from_held_item_id(spec.id.as_str());
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    let held = HeldItem::new(spec.clone());
    // The held item *replaces* the player's attack verbs (move-style/special
    // are kept), exactly as the world pickup does.
    action_set.melee = spec.melee;
    action_set.ranged = spec.ranged;
    commands.entity(player).insert(held);
    if let (Some(owned), Some(item)) = (owned, slot) {
        owned.set_equipped(Some(item));
    }
}

/// RELEASE custody of a held item — the twin of [`equip_held_spec`].
///
/// Restore the stashed action set, detach [`HeldItem`], and clear the catalog's
/// equipped slot. The body stops holding it here and nowhere else.
///
/// Nothing here has an item query to fix that with (the menu calls this from `Update`, in another
/// crate), so custody is re-derived from the hand instead: see [`return_released_items`]. A caller
/// that simply lets go can no longer destroy an authored object.
pub fn unequip_held(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
    owned: Option<&mut crate::items::OwnedItems>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<HeldItem>();
    commands.entity(player).remove::<StashedActionSet>();
    if let Some(owned) = owned {
        owned.set_equipped(None);
    }
}

/// TAKE custody of the portal gun — the portal-gun twin of
/// [`equip_held_spec`]. Stash the action set, attach an active [`PortalGun`],
/// clear the melee swing so `Attack` fires portals, and name the catalog slot.
///
/// The gun equips through its own component rather than a `HeldItemSpec`, which
/// is why it needs a twin at all; the catalog slot is spelled out here because
/// `Item::PortalGun` deliberately carries no `held_item_id` (nothing to look it
/// up from).
#[cfg(feature = "portal")]
pub fn equip_portal_gun(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    owned: Option<&mut crate::items::OwnedItems>,
) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    commands.entity(player).insert(PortalGun {
        active: true,
        ..PortalGun::default()
    });
    action_set.melee = None;
    if let Some(owned) = owned {
        owned.set_equipped(Some(crate::items::Item::PortalGun));
    }
}

/// RELEASE custody of the portal gun — the portal-gun twin of
/// [`unequip_held`]. Detach [`PortalGun`], restore the stashed action set, and
/// clear the catalog's equipped slot.
#[cfg(feature = "portal")]
pub fn unequip_portal_gun(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
    owned: Option<&mut crate::items::OwnedItems>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<PortalGun>();
    commands.entity(player).remove::<StashedActionSet>();
    if let Some(owned) = owned {
        owned.set_equipped(None);
    }
}

/// `Attack` while empty-handed and overlapping a `GroundItem` picks it up:
/// stash the current action set, overlay the item's verbs, attach `HeldItem`.
///
/// SUBJECT-GENERIC (like `fire_held_ranged_system`): it acts on the
/// [`ControlledSubject`](ambition_platformer2d_shared_tangle::markers::ControlledSubject)
/// — the body you are DRIVING physically grabs the item — reading that body's own
/// `ActorControl` (brain output), not a player-identity-specific input path. The
/// held item is EXPLICITLY owned by the controlled body; the catalog grant lands
/// on the global `OwnedItems` home inventory. One item at a time: a body already
/// holding an item (or the portal gun) can't grab another.
///
/// this is a PRESS-gated action, and that is why it stays on one subject
/// while the touch-collectors are body-generic. Picking a weapon off the floor
/// spends an Attack press on a specific body's `ActorControl`; walking into a
/// coin spends nothing. The `ControlledSubject` here is not a player-centrism
/// leftover — it is "the body whose press this is". The touch-collect fork lives
/// in `features::ecs::pickups`.
///
/// it does not DESTROY the item. See [`ItemCustody`].
pub fn pickup_held_item_system(
    mut commands: Commands,
    controlled: Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
    mut bodies: Query<(
        &mut ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        Option<&HeldItem>,
    )>,
    // Holding the portal gun blocks a pickup (portal builds only).
    #[cfg(feature = "portal")] portal_guns: Query<&PortalGun>,
    mut grounds: Query<(&mut GroundItem, &mut ItemCustody)>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    let Some(player) = controlled.0 else {
        return;
    };
    let Ok((mut control, kin, mut action_set, held)) = bodies.get_mut(player) else {
        return;
    };
    // One item at a time: already holding a physical item, or the portal gun.
    if held.is_some() {
        return;
    }
    #[cfg(feature = "portal")]
    if portal_guns.get(player).is_ok() {
        return;
    }
    // Gameplay authority is the body's brain-resolved `ActorControl`.
    if !control.0.melee_pressed {
        return;
    }
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    for (mut ground, mut custody) in &mut grounds {
        // Only an item that is IN THE WORLD can be grabbed.
        if !custody.in_world() {
            continue;
        }
        let ground_aabb = ae::Aabb::new(ground.pos, ground.half_extent);
        // AMBITION_REVIEW(discrete_ok): CC2 §3.3 GroundItem pickup — gated on a
        // deliberate `melee_pressed` while overlapping (the button-press branch
        // above), not a path-dependent auto-collect. You cannot fly THROUGH and
        // grab it, so there is no tunnel to sweep. An auto-collect (touch-to-grab
        // ring/coin) would instead route through `cast::aabb_path_contacts`.
        if player_aabb.strict_intersects(ground_aabb) {
            // A player who picked a weapon up after the last shrine and died got the object back on
            // its pedestal AND kept the row; the inventory menu would then equip the phantom and
            // mint a SECOND weapon on the first throw, and the durable save wrote it to disk on the
            // way past.
            //
            // the object is the record. `OwnedItems::count` PROJECTS the hand
            // (via the equipped slot `equip_held_spec` writes below), so the grid
            // still shows the axe you are carrying — derived, retracted by the
            // same reset that retracts the object, and impossible to disagree
            // with. See [`OwnedItems`](crate::items::OwnedItems)'s own docs.
            //
            // CUSTODY: the ONE take-custody operation, shared with the inventory menu.
            equip_held_spec(
                &mut commands,
                player,
                &mut action_set,
                ground.spec.clone(),
                owned.as_deref_mut(),
            );
            // The Attack press is *consumed* by the pickup so the same press
            // doesn't also fire the just-equipped item this frame. Clear the
            // brain-resolved `ActorControl` (the subject-generic held-item / ability
            // systems — blink/grapple/gun — read `melee_pressed` there). Raw slot
            // input is immutable intent for this tick; action consumers arbitrate
            // on body state and commit by spending the semantic control edge.
            control.0.melee_pressed = false;
            *custody = ItemCustody::Held { holder: player };
            // A carried item is not in flight. Zeroing here (rather than relying
            // on the custody gate alone) also keeps the fuse arming honest:
            // `arm_thrown_bombs` / `arm_thrown_gravity_grenades` treat "moving"
            // as "thrown", and a bomb picked up mid-arc must not stay armed in
            // a hand because its last world velocity was nonzero.
            ground.vel = Vec2::ZERO;
            break;
        }
    }
}

/// Which way a body let go of what it was holding.
///
/// One enum rather than a bool, because the two are different DECISIONS and a
/// third (a soft toss, a hand-off) would be a variant rather than a second
/// parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Release {
    /// Forward and up — an attack.
    Throw,
    /// Straight down where the body stands — the genre's Z-drop.
    Drop,
}

/// Let go of the held item: restore the stashed action set, detach `HeldItem`,
/// and put the item back into the world. Fires on `Grab` (a [`Release::Drop`],
/// where the body stands), on `Shield + Attack` for any item, or on a plain
/// `Attack` for a pure throwable (throw-on-use) — the last two being a
/// [`Release::Throw`], ahead of the body.
///
/// "put back", not "spawn". The object the body took custody of is still
/// alive and still carries its identity, so the throw resets its
/// [`ItemCustody`] and writes the launch onto it. A fresh instance is minted
/// only when there is no object behind the hand at all — the inventory menu
/// equips out of a count table — and that mint takes a `SimId::spawned` under
/// the thrower.
///
/// SUBJECT-GENERIC: acts on the
/// [`ControlledSubject`](ambition_platformer2d_shared_tangle::markers::ControlledSubject)
/// — the body you drive throws the item it holds — reading that body's own
/// `ActorControl`, not a player-identity-specific input path.
///
/// and it CONSUMES the press, exactly as the pickup does. A held weapon
/// owns the Attack press (`trigger_moveset_moves` arbitrates that from
/// `HeldItem`), but a throw is the one item action that ENDS the holding: it
/// removes `HeldItem` in `PlayerSimulation`, and the move trigger looks in
/// `Combat`, one phase later — so on the throw tick the trigger would find an
/// empty hand and hand the very same press to the wearer's jab. Marking the
/// press spent where it is spent is what makes "one press, one action" hold
/// across the phase boundary; the alternative is an ordering constraint between
/// two phases that exist to be independent.
pub fn throw_held_item_system(
    mut commands: Commands,
    controlled: Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
    gravity: ambition_platformer2d_shared_tangle::gravity::GravityCtx,
    mut bodies: Query<(
        &mut ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        &HeldItem,
        Option<&StashedActionSet>,
    )>,
    // The object this body is CARRYING, found by the custody it records rather
    // than by the hand remembering an entity handle.
    mut carried: Query<(Entity, &mut GroundItem, &mut ItemCustody)>,
    // The thrower's identity stream, for the one case that genuinely mints a new
    // object (see below). N3.1: a dynamically-spawned sim entity takes
    // `(spawner SimId, per-spawner counter)`, never a global counter.
    mut identities: Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter,
    )>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    let Some(player) = controlled.0 else {
        return;
    };
    let Ok((mut control, kin, mut action_set, held, stashed)) = bodies.get_mut(player) else {
        return;
    };
    // ⭐ TWO WAYS TO LET GO, AND THEY DIFFER ONLY IN THE LAUNCH.
    //
    // A THROW sends the item forward and up; a Z-DROP (Grab, while holding) lets
    // it go where the body is standing, with nothing added. The genre keeps them
    // apart because they are different decisions — one is an attack, the other is
    // handing the item to the floor or to somebody below you — and both are the
    // SAME custody transition, `Held → InWorld`.
    //
    // ⛔ NOT A SECOND SYSTEM. Everything after this point — emptying the hand,
    // clearing the equipped slot, returning the live object rather than minting a
    // replacement, and the mint arm for a body holding a quantity — is identical,
    // and a copy of it would be a second place for the custody rules to drift.
    let release = if control.0.grab_pressed {
        control.0.grab_pressed = false;
        Release::Drop
    } else if control.0.melee_pressed
        // Shield+Attack throws anything; plain Attack throws only items whose
        // authored `use_behavior` opts in, leaving `UseSystem` abilities to
        // their own systems.
        && (control.0.shield_held || held.spec.throws_on_plain_attack())
    {
        // The throw IS this press's action — see the note on the signature.
        control.0.melee_pressed = false;
        Release::Throw
    } else {
        return;
    };
    let spec = held.spec.clone();
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // The launch is authored in the body's LOCAL frame (x = forward/side,
    // y = toward-feet) and rotated into the body's gravity frame, so the throw
    // arcs "ahead + away from feet" under ANY gravity — identity under normal
    // gravity. The subsequent free-fall (`ground_item_physics`) is already
    // gravity-relative, so the whole toss now flips with the field.
    let frame = ae::AccelerationFrame::new(gravity.dir_for(ae::Aabb::new(kin.pos, kin.size * 0.5)));
    let (throw_pos, throw_vel) = match release {
        // Forward + away-from-feet, in the local frame → world.
        Release::Throw => (
            kin.pos + frame.to_world(Vec2::new(facing * THROW_AHEAD, 0.0)),
            frame.to_world(Vec2::new(facing * THROW_SPEED_X, -THROW_SPEED_UP)),
        ),
        // ⭐ WHERE THE BODY IS, WITH NOTHING ADDED. Not "a weak throw": a drop
        // that inherited a fraction of the forward offset would still place the
        // item ahead of the body, and the whole point is that it lands where you
        // are. `ground_item_physics` takes it from there under the live frame, so
        // a drop in flipped gravity falls the way that room falls.
        Release::Drop => (kin.pos, Vec2::ZERO),
    };
    // CUSTODY, one operation: the hand empties and the catalog's equipped slot clears together.
    // A weapon PICKED UP has no stored row at all, so letting go of it is letting go of it: the
    // object on the floor is the only record, and the grid dims. A weapon equipped out of a
    // GRANTED quantity still has its row, so the thrower keeps catalog ownership and can
    // re-equip.
    //
    // The rule the note still carries is the useful part: only the equipped slot moves here, never
    // the stored quantity — the spend belongs at the MINT, where the quantity actually becomes an
    // object.
    unequip_held(
        &mut commands,
        player,
        &mut action_set,
        stashed,
        owned.as_deref_mut(),
    );
    // RETURN THE OBJECT, do not manufacture a replacement. The item this body took custody of
    // is still a live entity carrying its own identity, so the throw resets its custody and writes
    // the launch onto it.
    if let Some((entity, mut ground, mut custody)) = carried
        .iter_mut()
        .find(|(_, _, custody)| custody.held_by(player))
    {
        ground.pos = throw_pos;
        ground.vel = throw_vel;
        *custody = ItemCustody::InWorld;
        // ⭐ A DROP IS THE CASE THAT NEEDS THIS, not the throw. `Release::Drop`
        // launches at ZERO velocity, so an object that kept the settled marker
        // it wore when it was picked up would hang at head height forever.
        commands.entity(entity).remove::<SettledItem>();
        return;
    }
    // NO OBJECT BEHIND THE HAND — materialize one. A body can come to hold
    // an item with no world instance at all: the inventory menu equips straight
    // out of `OwnedItems`, which is a count table. Throwing that turns a
    // QUANTITY into an INSTANCE, and an instance owes an identity, so it takes
    // `SimId::spawned(thrower, counter.next())` here rather than joining the
    // population of anonymous dropped items. This arm is the visible edge of the
    // unclosed inventory leg described on [`ItemCustody`] — not a fallback that
    // should quietly absorb the common case.
    //
    // the pair is `Option` as ONE value: a thrower with no identity mints
    // neither half, so "dynamic, parent unknown" stays unspellable.
    let minted = identities.get_mut(player).ok().map(|(id, mut counter)| {
        let sequence = counter.next();
        (
            ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(id, sequence),
            ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic {
                parent: id.clone(),
                sequence,
            },
        )
    });
    // AND THE MINT SPENDS THE ENTITLEMENT IT CAME FROM — gate, opened. A quantity that
    // turns into an object must stop being a quantity, or the row and the object both claim it
    // and the next equip throws a second one.
    //
    // `OwnedItemsBaseline` answers that — the reset puts the row back — so the two halves land
    // together and neither is a bug on its own.
    //
    // `take`, not a `count` write. `count` PROJECTS the equipped slot, and
    // writing a projection back into the table is the fork this domain already
    // paid for once; `take` is the stored quantity alone.
    // resolved from the SPEC's id, the same way `unequip_held` finds the slot
    // it clears — a held spec that answers to no catalog Item was never a
    // quantity and has no row to spend.
    if let (Some(owned), Some(item)) = (
        owned.as_deref_mut(),
        crate::items::Item::from_held_item_id(spec.id.as_str()),
    ) {
        owned.take(item, 1);
    }
    let mut thrown = commands.spawn_room_scoped((
        GroundItem {
            spec,
            vel: throw_vel,
            pos: throw_pos,
            half_extent: MINTED_ITEM_HALF_EXTENT,
        },
        Name::new("Ground item: thrown"),
    ));
    if let Some((sim_id, origin)) = minted {
        thrown.insert((sim_id, origin));
    }
}

// ---------------------------------------------------------------------------
// Held *ranged* items (the gun-sword): `Attack` fires a traveling laser bolt.
//
// Self-contained like the portal shot — a `HeldProjectile` travels each tick,
// damages the first enemy / boss / breakable it overlaps (reusing the shared
// feature-damage `HitEvent` channel), and expires on a solid wall or past max
// range. This is the player end of the held-gun-sword unification: the same
// `RangedActionSpec` the pirates fire, driven by the player's `Attack`.

/// Held-shot-specific gameplay for an in-flight ranged item. Position and
/// velocity live in shared [`BodyKinematics`]; this component carries damage,
/// range traveled, and optional splash radius.
#[derive(Component, Clone, Copy, Debug)]
pub struct HeldProjectile {
    pub damage: i32,
    pub traveled: f32,
    /// Half-extent of an explosion this shot triggers when it hits something.
    /// `0.0` for a plain bolt (the gun-sword); a Fireball sets it so the impact
    /// deals splash damage to everything in the box, not just the first body.
    pub explode_half: f32,
}

const HELD_SHOT_MAX_RANGE: f32 = 1600.0;
const HELD_SHOT_HALF: Vec2 = Vec2::new(12.0, 9.0);

impl HeldProjectile {
    /// The box that actually registers a hit on a body this tick, centered on the
    /// body's current `pos`. ONE source of truth shared by the collision system
    /// (`held_projectile_step`) and the debug overlay so the drawn box can never
    /// drift from the box that hits — the cause of the "fireball hits before it
    /// touches the visible box" report was that this contact box was never drawn.
    pub fn contact_aabb(pos: Vec2) -> ae::Aabb {
        ae::Aabb::new(pos, HELD_SHOT_HALF)
    }

    /// The splash box a Fireball detonates with on contact (`None` for a plain
    /// bolt). Drawn faintly around an in-flight fireball so the player can see
    /// the whole area-of-effect that will trigger, not just the thin bolt.
    pub fn splash_aabb(&self, pos: Vec2) -> Option<ae::Aabb> {
        (self.explode_half > 0.0).then(|| ae::Aabb::new(pos, Vec2::splat(self.explode_half)))
    }
}

/// Held-item id of the Fireball ability — a ranged held item whose shot
/// explodes on contact (see [`fire_held_ranged_system`]).
pub const FIREBALL_ID: &str = "fireball";

/// Splash half-extent a Fireball shot detonates with on contact.
const FIREBALL_EXPLODE_HALF: f32 = 56.0;

/// Detonate a Fireball shot at `pos`: a boxed splash `HitEvent` (damages every
/// body in the box, not just the first), an explosion VFX, and a boom SFX. A
/// free fn (not a closure) so it can borrow the loop's writers at each call site
/// without holding them across the projectile loop.
fn emit_fireball_explosion(
    pos: Vec2,
    damage: i32,
    half: f32,
    attacker: Option<Entity>,
    feature_damage: &mut MessageWriter<crate::features::HitEvent>,
    sfx: &mut ambition_sfx::SfxWriter,
    vfx: &mut MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    feature_damage.write(crate::features::HitEvent {
        strike_sfx: None,
        volume: ae::Aabb::new(pos, Vec2::splat(half)).into(),
        damage,
        source: crate::features::HitSource::Projectile,
        attacker,
        target: crate::features::HitTarget::Volume,
        mode: crate::features::HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
    });
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_ROCK_HIT,
        pos,
    });
    vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
        pos,
        fx: ambition_vfx::fx::ids::CLASSIC_BURST,
        scale: 1.0,
        pose: ambition_vfx::FxPose::UPRIGHT,
    });
}

/// Body-generic ability aim in the CONTROLLED BODY'S LOCAL frame, taken from the
/// brain-resolved [`ActorControlFrame::aim`] (the brain already crossed the input
/// seam via the aim frame mode), falling back to local facing when neutral. It reads
/// the body's `ActorControl` (present on any controlled body — home body or
/// possessed actor), so an ability fires from whichever body is being driven.
pub fn ability_aim_local(
    control: &ambition_characters::actor::control::ActorControlFrame,
    facing: f32,
) -> Vec2 {
    // Resolve the brain-authored fallback chain: the aim stick, else the movement
    // stick (`locomotion` — so you can
    // steer a held-item cast with the direction you're moving), else facing.
    if control.aim.length() > 0.1 {
        control.aim.vec()
    } else if control.locomotion.length() > 0.1 {
        control.locomotion.vec()
    } else {
        Vec2::new(facing, 0.0)
    }
}

/// [`ability_aim_local`] rotated into world space for the body's gravity frame.
pub fn ability_aim_world(
    control: &ambition_characters::actor::control::ActorControlFrame,
    facing: f32,
    gravity_dir: Vec2,
) -> Vec2 {
    ae::AccelerationFrame::new(gravity_dir).to_world(ability_aim_local(control, facing))
}

/// `Attack` while holding a *ranged* item fires a laser bolt along the aim
/// direction. `Shield + Attack` is the throw/drop gesture, so don't fire on it.
pub fn fire_held_ranged_system(
    mut commands: Commands,
    // SUBJECT-GENERIC held-weapon fire: acts on the `ControlledSubject`, reading
    // that body's OWN `ActorControl` (brain output) + `HeldItem`. No
    // `With<PlayerEntity>` filter or entity-local input copy — a possessed body firing
    // its held gun works exactly like the home avatar.
    controlled: Res<ambition_platformer2d_shared_tangle::markers::ControlledSubject>,
    bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
    )>,
    mut sfx: ambition_sfx::SfxWriter,
) {
    let Some(subject) = controlled.0 else {
        return;
    };
    let Ok((control, kin, resolved_frame, held)) = bodies.get(subject) else {
        return;
    };
    let c = control.0;
    if !c.melee_pressed || c.shield_held {
        return;
    }
    let Some(ranged) = held.spec.ranged.clone() else {
        return;
    };
    // The body's per-tick resolved frame (ADR 0024 frame law).
    let frame = resolved_frame.basis();
    let local_dir = ability_aim_local(&c, kin.facing);
    let dir = frame.to_world(local_dir).normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    let muzzle_side = if local_dir.x.abs() > 0.001 {
        local_dir.x.signum()
    } else {
        kin.facing.signum()
    };
    let muzzle = frame.to_world(Vec2::new(
        muzzle_side * (kin.size.x * 0.5 + 8.0),
        -kin.size.y * 0.12,
    ));
    let origin = kin.pos + muzzle;
    // A Fireball shot explodes on contact; every other ranged held item fires a
    // plain single-target bolt (`explode_half` 0).
    let explode_half = if held.spec.id == FIREBALL_ID {
        FIREBALL_EXPLODE_HALF
    } else {
        0.0
    };
    #[allow(unused_mut)]
    let mut shot = commands.spawn_room_scoped((
        // Position + velocity live in the shared body; size matches contact.
        BodyKinematics {
            pos: origin,
            vel: dir * ranged.speed(),
            size: HELD_SHOT_HALF * 2.0,
            facing: if dir.x >= 0.0 { 1.0 } else { -1.0 },
        },
        // The projectile *marker*: excludes the bolt from actor-generic queries
        // (auto-righting, actor portal tagging). Its kinematics are driven by
        // `held_projectile_step` (keyed on `HeldProjectile`), not the ECS
        // projectile step (keyed on `LiveProjectile`), so this marker never
        // double-steps the bolt.
        ambition_projectiles::ProjectileGameplay {
            age: 0.0,
            max_lifetime: f32::MAX,
            gravity: 0.0,
            damage: ranged.damage(),
            bounces_remaining: 0,
            // Stepped by `held_projectile_step` (keyed on `HeldProjectile`), not
            // the ECS projectile world-collision path, so this is inert here; a
            // detonate-on-contact bolt is `ExpireOnContact` in spirit.
            world_hit: ambition_projectiles::WorldHitPolicy::ExpireOnContact,
        },
        HeldProjectile {
            damage: ranged.damage(),
            traveled: 0.0,
            explode_half,
        },
        Name::new("Held ranged shot"),
    ));
    // `reorient: false, carry_velocity: true` is the free-flying projectile policy.
    #[cfg(feature = "portal")]
    shot.insert((
        ambition_portal2d::PortalBody,
        ambition_portal2d::PortalPolicy {
            reorient: false,
            carry_velocity: true,
        },
    ));
    let _ = &shot;
    // Fireball currently reuses the dash whoosh instead of the gun-sword zap.
    let fire_sfx = if held.spec.id == FIREBALL_ID {
        ambition_sfx::ids::PLAYER_DASH
    } else {
        ambition_sfx::SfxId::from_static("weapon.lasersword.fire")
    };
    sfx.write(ambition_sfx::SfxMessage::Play {
        id: fire_sfx,
        pos: origin,
    });
}

/// Advance held ranged shots; damage the first feature they overlap, or expire
/// on a solid wall / past max range.
#[allow(clippy::too_many_arguments)]
pub fn held_projectile_step(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
        ambition_platformer2d_core::RoomGeometry,
    >,
    overlay: Res<crate::features::FeatureEcsWorldOverlay>,
    mut commands: Commands,
    // `Without<FeatureSimEntity>` keeps this `&mut BodyKinematics` disjoint from
    // the boss cluster query below (which reads `BodyKinematics` via
    // `BossClusterRef`) — a held bolt is never a feature-sim entity (B0001).
    mut projectiles: Query<
        (Entity, &mut BodyKinematics, &mut HeldProjectile),
        Without<crate::features::FeatureSimEntity>,
    >,
    // SLOT-0 SCOPE, NOT BY DESIGN — a held-bolt fold candidate the S5/S6 fold
    // did not reach. A held projectile belongs to whichever body picked it up, so
    // this should key off `ControlledSubject` like `blink`/`grapple` do. Left as-is
    // because retargeting a thrown bolt's owner changes hit attribution (feel), and
    // that never ships blind. Tracked in refactor-chain.md R6.
    player: Query<Entity, crate::actor::PrimaryPlayerOnly>,
    boss_catalog: Res<ambition_boss_encounter::BossCatalog>,
    ecs_breakables: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            &crate::features::BreakableFeature,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    // `Option<&DamageableVolumes>` so a thrown bolt does not terminate on a body
    // that published no hurtbox and would take no damage. Optional, never required:
    // requiring it would drop every actor without one from the query.
    ecs_actors: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            &crate::features::ActorDisposition,
            // AC3.1.A: the liveness authority, not the once-per-frame mirror.
            &ambition_characters::actor::BodyHealth,
            Option<&crate::features::DamageableVolumes>,
        ),
        (
            With<crate::features::FeatureSimEntity>,
            Without<ambition_boss_encounter::BossConfig>,
        ),
    >,
    ecs_bosses: Query<
        (
            &crate::features::FeatureId,
            &crate::features::CenteredAabb,
            ambition_boss_encounter::BossClusterRef,
            &ambition_characters::actor::BodyHealth,
            &ambition_characters::brain::BossAttackState,
            Option<&crate::features::BossAnimationFrameSample>,
        ),
        With<crate::features::FeatureSimEntity>,
    >,
    mut feature_damage: MessageWriter<crate::features::HitEvent>,
    mut sfx: ambition_sfx::SfxWriter,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
) {
    let dt = time.sim_dt();
    if dt <= 0.0 {
        return;
    }
    // Collide against the room world with ONLY the portal apertures carved out: a
    // portal punched through a wall leaves the opening non-solid, so a bolt fired
    // at a wall portal flies INTO the opening instead of detonating on the wall —
    // and `portal_transit` (which already moves this bolt's `BodyKinematics`)
    // carries it out the far portal. Carves-only preserves the bolt's historical
    // raw-world collision (it passes through moving platforms).
    let collision_world = ambition_platformer2d_world::collision::world_with_portal_carves(
        &world.0,
        &overlay.portal_carves,
    );
    let attacker = player.single().ok();
    for (entity, mut kin, mut proj) in &mut projectiles {
        let pos = kin.pos;
        let vel = kin.vel;
        // Damage check against actors / bosses / breakables via the shared
        // attacker-side channel. Projectile hit events broadcast to features.
        let hit_event = crate::features::HitEvent {
            strike_sfx: None,
            volume: HeldProjectile::contact_aabb(pos).into(),
            damage: proj.damage,
            source: crate::features::HitSource::Projectile,
            attacker,
            target: crate::features::HitTarget::Volume,
            mode: crate::features::HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
        };
        let hit = crate::features::ecs_hit_event_hits_breakable(&hit_event, &ecs_breakables)
            || crate::features::ecs_hit_event_hits_actor(&hit_event, &ecs_actors)
            || crate::features::ecs_hit_event_hits_boss(&boss_catalog, &hit_event, &ecs_bosses);
        if hit {
            if proj.explode_half > 0.0 {
                // Fireball: the splash box covers the body we hit plus anything
                // around it, so skip the single-target write and detonate.
                emit_fireball_explosion(
                    pos,
                    proj.damage,
                    proj.explode_half,
                    attacker,
                    &mut feature_damage,
                    &mut sfx,
                    &mut vfx,
                );
            } else {
                feature_damage.write(hit_event);
                sfx.write(ambition_sfx::SfxMessage::Hit { pos });
            }
            commands.entity(entity).despawn();
            continue;
        }
        // Solid wall in this step → impact + expire (Fireball detonates here too).
        // Uses the carved world, so a portal opening is NOT a wall.
        let step = (vel * dt).length().max(1.0);
        if let Some((hit_pos, _normal)) = crate::platformer_runtime::collision::raycast_solids(
            &*collision_world,
            pos,
            vel,
            step,
            false,
        ) {
            if proj.explode_half > 0.0 {
                emit_fireball_explosion(
                    hit_pos,
                    proj.damage,
                    proj.explode_half,
                    attacker,
                    &mut feature_damage,
                    &mut sfx,
                    &mut vfx,
                );
            } else {
                vfx.write(ambition_vfx::vfx::VfxMessage::Impact { pos: hit_pos });
            }
            commands.entity(entity).despawn();
            continue;
        }
        let delta = vel * dt;
        kin.pos += delta;
        proj.traveled += delta.length();
        let oob = kin.pos.x < -64.0
            || kin.pos.y < -64.0
            || kin.pos.x > world.0.size.x + 64.0
            || kin.pos.y > world.0.size.y + 64.0;
        if proj.traveled > HELD_SHOT_MAX_RANGE || oob {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
