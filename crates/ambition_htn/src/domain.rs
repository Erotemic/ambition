//! An authored domain: what an NPC can do, and the ways to do it.
//!
//! A domain is data (RON). It names compound tasks, each with methods tried in
//! order, and primitive tasks, each with preconditions, effects on the facts,
//! and the action the executor carries out. It is validated once, when read,
//! so the planner never meets an unknown task.

use std::collections::{BTreeMap, BTreeSet};

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

/// The right side of a comparison: a constant, or another fact (an NPC's own
/// `arrive_hour`, so two NPCs share a domain and keep their own schedules).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum Operand {
    Is(Value),
    Fact(String),
}

/// A precondition. A fact that is absent equals nothing and orders with
/// nothing: `Eq` and the orderings are false, `Ne` is true.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum Condition {
    Eq(String, Operand),
    Ne(String, Operand),
    Lt(String, Operand),
    Ge(String, Operand),
}

/// What a primitive task does to the facts, as the planner predicts it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub enum Effect {
    Set(String, Value),
    Add(String, i64),
}

/// What the executor is asked to do: a verb it knows (`go_to`, `use`, `say`,
/// `wait_until`) and its target, both opaque to the planner.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub verb: String,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Method {
    /// Shown in an explanation ("why are you here?").
    pub name: String,
    #[serde(default)]
    pub when: Vec<Condition>,
    /// Task names, compound or primitive, in order.
    pub subtasks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Primitive {
    #[serde(default)]
    pub when: Vec<Condition>,
    #[serde(default)]
    pub effects: Vec<Effect>,
    pub action: Action,
}

/// An authored domain, validated.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Domain {
    /// The task an NPC with no plan decomposes.
    pub root: String,
    #[serde(default)]
    pub compound: BTreeMap<String, Vec<Method>>,
    #[serde(default)]
    pub primitive: BTreeMap<String, Primitive>,
}

/// Why a domain was not admitted. Each names the task it is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    Parse(String),
    UnknownRoot(String),
    /// A task is both compound and primitive.
    TwoKinds(String),
    NoMethods(String),
    /// (task, method, the subtask no task declares)
    UnknownSubtask(String, String, String),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => write!(f, "the domain does not read: {error}"),
            Self::UnknownRoot(root) => write!(f, "the root task `{root}` is not declared"),
            Self::TwoKinds(task) => write!(f, "`{task}` is declared both compound and primitive"),
            Self::NoMethods(task) => write!(f, "the compound task `{task}` has no method"),
            Self::UnknownSubtask(task, method, subtask) => {
                write!(f, "`{task}` / \"{method}\" names `{subtask}`, which no task declares")
            }
        }
    }
}

impl std::error::Error for DomainError {}

impl Domain {
    /// Read and validate a domain.
    pub fn from_ron(text: &str) -> Result<Self, DomainError> {
        let domain: Self = ron::from_str(text).map_err(|error| DomainError::Parse(error.to_string()))?;
        domain.validate()?;
        Ok(domain)
    }

    fn declares(&self, task: &str) -> bool {
        self.compound.contains_key(task) || self.primitive.contains_key(task)
    }

    /// Check a domain built some other way than [`Self::from_ron`] (one
    /// deserialized inside a larger file).
    pub fn validate(&self) -> Result<(), DomainError> {
        if !self.declares(&self.root) {
            return Err(DomainError::UnknownRoot(self.root.clone()));
        }
        let compound: BTreeSet<&String> = self.compound.keys().collect();
        if let Some(task) = self.primitive.keys().find(|task| compound.contains(task)) {
            return Err(DomainError::TwoKinds(task.clone()));
        }
        for (task, methods) in &self.compound {
            if methods.is_empty() {
                return Err(DomainError::NoMethods(task.clone()));
            }
            for method in methods {
                if let Some(subtask) = method.subtasks.iter().find(|subtask| !self.declares(subtask)) {
                    return Err(DomainError::UnknownSubtask(task.clone(), method.name.clone(), subtask.clone()));
                }
            }
        }
        Ok(())
    }
}
