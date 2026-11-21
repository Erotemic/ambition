//! What makes a runner one conversation's among several (SIMULATION-ISLANDS N2).
//!
//! - [`ConversationStorage`]: the runners share one Yarn variable store (a
//!   `<<set>>` in one talk is read in every other), but who is talking to whom
//!   (`$speaker_id`, `$listener_id`, `$speaker_is_self`) is each runner's own.
//! - [`attribute_vocabulary`]: each command and function a runner calls names
//!   that runner as the [`SpeakingRunner`] while it runs, so a command acts on
//!   its own conversation without the runner passing it.

use std::any::{Any, TypeId};
use std::fmt;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy_yarnspinner::prelude::*;
use bevy_yarnspinner::{TaskFinishedIndicator, UntypedYarnCommand, UntypedYarnFn};
use yarnspinner::runtime::VariableStorageError;

use crate::slots::SpeakingRunner;

/// The variables that describe one conversation's participants. Only the
/// bridge writes them (`publish_dialogue_context`).
const CONVERSATION_VARIABLES: [&str; 3] = ["$speaker_id", "$listener_id", "$speaker_is_self"];

fn is_conversations_own(name: &str) -> bool {
    CONVERSATION_VARIABLES.contains(&name)
}

/// One runner's view of the shared Yarn variables: the shared store, with the
/// conversation's own participants kept apart.
#[derive(Debug)]
pub(super) struct ConversationStorage {
    shared: Box<dyn VariableStorage>,
    own: HashMap<String, YarnValue>,
    /// Made beside a running one: the program's initial values, which a new
    /// runner writes into its store, fill only what the shared store lacks,
    /// so a second conversation does not reset the first one's `<<set>>`s.
    beside: bool,
}

impl ConversationStorage {
    /// The main runner's: a new shared store.
    pub(super) fn main() -> Self {
        Self { shared: Box::new(bevy_yarnspinner::default_impl::MemoryVariableStorage::new()), own: HashMap::default(), beside: false }
    }

    /// A runner beside the main one, on its shared store.
    pub(super) fn beside(shared: Box<dyn VariableStorage>) -> Self {
        Self { shared, own: HashMap::default(), beside: true }
    }

    /// The shared store under a runner's storage, when it is one of these.
    pub(super) fn shared_of(storage: &dyn VariableStorage) -> Option<Box<dyn VariableStorage>> {
        storage.as_any().downcast_ref::<Self>().map(|storage| storage.shared.clone_shallow())
    }
}

impl VariableStorage for ConversationStorage {
    fn clone_shallow(&self) -> Box<dyn VariableStorage> {
        Box::new(Self { shared: self.shared.clone_shallow(), own: self.own.clone(), beside: self.beside })
    }

    fn set(&mut self, name: String, value: YarnValue) -> Result<(), VariableStorageError> {
        if is_conversations_own(&name) {
            self.own.insert(name, value);
            return Ok(());
        }
        self.shared.set(name, value)
    }

    fn get(&self, name: &str) -> Result<YarnValue, VariableStorageError> {
        match self.own.get(name) {
            Some(value) => Ok(value.clone()),
            // Not yet published: the shared store says so in its own words.
            None => self.shared.get(name),
        }
    }

    fn extend(&mut self, values: HashMap<String, YarnValue>) -> Result<(), VariableStorageError> {
        let mut shared = HashMap::default();
        for (name, value) in values {
            if is_conversations_own(&name) {
                self.own.insert(name, value);
            } else if !self.beside || !self.shared.contains(&name) {
                shared.insert(name, value);
            }
        }
        VariableStorage::extend(&mut *self.shared, shared)
    }

    fn variables(&self) -> HashMap<String, YarnValue> {
        let mut all = self.shared.variables();
        all.extend(self.own.clone());
        all
    }

    fn clear(&mut self) {
        self.own.clear();
        self.shared.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Wrap every command and function `runner` holds so each names `entity` as
/// the [`SpeakingRunner`] while it runs.
pub(super) fn attribute_vocabulary(runner: &mut DialogueRunner, entity: Entity) {
    let commands = std::mem::replace(runner.commands_mut(), YarnCommands::new());
    runner.commands_mut().extend(commands.into_iter().map(|(name, command)| {
        (name, Box::new(RunnerCommand { runner: entity, inner: command }) as Box<dyn UntypedYarnCommand>)
    }));
    let library = std::mem::take(runner.library_mut());
    runner.library_mut().extend(library.into_iter().map(|(name, function)| {
        (name, Box::new(RunnerFunction { runner: entity, inner: function }) as Box<dyn UntypedYarnFn>)
    }));
}

/// Run `call` with `runner` as the [`SpeakingRunner`], and put back the one
/// that was speaking before (a command can run inside another's call).
fn speaking_as<T>(world: &mut World, runner: Entity, call: impl FnOnce(&mut World) -> T) -> T {
    let before = world.get_resource::<SpeakingRunner>().copied().unwrap_or_default();
    world.insert_resource(SpeakingRunner(Some(runner)));
    let out = call(world);
    world.insert_resource(before);
    out
}

#[derive(Debug)]
struct RunnerCommand {
    runner: Entity,
    inner: Box<dyn UntypedYarnCommand>,
}

impl UntypedYarnCommand for RunnerCommand {
    fn call(&mut self, input: Vec<YarnValue>, world: &mut World) -> Box<dyn TaskFinishedIndicator> {
        let inner = &mut self.inner;
        speaking_as(world, self.runner, |world| inner.call(input, world))
    }

    fn clone_box(&self) -> Box<dyn UntypedYarnCommand> {
        Box::new(Self { runner: self.runner, inner: self.inner.clone_box() })
    }
}

#[derive(Debug)]
struct RunnerFunction {
    runner: Entity,
    inner: Box<dyn UntypedYarnFn>,
}

impl fmt::Display for RunnerFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl UntypedYarnFn for RunnerFunction {
    fn call(&self, input: Vec<YarnValue>) -> YarnValue {
        self.inner.call(input)
    }

    fn call_with_world(&self, input: Vec<YarnValue>, world: &mut World) -> YarnValue {
        speaking_as(world, self.runner, |world| self.inner.call_with_world(input, world))
    }

    fn clone_box(&self) -> Box<dyn UntypedYarnFn> {
        Box::new(Self { runner: self.runner, inner: self.inner.clone_box() })
    }

    fn parameter_types(&self) -> Vec<TypeId> {
        self.inner.parameter_types()
    }

    fn return_type(&self) -> TypeId {
        self.inner.return_type()
    }
}
