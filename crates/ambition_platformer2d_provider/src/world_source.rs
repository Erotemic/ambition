//! A provider's world source and its refusal: what the shared lifecycle runs
//! when the shell asks an experience to prepare, and how a world that cannot
//! be built fails that preparation.
//!
//! A world can be read off disk in a development build (`content_text!`), so
//! a bad save reaches a running game with no build in between. It fails the
//! session's world preparation and names its reasons; the game keeps running.

use bevy::prelude::*;

use ambition_game_shell::{ProviderLoadTransaction, ShellEvent, PREPARE_WORLD_WORK_ID};
use ambition_platformer2d_runtime::PreparedPlatformerSource;

use crate::lifecycle::PlatformerPreparation;

/// Why a provider cannot build its authored world now: each reason, for the
/// developer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldRefusal {
    pub reasons: Vec<String>,
}

impl WorldRefusal {
    pub fn new(reasons: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self { reasons: reasons.into_iter().map(Into::into).collect() }
    }
}

/// What a provider's world source gives the shared lifecycle: the authored
/// world, or a [`WorldRefusal`]. A source that cannot fail returns the world
/// alone.
pub trait IntoPreparedSource: Send + Sync + 'static {
    fn into_prepared_source(self) -> Result<PreparedPlatformerSource, WorldRefusal>;
}

impl IntoPreparedSource for PreparedPlatformerSource {
    fn into_prepared_source(self) -> Result<PreparedPlatformerSource, WorldRefusal> {
        Ok(self)
    }
}

impl IntoPreparedSource for Result<PreparedPlatformerSource, WorldRefusal> {
    fn into_prepared_source(self) -> Result<PreparedPlatformerSource, WorldRefusal> {
        self
    }
}

/// True when the shell requested preparation for `experience_id` this frame —
/// the run gate that keeps a provider's session-world source from building
/// worlds on frames nobody asked for.
pub(crate) fn preparation_requested(
    experience_id: String,
) -> impl FnMut(MessageReader<ShellEvent>) -> bool {
    move |mut events: MessageReader<ShellEvent>| {
        events.read().any(|event| {
            matches!(
                event,
                ShellEvent::PreparationRequested(transaction)
                    if transaction.experience_id.as_str() == experience_id
            )
        })
    }
}

/// The shared preparation system body: piped after a provider's session-world
/// source (tagged with its experience id), it gives every matching transaction
/// an owned copy of that authored world value, or fails its world preparation
/// with the source's refusal.
pub(crate) fn prepare_requested_sessions(
    In((experience_id, source)): In<(String, Result<PreparedPlatformerSource, WorldRefusal>)>,
    mut events: MessageReader<ShellEvent>,
    mut preparation: PlatformerPreparation,
) {
    for event in events.read() {
        let ShellEvent::PreparationRequested(transaction) = event else {
            continue;
        };
        if transaction.experience_id.as_str() != experience_id {
            continue;
        }
        match &source {
            Ok(source) => {
                preparation.prepare(transaction, source.clone());
            }
            Err(refusal) => preparation.refuse_world(transaction, refusal),
        }
    }
}

impl PlatformerPreparation<'_> {
    /// Fail `transaction`'s world preparation: the provider could not build
    /// its world. Retryable, because the usual cause is a file the developer
    /// can save again.
    fn refuse_world(&mut self, transaction: &ProviderLoadTransaction, refusal: &WorldRefusal) {
        self.fail(
            transaction,
            PREPARE_WORLD_WORK_ID,
            ambition_load::LoadFailure::new("This world could not be built.", refusal.reasons.join("\n"))
                .retryable(true),
        );
    }
}
