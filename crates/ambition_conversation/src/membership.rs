//! Authored verbs that change who is in the live conversation:
//! `conversation.join` and `conversation.leave`.
//!
//! Asked from a `.yarn` line (`<<command "conversation.join" "npc_alice">>`),
//! a switch or a room's line. A Yarn request reaches the simulation through
//! the narrative-input ledger, stamped with the tick it applies on, so a
//! resimulation joins the same body on the same tick.

use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, CharacterForm};
use ambition_combat::components::ActorInteraction;
use ambition_platformer2d_core::CenteredAabb;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ParamKind, ParamSpec,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_shared_tangle::sim_id::SimId;

use super::authority::{ActiveConversation, Leaving};
use super::rules::EARSHOT;

const DOMAIN: &str = "conversation";

const CHARACTER: ParamSpec = ParamSpec {
    name: "character",
    kind: ParamKind::Name,
    summary: "the catalog character id the body wears or was placed as (`npc_alice`)",
};

pub fn join_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(DOMAIN, "join"),
        summary: "bring the nearest body of this character, within earshot, into the live conversation",
        params: &[CHARACTER],
    }
}

pub fn leave_descriptor() -> CommandDescriptor {
    CommandDescriptor {
        id: CommandId::new(DOMAIN, "leave"),
        summary: "take the body of this character out of the live conversation",
        params: &[CHARACTER],
    }
}

/// The character a body is, for an authored verb that names one: its
/// placement's character first (an NPC), else the character it wears.
fn character_of<'a>(form: Option<&'a CharacterForm>, interaction: Option<&'a ActorInteraction>) -> Option<&'a str> {
    interaction
        .and_then(|interaction| super::opening::character_id_of(&interaction.interactable))
        .or_else(|| form.map(CharacterForm::id))
}

type Bodies<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static CenteredAabb,
        Option<&'static CharacterForm>,
        Option<&'static ActorInteraction>,
        Option<&'static BodyHealth>,
        Option<&'static SimId>,
    ),
>;

/// `conversation.join` — see [`join_descriptor`].
///
/// The body must be within earshot of a member, in the same live room, and
/// alive: a body farther off would leave again on the next tick by the
/// continuity rule, so it is refused here, where an author reads why.
pub fn join(world: &mut World, args: &[AuthoredArg], _room: Option<LiveRoomInstance>) -> CommandOutcome {
    let Some(character) = args[0].as_name() else {
        return CommandOutcome::refused("`character` must be a name");
    };
    let mut state: SystemState<(Res<ActiveConversation>, LiveRooms, Bodies)> = SystemState::new(world);
    let found = {
        let Ok((conversation, rooms, bodies)) = state.get(world) else {
            return CommandOutcome::refused("the conversation's world cannot be read");
        };
        if !conversation.is_live() {
            return CommandOutcome::refused("no conversation is live to join");
        }
        let members: Vec<_> = conversation
            .participants()
            .filter_map(|member| bodies.get(member).ok().map(|(_, aabb, ..)| (member, aabb.center, rooms.of(member))))
            .collect();
        let mut near: Vec<(f32, Option<&SimId>, Entity)> = bodies
            .iter()
            .filter(|(body, _, form, interaction, health, _)| {
                character_of(*form, *interaction) == Some(character)
                    && health.is_none_or(|health| health.alive())
                    && !members.iter().any(|(member, ..)| member == body)
            })
            .filter_map(|(body, aabb, .., id)| {
                let room = rooms.of(body);
                members
                    .iter()
                    .filter(|(_, _, member_room)| *member_room == room)
                    .map(|(_, at, _)| (*at - aabb.center).abs().max_element())
                    .min_by(f32::total_cmp)
                    .filter(|distance| *distance < EARSHOT)
                    .map(|distance| (distance, id, body))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| (a.1, a.2).cmp(&(b.1, b.2))));
        near.first().map(|(_, _, body)| *body)
    };
    let Some(body) = found else {
        return CommandOutcome::refused(format!(
            "no living `{character}` is within earshot ({EARSHOT} px) of the conversation, in its room"
        ));
    };
    world.resource_mut::<ActiveConversation>().join(body);
    CommandOutcome::Done
}

/// `conversation.leave` — see [`leave_descriptor`].
pub fn leave(world: &mut World, args: &[AuthoredArg], _room: Option<LiveRoomInstance>) -> CommandOutcome {
    let Some(character) = args[0].as_name() else {
        return CommandOutcome::refused("`character` must be a name");
    };
    let mut state: SystemState<(Res<ActiveConversation>, Bodies)> = SystemState::new(world);
    let found = {
        let Ok((conversation, bodies)) = state.get(world) else {
            return CommandOutcome::refused("the conversation's world cannot be read");
        };
        let found = conversation.participants().find(|member| {
            bodies
                .get(*member)
                .is_ok_and(|(_, _, form, interaction, ..)| character_of(form, interaction) == Some(character))
        });
        found
    };
    let Some(body) = found else {
        return CommandOutcome::refused(format!("no `{character}` is in a live conversation"));
    };
    match world.resource_mut::<ActiveConversation>().leave(body) {
        Leaving::NotAMember => CommandOutcome::refused(format!("`{character}` is not in a live conversation")),
        Leaving::Left | Leaving::Closed => CommandOutcome::Done,
    }
}
