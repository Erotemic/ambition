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
use ambition_characters::brain::ActorControl;
use ambition_characters::brain::{
    ActionSet, HeldItemSpec, HeldUseBehavior, MeleeActionSpec, SwipeSpec,
};
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
        // ⭐ **THE ITEM DOMAIN PUBLISHES ITS OWN QUESTION.** Registered from the
        // domain's own plugin and naming no other domain — which is the whole
        // acceptance for the condition contract: a provider that had to be
        // listed somewhere central would not be a provider, it would be a case
        // in somebody else's match.
        {
            use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
            app.publish_condition(conditions::is_held_descriptor(), conditions::is_held);
        }
        // **Durable room state, and the only leg of it that has a producer.**
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
                // ⭐ **BEFORE the pickup, and that placement is load-bearing** —
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
                ground_item_physics
                    .run_if(ambition_platformer2d_shared_tangle::schedule::gameplay_allowed),
                // ⭐ **RESIDENCY FOLLOWS CUSTODY.** Last in the chain, so it sees
                // the custody this tick actually settled on — the release derive
                // above ran first, the pickup and the throw wrote directly. And
                // deliberately UNGATED: a room transition suspends gameplay
                // between the crossing and the commit, which is precisely the
                // window in which the room sweep reads residency.
                project_custody_onto_residency,
                // ⭐ **WHERE A CARRIED OCCURRENCE CAME TO REST.** Strictly
                // between residency and the custody projection below, and both
                // edges are load-bearing — see this system's own doc. It turns
                // the outgoing `InCustody` row of an object that was just put
                // down into the `Placed` row that survives the room's unload;
                // the custody projection below then finds nothing to retract.
                record_placed_ground_items,
                // ⭐ **AND WHAT THE WORLD REMEMBERS ABOUT IT.** Immediately
                // after residency, because it reads residency: an occurrence in
                // somebody's custody is alive and is not the room's to rebuild,
                // so the room that authored it must not mint a second one
                // behind the same `SimId::placement(..)`.
                //
                // ⚠ registered from here and written NOWHERE near here: the
                // system is generic lifecycle vocabulary (it queries
                // `InCustodyOf`, which knows nothing about items) and this is
                // simply the chain whose last link produces its input. ⛔ do not
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
                crate::physics::tick_temporary_zones
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

/// **The body a runtime-minted instance gets.** A mint has no authored record to
/// take a size from, so this is a property of the MINT SITE rather than of the
/// instance — which is why it is not part of the instance's durable description
/// and why the checkpoint restore rebuilds one with the same constant rather
/// than remembering a copy per object.
const MINTED_ITEM_HALF_EXTENT: Vec2 = Vec2::splat(PICKUP_HALF);

/// **WHERE A PHYSICAL ITEM IS — the state that replaced destroy-and-recreate.**
///
/// An item entity used to be DESPAWNED on pickup and a fresh one SPAWNED on
/// throw, so the axe you threw was a different object from the axe you picked
/// up: same spec, new entity, and — for an LDtk-authored ground item, which the
/// construction executor stamps with `SimId::placement(spec.id)` — a **destroyed
/// identity**. Nothing downstream could ask "is this the same axe": not a
/// snapshot row, not a desync report, not a save file.
///
/// So custody is a VALUE the item carries, never the presence or absence of the
/// item itself. ⛔ **retract by resetting, never by removing**: expressing "no
/// longer lying on the floor" by deleting the entity (or the `GroundItem`
/// component) drops it out of every query that needs it — including the throw,
/// which then has nothing to hand back and has to invent a replacement.
///
/// ## Which items are INSTANCES and which are QUANTITIES
///
/// This component is for the first kind only, and the split is the deliverable:
///
/// * **INSTANCE** — [`GroundItem`]. One physical object with a place in the
///   world: an axe, a javelin, a bomb, the gun-sword. It is authored at a
///   position, it falls, it can be thrown through a portal, and *which one it
///   is* is a fact the world can be asked about. It gets an identity and it
///   keeps it across custody.
/// * **QUANTITY** — a `PickupFeature` carrying
///   `PickupKind::Currency`/`Health`, and the `counts` table in `OwnedItems`.
///   A coin has no individuality worth preserving: two coins are the same coin,
///   and what survives collection is a NUMBER on the collector, not an object.
///   Its entity is a DISPENSER of that number, which is why collection marks it
///   `Collected` and credits a wallet rather than moving anything.
/// * **CONSUMABLE** — [`WorldItem`](crate::items::world_item::WorldItem). Touch
///   it and its equipment row transfers to the body; the object itself ends.
///   That despawn is a real end of life, not a custody change, so it stays.
///
/// ⚠ **the inventory leg of `world → held → inventory → world` is NOT closed,
/// and cannot be here.** `OwnedItems` is a process-global count table with no
/// row per object, so an instance stowed into it becomes a quantity and its
/// identity is gone. Equipping from the menu therefore MINTS a fresh instance
/// on throw (see [`throw_held_item_system`]). That is not a bug to patch at this
/// layer: **participant entitlement, body inventory, and physical custody are
/// three different facts with three different owners**, and until the inventory
/// is one of them there is no answer to *whose* inventory a possessed body
/// fills. `equip_held_spec`/`unequip_held` keep the two current ends coherent —
/// they are a migration seam, not the model.
///
/// ⛔⛔ **and MEASURED (2026-08-15): one entitlement can manifest UNBOUNDEDLY
/// MANY objects.** An entitlement in the count table equips into a hand with no
/// object behind it, the throw's mint arm materializes one, and the count
/// survives the throw — so equipping again materializes a SECOND. Repeat for a
/// third.
///
/// ⭐ **HALF OF THAT IS CLOSED (D132, 2026-08-16), and it is the half that came
/// from an OBJECT.** `pickup_held_item_system` used to `grant` a catalog count
/// for an item that was ALSO a live instance, so one acquisition left two
/// records — and only the object's rewound, because `OwnedItems` is not
/// checkpoint state. A death that returned a picked-up weapon to its pedestal
/// left the row behind, and the row then minted a second weapon. That grant is
/// DELETED: picking an object up writes nothing to the catalog, and
/// [`OwnedItems::count`](crate::items::OwnedItems::count) instead PROJECTS the
/// equipped slot, so the grid shows what the hand holds and loses it exactly when
/// the hand does. The two populations are disjoint now: a row is a quantity with
/// no object; an object is an occurrence the checkpoint owns.
///
/// ⛔ **the GRANTED half is still open, and the tempting fix is still wrong.** A
/// quantity conferred by `<<give_item>>`, a shop or a drop still keeps its row
/// through the mint, so it can still manifest a second object. ⛔ do not "fix" it
/// by spending the count on throw while the catalog sits outside the checkpoint
/// horizon: a death that retracts an instance minted after the checkpoint would
/// find the quantity already spent and annihilate it, which trades a duplication
/// bug for a deletion bug. THE GATE is `OwnedItems` participating in the
/// checkpoint baseline; the mint can spend the row in the same change and not
/// before.
///
/// ⭐ **CUSTODY ALSO DECIDES WHERE THE OBJECT LIVES.** An object in a travelling
/// body's custody is not resident in any room, so a room change does not retire
/// it and the identity survives the door as well as the hand. That is a
/// PROJECTION of this value, not a second fact:
/// [`project_custody_onto_residency`], which owns the whole story.
///
/// ⭐ **IT OPENED ONE, AND IT IS CLOSED — by durable room state, not by
/// anything here.** An authored `GroundItem` carries `SimId::placement(..)`, and
/// the room that authored it rebuilds its whole roster on every load, so
/// carrying an authored axe out of its room and back in produced the axe in your
/// hands AND a fresh one on the floor, both claiming the same placement id. The
/// fix is that construction now asks what became of the occurrence a record
/// minted last time
/// (`lifecycle::AuthoredOccurrences` / `OccurrenceWhereabouts`) and mints a new
/// one only for records whose last occurrence is neither alive elsewhere nor
/// deliberately gone. ⛔ it was NOT fixed by re-destroying carried objects at the
/// boundary: the projection that suppresses re-authoring reads `InCustodyOf`,
/// which knows nothing about items.
///
/// ⭐ **and where the object came to REST is this file's business**, because a
/// position is not generic vocabulary — see [`record_placed_ground_items`]. The
/// ledger's rows are occurrence-generic; its producers live with the families
/// that have a position to read.
///
/// ⚠ **rollback state, not a cache.** It gates whether the item is drawn,
/// simulated, or collectible on a later frame, so a rewind that restored the
/// wrong custody would leave an axe both in a hand and on the floor. This
/// domain registers it in `crate::rollback_registration` as
/// `entity:item_custody` (clone + entity-set probe), paired with
/// `rollback_map_entities` because the holder handle is remapped on load.
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
/// ⭐ **[`ItemCustody`] is REQUIRED, not inserted at each spawn site.** There are
/// eight production sites that build one (LDtk construction, the boss and actor
/// death drops, the sandbox reset, the bomb and grenade debug spawns, the
/// throw), and the `SimId`/`SimIdCounter` precedent in this repo is explicit
/// about what happens to a pairing that each site has to remember: two of six
/// forgot, and a shipped boss lost its summon. A required component makes
/// "every ground item has a custody" a property of the TYPE, so a ninth spawn
/// site cannot omit it and cannot default to a state that reads as "not in the
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

/// Integrate thrown ground items under gravity (y-down world) and settle them
/// when they'd enter a solid / one-way surface. Resting items (`vel == ZERO`)
/// are skipped, so pickup-able items stay put.
pub fn ground_item_physics(
    time: Res<ambition_time::WorldTime>,
    world: ambition_platformer2d_world::collision::CollisionWorld,
    gravity: crate::physics::GravityCtx,
    mut grounds: Query<(&mut GroundItem, &ItemCustody)>,
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
    for (mut item, custody) in &mut grounds {
        // A carried item has no independent motion — it is not in the world to
        // fall through. Checked on custody rather than inferred from `vel ==
        // ZERO`, because "resting" and "in a hand" are different states that
        // happen to share a velocity, and only one of them may be stepped.
        if !custody.in_world() {
            continue;
        }
        if item.vel == Vec2::ZERO {
            continue;
        }
        // Free bodies resolve gravity by the body-overlap rule, not the center
        // point (ADR 0024) — a zone grabs an item the item TOUCHES.
        let local = crate::physics::GravityField {
            dir: gravity.dir_for(ae::Aabb::new(item.pos, item.half_extent)),
        };
        crate::physics::apply_world_forces(&mut item.vel, GROUND_ITEM_GRAVITY, &local, dt);
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
        if blocked {
            // Settle in place (simple — no slide).
            item.vel = Vec2::ZERO;
        } else if outside_world {
            item.vel = Vec2::ZERO;
        } else {
            item.pos = next;
        }
    }
}

/// **AN OBJECT ITS HOLDER LET GO OF IS BACK IN THE WORLD, NOT NOWHERE.**
///
/// [`ItemCustody::Held`] names a body, and it is only TRUE while that body is
/// actually holding the thing. Two production paths empty a hand without ever
/// touching the object behind it, and neither can: the inventory menu's Stow,
/// and the menu's equip-swap (which releases the old weapon before taking the
/// new one). Each one used to leave the item recording a custody that had
/// stopped being true — a state that is in NEITHER arm of what the enum MEANS:
/// not in the world, so [`ground_item_physics`] skips it,
/// [`pickup_held_item_system`] skips it, and the drawn view skips it; and not in
/// anyone's hand, so [`throw_held_item_system`] cannot find it either. The
/// authored axe carrying `SimId::placement(..)` simply stopped existing — which
/// is exactly the outcome [`ItemCustody`] was introduced to prevent, reached
/// through the inventory menu instead of through a despawn.
///
/// ⛔ **retract by RESETTING, never by removing.** The object is not despawned
/// and its identity is never re-minted; its custody returns to
/// [`ItemCustody::InWorld`] at the holder's last position, which is where a body
/// that stops holding something leaves it. Picking it back up is the ordinary
/// pickup, so the authored identity survives a stow the same way it survives a
/// throw.
///
/// ⭐ **it runs FIRST in the item chain.** [`pickup_held_item_system`] writes
/// `Held { holder }` DIRECTLY but attaches [`HeldItem`] through `Commands`, so a
/// release check running after it in the same tick would be reading a hand that
/// has not been filled yet and would drop the item the pickup just took. Reading
/// last tick's settled state removes the question rather than depending on where
/// a sync point lands.
///
/// ⚠ **this is a DERIVE, not a follow-up call.** The repo's rule is that a second
/// step belongs inside the first, and it is not available here: every release
/// site that orphans an object lives in the inventory menu, which runs in
/// `Update`, in another crate, holding no item query. What a caller cannot be
/// given, a caller cannot be trusted to remember — so custody is re-derived from
/// the hand each tick instead. The write is idempotent (once `InWorld`, the first
/// guard below skips it forever), which is what makes it safe under rollback's
/// re-simulation of the same tick.
///
/// ⚠ **body-generic.** `holder` is whatever entity took the item — a couch seat,
/// a possessed actor, an NPC. Nothing here asks whether it is a player.
///
/// ⛔ **a holder that DESPAWNED is deliberately NOT released here, and that is a
/// known remaining orphan.** "What happens to a body's inventory when the body
/// dies" already has an owner — `caps.drops_held_item` in the actor death
/// resolver, which MINTS a fresh `GroundItem` from the corpse's `HeldItem` spec.
/// Releasing the carried object here as well would put two axes on the floor
/// where the design says one. Answering it properly means the death drop handing
/// BACK the object it has custody of instead of manufacturing a copy, which is
/// the same unclosed inventory leg described on [`ItemCustody`] — not this
/// function's to decide.
pub fn return_released_items(
    // The hand, and where the body is standing. `Option<&HeldItem>` rather than
    // `Has<..>`: an equip-SWAP leaves the body holding a DIFFERENT item, so
    // "there is a hand" is not the question — "is this object the thing in it"
    // is, and only the spec id can answer that.
    holders: Query<(&BodyKinematics, Option<&HeldItem>)>,
    mut carried: Query<(&mut GroundItem, &mut ItemCustody)>,
) {
    for (mut item, mut custody) in &mut carried {
        let ItemCustody::Held { holder } = *custody else {
            continue;
        };
        // A holder that no longer exists is the death-drop resolver's question,
        // not this one's — see the ⛔ above.
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
    }
}

/// **CUSTODY OWNS RESIDENCY — an object in a hand does not live in the room.**
///
/// [`ItemCustody`] kept an object's ENTITY and its `SimId` alive across
/// world → held → world, and that was true only inside one room: an authored
/// [`GroundItem`] is spawned
/// [`RoomScopedEntity`](ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity),
/// it kept that scope while `Held`, and `RoomConstructionPlan::retire_outgoing`
/// despawns every room-scoped entity except the transiting body. So the axe you
/// carried through a door was destroyed at the door — the same destroyed
/// identity `ItemCustody` exists to prevent, reached through the room boundary
/// instead of through the pickup.
///
/// ⛔ **the fix does NOT live in the transition.** Teaching the room commit to
/// walk a body's held items would make the boundary know that inventories
/// exist, which is the player-centric composition-root special case this
/// architecture removes. Custody already NAMES the holder; residency is a
/// PROJECTION of it, and this is the projection:
///
/// * `Held { holder }` by a body that is not itself a room resident ⇒ the
///   object's residency is that body's, spelled
///   [`InCustodyOf`](ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf).
///   It keeps its room SCOPE (so nothing that requires the scope loses it, and a
///   sandbox reset — which empties the hand too — still destroys it) and drops
///   out of the roster a room CHANGE retires;
/// * `Held { holder }` by a body that IS a fixture of this room (an unpossessed
///   NPC still carries `RoomScopedEntity`) ⇒ resident. The object dies with the
///   room exactly as the body holding it does. ⭐ possession promotes a body it
///   takes over OUT of room scope for precisely this reason, so a possessed
///   carrier gets the travelling answer without anyone asking who the player is;
/// * a holder that no longer EXISTS confers nothing, so the object is resident
///   again. The orphan a dead holder leaves (see [`return_released_items`]) is
///   still the death-drop resolver's question — but it now dies with its room
///   instead of escaping every sweep in the engine forever.
/// * `InWorld` ⇒ resident, in whatever room is active. That is what "dropped in
///   the destination room" means, and it needs no memory of where the object was
///   picked up: room residency is presence-driven and carries no room id.
///
/// ⚠ **a DERIVE, not a follow-up call at each custody write.** Four sites move
/// custody (pickup, throw, the release derive above, and the throw's mint), and
/// the repo's own rule is that the second step belongs inside the first — but
/// the first step is a bare `*custody = ..` in systems that do not all hold
/// `Commands` over the object. Re-deriving the whole projection each tick from
/// the state that already decides it removes the question, exactly as
/// [`return_released_items`] re-derives custody from the hand. It is a pure
/// function of [`ItemCustody`] (which IS rollback state) refreshed
/// unconditionally, so it carries no "already applied" gate of its own and needs
/// no rollback registration to converge after a rewind.
///
/// ⭐ **and it is deliberately NOT gated on `gameplay_allowed`.** A transition
/// suspends gameplay between the crossing and the commit; residency must be
/// right on those frames above all others, because that is exactly when the
/// sweep runs.
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
    // Whether a holder is itself a resident of the room. Deliberately unfiltered
    // — ANY entity can hold something, and a query that required a body cluster
    // would answer "not room-scoped" for a holder it simply could not see.
    holders: Query<
        bevy::prelude::Has<ambition_platformer2d_shared_tangle::lifecycle::RoomScopedEntity>,
    >,
) {
    use ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf;
    for (entity, custody, suspended) in &items {
        let holder = match *custody {
            ItemCustody::InWorld => None,
            ItemCustody::Held { holder } => match holders.get(holder) {
                // A room fixture's hand, or a holder that is gone: resident.
                Ok(true) | Err(_) => None,
                Ok(false) => Some(holder),
            },
        };
        match (holder, suspended) {
            // Already says what it should say. Checked before writing so this
            // does not queue a command per ground item per tick.
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

/// **PUT THE HANDS BACK TO WHAT THE CHECKPOINT SAW — the item domain's leg of
/// the reset horizon, and both directions of it.**
///
/// ```text
/// carried now, NOT in the baseline   → take it back: unequip, despawn, let the rebuild author it
/// in the baseline, NOT in that hand  → put it back: reset custody and re-equip
/// in the baseline AND in that hand   → nothing to do
/// ```
///
/// ⭐⭐ **ONE system for both directions, deliberately.** They read one value and
/// they are the same decision seen from two sides; split across two systems,
/// each would have to re-derive what the other concluded, and the pair would
/// drift the first time only one of them learned about a new case.
///
/// ⛔⛔ **IT LIVES HERE, NOT WITH THE BASELINE, BECAUSE CUSTODY IS A FORKED
/// RELATION.** `ItemCustody`/`InCustodyOf` on the object and `HeldItem` on the
/// body are two halves of one fact, and `HeldItem` is in a crate the lifecycle
/// crate cannot see. A retraction that answered only the half it could reach
/// left the body holding a spec whose object no longer existed, and the player
/// could never pick anything up again — the acceptance fixture caught exactly
/// that, and caught it because it drove a SECOND pickup.
///
/// ⛔ **and the tempting generic repair is worse than the bug**: "empty any hand
/// whose spec matches nothing in that body's custody" would disarm every
/// authored fighter on its first frame, because a character definition's
/// `held_item` puts a `HeldItem` on a body with no world object behind it at all.
///
/// ⭐ **it goes through [`equip_held_spec`] and [`unequip_held`] rather than
/// inserting and removing [`HeldItem`].** Those verbs also swap the body's
/// `ActionSet` and its `StashedActionSet`, so a hand-written `remove::<HeldItem>`
/// leaves the body wielding the item's attack verbs with no item — which is what
/// the first draft of this did.
///
/// ⚠ **[`InCustodyOf`] is NOT written here.** It is derived from `ItemCustody`
/// by [`project_custody_onto_residency`] on the same tick; writing it too would
/// be a second producer of a projected value.
///
/// ⛔ **it must NOT touch the ledger.** The occurrence leg restores that whole
/// value from its own baseline. Two systems retracting one fact is how a
/// retraction survives one of them being deleted and quietly stops working.
///
/// ⭐⭐ **AND THE THIRD ARM: A BASELINE ROW WHOSE OCCURRENCE THE WORLD NO LONGER
/// HAS AT ALL.** Carried at the checkpoint, later put down in a room, that room
/// unloaded and took the entity with it, then a death. Re-assignment cannot
/// reach it — there is nothing to re-assign — and no room rebuild will ever
/// produce it either, because a baseline that says `InCustody` makes
/// `outlook_for` answer `Suppressed` in *every* room, which is correct: a thing
/// in a hand is not a thing in a room. So the occurrence has to be
/// MATERIALIZED, by identity, from the record that minted it wherever in the
/// world that record lives — see
/// [`authored_occurrence_request`](crate::construction::authored_occurrence_request).
/// It comes back with the record's own `SimId` and provenance, which is what
/// makes it the same occurrence rather than a look-alike.
///
/// ⭐⭐ **AND THE FOURTH: A ROW NO RECORD ANYWHERE CAN DESCRIBE.** Materializing
/// from a record is bounded by *"some room authors this id"*, and a
/// RUNTIME-MINTED instance ([`SpawnOrigin::Dynamic`](ambition_platformer2d_shared_tangle::construction::SpawnOrigin))
/// is room-scoped and carryable — it can enter the baseline, and no record can
/// rebuild it. The checkpoint therefore captures a durable DESCRIPTION of one at
/// commit time ([`minted_horizon`]) and this arm rebuilds from that. The two
/// describers are disjoint populations, not a preference order.
#[allow(clippy::too_many_arguments)]
pub fn restore_custody_to_checkpoint(
    // ⭐ **`SessionCommands`, because materialization SPAWNS.** An occurrence
    // rebuilt into a hand is owned by the activation that is restoring, exactly
    // as the room build's would be; a bare `Commands` could only produce a
    // process-resident stranger that outlives the session.
    mut commands: ambition_platformer2d_shared_tangle::lifecycle::SessionCommands,
    mut resets: MessageReader<ambition_platformer2d_shared_tangle::lifecycle::ResetToCheckpoint>,
    baseline: Option<Res<ambition_platformer2d_shared_tangle::lifecycle::CustodyBaseline>>,
    // **The world's DEFINITIONS**, so an identity with no live occurrence behind
    // it can still be turned back into one. Every room, not the neighbours: a
    // body can carry an object any distance before putting it down, so the room
    // holding the record is not reachable by adjacency.
    world: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
    >,
    // **The checkpoint's own DESCRIPTIONS of what the simulation minted**, for
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
    // ⚠ a `BTreeMap` rather than the query's order: this drives despawns, and
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

    // ⛔⛔ **RETRACTIONS FIRST, REINSTATEMENTS SECOND, AND THE ORDER IS THE BUG
    // THIS PARAGRAPH EXISTS FOR.** A body has one hand. Interleaved, an
    // occurrence being put back can be equipped while the object being taken
    // away is still in that hand; the equip and the unequip are both commands on
    // one entity, the later one wins, and `return_released_items` then sees a
    // hand whose spec does not match and releases the object this reset just put
    // there. The fixture reported it as `InWorld` — the reinstatement had
    // happened and been quietly undone one phase later.
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
                // ⭐ a DESPAWN, and that is the point rather than a shortcut. The
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

    // ── ⭐⭐ AND THE ROWS NO OBJECT IN THE WORLD ANSWERS FOR ──────────────────
    //
    // ⛔ **this pass is driven from the BASELINE, not from the world, and that
    // is the whole difference.** Everything above starts at a live occurrence
    // and asks whether the checkpoint agrees with where it is — a question that
    // cannot be asked at all about an occurrence whose entity is gone. Those
    // rows are invisible to every query in the engine, so the only place they
    // exist is the baseline, and the only way to find them is to enumerate it.
    //
    // ⚠ **LAST, after the retractions, for the reason the partition above
    // exists**: a body has one hand, and the object being taken out of it must
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
    // ⛔⛔ **A MINT LEFT LYING IS NOT THIS SYSTEM'S TO REBUILD, and trying it
    // here was measured wrong on 2026-08-19.** A dropped occurrence has no
    // custodian, so it is structurally absent from `CustodyBaseline`; the
    // obvious repair is a second loop over `Placed { room, at }` rows. It does
    // not work, and the reason is TIMING: `ResetToCheckpoint` is processed while
    // the active room is still the one the player died in, so the row's room is
    // not loaded and nothing may be spawned into it.
    //
    // ⇒ **the debt belongs to the ROOM BUILD**, which already settles exactly
    // this obligation — `outlook.reinstatements()` in `features/ecs/spawn`,
    // relocating an authored record to where the ledger says the object lies. A
    // runtime mint falls through it to a warn because no room authors a record
    // for it. See D133; the fix is a describer arm there, not a second
    // materializer here.
    if missing.is_empty() {
        return;
    }
    // Resolved once, and only when there is something to materialize: a shell
    // host at a non-gameplay route must not author gameplay entities.
    //
    // ⚠ **the session world is NOT required here any more, and that is the
    // second describer arriving.** It used to be, because the authored record
    // was the only thing that could rebuild an occurrence; a runtime mint has no
    // record and is rebuilt from the checkpoint's own description, so a
    // composition with no world can still put one back. The authored arm below
    // asks for the world itself.
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
        // ⭐⭐ **TWO DESCRIBERS, AND WHICH ONE ANSWERS IS DECIDED BY WHERE THE
        // OCCURRENCE CAME FROM.** An authored occurrence is rebuilt by the record
        // that minted it; a runtime mint has no record anywhere and is rebuilt
        // from the description the checkpoint captured of it. Asking the
        // checkpoint FIRST is not a preference between two answers — the two
        // populations are disjoint by construction, because the capture takes
        // only `SpawnOrigin::Dynamic` rows and an authored record can never
        // spell one.
        let described = minted
            .as_deref()
            .and_then(|minted| minted.description_of(&occurrence));
        let rebuilt = match described {
            // ── the simulation minted it: identity + provenance + spec id ─────
            Some(description) => match held_spec_by_id(&description.held_item) {
                Some(spec) => Some((
                    description.origin.clone(),
                    // ⭐ **NO POSITION IS REMEMBERED AND NONE IS NEEDED.** The
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
                    // ⚠ a CONTENT change: the item's spec has been edited out of
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
                    // ⚠ the family that can be CARRIED and the family that can be
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
                    // ⚠ a CONTENT change rather than a defect: the record has
                    // been edited away since the checkpoint was taken (or this
                    // composition has no session world to look in). Loud, and
                    // the player simply no longer has the thing.
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
        // ⛔ **the occurrence's OWN `SimId` and provenance**, which is what makes
        // this the same occurrence coming back rather than a copy wearing its
        // name. A fresh identity here would be a silent duplication the moment
        // the home room decided the original was still out there — and for a
        // runtime mint, a rebuilt entity with no `SpawnOrigin::Dynamic` would be
        // invisible to the NEXT capture, so the object would survive exactly one
        // death and then become unrecoverable.
        //
        // ⚠ **`InCustodyOf` is NOT written here**, for the same reason the arms
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

/// **WHERE A CARRIED OCCURRENCE CAME TO REST — the second producer of the
/// whereabouts ledger, and the first one that outlives the thing it describes.**
///
/// Custody answers *"in a hand"*, which suppresses re-authoring for exactly as
/// long as the hand exists. An object PUT DOWN is in nobody's custody, so
/// custody can say nothing about it — and the room it was dropped in destroys it
/// on unload, after which the world's only memory of where it was is this row.
///
/// ⭐ **it tracks only occurrences the ledger ALREADY remembers.** The condition
/// is `remembers(sim_id)`, which on the tick a hand empties is still the
/// outgoing `InCustody` row — so the population is exactly "things somebody
/// carried", never "every object in the room". A producer that recorded every
/// authored occurrence's position would be the universal instance registry this
/// ledger exists to not be, and would rewrite an enemy's row on every step it
/// took.
///
/// ⛔ **ORDER: strictly BEFORE
/// [`project_custody_onto_authored_occurrences`](ambition_platformer2d_shared_tangle::lifecycle::project_custody_onto_authored_occurrences),
/// and strictly after
/// [`project_custody_onto_residency`].** The custody projection retracts the
/// `InCustody` row of anything no longer carried; run first, it would erase the
/// only evidence this system uses to decide that an object is worth tracking,
/// and a dropped object would be forgotten on the very tick it was dropped.
///
/// ⚠ **it re-states the position every tick while the room is loaded**, which is
/// why a THROWN object records where it landed rather than where it left the
/// hand: the row is republished through the whole arc and simply stops being
/// republished when the room unloads. That per-tick republish is also what keeps
/// the row a re-derivable value inside the rollback window.
///
/// ⚠ **item-domain producer, generic vocabulary.** The ledger's rows are
/// occurrence-generic; a producer must read a POSITION, and there is no generic
/// position for a simulated occurrence — so the producers live with the families
/// that have one. Room transition still knows nothing about items.
pub fn record_placed_ground_items(
    room_set: Option<
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<crate::rooms::RoomSet>,
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
    // ⭐ **BTreeMap, not the query's order.** This value reaches a construction
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
        // ⛔⛔ **AN OCCURRENCE COMES TO REST HERE ONLY IF IT WAS IN A HAND, OR
        // WAS ALREADY RESTING HERE — and that is an invariant, not a filter.**
        //
        // ⭐ **an object cannot change rooms without being carried.** Every
        // legitimate relocation passes through custody: picked up in A (the row
        // becomes `InCustody`), carried, put down in B (this system writes
        // `Placed { B }`). So a live object lying in the ACTIVE room whose row
        // says it is `Placed` in ANOTHER room — or `Consumed` — is not an object
        // that moved. It is a stale duplicate the world should not be holding,
        // and believing it would be the ledger taking dictation from the very
        // duplication it exists to prevent.
        //
        // ⚠ **the case that forced this is the DURABLE LOAD** (2026-08-16). A
        // session builds its start room from authored records before anything has
        // read a save file, so the moment the file's ledger is installed the world
        // holds an occurrence the file says is somewhere else. Without this arm
        // the very next tick republished the stale position over the loaded row,
        // and the rebuild that was already on its way then put the object back in
        // the room the player had carried it out of. The same tick order also
        // resurrected an occurrence whose row was terminal.
        //
        // ⭐ **it refuses rather than repairs**, deliberately. Retracting the
        // stale entity here would make this a second reconstruction authority
        // beside `outlook_for`; the room rebuild that the load requests sweeps it
        // with everything else this room minted, and until then the ledger simply
        // keeps saying the true thing.
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
    // Compared before writing: a resting object would otherwise mark the ledger
    // changed on every tick of its life.
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

/// **Resolve a held item's authored spec id back to its spec — the reverse of
/// [`HeldItemSpec::id`], and the item domain's answer to "what is this thing".**
///
/// ⭐ **the reverse direction is what makes a durable description possible.** A
/// checkpoint that wants to rebuild a runtime-minted instance stores the id and
/// nothing else (see
/// [`minted_horizon`](crate::items::pickup::minted_horizon)); storing the
/// resolved spec would put a second authority for *what a javelin is* inside a
/// snapshot, so the id has to be resolvable from outside.
///
/// ⚠ **both registries, in that order, because there are two.** The three wired
/// weapons are built here by [`held_spec_for_item`] and are NOT rows in
/// `ambition_characters`'s table; the pirates' `gun_sword_heavy` is a row there
/// and has no catalog slot. Consulting one alone silently loses half the items.
pub fn held_spec_by_id(id: &str) -> Option<HeldItemSpec> {
    crate::items::Item::from_held_item_id(id)
        .and_then(held_spec_for_item)
        .or_else(|| ambition_characters::brain::held_item_by_id(id))
}

/// **TAKE custody of a held item — one operation, both ends.**
///
/// Stash the current action set, overlay the item's verbs, attach [`HeldItem`],
/// and name the catalog slot this body is now holding. Every way a body comes to
/// hold a weapon calls this: the world pickup ([`pickup_held_item_system`]) and
/// the inventory menu. There is no second place that writes half of it.
///
/// ⭐ **`owned` is a PARAMETER, not a follow-up call.** `OwnedItems::equipped`
/// and the presence of `HeldItem` on the body are ONE FACT STORED TWICE, and
/// while the two were maintained at separate sites they drifted — dropping the
/// portal gun cleared the component and left the menu still claiming the gun was
/// equipped, because that one release path forgot the second edit. Handing the
/// catalog to the transfer is what makes the two ends move together or not at
/// all. `None` means "this body has no catalog behind it" (a headless fixture, a
/// game with no inventory) — never "skip the bookkeeping".
///
/// ⚠ **it does NOT grant, and after D132 nothing on this road does.** Entitlement
/// (*do you own an axe as a quantity, with no object behind it*) and custody
/// (*are you holding one*) are different questions, and picking an object up
/// answers only the second: the object is the record, and
/// [`OwnedItems::count`](crate::items::OwnedItems::count) projects the equipped
/// slot this function writes. `grant` belongs to the sites that confer a
/// quantity with no object — `<<give_item>>`, the shop, ability drops.
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

/// **RELEASE custody of a held item — the twin of [`equip_held_spec`].**
///
/// Restore the stashed action set, detach [`HeldItem`], and clear the catalog's
/// equipped slot. The body stops holding it here and nowhere else.
///
/// ⛔ **the OBJECT's end is NOT this function's, and it used to be nobody's.** The
/// throw writes the launch onto the object itself; the inventory menu writes
/// nothing at all, and for a picked-up instance that left the object recording
/// `ItemCustody::Held` by a body with an empty hand — invisible, unpickable and
/// unthrowable for the rest of its life. Nothing here has an item query to fix
/// that with (the menu calls this from `Update`, in another crate), so custody is
/// re-derived from the hand instead: see [`return_released_items`]. A caller that
/// simply lets go can no longer destroy an authored object.
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

/// **TAKE custody of the portal gun** — the portal-gun twin of
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

/// **RELEASE custody of the portal gun** — the portal-gun twin of
/// [`unequip_held`]. Detach [`PortalGun`], restore the stashed action set, and
/// clear the catalog's equipped slot.
///
/// ⛔ **the world DROP used to inline this and omit the last step**, so a player
/// who threw the gun on the floor kept an inventory screen insisting it was
/// equipped. The throw path did clear it. Same operation, two hand-written
/// copies, one of them missing an edit — which is the whole argument for the
/// slot being this function's business rather than each caller's.
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
/// ⚠ **this is a PRESS-gated action, and that is why it stays on one subject
/// while the touch-collectors are body-generic.** Picking a weapon off the floor
/// spends an Attack press on a specific body's `ActorControl`; walking into a
/// coin spends nothing. The `ControlledSubject` here is not a player-centrism
/// leftover — it is "the body whose press this is". The touch-collect fork lives
/// in `features::ecs::pickups`.
///
/// ⛔ **it does not DESTROY the item.** See [`ItemCustody`].
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
        // Only an item that is IN THE WORLD can be grabbed. Without this a
        // second body could take an item straight out of the first body's hand,
        // because the item entity is still alive — the exact class of bug the
        // old despawn hid by destroying the evidence.
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
            // ⛔⛔ **THE CATALOG IS NOT WRITTEN HERE, AND THE DELETED WRITE IS
            // D132.** This used to `owned.grant(item, 1)` — "you now OWN an axe"
            // — beside taking custody, so ONE acquisition left TWO records: the
            // object, which the checkpoint captures and a death rewinds, and a
            // catalog row, which no checkpoint has ever seen. A player who picked
            // a weapon up after the last shrine and died got the object back on
            // its pedestal AND kept the row; the inventory menu would then equip
            // the phantom and mint a SECOND weapon on the first throw, and the
            // durable save wrote it to disk on the way past.
            //
            // ⭐ the object is the record. `OwnedItems::count` PROJECTS the hand
            // (via the equipped slot `equip_held_spec` writes below), so the grid
            // still shows the axe you are carrying — derived, retracted by the
            // same reset that retracts the object, and impossible to disagree
            // with. See [`OwnedItems`](crate::items::OwnedItems)'s own docs.
            //
            // CUSTODY: the ONE take-custody operation, shared with the inventory
            // menu. This used to be four hand-written edits here and the same
            // four inside `equip_held_spec`, whose own doc called itself a mirror
            // of this loop — so the two could only ever agree by remembering to.
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
            // ⭐ **THE ITEM IS NOT DESTROYED — its custody changes.** This used
            // to be `commands.entity(ground_entity).despawn()`, and with it went
            // the item's `SimId`, its `SpawnOrigin`, and any possibility of the
            // thing you throw being the thing you picked up.
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

/// Throw the held item: restore the stashed action set, detach `HeldItem`, and
/// put the item back into the world ahead of the player. Fires on
/// `Shield + Attack` for any item, or on a plain `Attack` for a pure throwable
/// (throw-on-use).
///
/// ⭐ **"put back", not "spawn".** The object the body took custody of is still
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
/// ⭐ **and it CONSUMES the press, exactly as the pickup does.** A held weapon
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
    gravity: crate::physics::GravityCtx,
    mut bodies: Query<(
        &mut ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        &HeldItem,
        Option<&StashedActionSet>,
    )>,
    // The object this body is CARRYING, found by the custody it records rather
    // than by the hand remembering an entity handle.
    mut carried: Query<(&mut GroundItem, &mut ItemCustody)>,
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
    if !control.0.melee_pressed {
        return;
    }
    // Shield+Attack throws anything; plain Attack throws only items whose
    // authored `use_behavior` opts in, leaving `UseSystem` abilities to their
    // own systems.
    if !(control.0.shield_held || held.spec.throws_on_plain_attack()) {
        return;
    }
    // The throw IS this press's action — see the note on the signature.
    control.0.melee_pressed = false;
    let spec = held.spec.clone();
    let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
    // The launch is authored in the body's LOCAL frame (x = forward/side,
    // y = toward-feet) and rotated into the body's gravity frame, so the throw
    // arcs "ahead + away from feet" under ANY gravity — identity under normal
    // gravity. The subsequent free-fall (`ground_item_physics`) is already
    // gravity-relative, so the whole toss now flips with the field.
    let frame = ae::AccelerationFrame::new(gravity.dir_for(ae::Aabb::new(kin.pos, kin.size * 0.5)));
    let throw_pos = kin.pos + frame.to_world(Vec2::new(facing * THROW_AHEAD, 0.0));
    // Forward + away-from-feet, in the local frame → world.
    let throw_vel = frame.to_world(Vec2::new(facing * THROW_SPEED_X, -THROW_SPEED_UP));
    // CUSTODY, one operation: the hand empties and the catalog's equipped slot
    // clears together. Only the equipped slot moves, never the stored quantity —
    // and after D132 that sentence means two different things depending on which
    // road put the item in the hand. A weapon PICKED UP has no stored row at all,
    // so letting go of it is letting go of it: the object on the floor is the
    // only record, and the grid dims. A weapon equipped out of a GRANTED quantity
    // still has its row, so the thrower keeps catalog ownership and can re-equip.
    //
    // ⛔⛔ **and that second case is the surviving half of D132, deliberately left
    // open.** Throwing a granted quantity MINTS an instance (below) without
    // spending the row, so the row and the object both claim it and a second
    // throw makes a second object. The row cannot simply be spent here: the
    // catalog is not checkpoint state, so a death that retracts a
    // minted-after-the-checkpoint instance would find the quantity already spent
    // and ANNIHILATE it — the mirror image of the phantom this slice removed.
    // ⇒ THE GATE: spending the row at the mint is correct only once `OwnedItems`
    // participates in the checkpoint horizon.
    unequip_held(
        &mut commands,
        player,
        &mut action_set,
        stashed,
        owned.as_deref_mut(),
    );
    // ⭐ **RETURN THE OBJECT, do not manufacture a replacement.** The item this
    // body took custody of is still a live entity carrying its own identity, so
    // the throw resets its custody and writes the launch onto it. This used to
    // be an unconditional `spawn_room_scoped`, which is why picking an authored
    // axe up and dropping it produced an anonymous axe: `SimId::placement(...)`
    // died at the pickup and nothing put it back.
    if let Some((mut ground, mut custody)) = carried
        .iter_mut()
        .find(|(_, custody)| custody.held_by(player))
    {
        ground.pos = throw_pos;
        ground.vel = throw_vel;
        *custody = ItemCustody::InWorld;
        return;
    }
    // ⚠ **NO OBJECT BEHIND THE HAND — materialize one.** A body can come to hold
    // an item with no world instance at all: the inventory menu equips straight
    // out of `OwnedItems`, which is a count table. Throwing that turns a
    // QUANTITY into an INSTANCE, and an instance owes an identity, so it takes
    // `SimId::spawned(thrower, counter.next())` here rather than joining the
    // population of anonymous dropped items. This arm is the visible edge of the
    // unclosed inventory leg described on [`ItemCustody`] — not a fallback that
    // should quietly absorb the common case.
    //
    // ⭐⭐ **IDENTITY AND PROVENANCE ARE MINTED TOGETHER, and the second half used
    // to be missing.** The id alone made this instance nameable and left it
    // unreconstructable: `SpawnOrigin::Dynamic`'s doc says a dynamic entity
    // states which spawner it descends from or it cannot be rebuilt, and
    // `SimId::as_str`'s doc says the spelling may never be parsed to recover
    // that fact. So the checkpoint horizon had no legitimate way to tell a mint
    // from an authored placement, and
    // [`minted_horizon::capture_minted_item_baseline`] — which discriminates on
    // exactly this component — would have described nothing.
    //
    // ⚠ the pair is `Option` as ONE value: a thrower with no identity mints
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
        &crate::physics::ResolvedMotionFrame,
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
        crate::projectile::ProjectileGameplay {
            age: 0.0,
            max_lifetime: f32::MAX,
            gravity: 0.0,
            damage: ranged.damage(),
            bounces_remaining: 0,
            // Stepped by `held_projectile_step` (keyed on `HeldProjectile`), not
            // the ECS projectile world-collision path, so this is inert here; a
            // detonate-on-contact bolt is `ExpireOnContact` in spirit.
            world_hit: crate::projectile::WorldHitPolicy::ExpireOnContact,
        },
        HeldProjectile {
            damage: ranged.damage(),
            traveled: 0.0,
            explode_half,
        },
        Name::new("Held ranged shot"),
    ));
    // Opt the bolt into the ONE generic portal transit AT SPAWN (not via the
    // deferred `ensure_projectile_portal_bodies`), so the host-surface carve opens
    // the SAME frame even for a point-blank shot at a portal — otherwise the bolt
    // would detonate on the still-solid surface one frame before it gets tagged.
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
