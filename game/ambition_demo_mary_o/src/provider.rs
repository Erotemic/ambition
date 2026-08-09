//! The Mary-O experience provider.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::presentation::profiles;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{
    ActiveRoomMetadata, LdtkRuntimeIndex, RoomSet, StartingCharacter,
};
use ambition_platformer2d::runtime::PreparedPlatformerSource;

use crate::{level_1_1, MaryORulesPlugin, LEVEL_1_1_ROOM_ID};

pub const MARY_O_EXPERIENCE: &str = "mary_o";
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
pub const MARY_O_CHARACTER_ID: &str = "mary_o";
pub const MARY_O_MUSIC_TRACK: &str = "support_theme";
pub const MARY_O_MUSIC_ASSET_PATH: &str = "audio/music/generated/support_theme/full.ogg";

/// The track that plays over her death.
///
/// Authored as its own score (`scores/active/mary_o_you_died.music.yaml`) with a
/// `death_sting` section. It resolves its OGG by the ordinary convention
/// (`audio/music/generated/<id>/full.ogg`), so unlike the level theme it needs
/// no explicit path.
pub const MARY_O_DEATH_MUSIC_TRACK: &str = "mary_o_you_died";

/// The course-clear sting, played over the flagpole sequence.
///
/// Same arrangement as the death track — its own score
/// (`scores/active/mary_o_flag_victory.music.yaml`), resolved by the ordinary
/// `audio/music/generated/<id>/full.ogg` convention. Two bars at 156bpm, so
/// about 3.1 seconds, which is what [`crate::flag`] sizes its beats against.
pub const MARY_O_VICTORY_MUSIC_TRACK: &str = "mary_o_flag_victory";

/// The star's theme, played while the pocket quasar burns.
///
/// Jon: "We already have a 'super star' invincibility music track ready to go" —
/// this is it, the authored `invincible_maryo` score, resolved by the ordinary
/// `audio/music/generated/<id>/full.ogg` convention like the other two stings.
/// Declaring it here is what AUTHORIZES the session to select it.
pub const MARY_O_STAR_MUSIC_TRACK: &str = "invincible_maryo";

/// **The coin-collect ding — an id this crate DECLARES but never EMITS.**
///
/// Every other id in [`mary_o_sfx_specs`] is written by Mary-O's own code. This
/// one is written by the engine: her coins are authored as `currency:1` pickups,
/// so the shared `collect_ecs_pickups` loop emits
/// [`ids::WORLD_COIN_PICKUP`](ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP)
/// when one is collected, with no demo-side collection code at all.
///
/// ⛔ **so the sound was never missing — the AUTHORIZATION was.** The emit has
/// always fired; under provider-relative audio a session plays only the cues its
/// own fragment declares, and an undeclared id is dropped on the floor. That is
/// the failure mode that reads as *"we never wrote that sound"* and is really
/// *"we play it and then throw it away"*. Sanic's rings ride the identical path
/// and its provider declares the identical id (`demo_sanic`'s `SFX_RING`).
///
/// ⚠ **voicing a PRIVATE `mary_o.coin` id here would be silence**, because the
/// gate compares against what the engine emits, not against what reads well.
/// [`the_coin_collect_cue_is_the_shared_currency_pickup_id`] pins this constant
/// to that engine id so a rename on either side cannot silently re-mute the coin.
///
/// [`the_coin_collect_cue_is_the_shared_currency_pickup_id`]: self::tests::the_coin_collect_cue_is_the_shared_currency_pickup_id
pub const COIN_PICKUP_SFX: &str = "world.coin.pickup";

#[derive(Clone)]
pub struct MaryOSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub metadata: ActiveRoomMetadata,
    pub starting_character: StartingCharacter,
}

/// **Which room a session starts in.**
///
/// ⛔ the entry was hardcoded to `LEVEL_1_1_ROOM_ID`, so a test could not boot
/// into anything else — which is why the playthrough tests had to walk the real
/// level and went stale the moment it was authored (Jon, 2026-08-04). The
/// source is installed as a SYSTEM and its own doc says it *"may read the
/// provider's own resources"*, so this is the seam that was already there.
///
/// ⚠ absent means 1-1: a shipped game must not depend on a resource a test
/// inserts.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct MaryOEntryRoom(pub String);

impl Default for MaryOEntryRoom {
    fn default() -> Self {
        Self(LEVEL_1_1_ROOM_ID.to_string())
    }
}

pub fn mary_o_session_world() -> MaryOSessionWorld {
    mary_o_session_world_entering(LEVEL_1_1_ROOM_ID)
}

/// The same world, started in `entry`.
pub fn mary_o_session_world_entering(entry: &str) -> MaryOSessionWorld {
    let room = if entry == crate::test_course::TEST_COURSE_ROOM_ID {
        crate::test_course::test_course()
    } else {
        level_1_1()
    };
    let geometry = ae::RoomGeometry(room.world.clone());
    let metadata = ActiveRoomMetadata(room.metadata.clone());
    // TWO rooms, linked both ways. The demo could not express this until the
    // room-transition transaction became engine-side (2026-07-25): its consumer
    // lived in `ambition_app`, which no demo depends on, so a second room would
    // have been unreachable in this binary.
    //
    // ⭐ **the LINKS are read, not written (2026-08-05).** They were two
    // `RoomLink`s spelled out here, and the reason was structural rather than
    // lazy: a link needs both ends, and 1-2 was a Rust room that no world file
    // contained, so its half could not be authored. Both levels are areas of
    // `mary_o.ldtk` now and every zone names its partner in
    // `target_room`/`target_zone`, so retargeting the shaft is an editor edit.
    let room_set = RoomSet::from_parts(
        entry,
        vec![room, crate::level_1_2::level_1_2()],
        crate::authored_room_links(),
    );
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
        crate::quasar_shader::install(app);
        // Declare the star wand pickup art (pure id → path + size DATA; no render
        // dependency here). The render layer resolves it into a real sprite through
        // the shared `WorldItem` art seam, so the ?-block's wand draws as a wand
        // instead of the cream placeholder quad in BOTH the standalone app and the
        // multi-game host — this provider is the one seam both share. The flat prop
        // image is published by regen_sprites.sh; until then the render falls back
        // to the quad.
        {
            use ambition_platformer2d::platformer::world_item_art::{
                WorldItemArtAppExt, WorldItemArtEntry,
            };
            app.register_world_item_art([
                WorldItemArtEntry::new(
                    crate::powerups::STAR_WAND_SPRITE,
                    format!("sprites/props/{}.png", crate::powerups::STAR_WAND_SPRITE),
                    // Sized from the generated canonical's opaque bbox (53x69px)
                    // so the wand is not squashed to the carton's old aspect.
                    ae::Vec2::new(24.0, 31.0),
                ),
                // The second rung of the chain. Same seam, same fallback: until
                // the prop image is published the render draws the row-tinted
                // quad, so the pickup is always visible.
                WorldItemArtEntry::new(
                    crate::powerups::CINDER_BEACON_SPRITE,
                    format!(
                        "sprites/props/{}.png",
                        crate::powerups::CINDER_BEACON_SPRITE
                    ),
                    // Likewise from the beacon's bbox (39x59px) — a lantern is
                    // taller than it is wide.
                    ae::Vec2::new(24.0, 36.0),
                ),
                // The star. Round, and drawn a touch larger than the other two
                // because it is the rarest thing in the level and should read as
                // an event from across a screen.
                WorldItemArtEntry::new(
                    crate::star::QUASAR_SPRITE,
                    format!("sprites/props/{}.png", crate::star::QUASAR_SPRITE),
                    ae::Vec2::new(28.0, 28.0),
                ),
            ]);
        }
        {
            // The spark's LOOK, registered as content under the id her ranged
            // action authors. One registration, zero render edits — and because
            // the id lives on the action, the projectile domain never learns what
            // a spark is.
            use ambition_platformer2d::projectiles::visual::{
                ProjectileArt, ProjectileArtSource, ProjectileRenderSize, ProjectileRotation,
                ProjectileVisualAppExt,
            };
            app.register_projectile_visual(
                crate::powerups::SPARK_VISUAL,
                ProjectileArt {
                    source: ProjectileArtSource::EnergyTinted {
                        rgba: [1.0, 0.62, 0.16, 0.96],
                    },
                    // ⚠ **`min: 0.0` on purpose — the BODY is the size
                    // authority.** This used to be 7.0, the same number as the
                    // flight half-extent, maintained in two files: growing the
                    // shot in one place would have left the drawn quad clamped at
                    // the old size, which is the failure the pickup probes just
                    // had in another form. The floor exists to keep a sub-pixel
                    // projectile visible; a spark is 20 px and does not need one.
                    size: ProjectileRenderSize::Body {
                        min: 0.0,
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
            use ambition_platformer2d::audio::catalog::{AudioCatalogAppExt, AudioCatalogFragment};
            app.register_audio_catalog_fragment(
                AudioCatalogFragment::new(
                    MARY_O_EXPERIENCE,
                    // Mary-O runs on the "Support Theme" cue. Declaring it in the
                    // provider fragment is what authorizes the session to select
                    // and play it under provider-relative audio.
                    Some(ambition_platformer2d::audio::spec::MusicRegistry {
                        default_track: MARY_O_MUSIC_TRACK.to_string(),
                        tracks: vec![
                            ambition_platformer2d::audio::spec::MusicTrack {
                                id: MARY_O_MUSIC_TRACK.to_string(),
                                display_name: "Support Theme".to_string(),
                                asset_path: Some(MARY_O_MUSIC_ASSET_PATH.to_string()),
                                one_shot: false,
                            },
                            // Declaring the death track is what AUTHORIZES it:
                            // under provider-relative playback a session plays
                            // only what its own fragment names, so a cue nobody
                            // declared is gated to silence no matter who asks
                            // for it.
                            ambition_platformer2d::audio::spec::MusicTrack {
                                id: MARY_O_DEATH_MUSIC_TRACK.to_string(),
                                display_name: "Mary O You Died".to_string(),
                                asset_path: None,
                                one_shot: true,
                            },
                            ambition_platformer2d::audio::spec::MusicTrack {
                                id: MARY_O_VICTORY_MUSIC_TRACK.to_string(),
                                display_name: "Mary O Flag Victory".to_string(),
                                asset_path: None,
                                one_shot: true,
                            },
                            // The star. LOOPS (not one-shot): it plays for as
                            // long as the quasar burns and the level theme
                            // returns when the priority claim is released.
                            ambition_platformer2d::audio::spec::MusicTrack {
                                id: MARY_O_STAR_MUSIC_TRACK.to_string(),
                                display_name: "Invincible Mary-O".to_string(),
                                asset_path: None,
                                one_shot: false,
                            },
                        ],
                    }),
                    Some(ambition_platformer2d::audio::spec::SfxRegistry {
                        sample_rate: 44_100,
                        sfx: mary_o_sfx_specs(),
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
            ambition_platformer2d::presentation::HudDeclaration::new()
                .slot(hud_slot(SCORE_HUD_SLOT))
                .slot(hud_slot(COINS_HUD_SLOT))
                .slot(hud_slot(TIME_HUD_SLOT))
                .slot(hud_slot(LIVES_HUD_SLOT))
                // The transient card: level title on entry, course-clear tally
                // on the flag. One slot for both, because they never overlap —
                // you are either starting the level or finishing it.
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(CARD_HUD_SLOT)
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
fn mary_o_prepared_session_world(
    entry: Option<bevy::prelude::Res<MaryOEntryRoom>>,
) -> PreparedPlatformerSource {
    let source = mary_o_session_world_entering(
        entry
            .as_ref()
            .map_or(LEVEL_1_1_ROOM_ID, |room| room.0.as_str()),
    );
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
fn hud_slot(id: &str) -> ambition_platformer2d::presentation::HudSlotSpec {
    ambition_platformer2d::presentation::HudSlotSpec::new(id)
        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
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
    facts: bevy::prelude::Res<ambition_platformer2d::sim_view::PlayerHudFacts>,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
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
            ambition_platformer2d::presentation::HudReadout::bare(text),
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

/// **The whole Mary-O SFX table.** Every entry here is both an AUTHORIZATION
/// and a voice: under provider-relative audio a session only plays cues its
/// own fragment declares, so a cue that is emitted but not listed here is
/// gated to silence. (An undeclared `player.jump` is exactly that, which is
/// why the Jump cue below is what makes her jump audible at all.) All of
/// these are procedurally synthesized from the spec; no asset file needed.
///
/// ⚠ **the emitter is not always Mary-O.** Most rows voice a cue this crate
/// writes, but [`COIN_PICKUP_SFX`] voices one the ENGINE writes on her
/// behalf — see its doc. Declaring it is the only thing this crate does about
/// it, and it is the whole difference between a coin that dings and one that
/// does not.
fn mary_o_sfx_specs() -> Vec<ambition_platformer2d::audio::spec::SfxSpec> {
    vec![
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: Some(ambition_platformer2d::audio::spec::SoundCueKey::Jump),
            id: None,
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Sine,
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
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: Some(ambition_platformer2d::audio::spec::SoundCueKey::Hit),
            id: None,
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Square,
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
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: Some(ambition_platformer2d::audio::spec::SoundCueKey::Pogo),
            id: None,
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Square,
            frequency: 320.0,
            frequency_end: 120.0,
            duration: 0.09,
            volume: 0.24,
            attack: 0.001,
            release: 0.055,
            noise: 0.25,
        },
        // PLACEHOLDER TIMBRE: the coin ding — the classic bright
        // blip, a fast rising chip ping (roughly B5 up to E6, the
        // interval the original's two-note coin walks). Square
        // and noiseless so it cuts through the level theme at low
        // volume; a coin is heard many times a minute and must
        // not fatigue. Retune freely — the emit site names the
        // id, not the timbre.
        //
        // ⭐ **unlike every other row here, MARY-O DOES NOT EMIT
        // THIS.** The engine's `collect_ecs_pickups` does, on her
        // behalf, because her coins are authored as `currency:1`
        // pickups. So this entry is pure AUTHORIZATION: the cue
        // was already firing and being discarded by the
        // provider-relative gate. See `COIN_PICKUP_SFX`.
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(COIN_PICKUP_SFX.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Square,
            frequency: 988.0,
            frequency_end: 1319.0,
            duration: 0.10,
            volume: 0.18,
            attack: 0.001,
            release: 0.07,
            noise: 0.0,
        },
        // Mary-O's five form-change ids authorize distinct
        // packed, layered cues. These compact synth specs are
        // only fallbacks while the provider bank is unavailable;
        // normal playback upgrades to the authored bank clips.
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::powerups::SFX_SMALL_TO_BIG.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Triangle,
            frequency: 220.0,
            frequency_end: 880.0,
            duration: 0.38,
            volume: 0.22,
            attack: 0.004,
            release: 0.20,
            noise: 0.03,
        },
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::powerups::SFX_BIG_TO_FIRE.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Sine,
            frequency: 330.0,
            frequency_end: 1320.0,
            duration: 0.52,
            volume: 0.22,
            attack: 0.006,
            release: 0.28,
            noise: 0.05,
        },
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::powerups::SFX_BIG_TO_SMALL.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Triangle,
            frequency: 620.0,
            frequency_end: 150.0,
            duration: 0.34,
            volume: 0.21,
            attack: 0.002,
            release: 0.20,
            noise: 0.06,
        },
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::powerups::SFX_FIRE_TO_BIG.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Sine,
            frequency: 1040.0,
            frequency_end: 330.0,
            duration: 0.42,
            volume: 0.21,
            attack: 0.002,
            release: 0.25,
            noise: 0.08,
        },
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::powerups::SFX_FIRE_TO_SMALL.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Saw,
            frequency: 880.0,
            frequency_end: 110.0,
            duration: 0.56,
            volume: 0.19,
            attack: 0.002,
            release: 0.34,
            noise: 0.10,
        },
        // The warp: a long DESCENDING sine slide, voiced once
        // when a pipe transit begins and running roughly as
        // long as the swallow does — so the sound is the trip,
        // not a click at the start of it. Falling pitch reads
        // as "going in / going down a tube" whichever way the
        // tube actually points, the same way the classic warp
        // cue does. Procedural like the rest; retune freely,
        // the emit site names the id, not the timbre.
        ambition_platformer2d::audio::spec::SfxSpec {
            cue: None,
            id: Some(crate::pipe::PIPE_WARP_SFX.to_string()),
            waveform: ambition_platformer2d::audio::spec::WaveformSpec::Sine,
            frequency: 880.0,
            frequency_end: 165.0,
            duration: 0.45,
            volume: 0.22,
            attack: 0.006,
            release: 0.18,
            noise: 0.04,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The coin ding must voice the id the ENGINE emits, and the registry must
    /// authorize it.**
    ///
    /// Two assertions, and they fail for different reasons on purpose. The first
    /// catches a rename: [`COIN_PICKUP_SFX`] is a string literal in this crate
    /// standing in for a constant in another, and nothing but this line joins
    /// them. The second catches the actual bug this test was written for — the
    /// id being named but never DECLARED, which is silence.
    ///
    /// ⛔ **this went red before it went green.** Mary-O's registry authorized
    /// nine cues and this was not one of them, so every coin in 1-1's vault and
    /// 1-2's shelf was collected in silence while the engine dutifully emitted
    /// the cue. Jon: *"In mary-o we need an SFX for when you collect coins."*
    #[test]
    fn the_coin_collect_cue_is_the_shared_currency_pickup_id() {
        assert_eq!(
            ambition_platformer2d::sfx::SfxId::from_static(COIN_PICKUP_SFX),
            ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP,
            "the coin ding must name the id `collect_ecs_pickups` emits for a \
             Currency pickup — a private `mary_o.coin` id is gated to silence"
        );
        let registry = ambition_platformer2d::audio::spec::SfxRegistry {
            sample_rate: 44_100,
            sfx: mary_o_sfx_specs(),
        };
        assert!(
            registry
                .authorized_cue_ids()
                .contains(&ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP),
            "Mary-O's registry must AUTHORIZE the coin pickup cue; declaring it \
             is the whole difference between a coin that dings and one that does \
             not. Authorized: {:?}",
            registry.authorized_cue_ids()
        );
    }
}
