//! A hierarchical task network planner with no engine types.
//!
//! An authored [`Domain`] (RON) and a snapshot of one NPC's [`Facts`] go in; a
//! [`Plan`] of primitive tasks comes out, each with the method chain that chose
//! it and the alternatives set aside, so an inspector or a line of dialogue can
//! say why. The planner reads nothing else and keeps no state: the executor
//! that turns a step's [`Action`] into a goal on a body owns everything that
//! is simulation state. Design: `docs/planning/engine/radiant-npc-planning.md`.

mod domain;
mod plan;

pub use domain::{Action, Condition, Domain, DomainError, Effect, Method, Operand, Primitive, Value};
pub use plan::{Budget, Chosen, Facts, Plan, PlanFailure, Rejection, Step};
