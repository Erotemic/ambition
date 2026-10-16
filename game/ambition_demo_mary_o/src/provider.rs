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
    try_mary_o_session_world_entering(entry).unwrap_or_else(|reasons| panic!("{}", reasons.join("\n")))
}

/// [`mary_o_session_world_entering`], or every reason the world file cannot
/// be played: the road a running game takes, since the file is read off disk.
pub fn try_mary_o_session_world_entering(entry: &str) -> Result<MaryOSessionWorld, Vec<String>> {
    // The fixture course is neither of them: it is a self-contained probe room
    // with no loading zones that loops on its own goal (it names no `next_room`), so a
    // session running it carries it INSTEAD of the shipped levels. Its links go
    // with its rooms — an edge naming a room the set does not hold is a
    // `try_from_parts` warning on stderr and nothing else, which is how the course
    // has been printing two of them.
    let (rooms, links) = if entry == crate::test_course::TEST_COURSE_ROOM_ID {
        (vec![crate::test_course::test_course()], Vec::new())
    } else {
        crate::try_authored_levels_and_links()?
    };
    let room_set = RoomSet::from_parts_or_panic(entry, rooms, links);
    let active = room_set.activation_spec();
    let geometry = ae::RoomGeometry(active.world.clone());
    Ok(MaryOSessionWorld {
        geometry,
        room_set,
        starting_character: StartingCharacter::new(MARY_O_CHARACTER_ID),
    })
}

pub fn mary_o_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(MARY_O_CHARACTER_ID, MARY_O_EXPERIENCE)
}

/// Declare her pickup art: the rows of her pack's `data/world_item_art.ron`
/// (the `sprite` id a world item carries, its picture and size). Pure data, so
/// no render dependency here: the render layer resolves each row into a sprite
/// through the shared `WorldItem` art seam, in the standalone app and in the
/// multi-game host alike (this provider is the one seam both share). Until the
/// publish writes a picture, the render draws the quad.
pub(crate) fn register_world_item_art(app: &mut App) {
    use ambition_platformer2d::platformer::world_item_art::{WorldItemArtAppExt, WorldItemArtEntry};
    let rows = ambition_platformer2d::items::content_schema::world_item_art::lowered_world_item_art(
        crate::pack::PACK.prepared(),
    )
    .expect("the demo's pack states its world item art (`data/world_item_art.ron`)");
    app.register_world_item_art(rows.iter().map(|(id, art)| {
        WorldItemArtEntry::new(id.clone(), art.path.clone(), ae::Vec2::new(art.size.0, art.size.1))
    }));
}

/// Her shots' looks (the spark, the swallow's coconut, the void lizard's orb):
/// her pack's `data/projectile_visuals.ron`, under the ids her ranged actions
/// author. The projectile domain never learns what a spark is.
fn register_projectile_looks(app: &mut App) {
    use ambition_platformer2d::projectiles::visual::ProjectileVisualAppExt;
    let looks = ambition_platformer2d::projectiles::content_schema::lowered_projectile_looks(crate::pack::PACK.prepared())
        .expect("the demo's pack states its projectile looks (`data/projectile_visuals.ron`)");
    for (id, art) in looks {
        app.register_projectile_visual(id.clone(), art.clone());
    }
    app.add_plugins(ambition_platformer2d::content::live_reload::EmbeddedPackReloadPlugin::projectile_looks(&crate::pack::PACK));
}

pub struct MaryOExperiencePlugin;

impl Plugin for MaryOExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_mary_o_content(app);
        crate::quasar_shader::install(app);
        crate::shimmer_shader::install(app);
        crate::pipe_signs::install(app);
        crate::fire_shader::install(app);
        crate::void_look::install(app);
        crate::cosmic_aura::install(app);
        crate::cosmic_barrier::install(app);
        register_world_item_art(app);
        register_projectile_looks(app);
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
        // The look of its launcher row.
        .with_launcher_look("starlight")
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
                )
                // The boss of a castle: its name and a gauge of its health.
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(BOSS_HUD_SLOT)
                        .with_order(98)
                        .with_font_size(26.0)
                        .with_color([0.94, 0.94, 0.98, 1.0])
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

/// The provider's authored world for the shared preparation lifecycle, or the
/// reasons the world file cannot be played (a bad save fails the session's
/// preparation; the game keeps running).
fn mary_o_prepared_session_world(
    entry: Option<bevy::prelude::Res<MaryOEntryRoom>>,
) -> Result<PreparedPlatformerSource, ambition_platformer2d::provider::WorldRefusal> {
    let source = try_mary_o_session_world_entering(
        entry
            .as_ref()
            .map_or(LEVEL_1_1_ROOM_ID, |room| room.0.as_str()),
    )
    .map_err(ambition_platformer2d::provider::WorldRefusal::new)?;
    Ok(PreparedPlatformerSource::new(
        MARY_O_EXPERIENCE,
        source.room_set,
        source.geometry,
        source.starting_character,
    ))
}

/// Slot ids Mary-O publishes into. Opaque to the engine.
pub const SCORE_HUD_SLOT: &str = "mary_o_score";
pub const COINS_HUD_SLOT: &str = "mary_o_coins";
pub const TIME_HUD_SLOT: &str = "mary_o_time";
pub const LIVES_HUD_SLOT: &str = "mary_o_lives";
pub const CARD_HUD_SLOT: &str = "mary_o_card";
/// The name and the health of a boss, while the player is in its fight.
pub const BOSS_HUD_SLOT: &str = "mary_o_boss";

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
    rooms: Option<ambition_platformer2d::world::rooms::LiveRoomSpecs>,
    // The readouts are the primary player's (`PlayerHudFacts` above).
    player: bevy::prelude::Query<
        bevy::prelude::Entity,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    // The boss she fights, when she is near one that is alive.
    bodies: bevy::prelude::Query<(
        &ambition_platformer2d::combat::actor_tuning::ActorConfig,
        &ambition_platformer2d::characters::actor::BodyHealth,
        &ambition_platformer2d::engine_core::BodyKinematics,
    )>,
    her: bevy::prelude::Query<
        &ambition_platformer2d::engine_core::BodyKinematics,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    let Ok((level, flag)) = level.single() else {
        return;
    };
    let boss = her.single().ok().and_then(|her| {
        bodies.iter().find(|(config, health, kin)| {
            crate::void_lizard::is_void_lizard_brain(&config.brain)
                && health.alive()
                && (kin.pos.x - her.pos.x).abs() <= crate::void_lizard::SIGHT
        })
    });
    match boss {
        Some((_, health, _)) => readouts.set(
            BOSS_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::gauge(
                crate::void_lizard::VOID_LIZARD_DISPLAY_NAME.to_uppercase(),
                format!("{}/{}", health.health.current, health.health.max),
                health.health.current as f32 / (health.health.max.max(1)) as f32,
            ),
        ),
        None => readouts.clear_slot(BOSS_HUD_SLOT),
    }
    let title = rooms
        .as_ref()
        .zip(player.single().ok())
        .and_then(|(rooms, player)| rooms.spec_of(player))
        .and_then(|spec| spec.metadata.title.as_deref());
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

    /// Each id her ranged actions author has a look, and the look is the
    /// pack file's row: an edit to the file is drawn at the next start.
    #[test]
    fn her_shots_look_as_her_pack_file_states() {
        use ambition_platformer2d::content::EmbeddedPack;
        use ambition_platformer2d::projectiles::content_schema::{
            lowered_projectile_looks, PROJECTILE_VISUALS_SCHEMA,
        };
        use ambition_platformer2d::projectiles::{ProjectileRenderSize, ProjectileVisualCatalog};

        let mut app = App::new();
        register_projectile_looks(&mut app);
        let catalog = app.world().resource::<ProjectileVisualCatalog>();
        for id in [crate::powerups::SPARK_VISUAL, crate::swallow::COCONUT_VISUAL, crate::void_lizard::VOID_ORB_VISUAL] {
            assert!(catalog.get(id).is_some(), "the action's look `{id}` is not registered");
        }

        let authored =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/projectile_visuals.ron"))
                .expect("the demo's looks file is readable");
        let root = std::env::temp_dir().join(format!("mary_o_looks_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "mary_o_looks_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/projectile_visuals.ron", schema: "{PROJECTILE_VISUALS_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let spark_size = |text: &str| {
            std::fs::write(root.join("data/projectile_visuals.ron"), text).unwrap();
            // The engine's schemas hold `projectile_visuals`.
            let pack = EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None);
            let prepared = pack.compile().unwrap_or_else(|failure| panic!("{}", failure.render()));
            lowered_projectile_looks(&prepared).expect("the probe pack lowers the looks")[crate::powerups::SPARK_VISUAL].size
        };

        let file = spark_size(&authored);
        assert_eq!(Some(file), catalog.get(crate::powerups::SPARK_VISUAL).map(|art| art.size), "the demo draws the file");
        let scale = "scale: 1.8,";
        assert_eq!(authored.matches(scale).count(), 1, "the probe edits one line of the file");
        assert_eq!(
            spark_size(&authored.replace(scale, "scale: 2.4,")),
            ProjectileRenderSize::Body { min: 0.0, scale: 2.4 }
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Each pickup her code spawns has art, and the art is the pack file's
    /// row: the registered manifest is the file, an edit is the lowered row,
    /// and a row drawn at no size is refused with its id named.
    #[test]
    fn her_pickups_look_as_her_pack_file_states() {
        use ambition_platformer2d::content::EmbeddedPack;
        use ambition_platformer2d::items::content_schema::world_item_art::{
            lowered_world_item_art, WORLD_ITEM_ART_SCHEMA,
        };
        use ambition_platformer2d::platformer::world_item_art::WorldItemArtManifest;
        let mut app = App::new();
        register_world_item_art(&mut app);
        let manifest = app.world().resource::<WorldItemArtManifest>();
        let shipped = lowered_world_item_art(crate::pack::PACK.prepared()).expect("the pack states its pickup art");
        for id in [crate::powerups::STAR_WAND_SPRITE, crate::powerups::CINDER_BEACON_SPRITE, crate::star::QUASAR_SPRITE] {
            let row = shipped.get(id).unwrap_or_else(|| panic!("the pickup `{id}` her code spawns has no row"));
            let entry = manifest.effective().into_iter().find(|entry| entry.sprite_id == id).expect("registered");
            assert_eq!(
                (entry.asset_path.as_str(), entry.size.to_array()),
                (row.path.as_str(), [row.size.0, row.size.1]),
                "`{id}` is registered as the file states it"
            );
        }

        let authored = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/data/world_item_art.ron"))
            .expect("the demo's pickup art file is readable");
        let root = std::env::temp_dir().join(format!("mary_o_pickups_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("pack.ron"),
            format!(
                r#"(id: "mary_o_pickups_probe", version: "0.1.0", namespace: "probe", requires: [],
                    sources: [(path: "data/world_item_art.ron", schema: "{WORLD_ITEM_ART_SCHEMA}", version: 1)])"#
            ),
        )
        .unwrap();
        let compiled = |text: &str| {
            std::fs::write(root.join("data/world_item_art.ron"), text).unwrap();
            EmbeddedPack::rooted(Box::leak(root.display().to_string().into_boxed_str()), None).compile()
        };
        let size = "size: (28.0, 28.0)";
        assert_eq!(authored.matches(size).count(), 1, "the probe edits the quasar's row");
        let edited = compiled(&authored.replace(size, "size: (32.0, 32.0)")).unwrap_or_else(|failure| panic!("{}", failure.render()));
        assert_eq!(lowered_world_item_art(&edited).unwrap()[crate::star::QUASAR_SPRITE].size, (32.0, 32.0));
        let refused = match compiled(&authored.replace(size, "size: (28.0, 0.0)")) {
            Ok(_) => panic!("a quasar drawn at no height was admitted"),
            Err(failure) => failure.render(),
        };
        assert!(refused.contains("`super_mary_o_cosmic_quasar.size`"), "the refusal names the row and field: {refused}");
        let _ = std::fs::remove_dir_all(&root);
    }

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
                session.room_set.activation_spec().id,
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
        let forms = crate::forms::mary_o_forms();
        for id in [COIN_PICKUP_SFX, crate::pipe::PIPE_WARP_SFX]
            .into_iter()
            .chain(forms.cues.iter().map(|cue| cue.cue.as_str()))
        {
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
            crate::cosmic::COSMIC_MUSIC_TRACK,
        ] {
            assert!(music.track(track).is_some(), "her code claims `{track}` and her pack does not declare it");
        }
        assert_eq!(music.default_track, MARY_O_MUSIC_TRACK);
    }
}
