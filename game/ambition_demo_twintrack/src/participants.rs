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
    use ambition_platformer2d::game_shell::{RouteSeating, RouteSeatingAppExt, SeatCount};
    use ambition_platformer2d::input::{InputAssignmentPolicy, LocalChannelPlan, LocalInputSource};
    // Two observers: the keyboard drives the traveler and the first pad drives
    // the twin. The channel plan is known before the route opens, so the
    // rollback session is sized with two handles.
    app.declare_route_seating(
        TWINTRACK_GAMEPLAY_ROUTE,
        RouteSeating::new(
            SeatCount::Fixed(TWINTRACK_SEATS),
            InputAssignmentPolicy::JoinToClaim,
        )
        .with_channels(LocalChannelPlan::from_sources([
            LocalInputSource::Keyboard,
            LocalInputSource::FIRST_PAD,
        ])),
    )
    // This system also retires the second pane, so it must keep running after
    // the experience stops being active.
    .add_systems(Update, compose_the_panes)
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
    room: ambition_platformer2d::runtime::CurrentRoom,
    views: Query<(Entity, &LocalViewId, Option<&ViewPlacement>), With<LocalView>>,
    panes: Query<Entity, With<TwinTrackPaneCamera>>,
) {
    let live = room.in_scope(ambition_platformer2d::combat::scoped_rules::RulesScope::Mode(
        TWINTRACK_EXPERIENCE,
    ));
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
