//! **TWO PEOPLE, TWO OBSERVERS, ONE MINKOWSKI SIMULATION.**
//!
//! TwinTrack's exhibit is a disagreement between observers, and for its whole
//! life only one of those observers was a person. The laboratory twin — the
//! frame every clock in the plaza is compared against — was a bare entity with a
//! clock and a worldline and no way to be driven: scenery standing in for the
//! second half of the argument.
//!
//! This module makes it a seat. The laboratory twin is Emmy No-Ether, built
//! through the same character construction any other body takes, wearing
//! `DrivingParticipant(PlayerSlot(1))` so a second controller steers her. The
//! screen is split permanently, one pane per participant, each resolved by the
//! engine's own per-view seams.
//!
//! ⭐ **NOTHING HERE IS CONDITIONAL ON A SECOND CONTROLLER, and that is the
//! design.** A seat with no pad reads neutral input — `assign_local_seat_devices`
//! clears an association it cannot satisfy rather than handing seat two player
//! one's controller — so with one pad Emmy simply stands still in the
//! laboratory, which is exactly what the at-rest twin did before she was
//! playable. Her pane keeps showing the plaza from where she is standing, and
//! watching it is the point: the exhibit compares two frames, and one of them
//! being unattended does not make it less of a frame.
//!
//! ⛔ **the split is NOT a view mode.** It used to be `TwinTrackViewMode::SplitObservers`,
//! reachable only by cycling an in-world console, which is why Jon never saw it.
//! Two participants are two views for as long as there are two participants; the
//! ordering diagram that shares the word "split" is still an instrument you
//! bring up, and it is a different thing.

use bevy::prelude::*;

use ambition_platformer2d::actor::{ActorConfig, ActorFaction, SpawnActorKind, SpawnActorRequest};
use ambition_platformer2d::characters::brain::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::relativity2d::{
    OpticalSource2d, ProperTimeElapsed, RelativisticClock2d, RelativisticObserver2d,
    RelativityClockLabel, WorldlineTracked2d,
};
use ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata;
use ambition_platformer2d::sim_view::{LocalView, LocalViewId, ViewPlacement, ViewSubject};

use crate::{
    LaboratoryTwin, TwinTrackExperiment, LAB_POS, TWINTRACK_EXPERIENCE,
    TWINTRACK_GAMEPLAY_ROUTE, TWINTRACK_LAB_TWIN_CHARACTER_ID,
};

/// The seat the laboratory twin holds. Slot 0 is the traveler's, authored by the
/// session's own avatar spawn.
pub const LAB_TWIN_SLOT: PlayerSlot = PlayerSlot(1);

/// How many local seats this experience offers. Fixed, because TwinTrack's
/// exhibit IS two observers — a one-seat TwinTrack would have nothing to
/// compare.
const TWINTRACK_SEATS: u8 = 2;

/// The construction id of the laboratory twin's body, which is also how the
/// adoption below finds it after the spawn road has built it.
const LAB_TWIN_FEATURE_ID: &str = "twintrack_laboratory";

/// The view each participant watches through, left to right.
const TRAVELER_VIEW: LocalViewId = LocalViewId::FIRST;
const LAB_TWIN_VIEW: LocalViewId = LocalViewId(1);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<ambition_platformer2d::input::DeclaredInputSeats>()
        .init_resource::<ambition_platformer2d::input::InputAssignmentPolicy>()
        // ⚠ **NEITHER is gated on the experience**, because half of each one's
        // job is RETIRING what it declared. A system that only ran while
        // TwinTrack was live would leave the second pane standing over Mary-O
        // and leave a couch policy on a solo player's spare controller.
        .add_systems(Update, (compose_the_panes, declare_the_couch))
        .add_systems(
            Update,
            frame_each_participant
                .run_if(ambition_platformer2d::runtime::in_mode(
                    TWINTRACK_EXPERIENCE,
                ))
                .after(compose_the_panes),
        );
}

/// The marker on the pane rig this demo owns, so retiring it takes exactly the
/// camera it spawned and never the host's.
#[derive(Component, Clone, Copy, Debug)]
struct TwinTrackPaneCamera;

/// **THE SECOND PANE EXISTS FOR AS LONG AS THE SECOND PARTICIPANT DOES.**
///
/// ⛔⛔ **and that is why it is not composed at plugin BUILD time.** A view
/// spawned there is a view the WHOLE HOST has: `ambition_app` links this crate
/// beside Mary-O, Smash and the launcher, so a build-time second view split the
/// screen of every route in the game and left the shared camera-spawn site with
/// no view it could honestly bind. The rule the build-time helpers state — a
/// view must exist before any schedule runs — is about the FIRST view, the one
/// every reader assumes; a view APPEARING is the ordinary couch-co-op event of
/// somebody joining, and it is allowed to happen when they do.
///
/// ⚠ **`spawn_local_view`'s facts, not a hand-built row.** A view missing one
/// component does not error — it simply stops matching the resolve's query, and
/// the pane freezes at the origin with nothing in the log.
fn compose_the_panes(
    mut commands: Commands,
    roots: Query<&ActiveRoomMetadata>,
    views: Query<(Entity, &LocalViewId, Option<&ViewPlacement>), With<LocalView>>,
    panes: Query<Entity, With<TwinTrackPaneCamera>>,
) {
    let live = roots
        .iter()
        .any(|metadata| metadata.0.mode.as_deref() == Some(TWINTRACK_EXPERIENCE));
    let mut seen: Vec<LocalViewId> = Vec::new();
    for (view, id, placement) in &views {
        seen.push(*id);
        if *id == TRAVELER_VIEW {
            let wanted = live.then(|| ViewPlacement::column(0, 2));
            if placement.copied() != wanted {
                match wanted {
                    // Removed rather than set to FULL: absent IS full, and a
                    // component nobody wrote is one fewer thing to keep true.
                    None => commands.entity(view).try_remove::<ViewPlacement>(),
                    Some(placement) => commands.entity(view).try_insert(placement),
                };
            }
        } else if *id == LAB_TWIN_VIEW && !live {
            commands.entity(view).despawn();
        }
    }
    if !live {
        for pane in &panes {
            commands.entity(pane).despawn();
        }
        return;
    }
    if seen.contains(&LAB_TWIN_VIEW) {
        return;
    }
    let view = commands
        .spawn((
            LocalView,
            LAB_TWIN_VIEW,
            ambition_platformer2d::sim_view::local_view_facts(),
            ViewPlacement::column(1, 2),
        ))
        .id();
    spawn_pane_camera(&mut commands, view);
}

/// **THE RIG FOR THE SECOND PANE, bound to the view it presents.**
///
/// ⭐ **the shared presentation plugin's rig is untouched and still binds the
/// first view.** `spawn_main_camera` runs at `Startup`, when TwinTrack's session
/// has not begun and there is exactly one view to bind — so the host keeps its
/// gameplay camera, its front HUD camera, its room visuals and its sprite chain,
/// and this composition adds precisely the one rig the engine could not have
/// known about. Nothing is spawned and later deleted.
///
/// ⚠ **the rig is the caller's, deliberately** — see `compose_local_views`, whose
/// contract this follows. What a camera IS (layers, projection, order) is a
/// composition decision; only the `PresentsView` link is engine vocabulary.
#[cfg(feature = "visible")]
fn spawn_pane_camera(commands: &mut Commands, view: Entity) {
    use ambition_platformer2d::platformer::camera_layers::{MainCamera, PARALLAX_BACKGROUND_LAYER};

    commands.spawn((
        TwinTrackPaneCamera,
        Camera2d,
        Camera {
            // Above the host's gameplay rig (0) and below the observatory (4, 5),
            // the ordering exhibit (6, 7) and the front HUD (9).
            order: 1,
            ..default()
        },
        MainCamera,
        bevy::camera::visibility::RenderLayers::layer(0).with(PARALLAX_BACKGROUND_LAYER),
        ambition_platformer2d::sim_view::PresentsView(view),
        Name::new("TwinTrack laboratory pane camera"),
    ));
}

/// Headless builds draw nothing, so the second pane is a view and no rig — which
/// is exactly what the integration suite measures.
#[cfg(not(feature = "visible"))]
fn spawn_pane_camera(_commands: &mut Commands, _view: Entity) {}

/// **Say that two people may play, and say which devices they are holding.**
///
/// A gameplay seat is normally declared by a match roster — deliberately, so a
/// controller left plugged into a machine does not silently become a second
/// player. TwinTrack has no match and no roster; what it has is an EXPERIENCE
/// whose exhibit is two observers, which is the same kind of statement a
/// character-select surface makes while it is up.
///
/// ⛔⛔ **AND THE SEAT COUNT ALONE IS NOT ENOUGH — measured on real hardware.**
/// Jon, 2026-08-20: *"I have a keyboard and controller hooked up to twin track,
/// but they both control patent clerk, neither controls emmy."* Two seats
/// existed and seat one was inert, because the default assignment policy is
/// `UnifiedPrimary` — *"Keyboard, gamepads and everything else drive the PRIMARY
/// participant"* — which is right for solo play and hands the only pad to the
/// seat that already has the keyboard.
///
/// ⇒ `JoinToClaim` is the couch statement: the keyboard stays with the seat that
/// has been playing, and an unclaimed pad becomes player two. A keyboard and one
/// controller are two people at TwinTrack, which is the whole point of the
/// exhibit.
///
/// ⚠ **and the headless suite could not have caught it.** The integration tests
/// build without the `input` feature, so they have no `InputParticipant`s, no
/// devices and no assignment pass to be wrong — they drive `SlotControls`
/// directly. That is why the declaration is asserted where a test CAN read it,
/// and why the mechanism it depends on is pinned upstream in
/// `ambition_input::local_seats`
/// (`a_single_pad_beside_a_keyboard_player_drives_the_second_seat`).
fn declare_the_couch(
    router: Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut seats: ResMut<ambition_platformer2d::input::DeclaredInputSeats>,
    mut policy: ResMut<ambition_platformer2d::input::InputAssignmentPolicy>,
    // **HOW MANY LOCAL CHANNELS THE SESSION OPENS.** The third half of the
    // couch, and the one whose absence made the other two useless — see the
    // block comment below.
    mut seating: ResMut<ambition_platformer2d::input::SessionSeatingSource>,
    // ⛔⛔ **WHETHER THIS EXPERIENCE IS THE ONE HOLDING THE CLAIM.** Both
    // resources are process-global and TwinTrack is one route in a host that
    // also runs Mary-O, Smash and a launcher — so "not live ⇒ write the
    // default" is a demo plugin stamping a global while another game owns the
    // screen. It is not hypothetical: writing `UnifiedPrimary` unconditionally
    // retracted SMASH's couch policy on every frame of every smash match, and
    // its select screen then offered one seat to two people
    // (`smash_in_the_host::two_participants_start_a_match_and_can_still_pause_it`).
    //
    // A claim is released by whoever made it, exactly once, and only if the
    // value is still the one that was claimed.
    mut claimed: Local<bool>,
) {
    // ⛔⛔ **THE ROUTE, NOT THE ROOM, and the difference is a whole frame that
    // matters.** This asked `ActiveRoomMetadata`, which exists only once the
    // plaza has been CONSTRUCTED — and the rollback session is sized the moment
    // a gameplay world appears, which is the same tick. A claim that lands with
    // the construction cannot precede it. The route is what the launcher
    // switched, it is live before any of the plaza exists, and it is the same
    // authority Smash's select screen reads for the same reason.
    let live = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == TWINTRACK_GAMEPLAY_ROUTE);
    let couch = ambition_platformer2d::input::InputAssignmentPolicy::JoinToClaim;
    if live {
        let wanted = ambition_platformer2d::input::DeclaredInputSeats(TWINTRACK_SEATS);
        // Written only on a change: this is read by a system that spawns and
        // despawns seat entities.
        if *seats != wanted {
            *seats = wanted;
        }
        if *policy != couch {
            *policy = couch;
        }
        // ⛔⛔ **AND THE TWO ABOVE ARE STILL NOT ENOUGH — measured a second time,
        // on the same hardware.** Jon, 2026-08-20: *"in twin track I still
        // cannot control emmy with the game pad."* Two seats existed, seat one
        // held the only controller, and the laboratory twin was inert — because
        // the SHIPPED host is a rollback host, where a seat's frame is published
        // by the GGRS session from the handles that session opened. Nothing had
        // told it to open two, so it sized itself from connected DEVICES (one
        // pad ⇒ one handle) and a GGRS session is never resized afterwards.
        //
        // ⭐ **the plan, not a count.** Which SOURCE drives each channel is the
        // half a number cannot carry: the traveler is on the keyboard and the
        // laboratory twin on the first pad, and a bare `2` leaves every consumer
        // to re-derive that and disagree.
        //
        // ⚠ **it does not go through `pending` first.** That state is for a
        // surface whose answer is not known yet — a lobby waiting on people to
        // pick. TwinTrack's exhibit IS two observers, so the answer exists
        // before the route opens and holding the session for it would be a stall
        // with nothing at the end of it.
        let plan = ambition_platformer2d::input::LocalChannelPlan::from_sources([
            ambition_platformer2d::input::LocalInputSource::Keyboard,
            ambition_platformer2d::input::LocalInputSource::FIRST_PAD,
        ]);
        if seating.channel_plan() != Some(&plan) {
            *seating = ambition_platformer2d::input::SessionSeatingSource::decided(
                TWINTRACK_EXPERIENCE,
                plan,
            );
        }
        *claimed = true;
        return;
    }
    if !*claimed {
        return;
    }
    *claimed = false;
    if seats.0 == TWINTRACK_SEATS {
        *seats = ambition_platformer2d::input::DeclaredInputSeats(0);
    }
    if *policy == couch {
        *policy = ambition_platformer2d::input::InputAssignmentPolicy::default();
    }
    // Same rule, and the type enforces it: `release` is a no-op on a claim that
    // is not this owner's.
    seating.release(TWINTRACK_EXPERIENCE);
}

/// **Each pane watches its own participant.**
///
/// ⚠ the traveler's pane names NO subject on purpose. A view with no
/// `ViewSubject` frames the session's controlled body, which is what seat zero's
/// is — including while that seat is possessing something else. Naming it here
/// would be a second answer to a question the engine already answers, and the
/// two would disagree the moment possession moved the seat.
fn frame_each_participant(
    mut commands: Commands,
    views: Query<(Entity, &LocalViewId, Option<&ViewSubject>), With<LocalView>>,
    twin: Query<Entity, With<LaboratoryTwin>>,
) {
    let Ok(twin) = twin.single() else {
        return;
    };
    for (view, id, subject) in &views {
        if *id != LAB_TWIN_VIEW {
            continue;
        }
        // Compared before writing: an unconditional insert marks the component
        // changed every frame for anything gated on `is_changed()`.
        if subject.map(|subject| subject.0) != Some(twin) {
            commands.entity(view).insert(ViewSubject(twin));
        }
    }
}

/// The request that builds the laboratory twin's body.
///
/// ⭐ **through the actor construction road, not by hand.** Every other body in
/// this plaza is a bare entity the demo assembles itself, which is why none of
/// them can be steered: a hand-built entity has no movement clusters, so nothing
/// integrates the intent a participant produces. A constructed character has
/// them, wears its own art, and is driven by the same `DrivingParticipant` →
/// `SlotControls` path the traveler is.
pub(crate) fn laboratory_twin_request() -> SpawnActorRequest {
    SpawnActorRequest {
        id: LAB_TWIN_FEATURE_ID.to_owned(),
        name: "Emmy No-Ether".to_owned(),
        pos: LAB_POS,
        half_size: ae::Vec2::splat(24.0),
        faction: ActorFaction::Npc,
        grudge_against: None,
        kind: SpawnActorKind::Enemy {
            brain: ambition_platformer2d::character::CharacterBrain::Passive,
            character: ambition_platformer2d::character::CharacterId::from(
                TWINTRACK_LAB_TWIN_CHARACTER_ID,
            ),
        },
    }
}

/// **Adopt the constructed body as the laboratory twin.**
///
/// ⚠ **a separate system because construction is a MESSAGE.** The request is
/// drained by the engine's spawn applier, so the body does not exist on the tick
/// the session asks for it. This runs until it finds one and then never matches
/// again — the clock facts, the worldline and the seat are inserted exactly once.
pub(crate) fn adopt_the_laboratory_twin(
    mut commands: Commands,
    // **The plaza's own clock**, so the twin's starts where the plaza's is
    // rather than at zero — see the `ProperTimeElapsed` line below.
    coordinate_time: Query<&ambition_platformer2d::relativity2d::SpacetimeCoordinateTime2d>,
    already: Query<(), With<LaboratoryTwin>>,
    candidates: Query<(Entity, &ActorConfig), Without<LaboratoryTwin>>,
) {
    if !already.is_empty() {
        return;
    }
    let Some((body, _)) = candidates
        .iter()
        .find(|(_, config)| config.id == LAB_TWIN_FEATURE_ID)
    else {
        return;
    };
    commands.entity(body).insert((
        LaboratoryTwin,
        RelativisticClock2d,
        RelativityClockLabel("laboratory".to_owned()),
        WorldlineTracked2d::new("laboratory"),
        OpticalSource2d::new("laboratory", 180.0, 1.0, 18.0),
        // ⭐⭐ **SHE SEES, she is not only seen.** An `OpticalSource2d` is what
        // OTHER observers receive light FROM; this is what makes the laboratory
        // twin an observer in her own right, with her own retarded image of
        // every source and her own null intercepts — the second half of an
        // exhibit whose whole claim is that two observers disagree.
        //
        // ⛔ **it is also what makes every `Deref` read of those two resources a
        // lie**, because "laboratory" sorts before "traveler" and the first row
        // is now hers. Every TwinTrack consumer names its observer explicitly;
        // adding this without that would have silently redrawn the traveler's
        // instruments from the lab twin's eyes.
        RelativisticObserver2d("laboratory".to_owned()),
        // ⛔ **NOT `ZERO`, and the difference is the whole reference frame.**
        // The laboratory twin is at rest in the laboratory, so her proper time
        // IS the plaza's coordinate time — that identity is what every other
        // clock in the exhibit is compared against. Starting her at zero on the
        // tick her body was built says the reference clock was created late, and
        // every light-delay reading taken against it is then short by exactly
        // how long construction took.
        ProperTimeElapsed {
            seconds: coordinate_time
                .iter()
                .next()
                .map_or(0.0, |clock| clock.seconds),
        },
        TwinTrackExperiment::default(),
        // **THE SEAT.** Everything a person does with this body follows from
        // this one component: `tick_controlled_brains` reads `SlotControls[1]`
        // through it, and the actor tick declines to decide for a body that
        // holds one.
        DrivingParticipant(LAB_TWIN_SLOT),
        // The plaza has no gravity and no floor; the twin flies for the same
        // reason the traveler does.
        ae::BodyFlightState {
            fly_enabled: true,
            ..default()
        },
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::input::{DeclaredInputSeats, InputAssignmentPolicy};

    /// ⚠ **the ROUTER is the fixture now, not a hand-spawned room.** The claim
    /// keys off the route (see `declare_the_couch`), and a fixture that
    /// manufactured room metadata would be testing a question the system no
    /// longer asks.
    fn couch_app(live: bool) -> App {
        use ambition_platformer2d::game_shell::{
            ActiveShellExperience, ShellActivationId, ShellExperienceId, ShellRouteId, ShellRouter,
        };
        let mut app = App::new();
        app.init_resource::<DeclaredInputSeats>()
            .init_resource::<InputAssignmentPolicy>()
            .init_resource::<ambition_platformer2d::input::SessionSeatingSource>()
            .init_resource::<ShellRouter>()
            .add_systems(Update, declare_the_couch);
        if live {
            app.world_mut().resource_mut::<ShellRouter>().active = Some(ActiveShellExperience {
                activation_id: ShellActivationId(1),
                route_id: ShellRouteId::new(TWINTRACK_GAMEPLAY_ROUTE),
                experience_id: ShellExperienceId::new(TWINTRACK_EXPERIENCE),
                parameters: Default::default(),
                load_authorization: None,
                prepared_session: None,
            });
        }
        app
    }

    fn seating_owner(app: &App) -> Option<String> {
        app.world()
            .resource::<ambition_platformer2d::input::SessionSeatingSource>()
            .owner()
            .map(str::to_owned)
    }

    /// The launcher takes the route back.
    fn leave_the_plaza(app: &mut App) {
        app.world_mut()
            .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
            .active = None;
    }

    fn couch(app: &App) -> (DeclaredInputSeats, InputAssignmentPolicy) {
        (
            *app.world().resource::<DeclaredInputSeats>(),
            *app.world().resource::<InputAssignmentPolicy>(),
        )
    }

    /// **⛔⛔ A DEMO PLUGIN MAY NOT RETRACT A CLAIM IT DID NOT MAKE.**
    ///
    /// Both of these are process-globals and `ambition_app` links this crate
    /// beside Mary-O, Smash and the launcher. The first version wrote the
    /// DEFAULTS on every frame TwinTrack was not live — which retracted Smash's
    /// couch policy on every frame of every smash match, and its select screen
    /// then offered one seat to two people
    /// (`app_it::smash_in_the_host::two_participants_start_a_match_and_can_still_pause_it`).
    ///
    /// ⚠ **and the plaza is ALWAYS live in its own standalone app**, which is
    /// why this is a unit over the system rather than a run of the demo: the
    /// dormant case does not exist there and the regression was invisible until
    /// another game was in the process.
    #[test]
    fn a_dormant_plaza_leaves_another_surfaces_couch_alone() {
        let mut app = couch_app(false);
        app.insert_resource(DeclaredInputSeats(4));
        app.insert_resource(InputAssignmentPolicy::JoinToClaim);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            couch(&app),
            (DeclaredInputSeats(4), InputAssignmentPolicy::JoinToClaim),
            "a dormant TwinTrack wrote over a couch somebody else was holding",
        );
    }

    /// **A LIVE PLAZA CLAIMS BOTH, and a count alone would not be enough.**
    ///
    /// `DeclaredInputSeats(2)` gets seat one an `InputParticipant`; it does NOT
    /// get it a device, because `UnifiedPrimary` means every local source drives
    /// the primary participant. Jon measured exactly that on hardware.
    #[test]
    fn a_live_plaza_claims_two_seats_and_the_couch_policy() {
        let mut app = couch_app(true);
        app.update();
        assert_eq!(
            couch(&app),
            (
                DeclaredInputSeats(TWINTRACK_SEATS),
                InputAssignmentPolicy::JoinToClaim
            ),
        );
    }

    /// **⛔⛔ AND THE THIRD CLAIM, WHICH IS THE ONE THAT WAS MISSING.**
    ///
    /// The two above were both landing and Jon still could not drive the
    /// laboratory twin (2026-08-20). A rollback host publishes a seat's frame
    /// from the GGRS handles its SESSION opened, and that session sizes itself
    /// once — from connected devices, unless somebody declares otherwise — and is
    /// never resized. So a plaza that declared two seats into a one-handle
    /// session had a second participant holding a controller and no way for its
    /// input to reach the simulation.
    ///
    /// ⚠ **the PLAN, not the count.** `channels()` alone would pass while
    /// naming the wrong device for each seat, which is the failure one layer up
    /// that `LocalChannelPlan` exists for.
    #[test]
    fn a_live_plaza_declares_which_source_drives_each_channel() {
        use ambition_platformer2d::input::LocalInputSource;
        let mut app = couch_app(true);
        app.update();
        let source = app
            .world()
            .resource::<ambition_platformer2d::input::SessionSeatingSource>()
            .clone();
        assert_eq!(source.owner(), Some(TWINTRACK_EXPERIENCE));
        assert_eq!(
            source.channel_plan().map(|plan| plan.sources().to_vec()),
            Some(vec![LocalInputSource::Keyboard, LocalInputSource::FIRST_PAD]),
            "the plaza did not tell the session that its two channels are the \
             keyboard and the first pad; a rollback session then sizes itself \
             from connected devices and seat one is inert for the whole visit",
        );
        assert_eq!(
            source.seat_count(),
            Some(TWINTRACK_SEATS as usize),
            "the declared channel count and the declared seat count disagree",
        );
    }

    /// **A DORMANT PLAZA DECLARES NO SEATING, so every single-player
    /// composition still seats from what is plugged in.**
    ///
    /// ⛔ the falsifier that matters is not the plaza's own claim — it is every
    /// OTHER route in the host. A declaration left standing sizes the next
    /// game's session, and a session is never resized.
    #[test]
    fn a_dormant_plaza_declares_no_seating_and_gives_its_claim_back() {
        let mut app = couch_app(false);
        app.update();
        assert_eq!(seating_owner(&app), None);

        let mut app = couch_app(true);
        app.update();
        assert_eq!(seating_owner(&app), Some(TWINTRACK_EXPERIENCE.to_owned()));
        leave_the_plaza(&mut app);
        app.update();
        assert_eq!(
            seating_owner(&app),
            None,
            "leaving the plaza left its seating declaration standing, so the next \
             game's session is sized by an exhibit that has ended",
        );
    }

    /// **⛔ AND IT GIVES BACK ONLY ITS OWN.** `release` is a no-op on a stranger's
    /// claim, and this pins that the plaza routes through it rather than
    /// resetting the resource.
    #[test]
    fn leaving_the_plaza_leaves_another_surfaces_seating_alone() {
        use ambition_platformer2d::input::{LocalChannelPlan, LocalInputSource, SessionSeatingSource};
        let mut app = couch_app(true);
        app.update();
        leave_the_plaza(&mut app);
        let theirs = SessionSeatingSource::decided(
            "smash",
            LocalChannelPlan::from_sources([LocalInputSource::Pad(0), LocalInputSource::Pad(1)]),
        );
        app.insert_resource(theirs.clone());
        app.update();
        assert_eq!(
            *app.world().resource::<SessionSeatingSource>(),
            theirs,
            "the plaza's release took a seating declaration that was not its own",
        );
    }

    /// **THE RELEASE UNDOES ITS OWN VALUE AND NOTHING ELSE.**
    ///
    /// ⛔ the falsifier is the value written BETWEEN: somebody else's claim
    /// arrives while TwinTrack still holds the latch, and the release must find
    /// a value that is no longer its own and leave it there.
    #[test]
    fn leaving_the_plaza_restores_only_what_it_claimed() {
        let mut app = couch_app(true);
        app.update();
        // The session ends.
        leave_the_plaza(&mut app);
        app.update();
        assert_eq!(
            couch(&app),
            (DeclaredInputSeats(0), InputAssignmentPolicy::UnifiedPrimary),
            "leaving the plaza left its couch behind for the next game",
        );

        // Now the same run again, but somebody else takes both over first.
        let mut app = couch_app(true);
        app.update();
        leave_the_plaza(&mut app);
        app.insert_resource(DeclaredInputSeats(4));
        app.update();
        assert_eq!(
            couch(&app),
            (DeclaredInputSeats(4), InputAssignmentPolicy::UnifiedPrimary),
            "the release retracted a seat offer that was no longer TwinTrack's",
        );
    }
}
