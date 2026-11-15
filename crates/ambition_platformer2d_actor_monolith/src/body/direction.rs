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
//!
//! And the questions a sequence waits on:
//!
//! - `actor.arrived <character>`: every live body of the character stands
//!   where it was sent (and at least one lives).
//! - `actor.gone <character>`: no live body of the character.
//! - `actor.count_is <character> <n>`: exactly `n` live bodies of the
//!   character.
//! - `actor.fallen_is <character> <n>`: exactly `n` fallen bodies of the
//!   character (the raid's last raider: `actor.fallen_is npc_lab_raider 2`;
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
    kind: ParamKind::Name,
    summary: "the character id of the body (every body of it in the asking room)",
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
const BRAIN: ParamSpec = ParamSpec {
    name: "brain",
    kind: ParamKind::Name,
    summary: "the brain preset the body walks with (`keeps_to_its_tasks` walks)",
};

const WAY: ParamSpec = ParamSpec {
    name: "way",
    kind: ParamKind::Name,
    summary: "`left`, `right`, or a character id to face toward",
};
const ROW: ParamSpec = ParamSpec {
    name: "row",
    kind: ParamKind::Name,
    summary: "a named row of the body's sheet",
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
            params: &[CHARACTER, COUNT],
        },
        count_is,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "fallen_is"),
            summary: "exactly this many bodies of the character have fallen and lie where they fell",
            params: &[CHARACTER, COUNT],
        },
        fallen_is,
    );
    app.publish_condition(
        ConditionDescriptor {
            id: ConditionId::new("actor", "gone"),
            summary: "no body of the character is live",
            params: &[CHARACTER],
        },
        gone,
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
) -> CommandOutcome {
    let (Some(character), Some(place), Some(brain)) = (args[0].as_name(), args[1].as_name(), args[2].as_name()) else {
        return CommandOutcome::refused(format!("`actor.{verb}` takes a character, a place and a brain preset"));
    };
    let cast = cast_in(world, character, room);
    if cast.is_empty() {
        return CommandOutcome::refused(format!("no body of `{character}` is live in the asking room"));
    }
    let mut sent = 0;
    for (body, sim_id, at) in cast {
        let Some(point) = place_of(world, body, place, leaving) else {
            continue;
        };
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
        return CommandOutcome::refused(format!("the room of `{character}` has no {what} named `{place}`"));
    }
    CommandOutcome::Done
}

fn walk_to(world: &mut World, args: &[AuthoredArg], room: Option<LiveRoomInstance>) -> CommandOutcome {
    send(world, args, room, "walk_to", false)
}

fn leave_through(world: &mut World, args: &[AuthoredArg], room: Option<LiveRoomInstance>) -> CommandOutcome {
    send(world, args, room, "leave_through", true)
}

fn release(world: &mut World, args: &[AuthoredArg], room: Option<LiveRoomInstance>) -> CommandOutcome {
    let Some(character) = args[0].as_name() else {
        return CommandOutcome::refused("`actor.release` takes a character");
    };
    let cast = cast_in(world, character, room);
    if cast.is_empty() {
        return CommandOutcome::refused(format!("no body of `{character}` is live in the asking room"));
    }
    for (body, sim_id, at) in cast {
        world.entity_mut(body).remove::<(Visit, Leaving, Posed)>();
        world.write_message(BrainCommand::restore_default(LiveBodyId::new(sim_id, at)));
    }
    CommandOutcome::Done
}

fn face(world: &mut World, args: &[AuthoredArg], room: Option<LiveRoomInstance>) -> CommandOutcome {
    let (Some(character), Some(way)) = (args[0].as_name(), args[1].as_name()) else {
        return CommandOutcome::refused("`actor.face` takes a character and a way");
    };
    let cast = cast_in(world, character, room);
    if cast.is_empty() {
        return CommandOutcome::refused(format!("no body of `{character}` is live in the asking room"));
    }
    let toward: Vec<(Option<LiveRoomInstance>, ae::Vec2)> = match way {
        "left" | "right" => Vec::new(),
        other => {
            let others = cast_in(world, other, room);
            if others.is_empty() {
                return CommandOutcome::refused(format!(
                    "`actor.face` faces `left`, `right` or a character, and no body of `{other}` is live in the asking room"
                ));
            }
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
            "left" => -1.0,
            "right" => 1.0,
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

fn pose(world: &mut World, args: &[AuthoredArg], room: Option<LiveRoomInstance>) -> CommandOutcome {
    let (Some(character), Some(row), Some(mode)) = (args[0].as_name(), args[1].as_name(), args[2].as_name()) else {
        return CommandOutcome::refused("`actor.pose` takes a character, a row and `loop` or `hold`");
    };
    let looping = match mode {
        "loop" => true,
        "hold" => false,
        other => return CommandOutcome::refused(format!("`actor.pose` loops (`loop`) or holds (`hold`) its row, not `{other}`")),
    };
    let cast = cast_in(world, character, room);
    if cast.is_empty() {
        return CommandOutcome::refused(format!("no body of `{character}` is live in the asking room"));
    }
    let since = world.get_resource::<ambition_time::SimTick>().map_or(0, |tick| tick.get());
    for (body, _, _) in cast {
        world.entity_mut(body).insert(Posed { row: row.to_owned(), since, looping });
    }
    CommandOutcome::Done
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
    let Some(character) = args[0].as_name() else {
        return ConditionOutcome::Unanswerable("`actor.arrived` takes a character".into());
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
    let Some(character) = args[0].as_name() else {
        return ConditionOutcome::Unanswerable("`actor.gone` takes a character".into());
    };
    let live = live_count(world, character).unwrap_or(0);
    if live == 0 {
        ConditionOutcome::Satisfied
    } else {
        ConditionOutcome::NotSatisfied(WhyNot::new("actor.gone", character, format!("{live} live")))
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
