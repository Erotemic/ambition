//! A life: a body that follows an authored plan (RADIANT-NPC-HTN).
//!
//! The executor of `docs/planning/engine/radiant-npc-planning.md`. A body
//! whose character has a row in the game's [`LifeBook`] gets a [`Life`]: its
//! domain's name and its own facts. On each new hour of the day the executor
//! plans from those facts (the planner, `ambition_htn`, reads nothing else),
//! then carries the plan's steps out one at a time:
//!
//! - `go_to <place>` is a [`Visit`] to the place named in the body's room, and
//!   is done when the visit arrives. The room's own name is the room: a body
//!   already in it is there.
//! - any other verb holds the body at its task for the hours the book says
//!   (`holds`), or is done the tick it starts.
//!
//! When a step is done its effects become the body's facts, so the next plan
//! starts from what the body did, not from what it planned. A step that
//! cannot be carried out (a place the room does not name, a place no route
//! gets to) stops the plan with the reason, and the body waits for the next
//! hour to plan again.
//!
//! Rollback state: the plan, how far it is, and the facts. The planner is
//! not: it runs on the tick a plan is asked for, from the restored facts, so
//! a replay plans the same steps.

use std::collections::BTreeMap;
use std::sync::Arc;

use ambition_characters::actor::WornCharacter;
use ambition_htn::{Budget, Domain, Facts, PlanFailure, Step};
use ambition_platformer2d_core as ae;
use ambition_platformer2d_world::rooms::{LiveRoomSpecs, RoomSpec};
use bevy::prelude::{Commands, Component, Entity, Query, Res, Resource, Without};

use super::visit::{Visit, VisitOutcome};

/// The planner's fact types, so a reader of a [`Life`] needs no second crate.
pub use ambition_htn::Value;

/// The verb that sends a body to a place.
pub const GO_TO: &str = "go_to";

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
    pub facts: Facts,
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
    /// How many plans this body has made.
    pub revision: u32,
    /// Why the last plan stopped short of its end, for an inspector and a
    /// line of dialogue. `None` while it runs, and once it ends.
    pub stopped: Option<LifeStop>,
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
}

impl Life {
    pub fn new(spec: &LifeSpec) -> Self {
        Self {
            domain: spec.domain.clone(),
            facts: spec.facts.clone(),
            planned_in: None,
            steps: Vec::new(),
            doing: Doing::Idle,
            revision: 0,
            stopped: None,
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

/// Plan on each new hour, and carry the plan out one step at a time.
#[allow(clippy::type_complexity)]
pub fn live_lives(
    mut commands: Commands,
    book: Res<LifeBook>,
    clock: Res<ambition_time::DayClock>,
    cycle: Option<Res<ambition_time::DayCycle>>,
    specs: LiveRoomSpecs,
    mut bodies: Query<(Entity, &mut Life, &ae::BodyKinematics, Option<&Visit>)>,
) {
    // No day, no hours to live by.
    let Some(cycle) = cycle else { return };
    if clock.day == 0 || cycle.day_length_s <= 0.0 {
        return;
    }
    let hour = clock.hour(&cycle).floor() as i64;
    let now = f64::from(clock.day) * f64::from(cycle.day_length_s) + f64::from(clock.seconds);
    for (body, mut life, kinematics, visit) in &mut bodies {
        if life.planned_in != Some((clock.day, hour)) {
            plan(&mut life, &book, hour);
            life.planned_in = Some((clock.day, hour));
            // A walk the last plan began is not this plan's.
            if visit.is_some() {
                commands.entity(body).remove::<Visit>();
            }
            if life.doing == Doing::Starting {
                carry_out(&mut commands, body, &mut life, &book, &cycle, now, None, &specs, kinematics.pos);
            }
            continue;
        }
        carry_out(&mut commands, body, &mut life, &book, &cycle, now, visit, &specs, kinematics.pos);
    }
}

fn plan(life: &mut Life, book: &LifeBook, hour: i64) {
    life.facts.insert("hour".into(), Value::Int(hour));
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

#[allow(clippy::too_many_arguments)]
fn carry_out(
    commands: &mut Commands,
    body: Entity,
    life: &mut Life,
    book: &LifeBook,
    cycle: &ambition_time::DayCycle,
    now: f64,
    visit: Option<&Visit>,
    specs: &LiveRoomSpecs,
    here: ae::Vec2,
) {
    // Each pass either waits (return) or finishes a step; a plan's steps are
    // bounded, so this ends.
    for _ in 0..=life.steps.len() {
        let Some(step) = life.steps.first() else {
            return;
        };
        match life.doing.clone() {
            Doing::Idle => return,
            Doing::Starting if step.action.verb == GO_TO => {
                let target = step.action.target.clone().unwrap_or_default();
                let Some(place) = specs.spec_of(body).and_then(|room| place_named(room, &target, here)) else {
                    stop(life, LifeStop::NoPlace(target));
                    return;
                };
                if place == here {
                    finish_step(life, book);
                    continue;
                }
                commands.entity(body).insert(Visit::to(place));
                life.doing = Doing::GoingTo(place);
                return;
            }
            Doing::Starting => {
                let hours = book.holds.get(&step.action.verb).copied().unwrap_or(0.0);
                if hours > 0.0 {
                    let until = now + f64::from(hours) / 24.0 * f64::from(cycle.day_length_s);
                    life.doing = Doing::Holding { until };
                    return;
                }
                finish_step(life, book);
            }
            Doing::GoingTo(place) => {
                // The visit is inserted by command: until it lands, or for
                // another place, it says nothing about this step.
                match visit.filter(|visit| visit.place == place).map(|visit| visit.outcome) {
                    Some(VisitOutcome::Arrived) => {
                        commands.entity(body).remove::<Visit>();
                        finish_step(life, book);
                        // The rest of the plan starts on the next tick, when
                        // the removal has landed.
                        return;
                    }
                    Some(VisitOutcome::NoRoute) => {
                        let target = step.action.target.clone().unwrap_or_default();
                        commands.entity(body).remove::<Visit>();
                        stop(life, LifeStop::NoRoute(target));
                        return;
                    }
                    _ => return,
                }
            }
            Doing::Holding { until } => {
                if now < until {
                    return;
                }
                finish_step(life, book);
            }
        }
    }
}
