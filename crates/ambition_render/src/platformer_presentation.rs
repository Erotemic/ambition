//! Generic platformer room presentation.
//!
//! [`PlatformerPresentationPlugin`] installs the gameplay camera, room/parallax
//! visuals, sprite animation, and player-visual scheduling. Session room visuals
//! can also be installed independently by hosts with their own camera stack.
//! HUD, menus, audio, dev overlays, and game-specific presentation remain host
//! responsibilities. Missing art falls back to the renderer's ordinary block
//! representation.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::MainCamera;
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopeSet, SessionSpawnScope,
};
use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
use ambition_platformer2d_world::rooms::RoomSet;
use ambition_sprite_sheet::game_assets::GameAssets;

use crate::rendering::{
    spawn_parallax_layers, spawn_room_visuals, PlayerVisualSchedulePlugin,
    PresentationVisualAnimationPlugin,
};

/// System set for this plugin's one-shot host-resident `Startup` work, so a game
/// can order its own presentation setup against it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformerPresentationSetupSet;

#[derive(Resource, Default)]
struct PresentedSessionScope(Option<SessionScopeId>);

/// The provider-agnostic per-session room presentation: whenever a fresh session scope goes
/// live, spawn the active `RoomSet` room's parallax layers and static visuals exactly once,
/// owned by that scope.
///
/// `PlatformerPresentationPlugin` includes it; a host with its own camera and
/// presentation stack (the Ambition shell host) adds JUST this plugin.
pub struct SessionRoomVisualsPlugin;

impl Plugin for SessionRoomVisualsPlugin {
    fn build(&self, app: &mut App) {
        // `spawn_room_visuals` below spawns SIGNAGE and FIXTURE world labels, so
        // the composition that spawns them is the composition that must install
        // the pass which places, fades and typefaces them. Leaving that to
        // `ActorNameplatePresentationPlugin` made the AC12/AC20 policy true of
        // the full Ambition app only: the external consumer, Mary-O and Sanic
        // drew the same labels at raw anchors in Bevy's fallback font. Adding it
        // twice is a no-op — see `WorldLabelLayoutPlugin`.
        app.add_plugins(crate::rendering::WorldLabelLayoutPlugin);
        // The resolved quality budget and the system that keeps it true. Every
        // parallax and sprite pass below reads it, and the SYNC was app-local —
        // so in every other composition the resource sat at its `Default`
        // forever. Idempotent; see `VisualQualityPlugin`.
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(
            Update,
            sync_session_room_visuals.in_set(SessionScopeSet::Presentation),
        );
        // the per-block art override, beside the pass that spawns blocks.
        // `spawn_room_visuals` resolves a block's texture from its `BlockKind`
        // alone, so every solid in a room is one picture; `BlockArt` is how a
        // game says otherwise, and `apply_block_art` is what makes saying it
        // work. Registered HERE for the reason the three notes around it give:
        // the composition that spawns the thing installs the pass that completes
        // it, or the seam exists everywhere and functions in the shipped app
        // only.
        app.add_systems(Update, crate::rendering::apply_block_art);
        // the same lesson as the label pass above, one family over. The
        // parallax THEME load lived in `game/ambition_app`'s room-transition
        // machinery, so a room in a second biome had a backdrop in the shipped
        // host and none anywhere else — silently, because `spawn_parallax_layers`
        // skips a layer whose handle is absent. The composition that spawns the
        // layers is the composition that must load them.
        //
        // The refresh moves with it: it is what turns the load into visible
        // layers (it watches `GameAssets` for change), and leaving it in the app
        // would have made the load a no-op everywhere else — a fix that reads as
        // a fix and changes no picture.
        app.add_systems(
            Update,
            (
                crate::rendering::ensure_active_room_parallax_theme,
                crate::rendering::refresh_parallax_layers_on_quality_change,
            )
                .chain()
                .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
        );
        // Each live view owns its own parallax layers because placement depends on that view's
        // camera and viewport. Chain mirror -> sync so spawned/re-keyed copies flush before sync,
        // and run after refresh so quality-driven respawns have settled. No session-world guard is
        // needed; these systems operate only on the view/layer entities that exist.
        app.add_systems(
            Update,
            (
                crate::rendering::mirror_parallax_layers_per_view,
                crate::rendering::sync_parallax_layers,
            )
                .chain()
                .after(crate::rendering::camera_follow)
                .after(crate::rendering::refresh_parallax_layers_on_quality_change),
        );
    }
}

/// See the module docs. The generic platformer presentation: a camera, the room's
/// static visuals, and the sprite/animation chain.
pub struct PlatformerPresentationPlugin;

impl Plugin for PlatformerPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.add_systems(
            Startup,
            (spawn_main_camera, spawn_initial_room_visuals)
                .chain()
                .in_set(PlatformerPresentationSetupSet),
        );
        app.add_plugins(SessionRoomVisualsPlugin);
        // Room TRANSITIONS rebuild the visuals through
        // `respawn_room_visuals_on_request`, which `PresentationVisualAnimationPlugin`
        // already registers — the sim emits the request and never imports render.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
        ));
    }
}

/// The gameplay camera plus a full-screen front UI camera. The main camera
/// renders layer 0 (sprites) plus the parallax background layer. A game that
/// wants extra layers — Ambition adds the portal-window layer — spawns its own
/// and skips this plugin's `Startup` set, or adds the layer to this entity
/// afterwards.
///
/// Under such a profile the main camera gets a `Camera::viewport`, and `bevy_ui` lays every
/// node out against its TARGET camera's rect — so a single camera doing both jobs would
/// letterbox the HUD, the menus, the load screen, and the surround bars themselves into the
/// gameplay rectangle. Node→camera resolution is by `IsDefaultUiCamera` / `UiTargetCamera` and
/// is independent of sprite render layers, so UI renders here regardless of the dedicated
/// layer. This mirrors the full host's scaffold deliberately: a demo host should not have to
/// hand-build a camera rig to get correct framing.
fn spawn_main_camera(
    mut commands: Commands,
    // The view is spawned at plugin BUILD time, so it is already here; binding the link at
    // SPAWN makes "which view does this camera show" a composition decision on the entity
    // rather than a uniqueness assumption re-derived every frame in `camera_follow`.
    views: Query<Entity, With<ambition_sim_view::LocalView>>,
) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER);
    // this read `views.iter().next()`, which is the take this whole seam
    // exists to delete. With one view it is right; with two it silently binds
    // this rig to whichever view the archetype happened to yield first, and every
    // downstream resolve then faithfully honours a link that was a coin flip —
    // the process-global "the gameplay view" restored as a spawn-time guess, and
    // invisible because a wrong link still draws a picture.
    //
    // the rule is `ViewsOnHand`'s, the same one `camera_follow`, the viewport
    // applier and the draw-side lookup share: the only view in a single-view
    // composition, and a REFUSAL when several exist. This plugin spawns exactly
    // ONE main camera, so with several views there is no honest answer for it to
    // give — a composition that wants two rigs binds them itself. Leaving the
    // link off makes every consumer decline loudly rather than present the wrong
    // view, which is the standard the rest of this seam already holds.
    //
    // and "binds them itself" is now a call, not an instruction to copy this
    // wiring: `ambition_sim_view::compose_local_views` spawns N views with
    // exactly the facts the engine's single-view path spawns, binds one camera to
    // each, and takes a `ViewPlacement` to lay them out.
    //
    // AND THE UNBINDABLE RIG IS NOT SPAWNED AT ALL. Declining only the
    // LINK left a full-screen `MainCamera` in the world that every consumer
    // refused — so a split-screen composition got its two correct panes plus a
    // third camera drawing the world at the origin over the top of them, and the
    // only trace was an `error_once` line. A rig this function cannot honestly
    // bind is a rig nobody asked for; the front HUD camera below is spawned
    // either way, because a composition owning its own gameplay rigs still wants
    // one full-screen UI camera and there is nothing ambiguous about that one.
    let on_hand = ambition_sim_view::ViewsOnHand::survey(views.iter());
    match on_hand.presented_by(None) {
        Some(view) => {
            let camera = commands
                .spawn((
                    Camera2d,
                    MainCamera,
                    layers,
                    ambition_sim_view::PresentsView(view),
                    Name::new("Main Camera"),
                ))
                .id();
            // published through the shared writer, which refuses a SECOND rig
            // instead of letting the last one win. `MainCameraEntity` is a
            // single-camera spawn record with no production reader — a full-screen
            // UI node that wants the whole display targets a display-scoped
            // camera, not whichever gameplay rig this happens to be.
            ambition_platformer2d_shared_tangle::camera_layers::publish_main_camera(
                &mut commands,
                camera,
            );
        }
        None => bevy::log::info_once!(
            "several local views exist, so the shared presentation plugin spawned no              gameplay camera: the composition that asked for those views owns their rigs              (`ambition_sim_view::compose_local_views`)."
        ),
    }

    commands.spawn((
        Camera2d,
        Camera {
            order: 9,
            clear_color: bevy::camera::ClearColorConfig::None,
            ..default()
        },
        ambition_platformer2d_shared_tangle::camera_layers::FrontHudCamera,
        bevy::ui::IsDefaultUiCamera,
        bevy::camera::visibility::RenderLayers::layer(
            ambition_platformer2d_shared_tangle::camera_layers::FRONT_HUD_LAYER,
        ),
        Name::new("Front HUD Camera"),
    ));
}

/// Spawn the active room once for legacy hosts that do not install the
/// gameplay-session lifecycle. Shell hosts wait for a real session activation.
fn spawn_initial_room_visuals(
    mut commands: Commands,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
    active_session: Option<Res<ActiveSessionScope>>,
) {
    if active_session.is_some() {
        return;
    }
    // No world installed (a minimal test app) → nothing to draw, and that is not
    // an error: the same shape every optional-resource system in the engine uses.
    let Some(room_set) = room_set else {
        return;
    };
    let spec = room_set.active_spec();
    spawn_parallax_layers(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        &mut commands,
        SessionSpawnScope::UNSCOPED,
        spec,
        *physics_settings,
        assets.as_deref(),
    );
}

/// Materialize the active session's room presentation exactly once. The scope
/// is captured before any spawn request, so route retirement owns every static
/// visual and parallax entity created here.
fn sync_session_room_visuals(
    mut commands: Commands,
    active_session: Option<Res<ActiveSessionScope>>,
    mut presented: ResMut<PresentedSessionScope>,
    room_set: Option<ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
) {
    let Some(active_session) = active_session else {
        return;
    };
    let current = active_session.current();
    let Some(scope) = current else {
        presented.0 = None;
        return;
    };
    if presented.0 == Some(scope) {
        return;
    }
    let Some(room_set) = room_set else {
        // Keep the scope unpresented so a provider that publishes its world on a
        // later frame is retried rather than permanently skipped.
        return;
    };
    presented.0 = Some(scope);
    let spawn_scope = SessionSpawnScope::scoped(scope);
    let spec = room_set.active_spec();
    spawn_parallax_layers(
        &mut commands,
        spawn_scope,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(
        &mut commands,
        spawn_scope,
        spec,
        *physics_settings,
        assets.as_deref(),
    );
}
