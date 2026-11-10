//! The planner: a fact snapshot in, a plan and its reasons out.
//!
//! Total-order decomposition with backtracking: the first task of the agenda
//! is expanded by its first method whose conditions hold; when the rest of
//! that expansion cannot be planned, the next method is tried. The planner
//! reads only the snapshot it is given and keeps no state, so the same domain
//! and facts give the same plan on every machine and every replay.

use std::collections::BTreeMap;

use crate::domain::{Action, Condition, Domain, Effect, Operand, Value};

/// What one NPC knows, by name.
pub type Facts = BTreeMap<String, Value>;

/// One method chosen on the way to a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    pub task: String,
    pub method: String,
}

/// One primitive task of the plan, with the methods that led to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub task: String,
    pub action: Action,
    pub chain: Vec<Chosen>,
}

/// A method (or a primitive task, `method: None`) the planner tried and set
/// aside, with the first condition that did not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub task: String,
    pub method: Option<String>,
    pub failed: Condition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<Step>,
    /// Every alternative set aside while planning, in the order tried.
    pub rejected: Vec<Rejection>,
    /// The facts the plan predicts once its steps are done.
    pub predicted: Facts,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanFailure {
    /// No decomposition of the root holds; what was tried is attached.
    NoPlan(Vec<Rejection>),
    /// The search expanded more tasks than its budget allows.
    OverBudget,
}

/// How far one plan may search. Bounds a recursive domain ("pause, then live
/// the day again") and a pathological one alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub expansions: usize,
    pub steps: usize,
    /// How deep one decomposition may nest. The search recurses once per
    /// expansion, so this also keeps a recursive domain off the stack's end.
    pub depth: usize,
}

impl Default for Budget {
    fn default() -> Self {
        Self { expansions: 4096, steps: 64, depth: 256 }
    }
}

fn read<'a>(facts: &'a Facts, operand: &'a Operand) -> Option<&'a Value> {
    match operand {
        Operand::Is(value) => Some(value),
        Operand::Fact(name) => facts.get(name),
    }
}

fn holds(condition: &Condition, facts: &Facts) -> bool {
    let pair = |key: &String, operand| facts.get(key).zip(read(facts, operand));
    match condition {
        Condition::Eq(key, operand) => pair(key, operand).is_some_and(|(left, right)| left == right),
        Condition::Ne(key, operand) => !pair(key, operand).is_some_and(|(left, right)| left == right),
        Condition::Lt(key, operand) => matches!(pair(key, operand), Some((Value::Int(left), Value::Int(right))) if left < right),
        Condition::Ge(key, operand) => matches!(pair(key, operand), Some((Value::Int(left), Value::Int(right))) if left >= right),
    }
}

fn first_failed<'a>(conditions: &'a [Condition], facts: &Facts) -> Option<&'a Condition> {
    conditions.iter().find(|condition| !holds(condition, facts))
}

fn apply(effects: &[Effect], facts: &mut Facts) {
    for effect in effects {
        match effect {
            Effect::Set(key, value) => {
                facts.insert(key.clone(), value.clone());
            }
            Effect::Add(key, delta) => {
                let next = match facts.get(key) {
                    Some(Value::Int(now)) => now + delta,
                    _ => *delta,
                };
                facts.insert(key.clone(), Value::Int(next));
            }
        }
    }
}

struct Search<'d> {
    domain: &'d Domain,
    budget: Budget,
    expansions: usize,
    rejected: Vec<Rejection>,
}

/// One entry of the agenda: a task and the methods that put it there.
type Pending = (String, Vec<Chosen>);

impl Search<'_> {
    fn seek(&mut self, agenda: &[Pending], facts: &Facts, steps: &mut Vec<Step>, depth: usize) -> Result<Option<Facts>, PlanFailure> {
        let Some(((task, chain), rest)) = agenda.split_first() else {
            return Ok(Some(facts.clone()));
        };
        self.expansions += 1;
        if self.expansions > self.budget.expansions || depth > self.budget.depth {
            return Err(PlanFailure::OverBudget);
        }
        if let Some(primitive) = self.domain.primitive.get(task) {
            if let Some(failed) = first_failed(&primitive.when, facts) {
                self.rejected.push(Rejection { task: task.clone(), method: None, failed: failed.clone() });
                return Ok(None);
            }
            if steps.len() >= self.budget.steps {
                return Ok(None);
            }
            let mut after = facts.clone();
            apply(&primitive.effects, &mut after);
            steps.push(Step { task: task.clone(), action: primitive.action.clone(), chain: chain.clone() });
            let found = self.seek(rest, &after, steps, depth + 1)?;
            if found.is_none() {
                steps.pop();
            }
            return Ok(found);
        }
        let methods = &self.domain.compound[task];
        for method in methods {
            if let Some(failed) = first_failed(&method.when, facts) {
                self.rejected.push(Rejection { task: task.clone(), method: Some(method.name.clone()), failed: failed.clone() });
                continue;
            }
            let mut through = chain.clone();
            through.push(Chosen { task: task.clone(), method: method.name.clone() });
            let mut expanded: Vec<Pending> = method.subtasks.iter().map(|subtask| (subtask.clone(), through.clone())).collect();
            expanded.extend_from_slice(rest);
            let mark = steps.len();
            if let Some(after) = self.seek(&expanded, facts, steps, depth + 1)? {
                return Ok(Some(after));
            }
            steps.truncate(mark);
        }
        Ok(None)
    }
}

impl Domain {
    /// Plan the root task from `facts`.
    pub fn plan(&self, facts: &Facts, budget: Budget) -> Result<Plan, PlanFailure> {
        let mut search = Search { domain: self, budget, expansions: 0, rejected: Vec::new() };
        let mut steps = Vec::new();
        match search.seek(&[(self.root.clone(), Vec::new())], facts, &mut steps, 0)? {
            Some(predicted) => Ok(Plan { steps, rejected: search.rejected, predicted }),
            None => Err(PlanFailure::NoPlan(search.rejected)),
        }
    }
}
