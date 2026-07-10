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
//! app.add_plugins(ambition_runtime::PlatformerEnginePlugins::fixed_tick());
//! app.add_plugins(ambition_host::PlatformerHostPlugins);
//! app.add_plugins(ambition_render::PlatformerPresentationPlugin); // ← this
//! app.add_plugins(MyDemoContentPlugin);
//! ```
//!
//! Without a `GameAssets` resource every block draws as a colored rectangle,
//! which is exactly what a demo with no art should see, and exactly what
//! `spawn_block` already does.

use bevy::prelude::*;

use ambition_platformer_primitives::camera_layers::{MainCamera, MainCameraEntity};
use ambition_platformer_primitives::physics::PhysicsSandboxSettings;
use ambition_sprite_sheet::game_assets::GameAssets;
use ambition_world::rooms::RoomSet;

use crate::rendering::{
    spawn_parallax_layers, spawn_room_visuals, PlayerVisualSchedulePlugin,
    PresentationVisualAnimationPlugin,
};

/// System set for this plugin's one-shot `Startup` work, so a game can order its
/// own presentation setup against it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformerPresentationSetupSet;

/// See the module docs. The generic platformer presentation: a camera, the room's
/// static visuals, and the sprite/animation chain.
pub struct PlatformerPresentationPlugin;

impl Plugin for PlatformerPresentationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::quality::ResolvedVisualQuality>();
        // `spawn_room_visuals` reads this; the sim's physics plugin does not own it.
        app.init_resource::<PhysicsSandboxSettings>();
        app.add_systems(
            Startup,
            (spawn_main_camera, spawn_active_room_visuals)
                .chain()
                .in_set(PlatformerPresentationSetupSet),
        );
        // Room TRANSITIONS rebuild the visuals through
        // `respawn_room_visuals_on_request`, which `PresentationVisualAnimationPlugin`
        // already registers — the sim emits the request and never imports render.
        app.add_plugins((
            PresentationVisualAnimationPlugin,
            PlayerVisualSchedulePlugin,
        ));
    }
}

/// The gameplay camera. Renders layer 0 (sprites) plus the parallax background
/// layer. A game that wants extra layers — Ambition adds the portal-window layer
/// and a separate front UI camera — spawns its own and skips this plugin's
/// `Startup` set, or adds the layer to this entity afterwards.
fn spawn_main_camera(mut commands: Commands) {
    let layers = bevy::camera::visibility::RenderLayers::layer(0)
        .with(ambition_platformer_primitives::camera_layers::PARALLAX_BACKGROUND_LAYER);
    let camera = commands
        .spawn((Camera2d, MainCamera, layers, Name::new("Main Camera")))
        .id();
    commands.insert_resource(MainCameraEntity(camera));
}

/// Spawn the active room's static visuals once, at startup. Room TRANSITIONS
/// rebuild them through `respawn_room_visuals_on_request`, which the sim already
/// drives — so a demo gets room changes for free.
fn spawn_active_room_visuals(
    mut commands: Commands,
    room_set: Option<Res<RoomSet>>,
    physics_settings: Res<PhysicsSandboxSettings>,
    assets: Option<Res<GameAssets>>,
    quality: Option<Res<crate::quality::ResolvedVisualQuality>>,
) {
    // No world installed (a minimal test app) → nothing to draw, and that is not
    // an error: the same shape every optional-resource system in the engine uses.
    let Some(room_set) = room_set else {
        return;
    };
    let spec = room_set.active_spec();
    spawn_parallax_layers(
        &mut commands,
        &spec.world,
        &spec.metadata,
        assets.as_deref(),
        quality.as_deref().map(|q| &q.budget.parallax),
    );
    spawn_room_visuals(&mut commands, spec, *physics_settings, assets.as_deref());
}
