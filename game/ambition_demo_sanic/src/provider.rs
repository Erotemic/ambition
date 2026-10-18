//! The Sanic experience provider.

use bevy::prelude::*;

use ambition_platformer2d::content::hole;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::presentation::profiles;
use ambition_platformer2d::provider::{AuthoredCatalogFragments, PlatformerExperienceAuthoring};
use ambition_platformer2d::runtime::demo_fixture::{RoomSet, StartingCharacter};
use ambition_platformer2d::runtime::PreparedPlatformerSource;

use crate::{
    sanic_darkness, sanic_highway, sanic_speedway, SanicRulesPlugin, SANIC_CHARACTER_ID,
    SPEEDWAY_ROOM_ID,
};

pub const SANIC_EXPERIENCE: &str = "sanic";
pub const SANIC_GAMEPLAY_ROUTE: &str = "sanic_gameplay";
pub const SANIC_LAUNCHER_ROUTE: &str = "sanic_launcher";

#[derive(Clone)]
pub struct SanicSessionWorld {
    pub geometry: ae::RoomGeometry,
    pub room_set: RoomSet,
    pub starting_character: StartingCharacter,
}

/// Which act a session enters at. Set once, before the app is built, by a
/// binary's `--act` flag; unset means Act 1.
static START_ROOM: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Enter the session at act `act` (1, 2, or 3) instead of Act 1: for playing or
/// photographing a later act without finishing the ones before it. Call before
/// building the app; the first call wins.
pub fn start_at_act(act: u32) -> Result<(), String> {
    let acts = &crate::rules::sanic_rules().acts;
    let room = act
        .checked_sub(1)
        .and_then(|index| acts.get(index as usize))
        .ok_or_else(|| format!("Sanic has acts 1 to {}, not {act}", acts.len()))?;
    let _ = START_ROOM.set(room.room.as_str());
    Ok(())
}

pub fn sanic_session_world() -> SanicSessionWorld {
    let start = START_ROOM.get().copied().unwrap_or(SPEEDWAY_ROOM_ID);
    let rooms = vec![sanic_speedway(), sanic_highway(), sanic_darkness()];
    let geometry = ae::RoomGeometry(
        rooms
            .iter()
            .find(|room| room.id == start)
            .expect("the start room is one of the acts")
            .world
            .clone(),
    );
    // Each act's `next_room` says where its goal leads.
    let room_set = RoomSet::from_parts_or_panic(start, rooms, Vec::new());
    SanicSessionWorld {
        geometry,
        room_set,
        starting_character: StartingCharacter::new(SANIC_CHARACTER_ID),
    }
}

pub fn sanic_authored_catalogs() -> AuthoredCatalogFragments {
    AuthoredCatalogFragments::new(SANIC_CHARACTER_ID, SANIC_EXPERIENCE)
        .with_music()
        .with_procedural_sfx()
        .with_packed_sfx()
}

pub struct SanicExperiencePlugin;

impl Plugin for SanicExperiencePlugin {
    fn build(&self, app: &mut App) {
        crate::install_sanic_content(app);
        PlatformerExperienceAuthoring::new(
            SANIC_EXPERIENCE,
            SANIC_GAMEPLAY_ROUTE,
            crate::text::line("launcher.label"),
            crate::text::line("launcher.description"),
            crate::text::line("launcher.preparing"),
            sanic_authored_catalogs(),
        )
        // The look of its launcher row.
        .with_launcher_look("open_sky")
        // Speed is the whole game: soft velocity-aware framing on every
        // platform, so the camera leads the runner instead of trailing it.
        .with_presentation_profiles(profiles::high_speed_full_bleed())
        // The ring tally. One declared readout; the engine never learns what a
        // ring is — `publish_sanic_ring_readout` writes the word "RINGS".
        .with_hud(
            ambition_platformer2d::presentation::HudDeclaration::new()
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(RINGS_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_font_size(22.0)
                        .with_color([1.0, 0.85, 0.25, 1.0]),
                )
                // The clock of the act, beside the rings.
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(TIME_HUD_SLOT)
                        .with_region(ambition_platformer2d::presentation::SurroundRegion::Top)
                        .with_order(1)
                        .with_font_size(22.0)
                        .with_color([0.96, 0.97, 1.0, 1.0]),
                )
                // The name of the act, for its first seconds.
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(TITLE_HUD_SLOT)
                        .with_order(98)
                        .with_font_size(34.0)
                        .with_color([1.0, 1.0, 1.0, 1.0])
                        .centered(),
                )
                // The results card. Published only while the act is cleared, so it
                // needs no hide path — an unpublished slot draws nothing.
                .slot(
                    ambition_platformer2d::presentation::HudSlotSpec::new(RESULTS_HUD_SLOT)
                        .with_order(99)
                        .with_font_size(30.0)
                        .with_color([1.0, 0.94, 0.60, 1.0])
                        .centered(),
                ),
        )
        .with_defense_presentation(
            ambition_platformer2d::presentation::DefensePresentationPolicy::shared_iframe_blink(),
        )
        .install(app, sanic_prepared_session_world);
        app.add_systems(bevy::prelude::Update, publish_sanic_ring_readout);
        app.add_plugins(SanicRulesPlugin::hosted());
    }
}

/// The provider's authored speedway source for the shared preparation lifecycle.
fn sanic_prepared_session_world() -> PreparedPlatformerSource {
    let source = sanic_session_world();
    PreparedPlatformerSource::new(
        SANIC_EXPERIENCE,
        source.room_set,
        source.geometry,
        source.starting_character,
    )
}

/// The slot id Sanic's ring tally publishes into. Opaque to the engine.
pub const RINGS_HUD_SLOT: &str = "sanic_rings";

/// The end-of-act results card.
pub const RESULTS_HUD_SLOT: &str = "sanic_results";

/// The clock of the act.
pub const TIME_HUD_SLOT: &str = "sanic_time";

/// The name of the act, shown as it begins.
pub const TITLE_HUD_SLOT: &str = "sanic_title";

/// Publish the ring count into the declared HUD.
///
/// Rings are ordinary `currency:1` pickups: the shared economy credits the
/// collector's `BodyWallet`, and `PlayerHudFacts` republishes that balance
/// for the controlled subject every tick. This reads the fact, names it
/// "RINGS", and hands it to the slot.
fn publish_sanic_ring_readout(
    facts: bevy::prelude::Res<ambition_platformer2d::sim_view::PlayerHudFacts>,
    act: bevy::prelude::Query<&crate::SanicActState>,
    runner: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d::platformer::markers::PrimaryPlayer>,
    >,
    // The room the runner is in: its act has the name the title card shows.
    rooms: ambition_platformer2d::world::rooms::LiveRoomSpecs,
    mut readouts: bevy::prelude::ResMut<ambition_platformer2d::presentation::HudReadouts>,
) {
    if !facts.present {
        return;
    }
    readouts.set_labelled(RINGS_HUD_SLOT, crate::text::line("hud.rings"), facts.balance);

    // The clock: the act's own, which stops when the act is cleared.
    let running = act.iter().find_map(|state| match state.phase {
        crate::SanicActPhase::Running => Some(state.elapsed),
        crate::SanicActPhase::Cleared { .. } => None,
    });
    match running.or_else(|| {
        act.iter().find_map(|state| match state.phase {
            crate::SanicActPhase::Cleared { time, .. } => Some(time),
            crate::SanicActPhase::Running => None,
        })
    }) {
        Some(seconds) => readouts.set_labelled(TIME_HUD_SLOT, crate::text::line("hud.time"), crate::act_clock_text(seconds)),
        None => readouts.clear_slot(TIME_HUD_SLOT),
    }

    // The name of the act, while the act is young.
    let title = running
        .filter(|elapsed| *elapsed < crate::rules::sanic_rules().act.title_seconds)
        .and_then(|_| runner.single().ok())
        .and_then(|runner| rooms.spec_of(runner))
        .and_then(|spec| crate::act_title(&spec.id));
    match title {
        Some((name, number)) => readouts.set(
            TITLE_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(title_card(name, number)),
        ),
        None => readouts.clear_slot(TITLE_HUD_SLOT),
    }

    // The results card, published only while the act is cleared.
    match act.iter().find_map(|state| match state.phase {
        crate::SanicActPhase::Cleared { time, rings, .. } => Some((time, rings)),
        crate::SanicActPhase::Running => None,
    }) {
        Some((time, rings)) => readouts.set(
            RESULTS_HUD_SLOT,
            ambition_platformer2d::presentation::HudReadout::bare(results_card(time, rings)),
        ),
        None => readouts.clear_slot(RESULTS_HUD_SLOT),
    }
}

/// The card while an act is young: its name and its number.
pub fn title_card(name: &str, number: u8) -> String {
    crate::text::sanic_text().fill("card.act_title", &[hole("name", &name), hole("number", &number)])
}

/// The card when an act is cleared after `time` seconds with `rings` held.
pub fn results_card(time: f32, rings: i32) -> String {
    crate::text::sanic_text().fill(
        "card.act_clear",
        &[hole("time", &crate::act_time_text(time)), hole("rings", &rings), hole("score", &crate::act_score(time, rings))],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The act cards' words are his text file's; the code fills the act's
    /// name and number, and the time, rings and score of a clear.
    #[test]
    fn his_cards_say_what_his_text_file_states() {
        let text = crate::text::sanic_text();
        assert_eq!(title_card("COVE HIGHWAY", 2), text.fill("card.act_title", &[hole("name", &"COVE HIGHWAY"), hole("number", &2)]));
        let score = crate::act_score(65.0, 12);
        assert_eq!(
            results_card(65.0, 12),
            text.fill("card.act_clear", &[hole("time", &"1:05"), hole("rings", &12), hole("score", &score)])
        );
        assert!(results_card(65.0, 12).contains("1:05"), "the shipped card shows the time");
    }
}
