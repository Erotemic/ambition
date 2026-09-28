//! The Mary-O experience provider.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::presentation::profiles;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{RoomSet, StartingCharacter};
use ambition_platformer2d::runtime::PreparedPlatformerSource;

use crate::{MaryORulesPlugin, LEVEL_1_1_ROOM_ID};

pub const MARY_O_EXPERIENCE: &str = "mary_o";
pub const MARY_O_GAMEPLAY_ROUTE: &str = "mary_o_gameplay";
pub const MARY_O_LAUNCHER_ROUTE: &str = "mary_o_launcher";
pub const MARY_O_CHARACTER_ID: &str = "mary_o";
/// Her level theme. Every track id here is declared in her pack's
/// `audio/music_registry.ron`, which is what authorizes her session to play it.
pub const MARY_O_MUSIC_TRACK: &str = "support_theme";

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
/// this is it, the authored `invincible_maryo` score, resolved by the ordinary
/// `audio/music/generated/<id>/full.ogg` convention like the other two stings.
/// Her pack declaring it is what AUTHORIZES the session to select it.
pub const MARY_O_STAR_MUSIC_TRACK: &str = "invincible_maryo";

/// The coin-collect ding — an id this crate DECLARES but never EMITS.
///
/// Every other id in her pack's `audio/sfx_registry.ron` is written by Mary-O's own code. This
/// one is written by the engine: her coins are authored as `currency:1` pickups,
/// so the shared `collect_ecs_pickups` loop emits
/// [`ids::WORLD_COIN_PICKUP`](ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP)
/// when one is collected, with no demo-side collection code at all.
///
/// so the sound was never missing — the AUTHORIZATION was. The emit has always fired; under
/// provider-relative audio a session plays only the cues its own fragment declares, and an
/// undeclared id is dropped on the floor. Sanic's rings ride the identical path and its provider
/// declares the identical id (`demo_sanic`'s `SFX_RING`).
///
/// voicing a PRIVATE `mary_o.coin` id here would be silence, because the
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
    pub starting_character: StartingCharacter,
}

/// Which room a session starts in.
///
/// The source is installed as a SYSTEM and its own doc says it *"may read the provider's own
/// resources"*, so this is the seam that was already there.
///
/// absent means 1-1: a shipped game must not depend on a resource a test
/// inserts.
#[derive(bevy::prelude::Resource, Clone, Debug)]
pub struct MaryOEntryRoom(pub String);

impl Default for MaryOEntryRoom {
    fn default() -> Self {
        Self(LEVEL_1_1_ROOM_ID.to_string())
    }
}

/// Every room a Mary-O session may enter. Authored areas come from the content
/// file; the Rust-built fixture course is appended explicitly for tests/tools.
pub fn mary_o_room_ids() -> Vec<String> {
    let mut ids = crate::authored_area_ids();
    ids.push(crate::test_course::TEST_COURSE_ROOM_ID.to_string());
    ids
}

pub fn mary_o_session_world() -> MaryOSessionWorld {
    mary_o_session_world_entering(LEVEL_1_1_ROOM_ID)
}

/// Build the session world with `entry` active, then derive geometry
/// from the room the resulting `RoomSet` actually activated.
///
/// ⚠ **AN UNKNOWN `entry` PANICS, AND USED TO OPEN 1-1 INSTEAD.** This said
/// *"unknown ids use the set's fallback room"* until 2026-09-20, which is what
/// made both `--room` roads validate the id themselves. They still do, because
/// a named list of the rooms that exist beats a panic; this is the backstop
/// behind them.
pub fn mary_o_session_world_entering(entry: &str) -> MaryOSessionWorld {
    // The fixture course is neither of them: it is a self-contained probe room
    // with no loading zones that loops on its own goal (it names no `next_room`), so a
    // session running it carries it INSTEAD of the shipped levels. Its links go
    // with its rooms — an edge naming a room the set does not hold is a
    // `try_from_parts` warning on stderr and nothing else, which is how the course
    // has been printing two of them.
    let (rooms, links) = if entry == crate::test_course::TEST_COURSE_ROOM_ID {
        (vec![crate::test_course::test_course()], Vec::new())
    } else {
        (crate::authored_levels(), crate::authored_room_links())
    };
    let room_set = RoomSet::from_parts_or_panic(entry, rooms, links);
    let active = room_set.active_spec();
    let geometry = ae::RoomGeometry(active.world.clone());
    MaryOSessionWorld {
        geometry,
        room_set,
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
        // image is published by scripts/regen/sprites.sh; until then the render falls back
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
                    // The floor exists to keep a sub-pixel projectile visible; a spark is 20 px and
                    // does not need one.
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
            use ambition_platformer2d::audio::catalog::AudioCatalogAppExt;
            // Her pack states both: what her session may play, and how each
            // synth cue sounds. Her moves name bank cues as well as her own
            // voice.
            app.register_audio_catalog_fragment(
                crate::pack::PACK
                    .audio(MARY_O_EXPERIENCE)
                    .with_resident_sfx_bank(),
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
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
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
        source.starting_character,
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
    // The room she is in, for the title its level authors.
    rooms: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<
            ambition_platformer2d::runtime::demo_fixture::RoomSet,
        >,
    >,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let Ok((level, flag)) = level.single() else {
        return;
    };
    let title = rooms.as_deref().and_then(|rooms| rooms.active_metadata().title.as_deref());
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
    match card_text(level, flag, title) {
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
///
/// The intro names the room by the `title` its level authors, so a new level
/// is titled in the level file. A room with no title shows only her lives.
fn card_text(
    level: &crate::MaryOLevelState,
    flag: Option<&crate::flag::FlagSequence>,
    title: Option<&str>,
) -> Option<String> {
    if let Some(score) = flag.and_then(|f| f.score()) {
        return Some(format!(
            "COURSE CLEAR    {:06}",
            level.score.saturating_add(score)
        ));
    }
    (level.intro_card > 0.0).then(|| match title {
        Some(title) => format!("{title}    MARY-O x{}", level.lives),
        None => format!("MARY-O x{}", level.lives),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The room `id` names, built WITHOUT going through the provider — so this
    /// test's expectation comes from the level builder rather than from the seam
    /// it is checking.
    fn room_named(id: &str) -> ambition_platformer2d::world::rooms::RoomSpec {
        if id == crate::test_course::TEST_COURSE_ROOM_ID {
            crate::test_course::test_course()
        } else {
            crate::authored_level(id)
        }
    }

    /// Enough of a world to tell Mary-O's rooms apart.
    fn shape_of(world: &ae::World) -> (String, [f32; 2], [f32; 2], usize) {
        (
            world.name.clone(),
            world.size.to_array(),
            world.spawn.to_array(),
            world.blocks.len(),
        )
    }

    /// Every room id boots into ITS OWN geometry, not into 1-1's.
    ///
    /// this went red on `mary_o_1_2`. The seam branched on the test course and built 1-1
    /// for everything else, while handing `entry` straight to the room-set constructor — so asking
    /// for 1-2 produced a world whose active room WAS 1-2 and whose `geometry` was 1-1's.
    /// (Metadata has no copy to disagree: it is the room set's active entry.)
    ///
    /// The loop asks the same question of every room the demo has, and the distinctness guard
    /// above it means a future room that is a copy of another cannot make the comparison
    /// vacuous.
    #[test]
    fn every_room_id_starts_in_its_own_geometry() {
        let ids = mary_o_room_ids();
        // the roster is READ from the world file now, so a file that authored
        // nothing would make every loop below vacuous.
        assert!(
            ids.len() >= 2,
            "the roster came back as {ids:?}; a probe over one room proves nothing"
        );

        // The poison: if two rooms cannot be told apart, every assertion below
        // passes for the wrong reason.
        for (i, left) in ids.iter().enumerate() {
            for right in &ids[i + 1..] {
                assert_ne!(
                    shape_of(&room_named(left).world),
                    shape_of(&room_named(right).world),
                    "`{left}` and `{right}` are indistinguishable, so this test \
                     cannot detect one being served in place of the other"
                );
            }
        }

        for id in ids.iter().map(String::as_str) {
            let session = mary_o_session_world_entering(id);
            let expected = room_named(id);

            assert_eq!(
                session.room_set.active_spec().id,
                id,
                "a session entering `{id}` must be ACTIVE in `{id}`"
            );
            assert_eq!(
                shape_of(&session.geometry.0),
                shape_of(&expected.world),
                "a session entering `{id}` got another room's geometry"
            );

            // and the entry room is in the set ONCE. The obvious fix for the
            // above — select 1-2 as the entry room and keep appending it to the
            // room list — puts it in the graph twice, which is a second node
            // with the same id and a transition that can resolve to either.
            let copies = session
                .room_set
                .rooms
                .iter()
                .filter(|room| room.id == id)
                .count();
            assert_eq!(
                copies, 1,
                "`{id}` appears {copies} times in the room set; the entry room \
                 and the room list must come from ONE source",
            );
        }
    }

    /// The coin ding must voice the id the ENGINE emits, and the registry must
    /// authorize it.
    ///
    /// Two assertions, and they fail for different reasons on purpose. The first catches a
    /// rename: [`COIN_PICKUP_SFX`] is a string literal in this crate standing in for a constant
    /// in another, and nothing but this line joins them.
    #[test]
    fn the_coin_collect_cue_is_the_shared_currency_pickup_id() {
        assert_eq!(
            ambition_platformer2d::sfx::SfxId::from_static(COIN_PICKUP_SFX),
            ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP,
            "the coin ding must name the id `collect_ecs_pickups` emits for a \
             Currency pickup — a private `mary_o.coin` id is gated to silence"
        );
        let registry = ambition_platformer2d::audio::content_schema::lowered_sfx_registry(
            crate::pack::PACK.prepared(),
        )
        .expect("her pack states her SFX");
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

    /// Every cue and track her code names is one her pack declares.
    ///
    /// Her code emits these ids and her pack authorizes them, and nothing else
    /// joins the two: an id her code emits that the pack does not declare is
    /// silence under provider-relative audio, with no error.
    #[test]
    fn every_cue_her_code_names_is_declared_by_her_pack() {
        let pack = crate::pack::PACK.prepared();
        let sfx = ambition_platformer2d::audio::content_schema::lowered_sfx_registry(pack)
            .expect("her pack states her SFX");
        let authorized = sfx.authorized_cue_ids();
        for id in [
            COIN_PICKUP_SFX,
            crate::powerups::SFX_SMALL_TO_BIG,
            crate::powerups::SFX_BIG_TO_FIRE,
            crate::powerups::SFX_BIG_TO_SMALL,
            crate::powerups::SFX_FIRE_TO_BIG,
            crate::powerups::SFX_FIRE_TO_SMALL,
            crate::pipe::PIPE_WARP_SFX,
        ] {
            assert!(
                authorized.contains(&ambition_platformer2d::sfx::SfxId::from_static(id)),
                "her code emits `{id}` and her pack does not declare it"
            );
        }
        let music = ambition_platformer2d::audio::content_schema::lowered_music_registry(pack)
            .expect("her pack states her music");
        for track in [
            MARY_O_MUSIC_TRACK,
            MARY_O_DEATH_MUSIC_TRACK,
            MARY_O_VICTORY_MUSIC_TRACK,
            MARY_O_STAR_MUSIC_TRACK,
        ] {
            assert!(music.track(track).is_some(), "her code claims `{track}` and her pack does not declare it");
        }
        assert_eq!(music.default_track, MARY_O_MUSIC_TRACK);
    }
}
