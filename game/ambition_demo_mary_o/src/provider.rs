//! The Mary-O experience provider.

use bevy::prelude::*;

use ambition::engine_core as ae;
use ambition::presentation::profiles;
use ambition::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition::runtime::PreparedPlatformerSource;

use crate::{level_1_1, MaryORulesPlugin, LEVEL_1_1_ROOM_ID};

pub const MARY_O_EXPERIENCE: &str = "mary_o";
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
pub const MARY_O_CHARACTER_ID: &str = "mary_o";
pub const MARY_O_MUSIC_TRACK: &str = "support_theme";
pub const MARY_O_MUSIC_ASSET_PATH: &str = "audio/music/generated/support_theme/full.ogg";

#[derive(Clone)]
pub struct MaryOSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

pub fn mary_o_session_world() -> MaryOSessionWorld {
    let room = level_1_1();
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    let room_set = RoomSet::from_parts(LEVEL_1_1_ROOM_ID, vec![room], Vec::new());
    MaryOSessionWorld {
        geometry,
        room_set,
        metadata,
        starting_character: StartingCharacter::new(MARY_O_CHARACTER_ID),
    }
}

pub fn mary_o_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(MARY_O_CHARACTER_ID, MARY_O_EXPERIENCE)
}

pub struct MaryOExperiencePlugin;

impl Plugin for MaryOExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_mary_o_content(app);
        // Declare the milk-carton pickup art (pure id → path + size DATA; no render
        // dependency here). The render layer resolves it into a real sprite through
        // the shared `WorldItem` art seam, so the ?-block's milk draws as a carton
        // instead of the cream placeholder quad in BOTH the standalone app and the
        // multi-game host — this provider is the one seam both share. The flat prop
        // image is published by regen_sprites.sh; until then the render falls back
        // to the quad.
        {
            use ambition::platformer::world_item_art::{WorldItemArtAppExt, WorldItemArtEntry};
            app.register_world_item_art([
                WorldItemArtEntry::new(
                    crate::powerups::MILK_SPRITE,
                    format!("sprites/props/{}.png", crate::powerups::MILK_SPRITE),
                    ae::Vec2::new(24.0, 28.0),
                ),
                // The second rung of the chain. Same seam, same fallback: until
                // the prop image is published the render draws the row-tinted
                // quad, so the pickup is always visible.
                WorldItemArtEntry::new(
                    crate::powerups::BLOSSOM_SPRITE,
                    format!("sprites/props/{}.png", crate::powerups::BLOSSOM_SPRITE),
                    ae::Vec2::new(24.0, 24.0),
                ),
            ]);
        }
        {
            // The spark's LOOK, registered as content under the id her ranged
            // action authors. One registration, zero render edits — and because
            // the id lives on the action, the projectile domain never learns what
            // a spark is.
            use ambition::projectiles::visual::{
                ProjectileArt, ProjectileArtSource, ProjectileRenderSize, ProjectileRotation,
                ProjectileVisualAppExt,
            };
            app.register_projectile_visual(
                crate::powerups::SPARK_VISUAL,
                ProjectileArt {
                    source: ProjectileArtSource::EnergyTinted {
                        rgba: [1.0, 0.62, 0.16, 0.96],
                    },
                    size: ProjectileRenderSize::Body {
                        min: 7.0,
                        scale: 1.0,
                    },
                    // It tumbles as it skips rather than pointing along travel —
                    // a spinning ember, not an arrow.
                    rotation: ProjectileRotation::GravityUpright,
                    debug_tint: [1.0, 0.62, 0.16, 1.0],
                    label: "spark".to_string(),
                    expiry_vfx: None,
                },
            );
        }
        {
            use ambition::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
            app.register_audio_catalog_fragment(
                AudioCatalogFragment::new(
                    MARY_O_EXPERIENCE,
                    // Mary-O runs on the "Support Theme" cue. Declaring it in the
                    // provider fragment is what authorizes the session to select
                    // and play it under provider-relative audio.
                    Some(ambition::audio::spec::MusicRegistry {
                        default_track: MARY_O_MUSIC_TRACK.to_string(),
                        tracks: vec![ambition::audio::spec::MusicTrack {
                            id: MARY_O_MUSIC_TRACK.to_string(),
                            display_name: "Support Theme".to_string(),
                            asset_path: Some(MARY_O_MUSIC_ASSET_PATH.to_string()),
                        }],
                    }),
                    // Mary-O AUTHORS the cues she emits. The movement kernel writes
                    // `SfxMessage::Jump` on every jump, but under provider-relative
                    // audio a session only plays cues its fragment declares — an
                    // undeclared `player.jump` is gated to silence. Declaring the
                    // Jump cue (the classic run+jump grammar's one voice) is what
                    // makes her jump audible. Procedurally synthesized from this
                    // spec; no asset file needed.
                    Some(ambition::audio::spec::SfxRegistry {
                        sample_rate: 44_100,
                        sfx: vec![ambition::audio::spec::SfxSpec {
                            cue: Some(ambition::audio::spec::SoundCueKey::Jump),
                            id: None,
                            waveform: ambition::audio::spec::WaveformSpec::Sine,
                            frequency: 460.0,
                            frequency_end: 720.0,
                            duration: 0.085,
                            volume: 0.22,
                            attack: 0.003,
                            release: 0.045,
                            noise: 0.0,
                        }],
                    }),
                )
                .expect("Mary-O audio catalog is valid"),
            );
        }
        PlatformerExperienceAuthoring::new(
            MARY_O_EXPERIENCE,
            MARY_O_GAMEPLAY_ROUTE,
            "Mary-O",
            "Level 1-1: run, jump, grab the flag",
            "Prepare Mary-O",
            mary_o_authored_catalogs(),
        )
        // A fixed 4:3 gameplay rectangle everywhere; the surround belongs to
        // HUD and controls rather than to the level.
        .with_presentation_profiles(profiles::fixed_four_by_three())
        .install(app, mary_o_prepared_session_world);
        app.add_plugins(MaryORulesPlugin::hosted());
    }
}

/// The provider's authored level 1-1 source for the shared preparation lifecycle.
fn mary_o_prepared_session_world() -> PreparedPlatformerSource {
    let source = mary_o_session_world();
    PreparedPlatformerSource::new(
        MARY_O_EXPERIENCE,
        source.room_set,
        source.geometry,
        source.metadata,
        source.starting_character,
        LdtkRuntimeIndex::default(),
    )
}
