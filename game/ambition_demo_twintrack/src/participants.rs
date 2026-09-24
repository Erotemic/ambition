//! TwinTrack models two participants as two observers in one simulation.
//!
//! The traveler occupies slot 0 and the laboratory twin occupies slot 1. Each
//! participant owns a permanent local view. If slot 1 has no controller it reads
//! neutral input; the second observer and view still remain part of the exhibit.

use bevy::prelude::*;

use ambition_platformer2d::actor::{ActorFaction, ActorIdentity, SpawnActorKind, SpawnActorRequest};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::relativity2d::{
    OpticalSource2d, ProperTimeElapsed, RelativisticClock2d, RelativisticObserver2d,
    RelativityClockLabel, WorldlineTracked2d,
};
use ambition_platformer2d::runtime::demo_fixture::RoomSet;
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

/// The second pane exists for as long as the second participant does.
///
/// So it is not composed at plugin build time. A view spawned there exists
/// for the whole host: `ambition_app` links this crate beside Mary-O, Smash,
/// and the launcher, so a build-time second view would split the screen of
/// every route. The rule that a view must exist before any schedule runs
/// applies to the first view. A view appearing later is the ordinary couch
/// co-op event of someone joining.
///
/// Use `spawn_local_view`'s facts, not a hand-built row. A view missing one
/// component does not error; it stops matching the resolve's query, and the
/// pane freezes at the origin with nothing in the log.
fn compose_the_panes(
    mut commands: Commands,
    roots: Query<&RoomSet>,
    views: Query<(Entity, &LocalViewId, Option<&ViewPlacement>), With<LocalView>>,
    panes: Query<Entity, With<TwinTrackPaneCamera>>,
) {
    let live = roots
        .iter()
        .any(|rooms| rooms.active_metadata().mode.as_deref() == Some(TWINTRACK_EXPERIENCE));
    let mut seen: Vec<LocalViewId> = Vec::new();
    for (view, id, placement) in &views {
        seen.push(*id);
        if *id == TRAVELER_VIEW {
            let wanted = live.then(|| ViewPlacement::column(0, 2));
            if placement.copied() != wanted {
                match wanted {
                    // Removed rather than set to full: absent IS full, and a
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

/// The rig for the second pane, bound to the view it presents.
///
/// The shared presentation plugin's rig still binds the first view.
/// `spawn_main_camera` runs at `Startup`, when TwinTrack's session has not
/// begun and there is one view, so the host keeps its gameplay camera, front
/// HUD camera, room visuals, and sprite chain. This adds only the rig the
/// engine could not know about.
///
/// The rig belongs to the caller (see `compose_local_views`). Layers,
/// projection, and order are composition decisions; only the `PresentsView`
/// link is engine vocabulary.
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

/// Headless builds draw nothing, so the second pane is a view with no rig,
/// which is what the integration suite measures.
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
    // Seat/channel resources are process-global; only their owning experience
    // may release them.
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
        // The source plan, not only a count, is known before this fixed
        // two-observer route opens.
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

/// Each pane watches its own participant.
///
/// The traveler's pane names nothing on purpose. A view that names neither a
/// subject nor a participant frames the session's controlled body, which is
/// seat zero's, even while that seat possesses something else. Naming it here
/// would be a second answer that disagrees once possession moves the seat.
///
/// Today the twin's pane and the twin resolve to the same entity, because the
/// twin carries `DrivingParticipant(LAB_TWIN_SLOT)`.
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
/// It goes through the actor construction path, not by hand. A hand-built
/// entity has no movement clusters, so nothing integrates a participant's
/// intent. A constructed character has them, wears its own art, and is
/// driven by the same `DrivingParticipant` → `SlotControls` path as the
/// traveler.
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
/// A separate system because construction is a message: the engine's spawn
/// applier drains the request, so the body does not exist on the tick the
/// session asks for it. This runs until it finds the body and then never
/// matches again, so the clock facts, worldline, and seat are inserted once.
pub(crate) fn adopt_the_laboratory_twin(
    mut commands: Commands,
    // The plaza's own clock, so the twin's starts where the plaza's is
    // rather than at zero — see the `ProperTimeElapsed` line below.
    coordinate_time: Query<&ambition_platformer2d::relativity2d::SpacetimeCoordinateTime2d>,
    already: Query<(), With<LaboratoryTwin>>,
    candidates: Query<(Entity, &ActorIdentity), Without<LaboratoryTwin>>,
) {
    if !already.is_empty() {
        return;
    }
    let Some((body, _)) = candidates
        .iter()
        .find(|(_, identity)| identity.id == LAB_TWIN_FEATURE_ID)
    else {
        return;
    };
    commands.entity(body).insert((
        LaboratoryTwin,
        RelativisticClock2d,
        RelativityClockLabel("laboratory".to_owned()),
        WorldlineTracked2d::new("laboratory"),
        OpticalSource2d::new("laboratory", 180.0, 1.0, 18.0),
        // She observes too. An `OpticalSource2d` is what other observers
        // receive light from; this makes the laboratory twin an observer with
        // her own retarded image of every source and her own null intercepts.
        //
        // "laboratory" sorts before "traveler", so the first row of those
        // resources is now hers. Every TwinTrack consumer names its observer
        // explicitly; a `Deref` read would get the wrong observer.
        RelativisticObserver2d("laboratory".to_owned()),
        // Not `ZERO`. The laboratory twin is at rest in the laboratory, so her
        // proper time is the plaza's coordinate time, the reference every
        // other clock is compared with. Starting at zero when her body is built
        // would make every light-delay reading short by the construction time.
        ProperTimeElapsed {
            seconds: coordinate_time
                .iter()
                .next()
                .map_or(0.0, |clock| clock.seconds),
        },
        TwinTrackExperiment::default(),
        // The seat. `tick_controlled_brains` reads `SlotControls[1]` through
        // this component, and the actor tick does not decide for a body that
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
    use ambition_platformer2d::input::{InputAssignmentPolicy, LocalSeatOffer};

    /// The router is the fixture, not a hand-spawned room: the claim keys off
    /// the route (see `declare_the_couch`).
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

    /// A live plaza claims the offer; a count alone is not enough.
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

    /// The third claim: the channel plan.
    ///
    /// A rollback host publishes a seat's frame from the GGRS handles its
    /// session opened. The session sizes itself once (from connected devices,
    /// unless declared otherwise) and is never resized, so two seats in a
    /// one-handle session leave the second participant's input unreachable.
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

    /// A dormant plaza declares no seating, so every single-player composition
    /// still seats from what is plugged in. A declaration left standing would
    /// size the next game's session, and a session is never resized.
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

    /// It gives back only its own. `release` is a no-op on another owner's
    /// claim, and the plaza must route through it instead of resetting the
    /// resource.
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

    /// The release undoes its own claim and nothing else. If another claim
    /// arrives before the release, the release must leave it in place.
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

    /// A successor with the same numbers is still a different owner.
    ///
    /// ```text
    /// if seats == 2      { seats = 0 }
    /// if policy == couch { policy = default }
    /// ```
    ///
    /// Release by value, as above, would wipe a later route that chose the same
    /// two-seat couch (a common configuration). Value equality cannot tell
    /// "still mine" from "the same answer someone else reached".
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

    /// Taking over an offer that already has the right values still makes
    /// TwinTrack the owner. Otherwise the claim stays in the previous
    /// surface's name, and that surface's teardown would withdraw the seats.
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
