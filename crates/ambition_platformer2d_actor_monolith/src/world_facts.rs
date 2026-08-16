//! **What the world-fact domain lets authored content ask about flags.**
//!
//! A world fact is the flat, durable "this happened" vocabulary the save layer
//! carries: a door has been unlocked, an intro beat is past, a boss is defeated.
//! It is written by `SetFlagRequested` and read by
//! [`AmbitionGameSaveData::flag`](ambition_persistence::save_data::AmbitionGameSaveData::flag).
//!
//! ⭐ **this is the SECOND provider, and its only architectural job is to be a
//! different domain from the first.** The milestone's acceptance is that adding
//! it edits nothing central — no enum, no match arm, no registration table
//! belonging to anyone else. It publishes through the same three-line surface a
//! crate outside this workspace would use.
//!
//! ⚠ **it lives in this crate rather than beside `SetFlagRequested`** because
//! `ambition_combat`, which owns that message, does not depend on
//! `ambition_persistence`, which owns the answer. Reading a flag needs both. ⛔
//! adding that dependency edge to make the placement prettier would be paying a
//! compile-graph cost for a doc-comment's sake — and this crate is already where
//! every other flag consumer lives.

use ambition_platformer2d_shared_tangle::authored_logic::{
    ConditionArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec,
};
use bevy::prelude::World;

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "world";

const FLAG: ParamSpec = ParamSpec {
    name: "flag",
    kind: ParamKind::Name,
    summary: "the world-fact flag id, as the save layer spells it",
};

/// `world.flag_set(flag)` — has this world fact been recorded?
pub fn flag_set_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "flag_set"),
        summary: "true once the named world-fact flag has been set in the save",
        params: &[FLAG],
    }
}

/// `world.flag_set` — see [`flag_set_descriptor`].
///
/// ⚠ **an absent flag is `NotSatisfied`, not `Unanswerable`, and that asymmetry
/// with the custody condition is deliberate.** A flag namespace is open by
/// construction — "has this happened yet" is a meaningful question about a fact
/// nobody has recorded, and answering *unanswerable* would make every gate
/// unopenable until something set its flag once. What IS unanswerable is having
/// no save layer at all, because then the question has no subject.
pub fn flag_set(world: &World, args: &[ConditionArg]) -> ConditionOutcome {
    let Some(flag) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`flag` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no world facts exist",
        );
    };
    ConditionOutcome::from_bool(save.data().flag(flag))
}

/// Publishes the world-fact domain's conditions.
///
/// ⭐ **a whole plugin for one registration line, and that is the point rather
/// than an overhead.** The domain owning its own installation is what makes
/// "adding a provider edits nothing central" true: composition adds this plugin,
/// and nothing else in the engine learns that world facts exist.
pub struct WorldFactConditionsPlugin;

impl bevy::prelude::Plugin for WorldFactConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(flag_set_descriptor(), flag_set);
    }
}
