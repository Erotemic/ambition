//! A provider's world source and its refusal: what the shared lifecycle runs
//! when the shell asks an experience to prepare, and how a world that cannot
//! be built fails that preparation.
//!
//! A world can be read off disk in a development build (`content_text!`), so
//! a bad save reaches a running game with no build in between. It fails the
//! session's world preparation and names its reasons; the game keeps running.

use ambition_platformer2d_runtime::PreparedPlatformerSource;

/// Why a provider cannot build its authored world now: each reason, for the
/// developer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldRefusal {
    pub reasons: Vec<String>,
}

impl WorldRefusal {
    pub fn new(reasons: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            reasons: reasons.into_iter().map(Into::into).collect(),
        }
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
