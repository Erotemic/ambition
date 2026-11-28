//! What a running session's world is read from when its file changes. Each
//! world provider states its file and how to read it, so a developer reload
//! reads the world of the session that is running, whichever game it is.

use ambition_platformer2d_runtime::PreparedPlatformerSource;
use bevy::prelude::{Resource, World};

/// Read a world provider's files at `paths` (in the order it declared them)
/// into a replacement of `active`'s world: its rooms, and for a format that installs one, its index
/// ([`PreparedPlatformerSource::with_installed_ldtk_index`]). Every other fact
/// of `active` is kept ([`PreparedPlatformerSource::with_world`]). Which room
/// the set activates into does not matter: the reload keeps the player's room.
pub type WorldReader =
    fn(&World, &[std::path::PathBuf], &PreparedPlatformerSource) -> Result<PreparedPlatformerSource, Vec<String>>;

/// One world provider's source.
#[derive(Clone, Debug)]
pub struct WorldReloadSource {
    /// The files a development build reads the world from, in the order the
    /// reader takes them; none when the build embedded them, so there is
    /// nothing to watch.
    pub files: Vec<std::path::PathBuf>,
    pub read: WorldReader,
}

/// Each world provider's [`WorldReloadSource`], by the provider id its
/// sessions name (`PlatformerSessionCatalogs::world_provider`).
#[derive(Resource, Default, Clone, Debug)]
pub struct WorldReloadSources(Vec<(String, WorldReloadSource)>);

impl WorldReloadSources {
    /// State `world_provider`'s source, replacing an earlier one.
    pub fn declare(&mut self, world_provider: impl Into<String>, source: WorldReloadSource) {
        let world_provider = world_provider.into();
        match self.0.iter_mut().find(|(provider, _)| *provider == world_provider) {
            Some((_, held)) => *held = source,
            None => self.0.push((world_provider, source)),
        }
    }

    /// `world_provider`'s source, when it stated one.
    pub fn of(&self, world_provider: &str) -> Option<&WorldReloadSource> {
        self.0.iter().find(|(provider, _)| provider == world_provider).map(|(_, source)| source)
    }
}

/// State `world_provider`'s source on `app`.
pub fn declare_world_reload_source(
    app: &mut bevy::prelude::App,
    world_provider: impl Into<String>,
    source: WorldReloadSource,
) {
    app.world_mut()
        .get_resource_or_insert_with(WorldReloadSources::default)
        .declare(world_provider, source);
}
