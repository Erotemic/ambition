//! TwinTrack models two participants as two observers in one simulation.
//!
//! The traveler occupies slot 0 and the laboratory twin occupies slot 1. Each
//! participant owns a permanent local view. If slot 1 has no controller it reads
//! neutral input; the second observer and view still remain part of the exhibit.

use bevy::prelude::*;

use ambition_platformer2d::actor::{ActorConfig, ActorFaction, SpawnActorKind, SpawnActorRequest};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::relativity2d::{
    OpticalSource2d, ProperTimeElapsed, RelativisticClock2d, RelativisticObserver2d,
    RelativityClockLabel, WorldlineTracked2d,
};
use ambition_platformer2d::runtime::demo_fixture::ActiveRoomMetadata;
use ambition_platformer2d::sim_view::{LocalView, LocalViewId, ViewParticipant, ViewPlacement};

use crate::{
    LaboratoryTwin, TwinTrackExperiment, LAB_POS, TWINTRACK_EXPERIENCE, TWINTRACK_GAMEPLAY_ROUTE,
    TWINTRACK_LAB_TWIN_CHARACTER_ID,
};

/// The seat the laboratory twin holds. Slot 0 is the traveler's, authored by the
/// session's own avatar spawn.
pub const LAB_TWIN_SLOT: PlayerSlot = PlayerSlot(1);

/// How many local seats this experience offers.
const TWINTRACK_SEATS: u8 = 2;

/// The construction id of the laboratory twin's body, which is also how the
/// adoption below finds it after the spawn road has built it.
const LAB_TWIN_FEATURE_ID: &str = "twintrack_laboratory";

/// The view each participant watches through, left to right.
const TRAVELER_VIEW: LocalViewId = LocalViewId::FIRST;
const LAB_TWIN_VIEW: LocalViewId = LocalViewId(1);

pub(crate) fn install(app: &mut App) {
    app.init_resource::<ambition_platformer2d::input::LocalSeatOffer>()
        // This demo may compose without the host input group, so initialize
        // the seating resource required by its systems here.
        .init_resource::<ambition_platformer2d::input::SessionSeatingSource>()
        // These systems also retire TwinTrack-owned view/seating state, so
        // they must keep running after the experience stops being active.
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

/// THE SECOND PANE EXISTS FOR AS LONG AS THE SECOND PARTICIPANT DOES.
///
/// and that is why it is not composed at plugin BUILD time. A view
/// spawned there is a view the WHOLE HOST has: `ambition_app` links this crate
/// beside Mary-O, Smash and the launcher, so a build-time second view split the
/// screen of every route in the game and left the shared camera-spawn site with
/// no view it could honestly bind. The rule the build-time helpers state — a
/// view must exist before any schedule runs — is about the FIRST view, the one
/// every reader assumes; a view APPEARING is the ordinary couch-co-op event of
/// somebody joining, and it is allowed to happen when they do.
///
/// `spawn_local_view`'s facts, not a hand-built row. A view missing one
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

/// THE RIG FOR THE SECOND PANE, bound to the view it presents.
///
/// the shared presentation plugin's rig is untouched and still binds the
/// first view. `spawn_main_camera` runs at `Startup`, when TwinTrack's session
/// has not begun and there is exactly one view to bind — so the host keeps its
/// gameplay camera, its front HUD camera, its room visuals and its sprite chain,
/// and this composition adds precisely the one rig the engine could not have
/// known about. Nothing is spawned and later deleted.
///
/// the rig is the caller's, deliberately — see `compose_local_views`, whose
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

/// Claim TwinTrack's two local seats and their device-to-channel plan.
///
/// The exhibit has no match roster, so it owns a `JoinToClaim` seat offer and a
/// decided channel plan directly: keyboard drives one observer and the first pad
/// drives the other. Both claims are experience-owned so another route cannot be
/// overwritten or released by value coincidence.
fn declare_the_couch(
    router: Res<ambition_platformer2d::game_shell::ShellRouter>,
    mut offer: ResMut<ambition_platformer2d::input::LocalSeatOffer>,
    // Decided local channels must exist before rollback session sizing.
    mut seating: ResMut<ambition_platformer2d::input::SessionSeatingSource>,
    // Seat/channel resources are process-global; only their owning experience may release them.
) {
    // Route state is available before room construction and rollback session sizing.
    let live = router
        .active
        .as_ref()
        .is_some_and(|active| active.route_id.as_str() == TWINTRACK_GAMEPLAY_ROUTE);
    let couch = ambition_platformer2d::input::InputAssignmentPolicy::JoinToClaim;
    if live {
        // Matching values still need TwinTrack ownership before they can be relied on.
        if !offer.is_owned_by(TWINTRACK_EXPERIENCE)
            || offer.seats() != TWINTRACK_SEATS
            || offer.policy() != couch
        {
            offer.claim(TWINTRACK_EXPERIENCE, TWINTRACK_SEATS, couch);
        }
        // The source plan, not just a count, is known before this fixed two-observer route opens.
        let plan = ambition_platformer2d::input::LocalChannelPlan::from_sources([
            ambition_platformer2d::input::LocalInputSource::Keyboard,
            ambition_platformer2d::input::LocalInputSource::FIRST_PAD,
        ]);
        // Matching channel values still need TwinTrack ownership.
        if !seating.is_owned_by(TWINTRACK_EXPERIENCE) || seating.channel_plan() != Some(&plan) {
            *seating = ambition_platformer2d::input::SessionSeatingSource::decided(
                TWINTRACK_EXPERIENCE,
                plan,
            );
        }
        return;
    }
    // Release is owner-checked and therefore idempotent while the route is inactive.
    offer.release(TWINTRACK_EXPERIENCE);
    seating.release(TWINTRACK_EXPERIENCE);
}

/// Each pane watches its own participant — and now it says so.
///
/// the traveler's pane names NOTHING on purpose. A view that names neither a
/// subject nor a participant frames the session's controlled body, which is what
/// seat zero's is — including while that seat is possessing something else.
/// Naming it here would be a second answer to a question the engine already
/// answers, and the two would disagree the moment possession moved the seat.
///
/// the two resolve to the same entity today, because the twin is what
/// carries `DrivingParticipant(LAB_TWIN_SLOT)`. That is what makes this safe to
/// land; it is not what makes it right.
fn frame_each_participant(
    mut commands: Commands,
    views: Query<(Entity, &LocalViewId, Option<&ViewParticipant>), With<LocalView>>,
) {
    for (view, id, participant) in &views {
        if *id != LAB_TWIN_VIEW {
            continue;
        }
        // Compared before writing: an unconditional insert marks the component
        // changed every frame for anything gated on `is_changed()`.
        if participant.map(|participant| participant.0) != Some(LAB_TWIN_SLOT) {
            commands.entity(view).insert(ViewParticipant(LAB_TWIN_SLOT));
        }
    }
}

/// The request that builds the laboratory twin's body.
///
/// through the actor construction road, not by hand. Every other body in
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

/// Adopt the constructed body as the laboratory twin.
///
/// a separate system because construction is a MESSAGE. The request is
/// drained by the engine's spawn applier, so the body does not exist on the tick
/// the session asks for it. This runs until it finds one and then never matches
/// again — the clock facts, the worldline and the seat are inserted exactly once.
pub(crate) fn adopt_the_laboratory_twin(
    mut commands: Commands,
    // The plaza's own clock, so the twin's starts where the plaza's is
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
        // SHE SEES, she is not only seen. An `OpticalSource2d` is what
        // OTHER observers receive light FROM; this is what makes the laboratory
        // twin an observer in her own right, with her own retarded image of
        // every source and her own null intercepts — the second half of an
        // exhibit whose whole claim is that two observers disagree.
        //
        // it is also what makes every `Deref` read of those two resources a
        // lie, because "laboratory" sorts before "traveler" and the first row
        // is now hers. Every TwinTrack consumer names its observer explicitly;
        // adding this without that would have silently redrawn the traveler's
        // instruments from the lab twin's eyes.
        RelativisticObserver2d("laboratory".to_owned()),
        // NOT `ZERO`, and the difference is the whole reference frame.
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
        // THE SEAT. Everything a person does with this body follows from
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

/// Put the laboratory twin back on her mark, once, the tick after she is adopted.
///
/// ⛔⛔ ONE STRAY STEP, AND IT IS PERMANENT. `adopt_the_laboratory_twin` QUEUES
/// `DrivingParticipant`; the insert does not land until the commands flush, so
/// exactly one tick of her life is spent as a seatless `Passive` NPC — which the
/// engine calls an "undescribed-pool STROLLER". Measured: she takes a single
/// stroll step worth -96 px/s and drag bleeds it over seven ticks into a
/// permanent 6.16px offset. (A twin that is never adopted keeps accelerating to
/// the -540 cap, which is what proves the seat is doing its job the moment it
/// arrives.)
///
/// ⭐ SO THIS RUNS ON `Added`, NOT INSIDE THE ADOPTION. Correcting her in the
/// same system samples the wrong moment: it reads a body that has not taken the
/// step yet and puts back a position she has not left. `Added<LaboratoryTwin>`
/// first matches after the flush, with the stray step already integrated.
///
/// ⭐ X AND VELOCITY ONLY, and the asymmetry is the physics. Her `x` is a
/// PREMISE — the beacons sit symmetrically about `LAB_POS.x` and the simultaneity
/// exhibit IS the claim that she is equidistant from both. Her `y` is the BODY's:
/// construction resolves a standing centre 3.98px above the authored value with
/// zero `y` velocity, before she has taken a step, and forcing that back would be
/// arguing with the body model over a number the beacons' symmetry ignores.
pub(crate) fn restore_the_laboratory_twins_mark(
    mut twin: Query<&mut ae::BodyKinematics, Added<LaboratoryTwin>>,
) {
    for mut kin in &mut twin {
        kin.pos.x = LAB_POS.x;
        kin.vel = ae::Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::input::{InputAssignmentPolicy, LocalSeatOffer};

    /// the ROUTER is the fixture now, not a hand-spawned room. The claim
    /// keys off the route (see `declare_the_couch`), and a fixture that
    /// manufactured room metadata would be testing a question the system no
    /// longer asks.
    fn couch_app(live: bool) -> App {
        use ambition_platformer2d::game_shell::{
            ActiveShellExperience, ShellActivationId, ShellExperienceId, ShellRouteId, ShellRouter,
        };
        let mut app = App::new();
        app.init_resource::<LocalSeatOffer>()
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

    fn couch(app: &App) -> (u8, InputAssignmentPolicy, Option<String>) {
        let offer = app.world().resource::<LocalSeatOffer>();
        (
            offer.seats(),
            offer.policy(),
            offer.owner().map(str::to_owned),
        )
    }

    /// A claim some other surface is holding, with the values it chose.
    fn someone_elses(seats: u8, policy: InputAssignmentPolicy) -> LocalSeatOffer {
        LocalSeatOffer::offered("another surface", seats, policy)
    }

    /// A demo plugin may not retract participant policy it does not own.
    /// TwinTrack shares a process with other experiences, so it writes policy only
    /// while TwinTrack is active.
    #[test]
    fn a_dormant_plaza_leaves_another_surfaces_couch_alone() {
        let mut app = couch_app(false);
        app.insert_resource(someone_elses(4, InputAssignmentPolicy::JoinToClaim));
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            couch(&app),
            (
                4,
                InputAssignmentPolicy::JoinToClaim,
                Some("another surface".to_owned())
            ),
            "a dormant TwinTrack wrote over a couch somebody else was holding",
        );
    }

    /// A LIVE PLAZA CLAIMS THE OFFER, and a count alone would not be enough.
    #[test]
    fn a_live_plaza_claims_two_seats_and_the_couch_policy() {
        let mut app = couch_app(true);
        app.update();
        assert_eq!(
            couch(&app),
            (
                TWINTRACK_SEATS,
                InputAssignmentPolicy::JoinToClaim,
                Some(TWINTRACK_EXPERIENCE.to_owned())
            ),
        );
    }

    /// AND THE THIRD CLAIM, WHICH IS THE ONE THAT WAS MISSING.
    ///
    /// A rollback host publishes a seat's frame from the GGRS handles its SESSION opened, and that
    /// session sizes itself once — from connected devices, unless somebody declares otherwise — and
    /// is never resized. So a plaza that declared two seats into a one-handle session had a second
    /// participant holding a controller and no way for its input to reach the simulation.
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
            Some(vec![
                LocalInputSource::Keyboard,
                LocalInputSource::FIRST_PAD
            ]),
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

    /// A DORMANT PLAZA DECLARES NO SEATING, so every single-player
    /// composition still seats from what is plugged in.
    ///
    /// the falsifier that matters is not the plaza's own claim — it is every
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

    /// AND IT GIVES BACK ONLY ITS OWN. `release` is a no-op on a stranger's
    /// claim, and this pins that the plaza routes through it rather than
    /// resetting the resource.
    #[test]
    fn leaving_the_plaza_leaves_another_surfaces_seating_alone() {
        use ambition_platformer2d::input::{
            LocalChannelPlan, LocalInputSource, SessionSeatingSource,
        };
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

    /// THE RELEASE UNDOES ITS OWN CLAIM AND NOTHING ELSE.
    ///
    /// the falsifier is the value written BETWEEN: somebody else's claim
    /// arrives while TwinTrack is still the one that made the last one, and the
    /// release must find a claim that is no longer its own and leave it there.
    #[test]
    fn leaving_the_plaza_restores_only_what_it_claimed() {
        let mut app = couch_app(true);
        app.update();
        // The session ends.
        leave_the_plaza(&mut app);
        app.update();
        assert_eq!(
            couch(&app),
            (0, InputAssignmentPolicy::UnifiedPrimary, None),
            "leaving the plaza left its couch behind for the next game",
        );

        // Now the same run again, but somebody else takes it over first.
        let mut app = couch_app(true);
        app.update();
        leave_the_plaza(&mut app);
        app.insert_resource(someone_elses(4, InputAssignmentPolicy::UnifiedPrimary));
        app.update();
        assert_eq!(
            couch(&app),
            (
                4,
                InputAssignmentPolicy::UnifiedPrimary,
                Some("another surface".to_owned())
            ),
            "the release retracted a seat offer that was no longer TwinTrack's",
        );
    }

    /// A SUCCESSOR THAT WANTS THE SAME NUMBERS IS STILL A DIFFERENT OWNER.
    ///
    /// ```text
    /// if seats == 2      { seats = 0 }
    /// if policy == couch { policy = default }
    /// ```
    ///
    /// so a route that independently arrived at TwinTrack's own two-seat couch —
    /// the most likely successor there is, since two-player couch is one
    /// configuration and not a rare one — had its claim wiped by an exhibit that
    /// had already ended. Value equality cannot tell "still mine" from "the same
    /// answer somebody else reached".
    #[test]
    fn a_successor_claiming_the_very_same_couch_keeps_it() {
        let mut app = couch_app(true);
        app.update();
        leave_the_plaza(&mut app);
        // Identical numbers, different owner.
        app.insert_resource(someone_elses(
            TWINTRACK_SEATS,
            InputAssignmentPolicy::JoinToClaim,
        ));
        app.update();
        assert_eq!(
            couch(&app),
            (
                TWINTRACK_SEATS,
                InputAssignmentPolicy::JoinToClaim,
                Some("another surface".to_owned())
            ),
            "TwinTrack's teardown erased a successor whose only mistake was \
             wanting the same two seats and the same couch policy",
        );
    }

    /// AND TAKING OVER AN OFFER THAT ALREADY READS RIGHT STILL MAKES YOU THE
    /// OWNER.
    ///
    /// A live plaza that skipped the write because the values already matched would leave the
    /// claim in the previous surface's name — and that surface's own teardown would then
    /// withdraw the seats TwinTrack is relying on.
    #[test]
    fn a_live_plaza_takes_ownership_of_an_offer_that_already_reads_right() {
        let mut app = couch_app(true);
        app.insert_resource(someone_elses(
            TWINTRACK_SEATS,
            InputAssignmentPolicy::JoinToClaim,
        ));
        app.update();
        assert_eq!(
            couch(&app).2,
            Some(TWINTRACK_EXPERIENCE.to_owned()),
            "the plaza read its own numbers off somebody else's claim and never \
             took it over",
        );
    }
}
