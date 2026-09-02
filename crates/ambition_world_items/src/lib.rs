//! The physical life of a collectible in the world.
//!
//! A [`WorldItem`] is a thing you walk into and thereby gain something from —
//! a mushroom, a heart, a ring, a spark-blossom. This crate owns three facts
//! about it and nothing else: that it is SOMEWHERE ([`WorldItem::pos`] and its
//! half-extent), that it may be MOVING ([`ItemMotion`], stepped against the
//! world per axis), and that TOUCHING it collects it
//! ([`collect_world_items`]).
//!
//! ⛔ **WHAT IT DELIBERATELY DOES NOT OWN.** What a collected item MEANS is an
//! `EquipmentRow` recorded on [`WornEquipment`](ambition_characters::equipment::WornEquipment);
//! the verbs that row grants are derived elsewhere by
//! `reconcile_equipment_grants`, which is the one place a body's granted actions
//! come from and stays in the actor kernel. How the item is DRAWN is an art id —
//! an `Option<String>` this crate never resolves — that a game maps through its
//! own `WorldItemArt`. So a collectible's presence, motion and collection are
//! here; its meaning and its picture are not.
//!
//! ⭐ **WHY IT IS ITS OWN CRATE (D33, 2026-09-02).** These modules were
//! `actor_monolith::items::{world_item, item_motion}`, and the reason they could
//! not leave was one type: the collect pass named
//! `features::ecs::pickups::TouchCollectorFilter`, which is composed of nothing
//! but `PlayerEntity` and `TemporaryControl` — both already in `shared_tangle`.
//! Publishing that filter and its value twin `body_collects_on_touch` downward
//! is what freed the rest, the same inversion `ActorDecisionSet` and
//! `AudioInitSet` made before it.
//!
//! ⛔ **AND THE SIBLING STAYED BEHIND, ON PURPOSE.** A `GroundItem` — a held
//! weapon grabbed with a deliberate `Attack` press — lives in the kernel's
//! `items::pickup`, which reaches `abilities`, `ability_cooldown`,
//! `construction` and `shrine`. That file holds 27 of the `items/` module's 51
//! references into the rest of the kernel and is a different, much larger
//! carve. The split here is along the collect TRIGGER (touched vs pressed),
//! which is the line the pickup module's own `AMBITION_REVIEW(discrete_ok)`
//! note had already drawn.

pub mod item_motion;
pub mod world_item;

pub use item_motion::{
    step_item_motion, ItemEmerge, ItemMotion, ItemMotionPlan, DEFAULT_ITEM_GRAVITY,
};
pub use world_item::{
    collect_world_items, spawn_moving_world_item, spawn_world_item, WorldItem, WorldItemPayload,
};

/// Steps moving world items, then collects the ones a body is touching.
///
/// ⛔ **THE ORDER IS LOAD-BEARING AND IS WHY THIS IS ONE PLUGIN RATHER THAN TWO
/// REGISTRATIONS.** A pickup is collected where it IS this tick: step first, so
/// a fast item cannot still be collectable from a box it has already left. The
/// two systems were adjacent in `ItemPickupSimulationPlugin` for exactly this
/// reason, with the rule written between them; moving them out separately would
/// have left that ordering to be re-derived by whoever noticed it was gone.
///
/// ⚠ **BOTH ARE `GameplayGated`**, unchanged from their old home: an item must
/// not drift or be collected while gameplay is suspended.
///
/// ⭐ The host composes this beside `ItemPickupSimulationPlugin`, which is how
/// that one is already added — so no registration for this domain lands in the
/// actor kernel.
pub struct WorldItemSimulationPlugin;

impl bevy::prelude::Plugin for WorldItemSimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::schedule::{GameplayGated, SimScheduleExt as _};
        use bevy::prelude::IntoScheduleConfigs;

        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            (
                item_motion::step_item_motion,
                world_item::collect_world_items,
            )
                .chain()
                .in_set(GameplayGated),
        );
    }
}
