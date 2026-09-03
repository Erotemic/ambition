//! The PRESSED collectible: pick up / use / throw held items.
//!
//! A `GroundItem` sits in the world; an empty-handed player presses `Attack`
//! while overlapping it to pick it up — the item's `HeldItemSpec` is overlaid
//! onto the player's `ActionSet` (so e.g. the axe grants its swing) and a
//! `HeldItem` component is attached. `Shield + Attack` throws the held item
//! back onto the ground ahead of the player, restoring the player's original
//! action set. One held item at a time.
//!
//! ⭐⭐ CARVED OUT OF `ambition_platformer2d_actor_monolith::items::pickup` (D33,
//! 2026-09-03), the sibling of `ambition_world_items` (the TOUCHED half) along
//! the collect TRIGGER — touched versus pressed. What moved is the domain: its
//! components, its specs, its systems and its tests. What stayed in the kernel
//! is checkpoint policy (`restore_custody_to_checkpoint`, which reads these
//! components) and the kernel's own systems that attach to this chain (a
//! shrine, a gun, a match spawn).
//!
//! ⛔⛔ THE SCHEDULE MOVED WITH THE LOGIC. `HeldItemSimulationPlugin` configures
//! `ItemPickupSet::CoreHeldItems` — nested in `PlayerSimulation`, after
//! `BodyCustodySettled` — and chains the `HeldItemStep`s inside it, then
//! registers its systems into those steps. The kernel declares the
//! three-variant `.chain()` (`CoreHeldItems → ThrownItemEffects →
//! WieldedAbilities`) because only it names all three; a composition that
//! adds THIS plugin without the kernel's gets a correctly placed
//! `CoreHeldItems` and no chain to the other two, which is right for a unit
//! test and wrong for a game. Guarded by shape in `schedule_tests`.
//! `ambition_world_items` shipped without its phase because its carve moved
//! `add_systems` and not `configure_sets`; this one moves both.

pub mod conditions;

use bevy::prelude::*;

use ambition_characters::brain::{
    ActionSet, HeldItemSpec, HeldUseBehavior, MeleeActionSpec, SwipeSpec,
};
use ambition_characters::control::ActorControl;
use ambition_combat::held_items::HeldItem;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_core::{self as ae, AabbExt};
use ambition_platformer2d_shared_tangle::prelude::SpawnScopedExt;
use ambition_platformer2d_shared_tangle::schedule::{HeldItemStep, ItemPickupSet, SimScheduleExt};
#[cfg(feature = "portal")]
use ambition_portal2d::PortalGun;

/// The pressed-collectible domain's plugin: the schedule it owns and the
/// systems that fill it. See the module doc for what the kernel keeps.
pub struct HeldItemSimulationPlugin;

impl Plugin for HeldItemSimulationPlugin {
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
        // ⛔ THE SET THIS DOMAIN OWNS, configured END TO END here: its phase
        // and its custody edge. The kernel adds the edge to its two sibling
        // variants; nothing else may configure this one.
        app.configure_sets(
            sim,
            ItemPickupSet::CoreHeldItems
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::PlayerSimulation)
                .after(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
        );
        // ⭐ THE DOMAIN'S OWN STEPS, as a chain of SETS. `HeldItemStep` is
        // shared_tangle vocabulary so a system that is not this domain's (the
        // kernel's shrine, gun and match spawn) can say where it runs without
        // naming a leaf function.
        app.configure_sets(
            sim,
            (
                HeldItemStep::Release,
                HeldItemStep::Pickup,
                HeldItemStep::Use,
                HeldItemStep::Throw,
                HeldItemStep::Settle,
                HeldItemStep::Physics,
                HeldItemStep::Residency,
            )
                .chain()
                .in_set(ItemPickupSet::CoreHeldItems),
        );
        app.add_systems(
            sim,
            (
                // BEFORE the pickup, and that placement is load-bearing —
                // see [`return_released_items`]. It reads a hand that has already
                // settled, so it can never mistake an item the pickup below took
                // this very tick for one nobody is holding.
                return_released_items
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Release),
                pickup_held_item_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Pickup),
                fire_held_ranged_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Use),
                throw_held_item_system
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Throw),
                // ⭐ SUPPORT IS RE-VALIDATED BEFORE THE STEP, so an item whose
                // platform left falls THIS tick rather than hanging for one —
                // the same reason the kernel's match spawn attaches before this
                // step — and an item whose platform MOVED goes with it.
                carry_or_wake_settled_items
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Settle),
                ground_item_physics
                    .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                    .in_set(HeldItemStep::Physics),
                // RESIDENCY FOLLOWS CUSTODY. Last in the chain, so it sees the
                // custody this tick actually settled on — the release derive
                // above ran first, the pickup and the throw wrote directly. And
                // deliberately UNGATED: a room transition suspends gameplay
                // between the crossing and the commit, which is precisely the
                // window in which the room sweep reads residency.
                //
                // WHERE A CARRIED OCCURRENCE CAME TO REST, strictly between
                // residency and the custody projection, and both edges are
                // load-bearing — see `record_placed_ground_items`' own doc.
                //
                // AND WHAT THE WORLD REMEMBERS ABOUT IT, immediately after
                // residency because it reads residency. Registered from here
                // and written NOWHERE near here: the system is generic lifecycle
                // vocabulary (it queries `InCustodyOf`, which knows nothing
                // about items) and this is simply the chain whose last link
                // produces its input.
                (
                    project_custody_onto_residency,
                    record_placed_ground_items,
                    ambition_platformer2d_shared_tangle::lifecycle::project_custody_onto_authored_occurrences,
                )
                    .chain()
                    .in_set(HeldItemStep::Residency),
            ),
        );

        // Portal-gun ground pickups: arm the LDtk-authored pickup here; the
        // Ambition inventory grant (`pickup_portal_gun_system`) is registered
        // by the content layer (`AmbitionPortalAdaptersPlugin`), ordered
        // `.after(arm_portal_pickups)` inside this same set so the chain edge
        // is preserved without this generic crate naming content.
        #[cfg(feature = "portal")]
        app.add_systems(
            sim,
            ambition_portal2d::arm_portal_pickups
                .in_set(ambition_portal2d::PortalPickupArming)
                .in_set(ItemPickupSet::CoreHeldItems),
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
pub const MINTED_ITEM_HALF_EXTENT: Vec2 = Vec2::splat(PICKUP_HALF);

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

/// WHERE AN ITEM IS, WHOEVER HAS IT.
///
/// ⭐⭐ ONE QUESTION WITH TWO ANSWERS, and asking it wrongly is a whole class of
/// defect. `GroundItem::pos` is the world's copy of an item's position and
/// [`ground_item_physics`] deliberately stops updating it once the item is in a
/// hand — the item has left the world, so the world stops simulating it. Every
/// reader that then keeps using `GroundItem::pos` for a HELD item is reading the
/// spot where it was picked up. Measured on the Smash bomb: pick one up at X,
/// carry it to Y, let the fuse run out, and the blast happens at X.
///
/// ⛔ THE FIX IS NOT "KEEP WRITING `GroundItem::pos`". A held item that also
/// maintains a world position has two authorities for where it is, and the
/// pickup road exists precisely to have one. The position of a held thing is a
/// DERIVED fact about its holder, so it is derived.
///
/// ⭐ AND THE HELD ANSWER IS THE HAND, not the body centre: the same
/// [`ambition_mount::rider_hand_world_pos`] the wielded-item presentation draws
/// with. A blast that goes off at the sprite's midriff while the bomb is drawn in
/// the fist is a disagreement a player can see.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ItemWorldPos<'w, 's> {
    holders: Query<'w, 's, &'static ae::BodyKinematics>,
}

impl ItemWorldPos<'_, '_> {
    /// Where this item is right now.
    ///
    /// Falls back to `GroundItem::pos` when the holder cannot be read — a holder
    /// that despawned this tick is a stale relationship, and the item's last
    /// world position is a better answer than the origin.
    pub fn of(&self, custody: &ItemCustody, item: &GroundItem) -> Vec2 {
        match custody {
            ItemCustody::InWorld => item.pos,
            ItemCustody::Held { holder } => self
                .holders
                .get(*holder)
                .map(|kin| ambition_mount::rider_hand_world_pos(kin.pos, kin.facing, kin.size.y))
                .unwrap_or(item.pos),
        }
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

/// THIS ITEM REACHED A BODY THIS TICK, AND HOW FAST IT WAS GOING.
///
/// ⭐⭐ THE OTHER HARD CONTACT. `SettledItem` says an item stopped against the
/// collision world; this says it arrived at a fighter. Jon's rule for the live
/// bomb is *"4 seconds or if it hits something with enough velocity, whichever
/// comes first"* — and a fighter is something, so a bomb thrown into somebody's
/// chest kept its fuse for as long as "impact" meant "touched a block"
///.
///
/// ⛔ IT DOES NOT STOP THE ITEM. A body is not a wall: the first attempt settled
/// the item on contact, which made every fighter a shelf and left a minted drop
/// resting 47px above the floor it used to land on.
///
/// ⛔ REPUBLISHED EVERY TICK by the system that owns item motion, so a consumer
/// reads this tick's contact and never a stale one. `SettledItem` needs no such
/// rule because the stop it records ends the item's motion.
///
/// ⛔ THE SPEED, NOT A BOOLEAN, for the same reason `SettledItem` carries one:
/// the difference between a hit and a placement is how fast it arrived, and each
/// consumer sets its own bar.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ItemStruckBody {
    pub impact_speed: f32,
}

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
pub struct SettledItem {
    /// How fast the item was travelling on the tick geometry stopped it.
    ///
    /// ⭐⭐ PUBLISHED BY THE STEP THAT DESTROYS IT, which is the only place the
    /// answer exists. `ground_item_physics` applies gravity, finds the move
    /// blocked, and zeroes `vel` — so by the time any consumer runs, the speed
    /// the item hit at is gone. A consumer that reconstructs it from its own
    /// memory of last tick is wrong twice: a thrown item collides on its FIRST
    /// free tick, when the remembered speed is still the zero it had in a hand,
    /// and a falling one can cross a speed threshold on the gravity of the very
    /// tick it lands.
    ///
    /// ⭐ `0.0` MEANS "SETTLED WITHOUT IMPACT" and that is the honest default:
    /// an authored placement is put down at rest carrying this marker and never
    /// struck anything (see the note above).
    pub impact_speed: f32,
}

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
/// ⭐⭐ AND A SUPPORTED ONE RIDES, with no support IDENTITY and no local offset.
/// `Block::velocity` is the block's own PER-FRAME DISPLACEMENT and the sweep
/// already carries *"any body resting on the block"* by it, *"uniform across
/// every body, with no per-actor wiring"* — and the probe below already finds
/// the block, so the fact is at the site.
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
    // ⭐⭐ THE OTHER CONTACT POPULATION. A thrown item's world is not only the
    // collision world: it is also full of BODIES, and Jon's rule for a live bomb
    // is *"4 seconds or if it hits something with enough velocity, whichever
    // comes first"* — a fighter is something. `SettledItem` was published only
    // for a stop against static geometry, so "impact detonation" quietly meant
    // "touched a block" and a bomb thrown into somebody's chest bounced off them
    // and kept its fuse.
    //
    // ⛔ THE FACT IS PRODUCED HERE, by the system that owns an item's motion,
    // and not by a distance check in `bomb.rs`. A bomb is one consumer of "this
    // item stopped hard"; a gravity grenade and whatever comes next are others,
    // and each writing its own overlap loop is how one rule becomes three.
    bodies: Query<
        (
            Entity,
            &ambition_platformer2d_core::CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            Has<ambition_combat::death_rules::OutOfPlay>,
        ),
        With<ambition_characters::actor::BodyHealth>,
    >,
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
        ambition_platformer2d_shared_tangle::gravity::apply_world_forces(
            &mut item.vel,
            GROUND_ITEM_GRAVITY,
            &local,
            dt,
        );
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
        // ⛔⛔ A NEWLY ENTERED OVERLAP, NOT AN OVERLAP. A thrown item leaves a
        // HAND, so on its first free tick it is standing inside the thrower and
        // an "is it touching a body" test settles it before it has travelled a
        // pixel. Comparing the two positions asks the question that actually
        // means impact: did this item ARRIVE somewhere a body is?
        //
        // ⛔⛔ AND "NEWLY ENTERED" IS PER VICTIM, NOT AGGREGATE. This asked
        // "is the item touching ANY body" at each end, which loses WHICH body:
        // an item still standing inside its thrower while it reaches an opponent
        // reads touching-then-touching and reports no strike at all — the
        // ordinary case of a bomb thrown at somebody from arm's length.
        //
        // ⛔⛔ AND THE PATH IS SWEPT, because two endpoints are not a
        // trajectory. A fast item crosses a whole body between one tick's
        // position and the next while overlapping at neither, and the same
        // formula that reports the strike also decides a bomb's detonation.
        // `aabb_path_contacts` is the repo's own answer to this and takes the
        // end centre plus the delta it arrived by.
        let entered_now = |at: ae::Aabb, entity: Entity| {
            bodies
                .get(entity)
                .is_ok_and(|(_, aabb, _, _)| at.strict_intersects(aabb.aabb()))
        };
        let struck_bodies = || {
            let here = ae::Aabb::new(item.pos, item.half_extent);
            bodies
                .iter()
                .filter(|(_, _, health, out_of_play)| {
                    !ambition_combat::util::body_is_untouchable(Some(*health), *out_of_play)
                })
                .any(|(victim, aabb, _, _)| {
                    ambition_platformer2d_core::cast::aabb_path_contacts(
                        next,
                        item.half_extent,
                        item.vel * dt,
                        aabb.aabb(),
                    ) && !entered_now(here, victim)
                })
        };
        // ⛔⛔ AND A BODY IS NOT A WALL. The first version STOPPED the item here,
        // and that turns every fighter into a shelf: an item dropped or falling
        // near somebody parked in mid-air on them — measured, a minted drop came
        // to rest 47px above the floor it used to land on. Being STRUCK by
        // something and STOPPING it are different facts, and only the first is
        // what a thrown bomb asks about.
        let struck = struck_bodies().then(|| ItemStruckBody {
            impact_speed: item.vel.length(),
        });
        match struck {
            Some(hit) => {
                commands.entity(entity).try_insert(hit);
            }
            // Republished every tick, so a consumer reads THIS tick's contact
            // rather than a stale one.
            None => {
                commands.entity(entity).remove::<ItemStruckBody>();
            }
        }
        if blocked || outside_world {
            // Settle in place (simple — no slide), and SAY SO: the marker is
            // what stops this item being stepped again, replacing the
            // `vel == ZERO` reading that could not tell rest from release.
            // BEFORE the zeroing, which is the whole point — see
            // `SettledItem::impact_speed`.
            let impact_speed = item.vel.length();
            item.vel = Vec2::ZERO;
            commands
                .entity(entity)
                .try_insert(SettledItem { impact_speed });
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
    // ⛔⛔ AND WHETHER A MOVE IS HOLDING SOMETHING ELSE FOR A MOMENT. A brandish
    // OVERWRITES `HeldItem` with a temporary move weapon (the admiral's
    // gun-sword) and remembers what it displaced — so during those frames the
    // hand does not name the object this body has CUSTODY of, and comparing
    // against it dropped the real object on the floor. The move then ended and
    // rebuilt `HeldItem` from its remembered id, leaving the body logically
    // holding an item that was also lying in the world: ONE object, two owners
    //.
    holders: Query<(
        &BodyKinematics,
        Option<&HeldItem>,
        Option<&ambition_combat::held_items::MoveBrandishedItem>,
    )>,
    mut carried: Query<(Entity, &mut GroundItem, &mut ItemCustody)>,
) {
    for (entity, mut item, mut custody) in &mut carried {
        let ItemCustody::Held { holder } = *custody else {
            continue;
        };
        // A holder that no longer exists is the death-drop resolver's question,
        // not this one's — see the above.
        let Ok((kin, held, brandished)) = holders.get(holder) else {
            continue;
        };
        // Still the object in that hand — nothing to do. Compared by SPEC ID
        // rather than by "is there a hand at all", because an equip-SWAP leaves
        // the body holding a DIFFERENT item and orphans the old one just as
        // thoroughly as a Stow does.
        // ⭐ THE QUESTION IS CUSTODY, NOT WHAT IS VISIBLY IN THE HAND. While a
        // move is brandishing, the body's custody answer is what the brandish
        // DISPLACED; the drawn weapon is an overlay with no custody of its own.
        // Outside a brandish the two are the same string, which is why this read
        // like a hand comparison for so long.
        let in_custody = match brandished {
            Some(brandish) => brandish.previous.as_deref(),
            None => held.map(|held| held.id()),
        };
        if in_custody == Some(item.spec.id.as_str()) {
            continue;
        }
        // The body let go: the object lands where that body is standing.
        item.pos = kin.pos;
        // A released object is at rest, not mid-throw.
        item.vel = Vec2::ZERO;
        *custody = ItemCustody::InWorld;
        // ⭐ AND THIS IS NOT A `Release` AT ALL — it is a stow, from the menu or
        // a brandish swap. It stamps no `ReleasedAs`, so a bomb that leaves a
        // hand this way does not arm. That used to be true only because the
        // velocity happened to be zero here.
        commands.entity(entity).remove::<ReleasedAs>();
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
        ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<
            ambition_platformer2d_world::rooms::RoomSet,
        >,
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

/// Resolve a catalog [`ambition_items::Item`]'s held-item spec, for equipping from
/// a non-pickup source (the inventory menu). The three wired weapons each have a
/// spec; everything else returns `None`.
pub fn held_spec_for_item(item: ambition_items::Item) -> Option<HeldItemSpec> {
    use ambition_items::Item;
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
/// the kernel's `items::pickup::minted_horizon`); storing the
/// resolved spec would put a second authority for *what a javelin is* inside a
/// snapshot, so the id has to be resolvable from outside.
///
/// both registries, in that order, because there are two. The three wired
/// weapons are built here by [`held_spec_for_item`] and are NOT rows in
/// `ambition_characters`'s table; the pirates' `gun_sword_heavy` is a row there
/// and has no catalog slot. Consulting one alone silently loses half the items.
pub fn held_spec_by_id(id: &str) -> Option<HeldItemSpec> {
    ambition_items::Item::from_held_item_id(id)
        .and_then(held_spec_for_item)
        .or_else(|| ambition_characters::brain::held_item_by_id(id))
}

/// TAKE custody of a held item — one operation, both ends.
///
/// Stash the current action set, overlay the item's verbs, and attach
/// [`HeldItem`]. Every way a body comes to hold a weapon calls this: the world
/// pickup ([`pickup_held_item_system`]) and the inventory menu. There is no
/// second place that writes half of it.
///
/// ⛔ NOTHING ELSE IS WRITTEN. This used to name a catalog slot on
/// `OwnedItems` as well — a process-global mirror of "some body holds X" that
/// four seats could not share (I1, 2026-09-02). The hand IS the record; a
/// reader that wants the catalog's view of it projects with [`item_in_hand`].
///
/// `grant` belongs to the sites that confer a quantity with no object — `<<give_item>>`, the
/// shop, ability drops.
pub fn equip_held_spec(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    spec: HeldItemSpec,
) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    let held = HeldItem::new(spec.clone());
    // The held item *replaces* the player's attack verbs (move-style/special
    // are kept), exactly as the world pickup does.
    action_set.melee = spec.melee;
    action_set.ranged = spec.ranged;
    commands.entity(player).insert(held);
}

/// The catalog [`Item`](ambition_items::Item) a body's hand holds, if the
/// hand holds something the catalog has a row for: a [`HeldItem`] whose id
/// maps through `Item::from_held_item_id` (the pirates' `gun_sword_heavy` maps
/// to nothing and reads as an empty hand, as it always did), or an active
/// [`PortalGun`], which equips through its own component and carries no
/// held-item id. The ONE projection every menu-side "is it equipped" reads.
pub fn item_in_hand(
    held: Option<&HeldItem>,
    #[cfg(feature = "portal")] portal_gun: Option<&PortalGun>,
) -> Option<ambition_items::Item> {
    #[cfg(feature = "portal")]
    if portal_gun.is_some_and(|gun| gun.active) {
        return Some(ambition_items::Item::PortalGun);
    }
    held.and_then(|held| ambition_items::Item::from_held_item_id(held.id()))
}

/// RELEASE custody of a held item — the twin of [`equip_held_spec`].
///
/// Restore the stashed action set and detach [`HeldItem`]. The body stops
/// holding it here and nowhere else.
///
/// Nothing here has an item query to fix that with (the menu calls this from `Update`, in another
/// crate), so custody is re-derived from the hand instead: see [`return_released_items`]. A caller
/// that simply lets go can no longer destroy an authored object.
pub fn unequip_held(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<HeldItem>();
    commands.entity(player).remove::<StashedActionSet>();
}

/// TAKE custody of the portal gun — the portal-gun twin of
/// [`equip_held_spec`]. Stash the action set, attach an active [`PortalGun`],
/// and clear the melee swing so `Attack` fires portals.
///
/// The gun equips through its own component rather than a `HeldItemSpec`, which
/// is why it needs a twin at all; [`item_in_hand`] reads the active gun as
/// `Item::PortalGun`, which deliberately carries no `held_item_id`.
#[cfg(feature = "portal")]
pub fn equip_portal_gun(commands: &mut Commands, player: Entity, action_set: &mut ActionSet) {
    commands
        .entity(player)
        .insert(StashedActionSet(action_set.clone()));
    commands.entity(player).insert(PortalGun {
        active: true,
        ..PortalGun::default()
    });
    action_set.melee = None;
}

/// RELEASE custody of the portal gun — the portal-gun twin of
/// [`unequip_held`]. Detach [`PortalGun`] and restore the stashed action set.
#[cfg(feature = "portal")]
pub fn unequip_portal_gun(
    commands: &mut Commands,
    player: Entity,
    action_set: &mut ActionSet,
    stashed: Option<&StashedActionSet>,
) {
    if let Some(stash) = stashed {
        *action_set = stash.0.clone();
    }
    commands.entity(player).remove::<PortalGun>();
    commands.entity(player).remove::<StashedActionSet>();
}

/// EVERY BODY WHOSE PRESSES ARE SOMEBODY'S — the population a press-gated item
/// action acts on.
///
/// ⭐⭐ ONE ANSWER FOR EVERY ITEM VERB, because three of them asked the same
/// question separately and all three got it wrong the same way. Pickup, throw and
/// held-weapon fire each read `ControlledSubject`, which is ONE entity — correct
/// for the adventure game, where you drive one body, and wrong for a Smash stage
/// with two people on the couch. The second seat could not pick anything up, and
/// what it looked like from the sofa is a bomb that ignores you
///.
///
/// ⛔ IT IS A UNION, NOT A REPLACEMENT. `ControlledSubject` is still the answer
/// for a possessed body in a room with no match around it, and `DrivingParticipant`
/// is the answer for a seat. Dropping either would move the defect rather than
/// fix it.
///
/// ⛔⛔ ORDERED BY STABLE IDENTITY, NEVER BY QUERY ORDER. Two bodies standing on
/// one bomb is exactly the case this population exists for, and Bevy iteration
/// order is not a fact a resimulation reproduces — bevy_ggrs destroys and
/// recreates rollback entities, so the raw id a rewind sees is not the one the
/// confirmed timeline saw (ADR 0023). Whoever gets the bomb has to be decided by
/// something both timelines agree on.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DrivenBodies<'w, 's> {
    /// ⚠ `Option`: a composition with no possession road registers no
    /// `ControlledSubject`, and "nobody is driving a possessed body here" is an
    /// ordinary answer rather than a reason to panic the app. The seat half
    /// below still answers on its own.
    controlled: Option<Res<'w, ambition_platformer2d_shared_tangle::markers::ControlledSubject>>,
    seats: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
        With<ambition_characters::control::DrivingParticipant>,
    >,
}

impl DrivenBodies<'_, '_> {
    /// The driven bodies, deduplicated, in an order a rewind reproduces.
    pub fn entities(&self) -> Vec<Entity> {
        let mut seated: Vec<(Option<String>, Entity)> = self
            .seats
            .iter()
            .map(|(entity, sim)| (sim.map(|id| id.as_str().to_string()), entity))
            .collect();
        seated.sort_by(|a, b| a.0.cmp(&b.0));
        let mut out: Vec<Entity> = Vec::with_capacity(seated.len() + 1);
        // The possessed subject first: a body somebody is DRIVING outranks a
        // seat it may also occupy, and putting it first keeps the single-subject
        // adventure road byte-identical to what it was.
        if let Some(subject) = self.controlled.as_deref().and_then(|held| held.0) {
            out.push(subject);
        }
        for (_, entity) in seated {
            if !out.contains(&entity) {
                out.push(entity);
            }
        }
        out
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
    driven: DrivenBodies,
    mut bodies: Query<(
        &mut ActorControl,
        &BodyKinematics,
        &mut ActionSet,
        Option<&HeldItem>,
    )>,
    // Holding the portal gun blocks a pickup (portal builds only).
    #[cfg(feature = "portal")] portal_guns: Query<&PortalGun>,
    mut grounds: Query<(Entity, &mut GroundItem, &mut ItemCustody)>,
) {
    for player in driven.entities() {
        let Ok((mut control, kin, mut action_set, held)) = bodies.get_mut(player) else {
            continue;
        };
        // One item at a time: already holding a physical item, or the portal gun.
        if held.is_some() {
            continue;
        }
        #[cfg(feature = "portal")]
        if portal_guns.get(player).is_ok() {
            continue;
        }
        // Gameplay authority is the body's brain-resolved `ActorControl`.
        if !control.0.melee_pressed {
            continue;
        }
        let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
        for (item, mut ground, mut custody) in &mut grounds {
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
                // with. See [`OwnedItems`](ambition_items::OwnedItems)'s own docs.
                //
                // CUSTODY: the ONE take-custody operation, shared with the inventory menu.
                equip_held_spec(&mut commands, player, &mut action_set, ground.spec.clone());
                // The Attack press is *consumed* by the pickup so the same press
                // doesn't also fire the just-equipped item this frame. Clear the
                // brain-resolved `ActorControl` (the subject-generic held-item / ability
                // systems — blink/grapple/gun — read `melee_pressed` there). Raw slot
                // input is immutable intent for this tick; action consumers arbitrate
                // on body state and commit by spending the semantic control edge.
                control.0.melee_pressed = false;
                *custody = ItemCustody::Held { holder: player };
                // ⭐ THE RELEASE IS OVER, so the fact it recorded is retracted —
                // and `arm_thrown_bombs` puts out the fuse on the same tick,
                // because it is chained ahead of the ticker. Catching a live bomb
                // is now a defined outcome rather than a race the ticker wins.
                commands.entity(item).remove::<ReleasedAs>();
                // A carried item is not in flight. This is no longer what keeps
                // the fuse honest — `ReleasedAs` is — but a held object with a
                // stale world velocity would resume mid-arc the moment it is put
                // back down.
                ground.vel = Vec2::ZERO;
                break;
            }
        }
    }
}

/// Which way a body let go of what it was holding.
///
/// One enum rather than a bool, because the two are different DECISIONS and a
/// third (a soft toss, a hand-off) would be a variant rather than a second
/// parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Release {
    /// Forward and up — an attack.
    Throw,
    /// Straight down where the body stands — the genre's Z-drop.
    Drop,
}

/// How the object now lying in the world got there, stamped by the ONE release
/// transaction and removed the moment a body takes custody again.
///
/// ⛔⛔ THE FUSES USED TO INFER THIS FROM VELOCITY, and a velocity does not know
/// who moved it. `arm_thrown_bombs` armed any `bomb` whose `vel != ZERO`, so a
/// bomb the room authored at rest armed itself the instant `ground_item_physics`
/// gave it gravity — ordinary falling read as "a player threw this". The other
/// direction failed too: catching an armed bomb zeroed the velocity but left the
/// lit `BombFuse`, and the ticker did not care whose hand it was in, so it
/// counted down and detonated in custody.
///
/// ⭐ `Release` ALREADY DECIDED THIS and had nowhere to say it. The throw/drop
/// distinction was computed, used to pick a launch vector, and thrown away; the
/// fuses then guessed it back from the consequence. This is that decision made
/// durable, which is what lets the heuristic be deleted rather than patched.
///
/// ⚠ A `Drop` DOES NOT ARM. A Z-drop is handing the item to the floor, not an
/// attack — the same answer the velocity test gave for a drop, now for the
/// stated reason rather than because a drop happens to launch at zero.
///
/// Rollback state: it decides whether an object in the world is going to
/// explode, and it lives on a `GroundItem`, which is a rollback anchor.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReleasedAs(pub Release);

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
    driven: DrivenBodies,
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
    mut owned: Option<ResMut<ambition_items::OwnedItems>>,
) {
    for player in driven.entities() {
        let Ok((mut control, kin, mut action_set, held, stashed)) = bodies.get_mut(player) else {
            continue;
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
            // ⛔⛔ `continue`, NOT `return`. This loop is over every DRIVEN body, and
            // a `return` here ends the system: seat zero holding nothing to release
            // stopped seat one from releasing anything, on every tick seat zero was
            // idle — which is most of them.
            continue;
        };
        let spec = held.spec.clone();
        let facing = if kin.facing >= 0.0 { 1.0 } else { -1.0 };
        // The launch is authored in the body's LOCAL frame (x = forward/side,
        // y = toward-feet) and rotated into the body's gravity frame, so the throw
        // arcs "ahead + away from feet" under ANY gravity — identity under normal
        // gravity. The subsequent free-fall (`ground_item_physics`) is already
        // gravity-relative, so the whole toss now flips with the field.
        let frame =
            ae::AccelerationFrame::new(gravity.dir_for(ae::Aabb::new(kin.pos, kin.size * 0.5)));
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
        unequip_held(&mut commands, player, &mut action_set, stashed);
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
            // The release SAYS what it was. Every fuse downstream reads this
            // rather than re-deriving intent from the launch vector.
            commands.entity(entity).insert(ReleasedAs(release));
            // ⭐ A DROP IS THE CASE THAT NEEDS THIS, not the throw. `Release::Drop`
            // launches at ZERO velocity, so an object that kept the settled marker
            // it wore when it was picked up would hang at head height forever.
            commands.entity(entity).remove::<SettledItem>();
            // ⛔⛔ `continue`, NOT `return` — this body is done, the loop is not.
            continue;
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
            ambition_items::Item::from_held_item_id(spec.id.as_str()),
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
            ReleasedAs(release),
            Name::new("Ground item: thrown"),
        ));
        if let Some((sim_id, origin)) = minted {
            thrown.insert((sim_id, origin));
        }
    }
}

// ---------------------------------------------------------------------------
// Held *ranged* items (the gun-sword, the fireball): `Attack` fires the item's
// authored `RangedActionSpec` on the ONE projectile road.
//
// ⛔⛔ THERE USED TO BE A SECOND PROJECTILE SIMULATION HERE. `HeldProjectile`
// stepped itself: its own raycast against solids, its own body/boss/breakable
// hit test, its own range gate, its own fireball explosion, its own rollback
// registration — a second world-collision implementation and a second place
// anti-tunnelling had to be fixed (K2, `controlled-character-actor-kernel.md`).
// A held item's shot is now an `ActorActionMessage::Ranged`, exactly what a
// brain's ranged action is, and `spawn_projectiles_from_brain_actions` →
// `step_projectiles` carry it: swept world contact, the victim ledger, shields
// and parries, feature resolution, and — new to that road, absorbed from here —
// the landing splash (`ProjectileFlight::splash_half_extent`).

/// Held-item id of the Fireball ability. Its shot is authored to burst where it
/// lands (`ProjectileFlight::with_splash`), which is the whole difference from
/// the gun-sword's bolt.
pub const FIREBALL_ID: &str = "fireball";

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

/// `Attack` while holding a *ranged* item fires it along the aim direction, as
/// the brain road would: the request names the world-space direction and the
/// item's own `RangedActionSpec`, and the projectile spawner does the rest —
/// muzzle, cue, recoil, look, flight. `Shield + Attack` is the throw/drop
/// gesture, so don't fire on it.
///
/// ⚠ ONE DELIBERATE DIFFERENCE FROM THE BRAIN ROAD: a held weapon fired from
/// the hand applies NO recoil to the body holding it. The deleted held-shot
/// path never kicked the player; the gun-sword's authored discharge kicks the
/// PIRATE 380 px/s by design. Whether the player should feel that kick is a
/// feel ruling and is recorded in `awaiting-maintainer-decision.md`, not
/// decided here.
pub fn fire_held_ranged_system(
    driven: DrivenBodies,
    bodies: Query<(
        &ActorControl,
        &BodyKinematics,
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &HeldItem,
    )>,
    mut actions: MessageWriter<ambition_characters::brain::ActorActionMessage>,
) {
    for subject in driven.entities() {
        let Ok((control, kin, resolved_frame, held)) = bodies.get(subject) else {
            continue;
        };
        let c = control.0;
        if !c.melee_pressed || c.shield_held {
            continue;
        }
        let Some(ranged) = held.spec.ranged.clone() else {
            continue;
        };
        let frame = resolved_frame.basis();
        let dir = frame
            .to_world(ability_aim_local(&c, kin.facing))
            .normalize_or_zero();
        if dir == Vec2::ZERO {
            continue;
        }
        let mut discharge = ranged.discharge.clone().unwrap_or_default();
        discharge.recoil = 0.0;
        let spec = ranged.with_discharge(discharge);
        actions.write(ambition_characters::brain::ActorActionMessage {
            actor: subject,
            request: ambition_characters::brain::action_set::ActionRequest::Ranged {
                spec,
                origin: kin.pos,
                dir,
                dir_policy: ae::GameplayFramePolicy::WorldSpace,
                // A press is an attempt: the spec's own refire gate applies,
                // as it does to every other body that fires this weapon.
                commitment: ambition_characters::brain::action_set::RangedCommitment::Attempt,
            },
        });
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod schedule_tests;
