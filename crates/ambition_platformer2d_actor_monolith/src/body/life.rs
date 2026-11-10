//! A life: a body that follows an authored plan (RADIANT-NPC-HTN).
//!
//! The executor of `docs/planning/engine/radiant-npc-planning.md`. A body
//! whose character has a row in the game's [`LifeBook`] gets a [`Life`]: its
//! domain's name and its own facts. The executor plans from those facts (the
//! planner, `ambition_htn`, reads nothing else), then carries the plan's
//! steps out one at a time:
//!
//! - `go_to <place>` is a [`Visit`] to the place named in the body's room, and
//!   is done when the visit arrives. The room's own name is the room: a body
//!   already in it is there.
//! - `claim <thing>` takes the thing for this body alone, in its live room. A
//!   thing another body holds is a lost claim: the body lets go of what it
//!   held and plans again at once, from what it now sees.
//! - `release <thing>` lets go of it.
//! - any other verb holds the body at its task for the hours the book says
//!   (`holds`), or is done the tick it starts.
//!
//! **When it plans.** On each new hour of the day; at once after a lost
//! claim; and when its plan is spent and something it observes has changed
//! since it planned (the bench it found taken is free again). A plan replaced
//! lets go of what the body held.
//!
//! **Interrupts.** A body a conversation holds stops its plan (it lets go of
//! its claim and its walk) and plans again when the conversation ends. A step
//! that fails (no route to its place, a place the room no longer names) plans
//! again at once, and the failure becomes a fact the domain can read:
//! `unreachable:<place>` or `absent:<place>`, kept until the next hour. The
//! reason is the life's `because`. The same failure twice in a row waits for
//! the next hour, so a domain that ignores the fact does not plan every tick.
//!
//! **What it knows.** Its own facts; the facts its author owns (its hours),
//! read from the book each time it plans and never kept as the body's own, so
//! no fact a body carries holds an hour the book no longer states; and the
//! facts its row says it observes in the world, read when it plans: whether a thing is free (no other body
//! in the room holds it) and whether a character is present (in the room).
//! When a step is done its effects become the body's facts, so the next plan
//! starts from what the body did, not from what it planned. A step that
//! cannot be carried out (a place the room does not name, a place no route
//! gets to) stops the plan with the reason.
//!
//! Bodies are looked at in the order of their character ids, so which of two
//! bodies takes a thing first is a fact of the cast, not of query order.
//!
//! Rollback state: the plan, how far it is, the facts and the claim. The
//! planner is not: it runs on the tick a plan is asked for, from the restored
//! facts, so a replay plans the same steps.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ambition_characters::actor::WornCharacter;
use ambition_htn::{Budget, Domain, Facts, PlanFailure, Step};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveRoomInstance, LiveRooms};
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, RoomSpec};
use bevy::prelude::{Commands, Component, Entity, Query, Res, Resource, Without};

use super::visit::{Visit, VisitOutcome};

/// The planner's fact types, so a reader of a [`Life`] needs no second crate.
pub use ambition_htn::Value;

/// The verb that sends a body to a place.
pub const GO_TO: &str = "go_to";
/// The verb that takes a thing for one body alone.
pub const CLAIM: &str = "claim";
/// The verb that lets a claimed thing go.
pub const RELEASE: &str = "release";

/// Every authored life, published by the game from its content outside the
/// tick. Lives are keyed by the character id of the body that lives them.
#[derive(Resource, Clone, Debug, Default)]
pub struct LifeBook {
    pub domains: BTreeMap<String, Arc<Domain>>,
    pub lives: BTreeMap<String, LifeSpec>,
    /// How long a verb holds a body at its task (hours of the day).
    pub holds: BTreeMap<String, f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifeSpec {
    pub domain: String,
    /// The facts a body starts with (what it has done: `ate`).
    pub facts: Facts,
    /// The facts the author owns (its hours): written into the body's facts
    /// each time it plans, from the book as it is then.
    pub authored: Facts,
    /// Facts read from the world when the body plans, by fact name.
    pub observes: BTreeMap<String, Observation>,
}

/// A fact the world states, read when a body plans.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    /// No other body in the room holds a claim on this thing.
    Free(String),
    /// A body of this character is in the room.
    Present(String),
}

/// What a body is doing with its life.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Life {
    pub domain: String,
    pub facts: Facts,
    /// The (day, hour) the plan was made in. A new hour is a new plan.
    pub planned_in: Option<(u32, i64)>,
    /// The plan's steps still to do; the first is under way.
    pub steps: Vec<Step>,
    pub doing: Doing,
    /// The thing this body holds a claim on, in its live room.
    pub claim: Option<String>,
    /// How many plans this body has made.
    pub revision: u32,
    /// Why the last plan stopped short of its end, for an inspector and a
    /// line of dialogue. `None` while it runs, and once it ends.
    pub stopped: Option<LifeStop>,
    /// Why this plan was made when the clock did not ask for one: the stop
    /// that cut the last plan short ("the bench was taken"). `None` for a
    /// plan of a new hour, or one made because something seen changed.
    pub because: Option<LifeStop>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Doing {
    /// Between plans, or the plan is spent.
    Idle,
    /// The first step starts on the next look.
    Starting,
    /// Walking to this place on a visit.
    GoingTo(ae::Vec2),
    /// At the task until this time (seconds of the clock since day 0).
    Holding { until: f64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifeStop {
    /// The book has no domain of this name.
    NoDomain(String),
    /// No decomposition of the domain's root holds for these facts.
    NoPlan,
    OverBudget,
    /// The room names no place of this name.
    NoPlace(String),
    /// No route gets the body to this place.
    NoRoute(String),
    /// Another body holds this thing.
    Taken(String),
    /// Something outside the plan stopped it (a conversation).
    Interrupted(String),
}

impl LifeStop {
    /// A stop that plans again at once, rather than at the next hour.
    fn replans(&self) -> bool {
        matches!(self, Self::NoRoute(_) | Self::NoPlace(_) | Self::Interrupted(_))
    }

    /// The fact a failed step leaves for the next plan.
    fn fact(&self) -> Option<String> {
        match self {
            Self::NoRoute(place) => Some(format!("{UNREACHABLE}{place}")),
            Self::NoPlace(place) => Some(format!("{ABSENT}{place}")),
            _ => None,
        }
    }
}

/// The fact a place no route got to leaves (`unreachable:<place>`).
pub const UNREACHABLE: &str = "unreachable:";
/// The fact a place the room no longer names leaves (`absent:<place>`).
pub const ABSENT: &str = "absent:";
/// The reason a conversation gives an interrupted plan.
pub const CONVERSATION: &str = "conversation";

impl Life {
    pub fn new(spec: &LifeSpec) -> Self {
        Self {
            domain: spec.domain.clone(),
            facts: spec.facts.iter().chain(&spec.authored).map(|(name, value)| (name.clone(), value.clone())).collect(),
            planned_in: None,
            steps: Vec::new(),
            doing: Doing::Idle,
            claim: None,
            revision: 0,
            stopped: None,
            because: None,
        }
    }

    /// The task under way, if any.
    pub fn task(&self) -> Option<&Step> {
        self.steps.first()
    }

    /// One number for the life, for the rollback probe. Each part is printed
    /// by a derive over ordered maps and vectors, so the text, and the
    /// number, are the same on every peer.
    pub fn probe(&self) -> u64 {
        let text = format!("{self:?}");
        text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
    }
}

/// Give each body whose character has a life in the book its life. Inside the
/// timeline: a function of the world, so a replay gives it on the same tick.
pub fn attach_lives(
    mut commands: Commands,
    book: Res<LifeBook>,
    bodies: Query<(Entity, &WornCharacter), Without<Life>>,
) {
    for (body, worn) in &bodies {
        if let Some(spec) = book.lives.get(worn.id()) {
            commands.entity(body).insert(Life::new(spec));
        }
    }
}

/// The place `name` in `room`: the room itself when it is the room's name,
/// else the first authored thing of the room with that name, in a fixed order
/// of kinds (signs, placements, props, doors).
pub fn place_named(room: &RoomSpec, name: &str, here: ae::Vec2) -> Option<ae::Vec2> {
    if room.id == name {
        return Some(here);
    }
    let center = |aabb: &ae::Aabb| (aabb.min + aabb.max) * 0.5;
    room.debug_labels
        .iter()
        .find(|label| label.name == name)
        .map(|label| label.payload.position)
        .or_else(|| {
            room.placements
                .iter()
                .find(|record| record.id.as_str() == name || record.name == name)
                .map(|record| center(&record.aabb))
        })
        .or_else(|| room.props.iter().find(|prop| prop.name == name).map(|prop| prop.pos))
        .or_else(|| room.loading_zones.iter().find(|zone| zone.id == name).map(|zone| center(&zone.aabb)))
}

/// Who holds what, and who is where, as the executor reads it this tick. It
/// is updated as each body claims or lets go, so a later body sees an earlier
/// one's claim on the same tick.
struct RoomFacts {
    claims: BTreeMap<(Option<LiveRoomInstance>, String), Entity>,
    present: BTreeSet<(Option<LiveRoomInstance>, String)>,
}

impl RoomFacts {
    fn observe(&self, body: Entity, room: Option<LiveRoomInstance>, observation: &Observation) -> Value {
        match observation {
            Observation::Free(thing) => {
                Value::Bool(self.claims.get(&(room, thing.clone())).is_none_or(|holder| *holder == body))
            }
            Observation::Present(character) => Value::Bool(self.present.contains(&(room, character.clone()))),
        }
    }

    fn release(&mut self, body: Entity, room: Option<LiveRoomInstance>, life: &mut Life) {
        if let Some(thing) = life.claim.take() {
            let key = (room, thing);
            if self.claims.get(&key) == Some(&body) {
                self.claims.remove(&key);
            }
        }
    }
}

/// What one body is looked at with this tick.
struct Look<'a> {
    body: Entity,
    room: Option<LiveRoomInstance>,
    here: ae::Vec2,
    visit: Option<&'a Visit>,
    spec: Option<&'a LifeSpec>,
}

/// Plan when a plan is due, and carry the plan out one step at a time.
#[allow(clippy::type_complexity)]
pub fn live_lives(
    mut commands: Commands,
    book: Res<LifeBook>,
    clock: Res<ambition_time::DayClock>,
    cycle: Option<Res<ambition_time::DayCycle>>,
    specs: LiveRoomSpecs,
    rooms: LiveRooms,
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    characters: Query<(Entity, &WornCharacter)>,
    mut bodies: Query<(Entity, &mut Life, &WornCharacter, &ae::BodyKinematics, Option<&Visit>)>,
) {
    // No day, no hours to live by.
    let Some(cycle) = cycle else { return };
    if clock.day == 0 || cycle.day_length_s <= 0.0 {
        return;
    }
    let hour = clock.hour(&cycle).floor() as i64;
    let now = f64::from(clock.day) * f64::from(cycle.day_length_s) + f64::from(clock.seconds);
    let mut facts = RoomFacts {
        claims: bodies
            .iter()
            .filter_map(|(body, life, ..)| life.claim.clone().map(|thing| ((rooms.of(body), thing), body)))
            .collect(),
        present: characters.iter().map(|(body, worn)| (rooms.of(body), worn.id().to_string())).collect(),
    };
    let mut order: Vec<(String, Entity)> =
        bodies.iter().map(|(body, _, worn, ..)| (worn.id().to_string(), body)).collect();
    order.sort();
    for (character, body) in order {
        let Ok((_, mut life, _, kinematics, visit)) = bodies.get_mut(body) else { continue };
        let look = Look {
            body,
            room: rooms.of(body),
            here: kinematics.pos,
            visit,
            spec: book.lives.get(&character),
        };
        // A conversation holds the body (`interrupt_talked_to_lives` stopped
        // its plan): it waits.
        if conversation.as_ref().and_then(|conversation| conversation.talker()) == Some(body) {
            continue;
        }
        let new_hour = life.planned_in != Some((clock.day, hour));
        let seen_change = life.doing == Doing::Idle
            && look.spec.is_some_and(|spec| {
                spec.observes
                    .iter()
                    .any(|(name, observation)| life.facts.get(name) != Some(&facts.observe(body, look.room, observation)))
            });
        let failed = life
            .stopped
            .clone()
            // An interrupt does not repeat by itself; a failure can, and the
            // same one twice waits for the next hour.
            .filter(|why| {
                life.doing == Doing::Idle
                    && why.replans()
                    && (matches!(why, LifeStop::Interrupted(_)) || life.because.as_ref() != Some(why))
            });
        if new_hour || seen_change {
            if new_hour {
                // What failed last hour may not fail now.
                life.facts.retain(|name, _| !name.starts_with(UNREACHABLE) && !name.starts_with(ABSENT));
            }
            life.because = None;
            replan(&mut commands, &mut life, &book, &mut facts, &look, hour);
            life.planned_in = Some((clock.day, hour));
        } else if let Some(why) = failed {
            if let Some(fact) = why.fact() {
                life.facts.insert(fact, Value::Bool(true));
            }
            life.because = Some(why);
            replan(&mut commands, &mut life, &book, &mut facts, &look, hour);
        }
        if carry_out(&mut commands, &mut life, &book, &cycle, now, &specs, &mut facts, &look) == Carried::LostClaim {
            // Plan again at once, from what the body now sees; once a tick.
            life.because = life.stopped.clone();
            replan(&mut commands, &mut life, &book, &mut facts, &look, hour);
            let _ = carry_out(&mut commands, &mut life, &book, &cycle, now, &specs, &mut facts, &look);
        }
    }
}

/// Stop the plan of a body a conversation holds: it lets go of its claim and
/// its walk, and plans again when the conversation ends (`live_lives`).
///
/// Outside the gameplay gate: a conversation puts the game in dialogue, which
/// stops gated systems, so a gated reader would never see the conversation
/// open. `ActiveConversation` is the rollback authority it reads.
pub fn interrupt_talked_to_lives(
    mut commands: Commands,
    conversation: Option<Res<ambition_conversation::ActiveConversation>>,
    mut bodies: Query<(&mut Life, Option<&Visit>)>,
) {
    let Some(talker) = conversation.and_then(|conversation| conversation.talker()) else {
        return;
    };
    let Ok((mut life, visit)) = bodies.get_mut(talker) else {
        return;
    };
    if life.steps.is_empty() && life.claim.is_none() {
        return;
    }
    // The claim is the life's own field: other bodies read it from there on
    // their next look.
    life.claim = None;
    if visit.is_some() {
        commands.entity(talker).remove::<Visit>();
    }
    stop(&mut life, LifeStop::Interrupted(CONVERSATION.into()));
}

fn replan(commands: &mut Commands, life: &mut Life, book: &LifeBook, facts: &mut RoomFacts, look: &Look, hour: i64) {
    // What the last plan held and where it walked are not this plan's.
    facts.release(look.body, look.room, life);
    if look.visit.is_some() {
        commands.entity(look.body).remove::<Visit>();
    }
    life.facts.insert("hour".into(), Value::Int(hour));
    if let Some(spec) = look.spec {
        for (name, value) in &spec.authored {
            life.facts.insert(name.clone(), value.clone());
        }
        for (name, observation) in &spec.observes {
            life.facts.insert(name.clone(), facts.observe(look.body, look.room, observation));
        }
    }
    life.revision = life.revision.wrapping_add(1);
    let Some(domain) = book.domains.get(&life.domain) else {
        stop(life, LifeStop::NoDomain(life.domain.clone()));
        return;
    };
    match domain.plan(&life.facts, Budget::default()) {
        Ok(plan) => {
            life.steps = plan.steps;
            life.stopped = None;
            life.doing = if life.steps.is_empty() { Doing::Idle } else { Doing::Starting };
        }
        Err(PlanFailure::NoPlan(_)) => stop(life, LifeStop::NoPlan),
        Err(PlanFailure::OverBudget) => stop(life, LifeStop::OverBudget),
    }
}

fn stop(life: &mut Life, why: LifeStop) {
    life.steps.clear();
    life.doing = Doing::Idle;
    life.stopped = Some(why);
}

/// Make the first step done: its effects become the body's facts.
fn finish_step(life: &mut Life, book: &LifeBook) {
    if life.steps.is_empty() {
        return;
    }
    let step = life.steps.remove(0);
    if let Some(domain) = book.domains.get(&life.domain) {
        domain.apply_step(&step.task, &mut life.facts);
    }
    life.doing = if life.steps.is_empty() { Doing::Idle } else { Doing::Starting };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Carried {
    /// The body waits on its step, or its plan is spent or stopped.
    Waits,
    /// A claim was lost: the caller plans again.
    LostClaim,
}

#[allow(clippy::too_many_arguments)]
fn carry_out(
    commands: &mut Commands,
    life: &mut Life,
    book: &LifeBook,
    cycle: &ambition_time::DayCycle,
    now: f64,
    specs: &LiveRoomSpecs,
    facts: &mut RoomFacts,
    look: &Look,
) -> Carried {
    let body = look.body;
    // Each pass either waits (return) or finishes a step; a plan's steps are
    // bounded, so this ends.
    for _ in 0..=life.steps.len() {
        let Some(step) = life.steps.first() else {
            return Carried::Waits;
        };
        let target = step.action.target.clone().unwrap_or_default();
        match life.doing.clone() {
            Doing::Idle => return Carried::Waits,
            Doing::Starting if step.action.verb == GO_TO => {
                let Some(place) = specs.spec_of(body).and_then(|room| place_named(room, &target, look.here)) else {
                    stop(life, LifeStop::NoPlace(target));
                    return Carried::Waits;
                };
                if place == look.here {
                    finish_step(life, book);
                    continue;
                }
                commands.entity(body).insert(Visit::to(place));
                life.doing = Doing::GoingTo(place);
                return Carried::Waits;
            }
            Doing::Starting if step.action.verb == CLAIM => {
                let key = (look.room, target.clone());
                if facts.claims.get(&key).is_some_and(|holder| *holder != body) {
                    stop(life, LifeStop::Taken(target));
                    facts.release(body, look.room, life);
                    return Carried::LostClaim;
                }
                // One claim per body: a second lets the first go.
                facts.release(body, look.room, life);
                facts.claims.insert(key, body);
                life.claim = Some(target);
                finish_step(life, book);
            }
            Doing::Starting if step.action.verb == RELEASE => {
                if life.claim.as_deref() == Some(target.as_str()) {
                    facts.release(body, look.room, life);
                }
                finish_step(life, book);
            }
            Doing::Starting => {
                let hours = book.holds.get(&step.action.verb).copied().unwrap_or(0.0);
                if hours > 0.0 {
                    let until = now + f64::from(hours) / 24.0 * f64::from(cycle.day_length_s);
                    life.doing = Doing::Holding { until };
                    return Carried::Waits;
                }
                finish_step(life, book);
            }
            Doing::GoingTo(place) => {
                // The visit is inserted by command: until it lands, or for
                // another place, it says nothing about this step.
                match look.visit.filter(|visit| visit.place == place).map(|visit| visit.outcome) {
                    Some(VisitOutcome::Arrived) => {
                        commands.entity(body).remove::<Visit>();
                        finish_step(life, book);
                        // The rest of the plan starts on the next tick, when
                        // the removal has landed.
                        return Carried::Waits;
                    }
                    Some(VisitOutcome::NoRoute) => {
                        commands.entity(body).remove::<Visit>();
                        stop(life, LifeStop::NoRoute(target));
                        return Carried::Waits;
                    }
                    _ => return Carried::Waits,
                }
            }
            Doing::Holding { until } => {
                if now < until {
                    return Carried::Waits;
                }
                finish_step(life, book);
            }
        }
    }
    Carried::Waits
}
