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
                        sfx: vec![
                            ambition::audio::spec::SfxSpec {
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
                            },
                            // PLACEHOLDER: the brick smash. `break_bricks` emits the
                            // engine's existing `Hit` cue rather than a bespoke
                            // brick verb, and this is the timbre that cue resolves
                            // to for Mary-O — a short, noisy, falling thunk that
                            // reads as masonry giving way. Declaring it is what
                            // makes it audible at all: under provider-relative
                            // audio a session only voices cues its own fragment
                            // declares, so an undeclared `player.hit` is silence.
                            // Swap this spec (or point the cue at a real sample)
                            // when the sound gets authored properly; the emit site
                            // does not change, because it names a cue, not a sound.
                            ambition::audio::spec::SfxSpec {
                                cue: Some(ambition::audio::spec::SoundCueKey::Hit),
                                id: None,
                                waveform: ambition::audio::spec::WaveformSpec::Square,
                                frequency: 190.0,
                                frequency_end: 70.0,
                                duration: 0.11,
                                volume: 0.26,
                                attack: 0.001,
                                release: 0.075,
                                noise: 0.65,
                            },
                            // PLACEHOLDER: the stomp. A short descending square
                            // thud on the shared `Pogo` cue — the "you bounced off
                            // something" verb a head-stomp already is.
                            ambition::audio::spec::SfxSpec {
                                cue: Some(ambition::audio::spec::SoundCueKey::Pogo),
                                id: None,
                                waveform: ambition::audio::spec::WaveformSpec::Square,
                                frequency: 320.0,
                                frequency_end: 120.0,
                                duration: 0.09,
                                volume: 0.24,
                                attack: 0.001,
                                release: 0.055,
                                noise: 0.25,
                            },
                            // The power-up chime: a bright octave-up sine sweep on
                            // a TRANSFORM (grow, or gain fire), the classic "you
                            // leveled up" voice. `sync_grown_form` emits it through
                            // the `Play { id }` path when she steps UP a power tier
                            // (not on a downgrade — the hit already speaks there).
                            // Procedural + placeholder-quality like the Hit/Pogo
                            // specs above: a first pass to make transforms audible
                            // (Jon bug #14). Retune freely — the emit site names the
                            // id, not the timbre.
                            ambition::audio::spec::SfxSpec {
                                cue: None,
                                id: Some("mary_o.transform".to_string()),
                                waveform: ambition::audio::spec::WaveformSpec::Sine,
                                frequency: 520.0,
                                frequency_end: 1040.0,
                                duration: 0.22,
                                volume: 0.24,
                                attack: 0.004,
                                release: 0.12,
                                noise: 0.0,
                            },
                        ],
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
        // Four readouts across the reserved top surround — this profile keeps
        // a 4:3 gameplay rectangle precisely so the HUD has somewhere to live
        // that is not over the level.
        .with_hud(
            ambition::presentation::HudDeclaration::new()
                .slot(hud_slot(SCORE_HUD_SLOT))
                .slot(hud_slot(COINS_HUD_SLOT))
                .slot(hud_slot(TIME_HUD_SLOT))
                .slot(hud_slot(LIVES_HUD_SLOT))
                // The transient card: level title on entry, course-clear tally
                // on the flag. One slot for both, because they never overlap —
                // you are either starting the level or finishing it.
                .slot(
                    ambition::presentation::HudSlotSpec::new(CARD_HUD_SLOT)
                        .with_order(99)
                        .with_font_size(34.0)
                        .with_color([1.0, 0.96, 0.72, 1.0])
                        .centered(),
                ),
        )
        .install(app, mary_o_prepared_session_world);
        app.add_systems(bevy::prelude::Update, publish_mary_o_readouts);
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

/// Slot ids Mary-O publishes into. Opaque to the engine.
pub const SCORE_HUD_SLOT: &str = "mary_o_score";
pub const COINS_HUD_SLOT: &str = "mary_o_coins";
pub const TIME_HUD_SLOT: &str = "mary_o_time";
pub const LIVES_HUD_SLOT: &str = "mary_o_lives";
pub const CARD_HUD_SLOT: &str = "mary_o_card";

/// One readout in Mary-O's house style: top surround, chunky, white.
fn hud_slot(id: &str) -> ambition::presentation::HudSlotSpec {
    ambition::presentation::HudSlotSpec::new(id)
        .with_region(ambition::presentation::SurroundRegion::Top)
        .with_font_size(20.0)
        .with_color([0.97, 0.97, 0.99, 1.0])
}

/// Publish Mary-O's readouts from the state that already owns them.
///
/// Score and lives ride `MaryOLevelState` (the mode-scoped entity that already
/// carried the level clock); coins come from the shared economy's `BodyWallet`
/// through `PlayerHudFacts`, the same fact Sanic's ring tally reads — a coin and
/// a ring are the same `currency` pickup wearing different art.
fn publish_mary_o_readouts(
    level: bevy::prelude::Query<(&crate::MaryOLevelState, Option<&crate::flag::FlagSequence>)>,
    facts: bevy::prelude::Res<ambition::sim_view::PlayerHudFacts>,
    mut readouts: bevy::prelude::ResMut<ambition::presentation::HudReadouts>,
) {
    let Ok((level, flag)) = level.single() else {
        return;
    };
    // Zero-padded like the arcade original: the game owns its formatting, the
    // engine just draws the string.
    readouts.set_labelled(SCORE_HUD_SLOT, "SCORE", format!("{:06}", level.score));
    readouts.set_labelled(
        COINS_HUD_SLOT,
        "COINS",
        format!("{:02}", facts.present.then_some(facts.balance).unwrap_or(0)),
    );
    readouts.set_labelled(
        TIME_HUD_SLOT,
        "TIME",
        format!("{:03}", level.time_remaining.max(0.0).ceil() as u32),
    );
    readouts.set_labelled(LIVES_HUD_SLOT, "LIVES", level.lives);

    // The card is published ONLY while it should be on screen. An unpublished
    // slot draws nothing, so "stop showing it" needs no hide path and no
    // despawn — the card retires itself when the game stops talking about it.
    match card_text(level, flag) {
        Some(text) => readouts.set(
            CARD_HUD_SLOT,
            ambition::presentation::HudReadout::bare(text),
        ),
        None => readouts.clear_slot(CARD_HUD_SLOT),
    }
}

/// What the card says right now, or `None` when no card is up.
///
/// Course-clear WINS over the intro: grabbing the flag inside the intro window
/// is a legitimate (if unlikely) speedrun, and it should read as a clear rather
/// than as the title still hanging around.
fn card_text(
    level: &crate::MaryOLevelState,
    flag: Option<&crate::flag::FlagSequence>,
) -> Option<String> {
    if let Some(score) = flag.and_then(|f| f.score()) {
        return Some(format!(
            "COURSE CLEAR    {:06}",
            level.score.saturating_add(score)
        ));
    }
    (level.intro_card > 0.0).then(|| format!("WORLD 1-1    MARY-O x{}", level.lives))
}
