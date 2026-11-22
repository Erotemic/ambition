//! Directing actors from authored content (story sequences SS1,
//! `docs/planning/engine/story-sequences.md`).
//!
//! The commands a cutscene, a Yarn `<<command>>` or a room line uses to stage
//! a scene:
//!
//! - `actor.walk_to <character> <place> <brain>`: each body of the character
//!   in the asking room goes to the place and stays there. The place is any
//!   name [`place_named`](super::life::place_named) knows in the body's room
//!   (a sign, a placement, a prop, a zone). `<brain>` is the brain preset the
//!   body walks with (`keeps_to_its_tasks` walks; a roam preset with
//!   `playful: 1.0` runs); it is switched through the one brain authority
//!   ([`BrainCommand`](super::brain_command::BrainCommand)), so a rewind and
//!   a later `actor.release` see the same selection.
//! - `actor.form_up <character> <place> <spacing> <brain>`: walk to separate
//!   points from the named place. Simulation identity sets the body order.
//!   Positive spacing puts each next body to the right; negative goes left.
//! - `actor.leave_through <character> <zone> <brain>`: the same walk to the
//!   loading zone `<zone>`, and the body goes through it when it gets there
//!   ([`leave_through_zones`]).
//! - `actor.release <character>`: the body's own brain again, no visit and
//!   no pose.
//! - `actor.face <character> <left|right|other character>`: the body turns
//!   that way, or toward the nearest live body of the other character in its
//!   room. Once: its own movement can turn it again (a body a cutscene holds
//!   does not move).
//! - `actor.pose <character> <row> <loop|hold>`: the body is drawn with the
//!   named row of its sheet from now on, looping it or holding its last frame
//!   ([`Posed`], pinned by [`pin_poses`]), until `actor.release`. A sheet
//!   without the row draws the body as if nothing were pinned.
//! - `actor.hold <character> <held item>`: the body's empty hand takes a held
//!   item of the registry (the raid's last raider and the Creator's case). It
//!   is the body's own, not a world object: it goes where the body goes, and
//!   out of the world with it. A hand that holds something else is refused.
//! - `actor.shove <character> <radius>`: each body of the character throws
//!   back every other body within `radius` pixels of it in its room, away from
//!   it and off its feet (a launch, as a blow's is; it deals no damage).
//! - `actor.fight <character> <other character>`: each body of the character
//!   takes the nearest live body of the other character in its room as its
//!   foe, as a provocation does (hostile, a grudge, its character's provoked
//!   mind), without a provocation's record: it is the scene's, not the
//!   player's. A body whose character states no provoked mind stays as it is.
//! - `actor.throw <character> <other character>`: each body of the character
//!   throws what its hand holds at the nearest live body of the other
//!   character in its room: the hand empties and the item flies, thrown, on
//!   an arc that lands at that body (a bomb arms and goes off where it lands).
//!
//! And the questions a sequence waits on:
//!
//! - `actor.arrived <character>`: every live body of the character stands
//!   where it was sent (and at least one lives).
//! - `actor.gone <character>`: no live body of the character.
//! - `actor.beaten <character>`: a live body of the character has been
//!   beaten down to the health its character stops at (`beaten_at`).
//! - `actor.count_is <character> <n>`: exactly `n` live bodies of the
//!   character.
//! - `actor.fallen_is <character> <n>`: exactly `n` fallen bodies of the
//!   character, in every live room.
//! - `actor.fallen_in <character> <n> <room>`: exactly `n` fallen bodies of
//!   the character in the live room `room` (a room's id). A scene that starts
//!   on a condition is asked from no room, so a count meant for one room
//!   names it: the workshop raid's runaway counts the workshop's raiders,
//!   not the siding's (`actor.fallen_in npc_lab_raider 2 intro_wake_room`;
//!   its raiders come in one at a time, so "one live" is also true as the
//!   first walks in).
//!
//! A body that has fallen (health out) is not live here: a fight's bodies
//! stay where they fell, and none of them is directed or counted.
//!
//! The walk is the navigation the lives use ([`Visit`]): the advisor finds the
//! surface under the place, and the body walks there with its own movement.
//! A place no route reaches refuses the visit, and the body stays.

use ambition_characters::actor::CharacterForm;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, CommandDescriptor, CommandId, CommandOutcome, ConditionDescriptor, ConditionId, ConditionOutcome,
    ParamKind, ParamSpec, PublishCommand, PublishCondition, WhyNot,
};
use ambition_platformer2d_shared_tangle::lifecycle::{LiveBodyId, LiveRoomInstance};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::ecs::system::SystemState;
use bevy::prelude::{App, Commands, Component, Entity, Query, World};

use super::brain_command::BrainCommand;
use super::visit::{Visit, VisitOutcome};

/// A body sent out through a loading zone: when it reaches the zone it goes
/// through. Rollback state, beside its [`Visit`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Leaving {
    pub zone: String,
}

impl Leaving {
    /// One number for the rollback probe.
    pub fn probe(&self) -> u64 {
        let named = self
            .zone
            .bytes()
            .fold(0xCBF2_9CE4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3));
        ae::navigation::mix(named)
    }
}

const CHARACTER: ParamSpec = ParamSpec {
    name: "character",
    kind: ParamKind::Individual,
    summary: "a character id (every body of it in the asking room), or `@role`: the one body the invocation bound",
};
/// A character id only: the counts ask about a population.
const CHARACTER_KIND: ParamSpec = ParamSpec {
    name: "character",
    kind: ParamKind::Name,
    summary: "the character id of the bodies counted",
};
const PLACE: ParamSpec = ParamSpec {
    name: "place",
    kind: ParamKind::Name,
    summary: "a name in the body's room: a sign, a placement, a prop or a zone",
};
const ZONE: ParamSpec = ParamSpec {
    name: "zone",
    kind: ParamKind::Name,
    summary: "a loading zone of the body's room (an edge or a door)",
};
const COUNT: ParamSpec = ParamSpec {
    name: "count",
    kind: ParamKind::Number,
    summary: "how many live bodies of the character",
};
const ROOM: ParamSpec = ParamSpec {
    name: "room",
    kind: ParamKind::Name,
    summary: "a room's id (`intro_wake_room`): only bodies in that live room count",
};
const BRAIN: ParamSpec = ParamSpec {
    name: "brain",
    kind: ParamKind::Name,
    summary: "the brain preset the body walks with (`keeps_to_its_tasks` walks)",
};
const SPACING: ParamSpec = ParamSpec {
    name: "spacing",
    kind: ParamKind::Number,
    summary: "horizontal pixels between bodies, in simulation identity order; negative goes left",
};

const WAY: ParamSpec = ParamSpec {
    name: "way",
    kind: ParamKind::Individual,
    summary: "`left`, `right`, a character id to face toward, or `@role`: one bound body",
};
const ROW: ParamSpec = ParamSpec {
    name: "row",
    kind: ParamKind::Name,
    summary: "a named row of the body's sheet",
};
const HELD_ITEM: ParamSpec = ParamSpec {
    name: "held_item",
    kind: ParamKind::Name,
    summary: "a held item id of the registry (`creator_case`, `coconut`, ...)",
};
const RADIUS: ParamSpec = ParamSpec {
    name: "radius",
    kind: ParamKind::Number,
    summary: "pixels from the body: every other body this near is thrown back",
};
const TARGET: ParamSpec = ParamSpec {
    name: "target",
    kind: ParamKind::Individual,
    summary: "a character id (the nearest live body of it in the room is aimed at), or `@role`: one bound body",
};
const MODE: ParamSpec = ParamSpec {
    name: "mode",
    kind: ParamKind::Name,
    summary: "`loop` the row, or `hold` its last frame",
};

/// The name [`pin_poses`] writes its pins under.
pub const POSE_ROW_WRITER: &str = "actor.pose";

/// A body an `actor.pose` put in a pose: the row, the tick the pose began
/// (its clock), and whether the row loops. Rollback state; the drawn
/// [`PinnedRow`](ambition_sprite_sheet::character::PinnedRow) is derived
/// from it each tick.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct Posed {
    pub row: String,
    pub since: u64,
    pub looping: bool,
}

impl Posed {
    /// One number for the rollback probe.
    pub fn probe(&self) -> u64 {
        let named = self
            .row
            .bytes()
            .fold(0xCBF2_9CE4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3));
        ae::navigation::mix(named ^ self.since.rotate_left(17) ^ u64::from(self.looping))
    }
}

/// Publish the `actor` commands and conditions.
pub(crate) fn publish_actor_vocabulary(app: &mut App) {
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "form_up"),
            summary: "walk bodies to separate points from a named place, in simulation identity order",
            params: &[CHARACTER, PLACE, SPACING, BRAIN],
        },
        form_up,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "walk_to"),
            summary: "send each body of a character in the asking room to a named place, to stay there",
            params: &[CHARACTER, PLACE, BRAIN],
        },
        walk_to,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "leave_through"),
            summary: "send each body of a character in the asking room out through a loading zone",
            params: &[CHARACTER, ZONE, BRAIN],
        },
        leave_through,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "release"),
            summary: "give each body of a character in the asking room its own brain again, and no errand of place",
            params: &[CHARACTER],
        },
        release,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "face"),
            summary: "turn each body of a character in the asking room left, right, or toward another character",
            params: &[CHARACTER, WAY],
        },
        face,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "pose"),
            summary: "draw each body of a character in the asking room with a named row of its sheet, until released",
            params: &[CHARACTER, ROW, MODE],
        },
        pose,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "hold"),
            summary: "put a held item in the empty hand of each body of a character in the asking room",
            params: &[CHARACTER, HELD_ITEM],
        },
        hold,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "shove"),
            summary: "throw back every other body near each body of a character in the asking room",
            params: &[CHARACTER, RADIUS],
        },
        shove,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "fight"),
            summary: "set each body of a character on the nearest body of another character, as a provocation does",
            params: &[CHARACTER, TARGET],
        },
        fight,
    );
    app.publish_command(
        CommandDescriptor {
            id: CommandId::new("actor", "throw"),
            summary: "throw what each body of a character holds at the nearest body of another character",
            params: &[CHARACTER, TARGET],
        },
        throw,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "beaten"),
            summary: "a live body of the character is down to the health its character stops at",
            params: &[CHARACTER],
        },
        beaten,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "arrived"),
            summary: "every live body of the character stands where it was sent, and one lives",
            params: &[CHARACTER],
        },
        arrived,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "count_is"),
            summary: "exactly this many bodies of the character are live",
            params: &[CHARACTER_KIND, COUNT],
        },
        count_is,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "fallen_is"),
            summary: "exactly this many bodies of the character have fallen and lie where they fell",
            params: &[CHARACTER_KIND, COUNT],
        },
        fallen_is,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "fallen_in"),
            summary: "exactly this many bodies of the character have fallen in the named room",
            params: &[CHARACTER_KIND, COUNT, ROOM],
        },
        fallen_in,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "gone"),
            summary: "no body of the character is live",
            params: &[CHARACTER],
        },
        gone,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "throw_landed"),
            summary: "the character holds nothing to throw, and nothing it threw is still in the air or lit",
            params: &[CHARACTER],
        },
        throw_landed,
    );
}

/// The bodies of `character` in `room` (every live room when `None`), with
/// their identity and live room, in identity order.
fn cast_in(world: &mut World, character: &str, room: Option<LiveRoomInstance>) -> Vec<(Entity, SimId, Option<LiveRoomInstance>)> {
    let mut state: SystemState<(
        ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        Query<(Entity, &CharacterForm, &SimId, Option<&ambition_characters::actor::BodyHealth>)>,
    )> = SystemState::new(world);
    let Ok((rooms, bodies)) = state.get(world) else {
        return Vec::new();
    };
    let mut cast: Vec<(Entity, SimId, Option<LiveRoomInstance>)> = bodies
        .iter()
        .filter(|(_, worn, _, health)| worn.0.as_str() == character && alive(*health))
        .map(|(body, _, id, _)| (body, id.clone(), rooms.of(body)))
        .filter(|(_, _, at)| room.is_none() || *at == room)
        .collect();
    cast.sort_by(|a, b| a.1.cmp(&b.1));
    cast
}

/// Who an actor argument names: every live body of a character, or the one
/// body an invocation bound to a role.
#[derive(Clone, Copy)]
enum Who<'a> {
    Character(&'a str),
    Body(&'a SimId),
}

impl<'a> Who<'a> {
    fn of(arg: &'a AuthoredArg) -> Option<Self> {
        match arg {
            AuthoredArg::Name(character) => Some(Self::Character(character)),
            AuthoredArg::Reference(id) => Some(Self::Body(id)),
            _ => None,
        }
    }
}

impl std::fmt::Display for Who<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Character(character) => write!(f, "`{character}`"),
            Self::Body(id) => write!(f, "`{id}`"),
        }
    }
}

/// The bodies carrying `id`, fallen or not, with their room and whether
/// they live.
fn carrying(world: &World, id: &SimId) -> Vec<(Entity, Option<LiveRoomInstance>, bool)> {
    use ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance;
    let Some(mut bodies) = world.try_query_filtered::<(
        Entity,
        &SimId,
        Option<&ambition_characters::actor::BodyHealth>,
        Option<&InRoomInstance>,
    ), bevy::prelude::With<CharacterForm>>() else {
        return Vec::new();
    };
    bodies
        .iter(world)
        .filter(|(_, carried, ..)| *carried == id)
        .map(|(body, _, health, at)| (body, at.map(|at| at.0), alive(health)))
        .collect()
}

/// The live bodies `who` names. A character's are those in `room` (every
/// live room when `None`). A bound body is found wherever it lives, since a
/// directed actor can leave into another live room; `room` only tells
/// apart two bodies carrying its identity (one room live twice). A bound
/// body that has fallen or retired is refused, never replaced by another
/// body of its character.
fn cast_of(world: &mut World, who: Who, room: Option<LiveRoomInstance>) -> Result<Vec<(Entity, SimId, Option<LiveRoomInstance>)>, String> {
    match who {
        Who::Character(character) => {
            let cast = cast_in(world, character, room);
            if cast.is_empty() {
                return Err(format!("no body of `{character}` is live in the asking room"));
            }
            Ok(cast)
        }
        Who::Body(id) => {
            let mut found = carrying(world, id);
            if found.len() > 1 && room.is_some() {
                found.retain(|(_, at, _)| *at == room);
            }
            match found.as_slice() {
                [] => Err(format!("no body is `{id}`: it retired, and a role is not handed to another body")),
                [(_, _, false)] => Err(format!("`{id}` has fallen")),
                [(body, at, true)] => Ok(vec![(*body, id.clone(), *at)]),
                many => Err(format!("{} bodies are `{id}` and the asking room does not tell them apart", many.len())),
            }
        }
    }
}

/// The one body a condition's bound argument names, by the same rules as
/// [`cast_of`] (no room to tell two apart).
fn bound_body(world: &World, id: &SimId) -> Result<(Entity, bool), ConditionOutcome> {
    match carrying(world, id).as_slice() {
        [] => Err(ConditionOutcome::Unanswerable(format!("no body is `{id}`: it retired"))),
        [(body, _, alive)] => Ok((*body, *alive)),
        many => Err(ConditionOutcome::Unanswerable(format!("{} bodies are `{id}`", many.len()))),
    }
}

/// A body that has not fallen. A body with no health is never felled.
fn alive(health: Option<&ambition_characters::actor::BodyHealth>) -> bool {
    health.is_none_or(|health| health.health.current > 0)
}

/// How many live bodies wear `character`.
fn live_count(world: &World, character: &str) -> Option<usize> {
    let mut bodies = world.try_query::<(&CharacterForm, Option<&ambition_characters::actor::BodyHealth>)>()?;
    Some(bodies.iter(world).filter(|(worn, health)| worn.0.as_str() == character && alive(*health)).count())
}

/// How many fallen bodies wear `character`.
fn fallen_count(world: &World, character: &str) -> Option<usize> {
    let mut bodies = world.try_query::<(&CharacterForm, Option<&ambition_characters::actor::BodyHealth>)>()?;
    Some(bodies.iter(world).filter(|(worn, health)| worn.0.as_str() == character && !alive(*health)).count())
}

/// How many fallen bodies wear `character` in the live room whose id is
/// `room`.
fn fallen_count_in(world: &World, character: &str, room: &str) -> Option<usize> {
    use ambition_platformer2d_shared_tangle::lifecycle::{session_world_component, InRoomInstance};
    let rooms = session_world_component::<ambition_platformer2d_world::rooms::RoomSet>(world)?;
    let mut bodies =
        world.try_query::<(&CharacterForm, Option<&ambition_characters::actor::BodyHealth>, Option<&InRoomInstance>)>()?;
    Some(
        bodies
            .iter(world)
            .filter(|(worn, health, _)| worn.0.as_str() == character && !alive(*health))
            .filter(|(_, _, in_room)| {
                ambition_platformer2d_world::rooms::live_room_definition_in(world, in_room.map(|in_room| in_room.0))
                    .is_some_and(|definition| rooms.rooms[definition.index()].id == room)
            })
            .count(),
    )
}

/// Where `name` is in the room of `body`.
fn place_of(world: &mut World, body: Entity, name: &str, zone_only: bool) -> Option<ae::Vec2> {
    let mut state: SystemState<(
        ambition_platformer2d_world::rooms::LiveRoomSpecs,
        Query<&ae::BodyKinematics>,
    )> = SystemState::new(world);
    let Ok((specs, kinematics)) = state.get(world) else {
        return None;
    };
    let room = specs.spec_of(body)?;
    let here = kinematics.get(body).map(|k| k.pos).unwrap_or_default();
    if zone_only {
        // The point of the zone nearest the body: an edge zone's middle is in
        // the room's last strip, where no body stands, and a body going
        // through needs only to touch the zone.
        let zone = room.loading_zones.iter().find(|zone| zone.id == name)?;
        return Some(here.clamp(zone.aabb.min, zone.aabb.max));
    }
    super::life::place_named(room, name, here)
}

fn send(
    world: &mut World,
    args: &[AuthoredArg],
    room: Option<LiveRoomInstance>,
    verb: &str,
    leaving: bool,
    spacing: f32,
) -> CommandOutcome {
    let (Some(character), Some(place), Some(brain)) = (Who::of(&args[0]), args[1].as_name(), args[2].as_name()) else {
        return CommandOutcome::refused(format!("`actor.{verb}` takes a character, a place and a brain preset"));
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    if !(spacing * (cast.len() - 1) as f32).is_finite() {
        return CommandOutcome::refused("the formation span must be finite");
    }
    let mut sent = 0;
    for (index, (body, sim_id, at)) in cast.into_iter().enumerate() {
        let Some(mut point) = place_of(world, body, place, leaving) else {
            continue;
        };
        point.x += spacing * index as f32;
        let mut entity = world.entity_mut(body);
        // A body an encounter spawned carries no brain binding, and the one
        // brain authority switches only a bound body. Bind it to its own
        // character's policy (what `actor.release` restores), homed where it
        // stands.
        if !entity.contains::<ambition_characters::control::brain_binding::BrainBinding>() {
            let x = entity.get::<ae::BodyKinematics>().map(|k| k.pos.x).unwrap_or_default();
            entity.insert((
                ambition_characters::control::brain_binding::BrainBinding::from_character_profile(),
                ambition_characters::control::brain_binding::AuthoredBrainContext::from_placement(x, 0.0),
            ));
        }
        entity.insert(Visit::to(point));
        if leaving {
            entity.insert(Leaving { zone: place.to_owned() });
        } else {
            entity.remove::<Leaving>();
        }
        world.write_message(BrainCommand::use_preset(LiveBodyId::new(sim_id, at), brain.to_owned()));
        sent += 1;
    }
    if sent == 0 {
        let what = if leaving { "loading zone" } else { "place" };
        return CommandOutcome::refused(format!("the room of {character} has no {what} named `{place}`"));
    }
    CommandOutcome::Done
}

fn walk_to(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    send(world, args, room, "walk_to", false, 0.0)
}

fn leave_through(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    send(world, args, room, "leave_through", true, 0.0)
}

fn form_up(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    let Some(spacing) = args[2].as_number().map(|value| value as f32).filter(|value| value.is_finite()) else {
        return CommandOutcome::refused("actor.form_up needs finite spacing in pixels");
    };
    send(world, &[args[0].clone(), args[1].clone(), args[3].clone()], room, "form_up", false, spacing)
}

fn release(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    let Some(character) = Who::of(&args[0]) else {
        return CommandOutcome::refused("`actor.release` takes a character");
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    for (body, sim_id, at) in cast {
        world.entity_mut(body).remove::<(Visit, Leaving, Posed)>();
        world.write_message(BrainCommand::restore_default(LiveBodyId::new(sim_id, at)));
    }
    CommandOutcome::Done
}

fn face(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    let (Some(character), Some(way)) = (Who::of(&args[0]), Who::of(&args[1])) else {
        return CommandOutcome::refused("`actor.face` takes a character and a way");
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    let toward: Vec<(Option<LiveRoomInstance>, ae::Vec2)> = match way {
        Who::Character("left" | "right") => Vec::new(),
        other => {
            let others = match cast_of(world, other, room) {
                Ok(others) => others,
                Err(why) => return CommandOutcome::refused(format!("`actor.face` faces `left`, `right` or a body, and {why}")),
            };
            others
                .iter()
                .filter_map(|(body, _, at)| world.get::<ae::BodyKinematics>(*body).map(|k| (*at, k.pos)))
                .collect()
        }
    };
    for (body, _, at) in cast {
        let Some(mut kinematics) = world.get_mut::<ae::BodyKinematics>(body) else {
            continue;
        };
        let here = kinematics.pos;
        let facing = match way {
            Who::Character("left") => -1.0,
            Who::Character("right") => 1.0,
            _ => {
                let nearest = toward
                    .iter()
                    .filter(|(room, _)| *room == at)
                    .map(|(_, pos)| *pos)
                    .min_by(|a, b| (*a - here).length().total_cmp(&(*b - here).length()));
                match nearest {
                    Some(pos) if pos.x != here.x => (pos.x - here.x).signum(),
                    _ => continue,
                }
            }
        };
        kinematics.facing = facing;
    }
    CommandOutcome::Done
}

fn pose(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    let (Some(character), Some(row), Some(mode)) = (Who::of(&args[0]), args[1].as_name(), args[2].as_name()) else {
        return CommandOutcome::refused("`actor.pose` takes a character, a row and `loop` or `hold`");
    };
    let looping = match mode {
        "loop" => true,
        "hold" => false,
        other => return CommandOutcome::refused(format!("`actor.pose` loops (`loop`) or holds (`hold`) its row, not `{other}`")),
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    let since = world.get_resource::<ambition_time::SimTick>().map_or(0, |tick| tick.get());
    for (body, _, _) in cast {
        world.entity_mut(body).insert(Posed { row: row.to_owned(), since, looping });
    }
    CommandOutcome::Done
}

fn hold(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    use ambition_combat::held_items::HeldItem;
    let (Some(character), Some(item)) = (Who::of(&args[0]), args[1].as_name()) else {
        return CommandOutcome::refused("`actor.hold` takes a character and a held item");
    };
    let Some(spec) = ambition_characters::brain::held_item_by_id(item) else {
        return CommandOutcome::refused(format!("`{item}` is not a held item of the registry"));
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    if let Some(busy) = cast
        .iter()
        .filter_map(|(body, ..)| world.get::<HeldItem>(*body))
        .find(|held| held.id() != item)
    {
        return CommandOutcome::refused(format!("a body of {character} already holds `{}`", busy.id()));
    }
    for (body, _, _) in cast {
        let held = HeldItem::new(spec.clone());
        // The hand changes the body's repertoire: fold it for the new hand.
        ambition_combat::hand::refold_in_world_with_held(world, body, Some(&held));
        world.entity_mut(body).insert(held);
    }
    CommandOutcome::Done
}

/// How fast a shove throws a body at its middle; at its edge, slower.
const SHOVE_SPEED: f32 = 520.0;
/// How high a shove lifts a body off its feet (px/s, up).
const SHOVE_LIFT: f32 = 280.0;
/// How long a shoved actor reels before it acts again.
const SHOVE_STUN_S: f32 = 0.45;

fn shove(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    let (Some(character), Some(radius)) = (Who::of(&args[0]), args[1].as_number()) else {
        return CommandOutcome::refused("`actor.shove` takes a character and a radius");
    };
    let radius = radius as f32;
    if !(radius.is_finite() && radius > 0.0) {
        return CommandOutcome::refused("`actor.shove` needs a finite radius above zero");
    }
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    for (shover, _, at) in cast {
        let Some((center, facing)) = world.get::<ae::BodyKinematics>(shover).map(|k| (k.pos, k.facing)) else {
            continue;
        };
        let mut state: SystemState<(
            ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
            Query<(
                Entity,
                &ae::BodyKinematics,
                &mut ae::body_clusters::BodyFlightState,
                Option<&ambition_characters::actor::BodyHealth>,
                Option<&mut ambition_characters::actor::BodyCombat>,
            )>,
        )> = SystemState::new(world);
        let Ok((rooms, mut bodies)) = state.get_mut(world) else {
            continue;
        };
        for (body, kinematics, mut flight, health, combat) in &mut bodies {
            if body == shover || rooms.of(body) != at || !alive(health) {
                continue;
            }
            let apart = kinematics.pos - center;
            let distance = apart.length();
            if distance > radius {
                continue;
            }
            let away = if apart.x.abs() > 0.5 { apart.x.signum() } else { facing.signum() };
            let speed = SHOVE_SPEED * (1.0 - 0.4 * distance / radius);
            flight.stage_launch(ae::Vec2::new(away * speed, -SHOVE_LIFT), false, Default::default());
            if let Some(mut combat) = combat {
                combat.hitstun_timer = combat.hitstun_timer.max(SHOVE_STUN_S);
            }
        }
        state.apply(world);
        world.write_message(ambition_vfx::vfx::VfxInRoom {
            room: at,
            vfx: ambition_vfx::vfx::VfxMessage::Burst {
                pos: center,
                count: 22,
                speed: 260.0,
                color: [0.86, 0.78, 0.62, 0.9],
                kind: ambition_vfx::vfx::ParticleKind::Dust,
            },
        });
    }
    CommandOutcome::Done
}

fn fight(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    use ambition_combat::components::{ActorAggression, ActorDisposition, AggressionMode, Grudge};
    let (Some(character), Some(target)) = (Who::of(&args[0]), Who::of(&args[1])) else {
        return CommandOutcome::refused("`actor.fight` takes a character and a target character");
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    let foes: Vec<(Entity, Option<LiveRoomInstance>, ae::Vec2)> = match cast_of(world, target, room) {
        Ok(foes) => foes,
        Err(why) => return CommandOutcome::refused(why),
    }
    .iter()
        .filter_map(|(body, _, at)| world.get::<ae::BodyKinematics>(*body).map(|k| (*body, *at, k.pos)))
        .collect();
    let mut state: SystemState<(
        Commands,
        ambition_characters::prepared::SessionCast,
        Query<(
            &ae::BodyKinematics,
            &CharacterForm,
            &mut ActorAggression,
            &mut ActorDisposition,
            Option<&ambition_characters::brain::ActionSet>,
            &ambition_combat::actor_tuning::ActorConfig,
            &ambition_combat::components::ActorIdentity,
            &ae::BodyAbilities,
        )>,
    )> = SystemState::new(world);
    let mut set = 0;
    {
        let Ok((mut commands, prepared, mut bodies)) = state.get_mut(world) else {
            return CommandOutcome::refused("`actor.fight` found no bodies to set on");
        };
        for (body, _, at) in &cast {
            let Ok((kinematics, worn, mut aggression, mut disposition, repertoire, config, identity, abilities)) =
                bodies.get_mut(*body)
            else {
                continue;
            };
            let here = kinematics.pos;
            let Some(foe) = foes
                .iter()
                .filter(|(_, room, _)| room == at)
                .min_by(|a, b| (a.2 - here).length().total_cmp(&(b.2 - here).length()))
                .map(|(foe, ..)| *foe)
            else {
                continue;
            };
            let Some(policy) = prepared.get().and_then(|registry| {
                ambition_platformer2d_actor_spawn::brain_builders::authored_provoked_policy(registry, worn.id())
            }) else {
                continue;
            };
            aggression.mode = AggressionMode::Hostile;
            aggression.target = Some(foe);
            aggression.grudge = Some(Grudge::Body(foe));
            crate::body::actors::provoke_actor_in_place(
                &mut commands,
                *body,
                (config, identity, abilities.abilities),
                &mut disposition,
                repertoire,
                Some(policy),
            );
            set += 1;
        }
    }
    state.apply(world);
    if set == 0 {
        return CommandOutcome::refused(format!("no body of {character} has a provoked mind to fight with"));
    }
    CommandOutcome::Done
}

/// How long a scene's throw is in the air, at most and at least.
const THROW_FLIGHT_S: (f32, f32) = (0.45, 0.85);
/// How far a scene's throw travels in each second of its flight, before the
/// clamp above.
const THROW_PACE: f32 = 380.0;

fn throw(world: &mut World, args: &[AuthoredArg], context: &ambition_platformer2d_shared_tangle::authored_logic::CommandContext) -> CommandOutcome {
    let room = context.room;
    use ambition_combat::held_items::HeldItem;
    let (Some(character), Some(target)) = (Who::of(&args[0]), Who::of(&args[1])) else {
        return CommandOutcome::refused("`actor.throw` takes a character and a target character");
    };
    let cast = match cast_of(world, character, room) {
        Ok(cast) => cast,
        Err(why) => return CommandOutcome::refused(why),
    };
    let targets: Vec<(Option<LiveRoomInstance>, ae::Vec2)> = match cast_of(world, target, room) {
        Ok(targets) => targets,
        Err(why) => return CommandOutcome::refused(why),
    }
    .iter()
        .filter_map(|(body, _, at)| world.get::<ae::BodyKinematics>(*body).map(|k| (*at, k.pos)))
        .collect();
    let scope = world
        .get_resource::<ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope>()
        .map_or(ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope::UNSCOPED, |scope| scope.spawn_scope());
    let mut thrown = 0;
    for (body, sim_id, at) in cast {
        let Some(spec) = world.get::<HeldItem>(body).map(|held| held.spec.clone()) else {
            continue;
        };
        let Some((here, size)) = world.get::<ae::BodyKinematics>(body).map(|k| (k.pos, k.size())) else {
            continue;
        };
        let Some(aim) = targets
            .iter()
            .filter(|(room, _)| *room == at)
            .map(|(_, pos)| *pos)
            .min_by(|a, b| (*a - here).length().total_cmp(&(*b - here).length()))
        else {
            continue;
        };
        let side = if aim.x >= here.x { 1.0 } else { -1.0 };
        // Out of the hand ahead of the body, clear of its own box, so the
        // item does not strike the body that threw it.
        let from = here
            + ae::Vec2::new(
                side * (size.x * 0.5 + ambition_held_items::MINTED_ITEM_HALF_EXTENT.x + 6.0),
                -size.y * 0.3,
            );
        // The arc that lands at the target: a fall under the items' own
        // gravity, its time set by the distance.
        let gap = aim - from;
        let flight = (gap.x.abs() / THROW_PACE).clamp(THROW_FLIGHT_S.0, THROW_FLIGHT_S.1);
        let gravity = ambition_held_items::GROUND_ITEM_GRAVITY;
        let vel = ae::Vec2::new(gap.x / flight, (gap.y - 0.5 * gravity * flight * flight) / flight);
        let Some(sequence) = world
            .get_mut::<ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>(body)
            .map(|mut counter| counter.next())
        else {
            continue;
        };
        if let Some(mut kinematics) = world.get_mut::<ae::BodyKinematics>(body) {
            kinematics.facing = side;
        }
        world.entity_mut(body).remove::<HeldItem>();
        ambition_combat::hand::refold_in_world_with_held(world, body, None);
        let mut commands = world.commands();
        ambition_platformer2d_items::loot::throw_minted_item(
            &mut commands,
            scope.in_room(at),
            &sim_id,
            sequence,
            spec,
            from,
            vel,
            ambition_held_items::MINTED_ITEM_HALF_EXTENT,
        );
        world.flush();
        thrown += 1;
    }
    if thrown == 0 {
        return CommandOutcome::refused(format!("no body of {character} holds anything it can throw"));
    }
    CommandOutcome::Done
}

fn beaten(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let character = match Who::of(&args[0]) {
        Some(Who::Character(character)) => character,
        Some(Who::Body(id)) => {
            let body = match bound_body(world, id) {
                Ok((body, _)) => body,
                Err(refusal) => return refusal,
            };
            let beaten = world
                .get::<ambition_characters::actor::BodyHealth>(body)
                .zip(world.get::<ambition_combat::components::CombatCapabilities>(body))
                .is_some_and(|(health, caps)| {
                    health.health.current > 0 && caps.beaten_at.is_some_and(|floor| health.health.current <= floor)
                });
            return if beaten {
                ConditionOutcome::Satisfied
            } else {
                ConditionOutcome::NotSatisfied(WhyNot::new("actor.beaten", id.to_string(), "it is not beaten".to_owned()))
            };
        }
        None => return ConditionOutcome::Unanswerable("`actor.beaten` takes a character".into()),
    };
    let Some(mut bodies) = world.try_query::<(
        &CharacterForm,
        &ambition_characters::actor::BodyHealth,
        &ambition_combat::components::CombatCapabilities,
    )>() else {
        return ConditionOutcome::Unanswerable("no body has a character in this composition".into());
    };
    let beaten = bodies.iter(world).any(|(worn, health, caps)| {
        worn.0.as_str() == character
            && health.health.current > 0
            && caps.beaten_at.is_some_and(|floor| health.health.current <= floor)
    });
    if beaten {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.beaten", character, "no body of it is beaten".to_owned()))
    }
}

/// Draw each posed body with its pose's row, on the pose's own clock; and
/// take the pin away when the pose ends. A pin another writer holds (a
/// conductor's) is left alone; a body's rest row gives way to a pose.
/// (sim, after the rest rows)
pub fn pin_poses(
    tick: bevy::prelude::Res<ambition_time::SimTick>,
    mut bodies: Query<
        (Entity, Option<&Posed>, Option<&mut ambition_sprite_sheet::character::PinnedRow>),
        bevy::prelude::Or<(bevy::prelude::With<Posed>, bevy::prelude::With<ambition_sprite_sheet::character::PinnedRow>)>,
    >,
    mut commands: Commands,
) {
    use ambition_sprite_sheet::character::PinnedRow;
    const TICK_S: f32 = 1.0 / 60.0;
    for (body, posed, pinned) in &mut bodies {
        match (posed, pinned) {
            (Some(posed), None) => {
                let mut pin = PinnedRow::default();
                pin.pin_as(POSE_ROW_WRITER, &[posed.row.as_str()], tick.get().saturating_sub(posed.since) as f32 * TICK_S, posed.looping);
                commands.entity(body).try_insert(pin);
            }
            (Some(posed), Some(mut pinned))
                if pinned.written_by(POSE_ROW_WRITER)
                    || pinned.written_by(super::rest_row::REST_ROW_WRITER)
                    || !pinned.is_pinned() =>
            {
                let elapsed = tick.get().saturating_sub(posed.since) as f32 * TICK_S;
                pinned.pin_as(POSE_ROW_WRITER, &[posed.row.as_str()], elapsed, posed.looping);
            }
            (None, Some(mut pinned)) if pinned.written_by(POSE_ROW_WRITER) => pinned.clear(),
            _ => {}
        }
    }
}

fn arrived(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let character = match Who::of(&args[0]) {
        Some(Who::Character(character)) => character,
        Some(Who::Body(id)) => {
            let observed = match bound_body(world, id) {
                Ok((_, false)) => "it has fallen",
                Ok((body, true)) if world.get::<Visit>(body).is_some_and(|visit| visit.outcome == VisitOutcome::Arrived) => {
                    return ConditionOutcome::Satisfied;
                }
                Ok(_) => "it is not there yet",
                Err(refusal) => return refusal,
            };
            return ConditionOutcome::NotSatisfied(WhyNot::new("actor.arrived", id.to_string(), observed.to_owned()));
        }
        None => return ConditionOutcome::Unanswerable("`actor.arrived` takes a character".into()),
    };
    let Some(mut bodies) =
        world.try_query::<(&CharacterForm, Option<&Visit>, Option<&ambition_characters::actor::BodyHealth>)>()
    else {
        return ConditionOutcome::Unanswerable("no body has a character in this composition".into());
    };
    let visits: Vec<Option<VisitOutcome>> = bodies
        .iter(world)
        .filter(|(worn, _, health)| worn.0.as_str() == character && alive(*health))
        .map(|(_, visit, _)| visit.map(|v| v.outcome))
        .collect();
    let waiting = visits.iter().filter(|v| **v != Some(VisitOutcome::Arrived)).count();
    if !visits.is_empty() && waiting == 0 {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new(
            "actor.arrived",
            character,
            if visits.is_empty() { "no body of it lives".to_owned() } else { format!("{waiting} of {} not there yet", visits.len()) },
        ))
    }
}

fn gone(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let character = match Who::of(&args[0]) {
        Some(Who::Character(character)) => character,
        Some(Who::Body(id)) => {
            let live = carrying(world, id).iter().filter(|(.., alive)| *alive).count();
            return if live == 0 {
                ConditionOutcome::Satisfied
            } else {
                ConditionOutcome::NotSatisfied(WhyNot::new("actor.gone", id.to_string(), format!("{live} live")))
            };
        }
        None => return ConditionOutcome::Unanswerable("`actor.gone` takes a character".into()),
    };
    let live = live_count(world, character).unwrap_or(0);
    if live == 0 {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.gone", character, format!("{live} live")))
    }
}

/// Has what `who` threw (`actor.throw`) come down? A thrown item is still
/// in the air while it moves, and a thrown bomb is still to come while its
/// fuse burns; one that burst is gone, and one that was caught is no longer
/// thrown. The items answer by their identity (`SimId::spawned` of the
/// thrower), so a thrower that has since left still answers for its throw.
///
/// ⚠ A thrower that still holds what it could throw has not thrown yet, and
/// the answer is no. A scene's command is a request applied after the scene
/// asks its next wait, so on the first answer after a throw beat the item may
/// not exist yet: "nothing is in the air" must not be read as "it landed"
/// before the throw has happened.
///
/// It is the result a scene waits for when what follows the throw depends on
/// its impact (the brute's bomb must have burst before the Creator is let go
/// to his own mind, which would dodge it); a fixed delay is not that result,
/// and a skipped scene has no delays.
fn throw_landed(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let throwers: Vec<SimId> = match Who::of(&args[0]) {
        Some(Who::Body(id)) => vec![id.clone()],
        Some(Who::Character(character)) => {
            let Some(mut bodies) = world.try_query::<(&CharacterForm, &SimId)>() else {
                return ConditionOutcome::Satisfied;
            };
            bodies.iter(world).filter(|(worn, _)| worn.0.as_str() == character).map(|(_, id)| id.clone()).collect()
        }
        None => return ConditionOutcome::Unanswerable("`actor.throw_landed` takes a character".into()),
    };
    let still_holding = world.try_query::<(&SimId, &ambition_combat::held_items::HeldItem)>().is_some_and(|mut holders| {
        holders.iter(world).any(|(id, _)| throwers.contains(id))
    });
    if still_holding {
        let who = throwers.first().map_or_else(String::new, SimId::to_string);
        return ConditionOutcome::NotSatisfied(WhyNot::new("actor.throw_landed", who, "it has not thrown yet".to_owned()));
    }
    let Some(mut items) = world.try_query::<(
        &SimId,
        &ae::BodyKinematics,
        &ambition_held_items::ReleasedAs,
        bevy::prelude::Has<ambition_abilities::ranged::bomb::BombFuse>,
    )>() else {
        return ConditionOutcome::Satisfied;
    };
    let coming = items
        .iter(world)
        .filter(|(id, _, released, _)| {
            released.0 == ambition_held_items::Release::Throw && throwers.iter().any(|thrower| id.spawned_by(thrower))
        })
        .filter(|(_, kinematics, _, lit)| *lit || kinematics.vel.length_squared() > 1.0)
        .count();
    if coming == 0 {
        ConditionOutcome::Satisfied
    } else {
        let who = throwers.first().map_or_else(String::new, SimId::to_string);
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.throw_landed", who, format!("{coming} still coming down")))
    }
}

/// A leaving body that reaches its zone goes through it. Into a live room it
/// comes out at the zone's arrival there (what it carries goes with it, its
/// room stamp names the room, and a body with durable whereabouts gets its
/// `Placed` row there). Into a room that is not live, a body with durable
/// whereabouts leaves through the ledger (its `Placed` row names that room)
/// and is despawned, with what it carries; any other body is despawned. A
/// zone the room does not lead through (no transition) despawns the body:
/// it went out of the world the scene shows.
#[allow(clippy::type_complexity)]
pub fn leave_through_zones(
    mut commands: Commands,
    specs: Option<ambition_platformer2d_world::rooms::LiveRoomSpecs>,
    custody: Query<(Entity, &ambition_platformer2d_shared_tangle::lifecycle::InCustodyOf)>,
    mut bodies: Query<(
        Entity,
        &Leaving,
        &Visit,
        &SimId,
        &ambition_combat::actor_tuning::ActorConfig,
        Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ae::BodyClusterQueryData,
        &mut ae::movement::MotionModel,
    )>,
    mut occurrences: Option<bevy::prelude::ResMut<ambition_platformer2d_shared_tangle::lifecycle::AuthoredOccurrences>>,
) {
    use ae::AabbExt;
    let Some(specs) = specs else {
        return;
    };
    let mut edges: Option<Vec<(Entity, Entity)>> = None;
    for (body, leaving, visit, sim_id, config, origin, mut cluster, mut model) in &mut bodies {
        let Some(body_room) = specs.live().of(body) else {
            continue;
        };
        let Some(from) = specs.definition_in(body_room) else {
            continue;
        };
        let rooms = specs.rooms();
        let Some(zone) = rooms.spec(from).loading_zones.iter().find(|zone| zone.id == leaving.zone) else {
            continue;
        };
        // Touching the zone is reaching it: an edge zone is a wall for every
        // body but a driven one (`contribute_loading_zone_walls`), so a sent
        // body comes up against it and never inside.
        let reach = ae::Vec2::splat(ae::navigation::ARRIVE_TOLERANCE);
        let near = ae::Aabb { min: zone.aabb.min - reach, max: zone.aabb.max + reach };
        if visit.outcome != VisitOutcome::Arrived && !cluster.kinematics.aabb().strict_intersects(near) {
            continue;
        }
        let edges = edges.get_or_insert_with(|| custody.iter().map(|(entity, held)| (entity, held.custodian)).collect());
        let moving = ambition_platformer2d_shared_tangle::lifecycle::custody_closure([body], edges);
        let durable = ambition_platformer2d_construction::whereabouts::keeps_durable_whereabouts(&specs, config, origin);
        let Some(transition) = rooms.transition_through(from, &leaving.zone) else {
            for gone in moving {
                commands.entity(gone).despawn();
            }
            continue;
        };
        let into_id = rooms.rooms[transition.target_room].id.clone();
        let live_into: Vec<LiveRoomInstance> = specs
            .live_rooms()
            .filter(|(_, definition)| definition.index() == transition.target_room)
            .map(|(live, _)| live)
            .collect();
        let size = cluster.kinematics.size();
        let feet = ae::Vec2::new(transition.arrival.x, transition.arrival.y + size.y * 0.5);
        match live_into.as_slice() {
            [into] => {
                for moving in &moving {
                    commands
                        .entity(*moving)
                        .insert(ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance(*into))
                        .remove::<(Leaving, Visit)>();
                }
                let mut clusters = cluster.as_clusters_mut();
                ae::movement::transit_body(&mut model, &mut clusters, transition.arrival, ae::movement::TransitVelocity::Zero);
                if durable {
                    if let Some(occurrences) = occurrences.as_deref_mut() {
                        let _ = occurrences.admit_crossing(sim_id.clone(), &into_id, feet);
                    }
                }
            }
            _ => {
                if durable {
                    if let Some(occurrences) = occurrences.as_deref_mut() {
                        let _ = occurrences.admit_crossing(sim_id.clone(), &into_id, feet);
                    }
                }
                for gone in moving {
                    commands.entity(gone).despawn();
                }
            }
        }
    }
}

// An exact count, so a scene meant for "the others have fallen" does not
// play when they fell together and nobody is left to see it.
fn fallen_is(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let (Some(character), Some(count)) = (args[0].as_name(), args[1].as_number()) else {
        return ConditionOutcome::Unanswerable("`actor.fallen_is` takes a character and a count".into());
    };
    let fallen = fallen_count(world, character).unwrap_or(0);
    if fallen as f64 == count {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.fallen_is", character, format!("{fallen} fallen, not {count}")))
    }
}

fn fallen_in(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let (Some(character), Some(count), Some(room)) = (args[0].as_name(), args[1].as_number(), args[2].as_name()) else {
        return ConditionOutcome::Unanswerable("`actor.fallen_in` takes a character, a count and a room".into());
    };
    let fallen = fallen_count_in(world, character, room).unwrap_or(0);
    if fallen as f64 == count {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.fallen_in", character, format!("{fallen} fallen in `{room}`, not {count}")))
    }
}

fn count_is(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let (Some(character), Some(count)) = (args[0].as_name(), args[1].as_number()) else {
        return ConditionOutcome::Unanswerable("`actor.count_is` takes a character and a count".into());
    };
    let live = live_count(world, character).unwrap_or(0);
    if live as f64 == count {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.count_is", character, format!("{live} live, not {count}")))
    }
}
