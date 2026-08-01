//! **The presentation face a demo can add** — [`PlatformerPresentationPlugin`].
//!
//! Filed as oracle-violation **OV1** (`docs/planning/tracks.md`) and closed here.
//!
//! ## Why this exists
//!
//! `docs/planning/demos/README.md` says a demo's app shell is "~100 lines:
//! foundation + engine group + host group + content". The shell built at playbook
//! exit 3 (`game/ambition_demo_sanic_app`) proved that assembles and steps the
//! real sim — and then drew nothing, because **drawing a room was app-local**:
//! `ambition_app` spawned the main camera itself, called `spawn_room_visuals`
//! itself, and assembled the sprite pipeline from a dozen private `install_*`
//! helpers. Drawing a room is not content. Every demo would have copied the code.
//!
//! Everything this plugin needs already lived in this crate. What was missing was
//! a plugin that CALLS it. That is all OV1 ever was.
//!
//! ## What it does, and what it deliberately does not
//!
//! Adds the generic platformer presentation:
//! - the main `Camera2d` (gameplay layer + the parallax background layer), and
//!   the [`MainCameraEntity`] resource the host's camera-follow reads;
//! - the active room's static visuals — blocks, grid, water, ladders, props —
//!   spawned at `Startup`. Room transitions rebuild them through
//!   `respawn_room_visuals_on_request`, which the animation plugin already
//!   registers and the sim already drives, so a demo gets room changes for free;
//! - the per-frame sprite/animation chain ([`PresentationVisualAnimationPlugin`])
//!   and the player-visual schedule ([`PlayerVisualSchedulePlugin`]).
//!
//! It does NOT add Ambition's HUD, its menus, its dev overlays, its audio, its
//! portal-window render, or its kaleidoscope cube. Those are the GAME's, and
//! `ambition_app` keeps assembling them on top. A demo that wants a HUD builds its
//! own — that is what "owns" means in the demos doctrine.
//!
//! ```ignore
//! app.add_plugins(ambition_platformer2d_runtime::PlatformerEnginePlugins::fixed_tick());
//! app.add_plugins(ambition_platformer2d_host::PlatformerHostPlugins);
//! app.add_plugins(ambition_render::PlatformerPresentationPlugin); // ← this
//! app.add_plugins(MyDemoContentPlugin);
//! ```
//!
//! Without a `GameAssets` resource every block draws as a colored rectangle,
//! which is exactly what a demo with no art should see, and exactly what
//! `spawn_block` already does.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::camera_layers::{MainCamera, MainCameraEntity};
use ambition_platformer2d_shared_tangle::lifecycle::{
    ActiveSessionScope, SessionScopeId, SessionScopeSet, SessionSpawnScope,
};
use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_platformer2d_world::rooms::RoomSet;

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

/// The provider-agnostic per-session room presentation: whenever a fresh
/// session scope goes live, spawn the active `RoomSet` room's parallax layers
/// and static visuals exactly once, owned by that scope. Every provider's
/// activation republishes its own `RoomSet` before this runs, so one system
/// serves every game a host links — no per-provider visual wiring.
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
        // ⚠ **the same lesson as the label pass above, one family over.** The
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
        // ⚠ **and the layers have to MOVE.** `sync_parallax_layers` was
        // app-local too, which is the same class one step further along: in
        // every other composition the backdrop spawned at the world origin and
        // stayed there, so it slid out of frame as the camera walked away and
        // the one thing a parallax layer is for — moving at its own rate —
        // never happened. `camera_follow` is DEFINED in this crate and
        // REGISTERED by `ambition_platformer2d_host`, so ordering against it here is legal
        // and is a no-op in a composition that has no camera follow.
        //
        // No `session_world_exists` guard: it reads a camera transform and layer
        // transforms, both of which exist or do not on their own.
        app.add_systems(
            Update,
            crate::rendering::sync_parallax_layers.after(crate::rendering::camera_follow),
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
/// The FRONT camera exists so this minimal presentation can support a
/// fixed-aspect gameplay presentation profile at all. Under such a profile the
/// main camera gets a `Camera::viewport`, and `bevy_ui` lays every node out
/// against its TARGET camera's rect — so a single camera doing both jobs would
/// letterbox the HUD, the menus, the load screen, and the surround bars
/// themselves into the gameplay rectangle. Node→camera resolution is by
/// `IsDefaultUiCamera` / `UiTargetCamera` and is independent of sprite render
/// layers, so UI renders here regardless of the dedicated layer. This mirrors
/// the full host's scaffold deliberately: a demo host should not have to
/// hand-build a camera rig to get correct framing.
fn spawn_main_camera(mut commands: Commands) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer2d_shared_tangle::camera_layers::PARALLAX_BACKGROUND_LAYER);
    let camera = commands
        .spawn((Camera2d, MainCamera, layers, Name::new("Main Camera")))
        .id();
    commands.insert_resource(MainCameraEntity(camera));

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
