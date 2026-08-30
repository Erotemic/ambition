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
        // Room visuals include world labels, so this composition also installs their layout pass.
        // `WorldLabelLayoutPlugin` is idempotent.
        app.add_plugins(crate::rendering::WorldLabelLayoutPlugin);
        // Room/parallax passes consume the resolved quality budget; install its idempotent owner.
        app.add_plugins(crate::quality::VisualQualityPlugin);
        app.init_resource::<PresentedSessionScope>();
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(
            Update,
            sync_session_room_visuals.in_set(SessionScopeSet::Presentation),
        );
        // The composition that spawns blocks also applies authored per-block art overrides.
        app.add_systems(Update, crate::rendering::apply_block_art);
        // The layer-spawning composition owns active-room theme loading and the refresh that
        // materializes newly available/quality-changed parallax assets.
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
        // Room-transition requests rebuild visuals through the presentation animation plugin.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
        ));
    }
}

/// Spawn the single-view gameplay camera and full-screen front-UI camera.
///
/// Gameplay may occupy a viewport, while UI targets the full display. Multi-view compositions own
/// their gameplay rigs and use this path only for the unambiguous full-screen UI camera.
fn spawn_main_camera(
    mut commands: Commands,
    // Bind camera→view at spawn so view ownership is composition state, not a per-frame guess.
    views: Query<Entity, With<ambition_sim_view::LocalView>>,
) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER);
    // This plugin owns exactly one gameplay rig. Spawn it only when `ViewsOnHand` can identify one
    // view; multi-view callers use `ambition_sim_view::compose_local_views` and must not receive an
    // extra unbound full-screen gameplay camera.
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
            // The shared publisher rejects a second main-camera record rather than choosing a winner.
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
    let spec = room_set.active_spec();

    // ⛔⛔ DO NOT MARK THIS SCOPE PRESENTED UNTIL ITS BACKDROP CAN ACTUALLY BE
    // BUILT. `spawn_parallax_layers` early-returns when `GameAssets` has no
    // layers for the room's theme, and `GameAssets` loads ONE theme at startup
    // — every other theme lazy-loads via `ensure_active_room_parallax_theme`.
    // Setting the memo first meant a session that activated on the frame BEFORE
    // its theme arrived got no parallax and was never retried: one shot, and it
    // missed.
    //
    // ⭐ THIS IS WHY AMBITION HAD A BACKDROP AND SMASH DID NOT, on the same
    // assets and the same theme (`Hub`). The sandbox draws through
    // `spawn_initial_room_visuals`, which has no memo and simply tries again
    // next frame; the session path is memoized and does not. The bug was the
    // asymmetry, not the art — which is why regenerating assets never touched
    // it.
    //
    // ⚠ Deferral is conditional on the budget WANTING parallax. A tier that
    // disables it (or a room whose theme legitimately has no art) must present
    // normally, or the whole room's static visuals would be held hostage to a
    // backdrop that is never coming.
    let wants_parallax = quality
        .as_deref()
        .map(|q| q.budget.parallax.enabled)
        .unwrap_or(true);
    if wants_parallax {
        let theme = ambition_sprite_sheet::game_assets::ParallaxTheme::from_room_metadata(
            &spec.metadata,
        );
        let theme_loaded = assets.as_deref().is_some_and(|a| {
            ambition_sprite_sheet::game_assets::ParallaxLayerAsset::ALL
                .iter()
                .any(|layer| a.parallax_layers.get(theme, *layer).is_some())
        });
        if !theme_loaded {
            // Leave `presented` unset so the next frame retries, exactly as the
            // unscoped path does.
            return;
        }
    }

    presented.0 = Some(scope);
    let spawn_scope = SessionSpawnScope::scoped(scope);
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
