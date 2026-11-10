//! The contract between an NPC life's executor and its planner
//! (RADIANT-NPC-HTN, `docs/planning/engine/radiant-npc-planning.md`).
//!
//! An executor (the engine's `body::life`) keeps a body's [`Facts`], asks a
//! [`LifePlanner`] for the steps to take, carries out each step's [`Action`]
//! with the body, and tells the planner which step was done so it can apply
//! the step's effects. It never names the planning algorithm: an HTN
//! (`ambition_htn`) is one planner, a utility scorer could be another. The
//! planner reads only the facts it is given and keeps no state, so a replay
//! plans the same steps.

use std::collections::BTreeMap;
use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// A fact's value. Facts are what the executor knows about one NPC's world:
/// the hour, where it is, whether the bench is free.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Bool(bool),
    Int(i64),
    /// A name: a place, a person, a mood.
    Sym(String),
}

/// What one NPC knows, by name.
pub type Facts = BTreeMap<String, Value>;

/// What the executor is asked to do: a verb it knows (`go_to`, `use`, `say`)
/// and its target, both opaque to the planner.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub verb: String,
    #[serde(default)]
    pub target: Option<String>,
}

/// One step of a plan: the planner's name for it (handed back when it is
/// done), the action, and why the planner chose it, outermost first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub task: String,
    pub action: Action,
    pub because: Vec<String>,
}

/// Why a planner gave no steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Nothing the planner knows holds for these facts; what it tried, in
    /// its own words.
    NoPlan(Vec<String>),
    /// It searched as far as it may.
    OverBudget,
}

/// A planning policy for NPC lives.
pub trait LifePlanner: Debug + Send + Sync {
    /// The steps to take from `facts`.
    fn plan(&self, facts: &Facts) -> Result<Vec<Step>, Refusal>;

    /// `task` was carried out: make its effects true in `facts`.
    fn step_done(&self, task: &str, facts: &mut Facts);
}
