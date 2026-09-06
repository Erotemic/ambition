//! Authored-logic domain for durable world flags.
//!
//! Conditions read flags from `AmbitionGameSaveData`; commands request flag
//! writes through `SetFlagRequested`. The provider publishes both halves without
//! adding central enums/match arms. It lives here because resolving a flag spans
//! persistence and combat-owned write vocabulary without adding a new dependency
//! edge between those crates.

use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ConditionDescriptor, ConditionId,
    ConditionOutcome, ParamKind, ParamSpec,
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
/// an absent flag is `NotSatisfied`, not `Unanswerable`, and that asymmetry
/// with the custody condition is deliberate. A flag namespace is open by
/// construction — "has this happened yet" is a meaningful question about a fact
/// nobody has recorded, and answering *unanswerable* would make every gate
/// unopenable until something set its flag once. What IS unanswerable is having
/// no save layer at all, because then the question has no subject.
pub fn flag_set(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(flag) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`flag` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no world facts exist",
        );
    };
    ConditionOutcome::from_bool(save.data().flag(flag), || {
        ambition_platformer2d_shared_tangle::authored_logic::WhyNot::new(
            "world.flag_set",
            flag,
            "the save has no such flag set",
        )
    })
}

const SWITCH: ParamSpec = ParamSpec {
    name: "switch",
    kind: ParamKind::Name,
    summary: "the latched switch id, as the save layer and the authored level spell it",
};

/// `world.switch_on(switch)` — is this world MECHANISM latched on?
///
/// The world-mechanism gate family, and the distinction from `flag_set` is what
/// the family IS rather than where the bit lives. A flag is a story fact
/// something recorded about the player; a switch is a mechanism's own state,
/// flipped by the world doing what the world does. They share
/// `AmbitionGameSaveData` because both are durable, and they are two questions
/// because a route gated on "the arena has been cleared" and a route gated on
/// "you have been told about the survey" are different design.
///
/// ⭐ THE INPUT IS ALREADY FLOWING, which is why this is publishable rather than
/// dormant: completing a wave encounter latches every switch linked to it
/// (`ambition_encounter_features/src/systems.rs:496`, all of them and not the
/// first), and the ids are authored alongside the encounter.
pub fn switch_on_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "switch_on"),
        summary: "true while the named world mechanism is latched on in the save",
        params: &[SWITCH],
    }
}

/// `world.switch_on` — see [`switch_on_descriptor`].
///
/// ⚠ AN UNRECORDED SWITCH IS `NotSatisfied`, and here that is not the same
/// concession `flag_set` makes. A latched switch that nothing has flipped IS
/// off — `set_switch(id, false)` and "no row" describe the same mechanism state,
/// so `false` is the true answer rather than the tolerable one. What both share
/// is that a MISSPELT id is indistinguishable from an unflipped one; that is a
/// content-validation question about authored ids, not a runtime one, and it is
/// answered the same way for both.
pub fn switch_on(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(switch) = args[0].as_name() else {
        return ConditionOutcome::unanswerable("`switch` must be a name");
    };
    let Some(save) = world.get_resource::<ambition_persistence::save::AmbitionGameSave>() else {
        return ConditionOutcome::unanswerable(
            "no save layer is installed in this composition, so no mechanism state exists",
        );
    };
    ConditionOutcome::from_bool(save.data().switch(switch), || {
        ambition_platformer2d_shared_tangle::authored_logic::WhyNot::new(
            "world.switch_on",
            switch,
            "the mechanism is not latched on",
        )
    })
}

const ON: ParamSpec = ParamSpec {
    name: "on",
    kind: ParamKind::Truth,
    summary: "true to record the fact, false to unrecord it",
};

/// `world.set_flag(flag, on)` — record (or unrecord) a world fact.
///
/// the mirror of `world.flag_set`, and the asymmetry between the two names
/// is what named this as the command half's first customer. The question was
/// published and had consumers; the verb did not exist, so every authored way to
/// flip a flag was its own hand-written binding.
///
/// TWO parameters where the condition has one, and the second is the reason
/// this is one command rather than two. The Yarn vocabulary it replaces had
/// `set_flag` and `clear_flag` as separate hand-written systems differing by a
/// bool; a verb whose object is *"which fact, to what"* says that once.
pub fn set_flag_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(DOMAIN, "set_flag"),
        summary: "record or unrecord the named world-fact flag in the save",
        params: &[FLAG, ON],
    }
}

/// `world.set_flag` — see [`set_flag_descriptor`].
///
/// it writes the domain's OWN typed request rather than the save, and
/// that is the rollback argument in one line. `SetFlagRequested` already exists,
/// is already cleared on rollback, and is already applied by `apply_flag_effects`
/// in `GameplayEffects` — which the authored-command set is ordered before. So
/// this command introduces no new kind of write, mutates nothing the snapshot
/// does not already cover, and keeps the quest-advance mirror that the effect
/// bus does on every flag write.
///
/// reaching for `AmbitionGameSave` here would have been shorter and wrong.
/// It would skip the quest mirror, and it would put an authored verb's effect on
/// a different road than the same effect takes from a chest, a pickup or an
/// interaction — which is the second-authority shape this contract exists to
/// remove.
pub fn set_flag(world: &mut World, args: &[AuthoredArg]) -> CommandOutcome {
    let Some(flag) = args[0].as_name() else {
        return CommandOutcome::refused("`flag` must be a name");
    };
    let AuthoredArg::Truth(on) = args[1] else {
        return CommandOutcome::refused("`on` must be a truth");
    };
    if !world.contains_resource::<
        bevy::ecs::message::Messages<ambition_combat::events::SetFlagRequested>,
    >() {
        return CommandOutcome::refused(
            "no world-fact channel is installed in this composition, so nothing \
             would ever apply the fact",
        );
    }
    world.write_message(ambition_combat::events::SetFlagRequested {
        id: flag.to_string(),
        on,
    });
    CommandOutcome::Done
}

/// Publishes the world-fact domain's conditions and commands.
///
/// a whole plugin for two registration lines, and that is the point rather
/// than an overhead. The domain owning its own installation is what makes
/// "adding a provider edits nothing central" true: composition adds this plugin,
/// and nothing else in the engine learns that world facts exist.
///
/// one plugin for both halves rather than a `…ConditionsPlugin` and a
/// `…CommandsPlugin`. A domain is one thing; splitting its installation by
/// which contract each row uses would make composition name an internal detail
/// of this file.
pub struct WorldFactConditionsPlugin;

impl bevy::prelude::Plugin for WorldFactConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::{
            PublishCommand, PublishCondition,
        };
        app.publish_condition(flag_set_descriptor(), flag_set);
        app.publish_condition(switch_on_descriptor(), switch_on);
        app.publish_command(set_flag_descriptor(), set_flag);
    }
}
