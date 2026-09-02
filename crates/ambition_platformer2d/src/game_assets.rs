//! Install the asset resources required by generic platformer presentation.
//!
//! [`PlatformerAssetsPlugin`] bridges provider/content catalogs to the decoded
//! [`GameAssets`] and asset catalog consumed by presentation. Add it after content
//! providers are registered and before [`PlatformerPresentationPlugin`](crate::presentation::PlatformerPresentationPlugin).
//! The umbrella crate owns this composition because the lower host/presentation layers
//! intentionally do not depend on the actor/content catalogs.

use bevy::prelude::*;

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_actor_monolith::assets::platformer_assets::Platformer2dAssetCatalog;
use ambition_boss_encounter::BossCatalog;
use ambition_platformer2d_world::world_manifest::WorldManifest;
use ambition_platformer2d_world::rooms::RoomMetadata;
use ambition_sprite_sheet::game_assets::{GameAssetConfig, GameAssets};

/// Install the shared asset resources the generic presentation reads.
///
/// See the module docs. Every field has a defensible default, so the common case
/// is one line naming the experience whose music catalog to fold in.
pub struct PlatformerAssetsPlugin {
    /// Whose App-local music catalog contributes its track ids to the asset
    /// catalog. This is the one thing that cannot be defaulted: an experience id
    /// is the key the audio registry is keyed by.
    experience: String,
    /// World rows for a game whose rooms come from `.ldtk` files. A game with
    /// procedural or code-authored rooms leaves this empty and every other
    /// catalog entry still lands — which is what both in-repo demos do.
    worlds: WorldManifest,
    /// The room whose metadata picks the block/biome art at Startup.
    ///
    /// Startup asset binding precedes gameplay activation, so this cannot be
    /// read from a session root that does not exist yet. A game passes its
    /// authored starting room; the default is the engine's own default theme.
    room: RoomMetadata,
    /// Publish the resolved SFX bank path as
    /// [`SfxBankAssetPath`](ambition_audio::SfxBankAssetPath) attributed to
    /// `experience`. Off for a game with no audio face installed.
    publish_sfx_bank: bool,
}

impl PlatformerAssetsPlugin {
    /// The common case: name the experience whose music catalog to fold in.
    pub fn for_experience(experience: impl Into<String>) -> Self {
        Self {
            experience: experience.into(),
            worlds: WorldManifest::default(),
            room: RoomMetadata::default(),
            publish_sfx_bank: true,
        }
    }

    /// World rows for a game whose rooms are authored in `.ldtk`.
    pub fn with_worlds(mut self, worlds: WorldManifest) -> Self {
        self.worlds = worlds;
        self
    }

    /// The room whose metadata selects the block/biome art bound at `Startup`.
    pub fn with_room(mut self, room: RoomMetadata) -> Self {
        self.room = room;
        self
    }

    /// Skip publishing `SfxBankAssetPath` — for a game that installs no audio
    /// face, or binds the bank itself.
    pub fn without_sfx_bank(mut self) -> Self {
        self.publish_sfx_bank = false;
        self
    }
}

/// The room theme the `Startup` loader binds art for.
#[derive(Resource, Clone)]
struct AssetBindRoom(RoomMetadata);

impl Plugin for PlatformerAssetsPlugin {
    fn build(&self, app: &mut App) {
        let config = app
            .world()
            .get_resource::<GameAssetConfig>()
            .cloned()
            .unwrap_or_else(GameAssetConfig::from_args);

        // The catalogs the content plugins registered.
        let character_catalog = app
            .world()
            .get_resource::<CharacterCatalog>()
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "PlatformerAssetsPlugin found no CharacterCatalog. Add it AFTER the \
                     content/provider plugins that register one — before them there is \
                     nothing to build an asset catalog from, and the quiet result is a \
                     game that draws coloured rectangles."
                )
            });
        // REQUIRED, not optional (`engine.character-authority-is-app-local`):
        // silently substituting an empty catalog is how a game ships with its
        // bosses drawn as the fallback body and nobody notices.
        let boss_catalog = app.world().get_resource::<BossCatalog>().cloned().unwrap_or_else(|| {
            panic!(
                "PlatformerAssetsPlugin found no BossCatalog. It is App-local authority                  the content plugins install, so this means the same composition-order                  mistake as a missing CharacterCatalog."
            )
        });
        // A game with no music catalog gets an empty registry rather than a
        // panic: silent is a legitimate choice (the versus stage declares it),
        // and every other catalog entry still lands.
        let music = app
            .world()
            .get_resource::<ambition_audio::catalog::AudioCatalogRegistry>()
            .and_then(|registry| registry.music_for(&self.experience).cloned())
            .unwrap_or(ambition_audio::spec::MusicRegistry {
                default_track: String::new(),
                tracks: Vec::new(),
            });

        let catalog = ambition_platformer2d_actor_monolith::assets::platformer_assets::build_platformer2d_asset_catalog(
            &config,
            &character_catalog,
            &boss_catalog,
            &music,
            &self.worlds,
        );

        #[cfg(feature = "audio")]
        if self.publish_sfx_bank {
            if let Some(path) =
                catalog.path_for(&ambition_asset_manager::platformer_assets::ids::sfx_bank())
            {
                app.insert_resource(ambition_audio::SfxBankAssetPath::new(
                    self.experience.clone(),
                    path,
                ));
            }
        }
        #[cfg(not(feature = "audio"))]
        let _ = self.publish_sfx_bank;

        app.insert_resource(config);
        app.insert_resource(catalog);
        app.insert_resource(AssetBindRoom(self.room.clone()));
        // ⛔⛔ THE PLUGIN REGISTERS WHAT ITS OWN SYSTEM CONSUMES, GUARDED.
        // `bind_game_assets` takes `ResMut<Assets<TextureAtlasLayout>>`, and a
        // composition that never registered it panics on `Startup` instead of at
        // `build`. That is not hypothetical: `-p ambition_demo_smash_app
        // --features visible` hand-assembles its app rather than going through
        // `app.rs`'s composition road, so nothing supplied the atlas store and the
        // demo could be `cargo check`ed but never RUN (B3).
        //
        // ⛔ AND THE GUARD IS NOT OPTIONAL. `init_asset` is NOT idempotent — read
        // it: bevy_asset 0.19.1 `lib.rs:639` ends in an unconditional
        // `insert_resource(assets)`, so calling it where a store already exists
        // REPLACES it and drops every handle already registered, then re-adds its
        // systems and messages on top. `app.rs:1298` gets away with a bare call
        // only because it is behind `if !windowed`, which is a proxy for this
        // question; ask the question directly.
        if !app
            .world()
            .contains_resource::<Assets<TextureAtlasLayout>>()
        {
            app.init_asset::<TextureAtlasLayout>();
        }
        app.init_resource::<GameAssets>();
        app.add_systems(
            Startup,
            bind_game_assets.before(crate::presentation::PlatformerPresentationSetupSet),
        );
    }
}

/// Decode the sheets the presentation chain draws from.
///
/// `before(PlatformerPresentationSetupSet)` is load-bearing: the room's static
/// visuals are spawned in that set and read `GameAssets` as they go, so binding
/// afterwards produces one frame of placeholder art that never refreshes.
#[allow(clippy::too_many_arguments)]
fn bind_game_assets(
    config: Res<GameAssetConfig>,
    character_catalog: Res<CharacterCatalog>,
    authored_sheets: Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    boss_catalog: Res<BossCatalog>,
    catalog: Res<Platformer2dAssetCatalog>,
    room: Res<AssetBindRoom>,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    quality: Option<Res<ambition_render::quality::ResolvedVisualQuality>>,
    mut game_assets: ResMut<GameAssets>,
) {
    *game_assets = ambition_platformer2d_actor_monolith::assets::game_assets::load_game_assets(
        &config,
        &character_catalog,
        &authored_sheets,
        &boss_catalog,
        &catalog,
        &asset_server,
        &mut layouts,
        &room.0,
        quality.as_deref().map(|q| &q.budget),
    );
}
