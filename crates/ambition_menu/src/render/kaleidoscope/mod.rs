//! The ONE canonical bevy_lunex 3D-cube inventory renderer (#31).
//!
//! Generic over the host's `PageId`/`Action`: the host publishes the ordered
//! faces via [`ActiveMenuPages`], and [`KaleidoscopeMenuPlugin`] spawns a pause
//! `Camera3d` + a ring of bevy_lunex faces, rebuilds them when the pages change,
//! rotates the ring so the active face turns to the camera, and folds the faces
//! open/closed in the OoT "subscreen" style.
//!
//! This module is the consolidation of what used to be two drifted copies (the
//! `ambition_mock_demo` private cube and an earlier lib re-port). The demo's
//! look/fold/rotation/button-layout is the visual reference and is reproduced
//! here faithfully, generalized over N pages.
//!
//! ## Tuning seam
//! All geometry/speeds/visual knobs live in [`KaleidoscopeMenuConfig`] (a `Resource`).
//! The plugin inserts a default if the host has not; the host (or demo) may
//! insert its own before adding the plugin to match its exact values.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::picking::{Pickable, PickingSystems};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_lunex::prelude::*;

use crate::{
    scrollbar_thumb_layout, ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage,
    AmbitionMenuRoot, MenuColor, MenuControlKind, MenuCubeGeometry, MenuDynamicText,
    MenuDynamicTextContent, MenuFocusKey, MenuNode, MenuOpenCloseStyle, MenuPageModel, MenuRect,
    MenuTextAlign, MenuVisualState, ScrollThumb,
};

// Depth bands on each Lunex face (more negative = closer to the pause camera).
// Ported verbatim from the demo's `app.rs` so the layered look matches.
const DEPTH_BACKGROUND: f32 = -0.04;
const DEPTH_LARGE_PANEL: f32 = -0.16;
const DEPTH_CARD: f32 = -0.32;
const DEPTH_ACTION: f32 = -0.50;
// Edge page-turn buttons get their OWN band, closer than DEPTH_ACTION and away
// from the item-grid action controls, so the flanking L/R buttons never share a
// depth plane with the grid's item planes (which would z-fight / flicker as the
// ring rotates). See `is_edge_button_rect`.
const DEPTH_EDGE_BUTTON: f32 = -0.58;
const DEPTH_EDGE: f32 = -0.68;
// The draggable scrollbar (Feature C) gets its OWN band so it never shares a
// depth plane with the large list/system panel it overlays — two solid opaque
// planes at the same depth z-fight (the GPU depth test is undefined for equal
// depths), which read as the scrollbar flickering as the ring rotates.
const DEPTH_SCROLLBAR: f32 = -0.44;
// The scrollbar THUMB sits a hair in front of its track so the two solid planes
// never share a depth (equal depths z-fight under the GPU depth test → flicker as
// the ring rotates). Same dedicated-band reasoning as DEPTH_SCROLLBAR vs the panel.
const DEPTH_SCROLLBAR_THUMB: f32 = -0.46;
// Item icons sit in front of their cell's action plane (DEPTH_ACTION) but behind
// the text band (DEPTH_TEXT_TOP), so a sprite covers the cell yet any overlaid
// hint text still reads on top of it.
const DEPTH_ICON: f32 = -0.80;
const DEPTH_TEXT_TOP: f32 = -0.96;
const DEPTH_SELECTION: f32 = -1.12;
const FONT_FAMILY: &str = "DejaVu Sans";

/// Marks the rotating ring root that holds the cube faces.
#[derive(Component)]
pub struct MenuRing;

/// System set for the lib's in-place focus-visual readers
/// ([`sync_control_focus_visuals`] + [`sync_selection_corner_visuals`]), both gated on
/// `Changed<MenuVisualState>`.
///
/// These readers turn the host's `MenuVisualState` (which the host writes from its
/// ECS focus cursor) into the on-screen cursor — the material recolour and the white
/// selection corners. For the highlight to appear the host's writer MUST run BEFORE
/// this set (so the flags it flips are seen the same frame). The lib already orders
/// this set AFTER [`rebuild_cube_faces`] so a republish that respawns the controls
/// can't wipe the flags after the writer set them; the host completes the ordering by
/// running its writer `.before(KaleidoscopeFocusVisuals)`. Without that edge the
/// `Changed` readers can run before the writer and a republish-driven rebuild can
/// reset `MenuVisualState` after the write — the "cursor highlight is gone" regression.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KaleidoscopeFocusVisuals;

/// System set holding the cube's per-frame RENDER + render-adjacent systems
/// ([`rebuild_cube_faces`], [`animate_cube_ring`], [`apply_dynamic_text`],
/// [`fade_kaleidoscope_materials`], the [`KaleidoscopeFocusVisuals`] readers,
/// [`project_scrollbar_tracks`], [`scrollbar_press_drag`]). The `PreUpdate`-schedule
/// half of cube rendering (the 3D picking backend) lives in [`KaleidoscopeRenderPre`].
///
/// By default this set always runs (the demo/tests render the cube every frame).
/// Hosts that swap the cube for another presentation may `.run_if(...)` this set (and
/// [`KaleidoscopeRenderPre`]) to disable cube rendering when the cube is NOT the active
/// backend — a clean "only the active backend renders" invariant. Gating the set is
/// purely a CPU optimisation: the cube input systems already self-gate, so gating
/// render off when no cube is shown changes nothing visible.
///
/// [`KaleidoscopeFocusVisuals`] is a MEMBER of this set, so gating the set also gates
/// the focus-visual readers — while still preserving their `.after(rebuild_cube_faces)`
/// ordering edge (membership composes with, and does not drop, ordering constraints).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KaleidoscopeRender;

/// The `PreUpdate`-schedule companion to [`KaleidoscopeRender`], holding the cube's 3D
/// picking backend ([`cube_3d_picking`]). Separate because a Bevy system set cannot
/// span two schedules; hosts gate BOTH to fully disable cube rendering. Same default
/// (always runs) and the same `.run_if(...)` gating story as [`KaleidoscopeRender`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KaleidoscopeRenderPre;

/// Marks a selection corner-bracket piece (child of a control). Spawned hidden on
/// every focusable cell; [`sync_selection_corner_visuals`] shows the focused
/// control's corners and hides the rest, so the keyboard/gamepad cursor and pointer
/// hover have a clear in-place indicator without rebuilding the face.
#[derive(Component)]
pub struct SelectionCorner;

/// Non-generic style metadata stashed on each interactive control so a
/// non-generic system ([`sync_control_focus_visuals`]) can recolor the control's
/// material from its [`MenuVisualState`] (focus / selection / hover) without being
/// generic over the host's `Action`. This is what makes keyboard / gamepad focus
/// movement VISIBLE on the cube: the lib otherwise only colours the selected cell
/// once at build time and never re-reads the focus flag.
#[derive(Component, Clone, Copy)]
pub struct KaleidoscopeControlStyle {
    kind: MenuControlKind,
    important: bool,
    disabled: bool,
}

/// Marks the dedicated pause camera that frames the cube.
#[derive(Component)]
pub struct KaleidoscopePauseCamera;

/// Per-entity record of a control/panel/text/icon material's INTENDED (design)
/// base-color alpha, so [`fade_kaleidoscope_materials`] can drive the live material
/// alpha to `base_alpha * KaleidoscopeOpenState::amount` each frame — the cube
/// fades in/out with the open/close fold instead of popping (Feature B). Every
/// spawned mesh material on the ring carries this; the fade system scales their
/// alpha cheaply in place (no rebuild). The scrim already fades by `amount`; this
/// matches it so the whole menu cross-fades together.
#[derive(Component, Clone, Copy)]
pub struct KaleidoscopeFade {
    /// The material's fully-open base-color alpha (0..=1).
    pub base_alpha: f32,
}

/// Backend-agnostic scroll-drag channel (Feature C).
///
/// The lib renders a draggable scrollbar control (a `MenuControlKind::Scrollbar`
/// node) and, when a pointer presses + drags on it, emits [`MenuScrollDragged`]
/// carrying a NEUTRAL fraction in `0..=1` (0 = top of the track, 1 = bottom). The
/// lib has NO notion of "scroll position" — the host interprets the fraction
/// against its own scrollable range (e.g. maps it to a window-start row). This
/// mirrors the [`crate::MenuDynamicTextContent`] content-channel pattern: the lib
/// exposes a neutral signal, the host applies the meaning.
#[derive(bevy::prelude::Message, Clone, Copy, Debug, PartialEq)]
pub struct MenuScrollDragged {
    /// Drag position along the track, `0.0` (top) .. `1.0` (bottom).
    pub fraction: f32,
}

/// Marks a draggable scrollbar TRACK control + the screen-space extent the drag
/// observer maps the pointer onto (Feature C). The lib keeps `track_top_y` /
/// `track_height` updated each frame by projecting the track plane through the
/// pause camera ([`project_scrollbar_tracks`]); the drag observer then maps the
/// pointer's vertical screen position into the neutral `0..=1` fraction reported
/// by [`MenuScrollDragged`]. Also carries the host's last-published `fraction`
/// so the rendered thumb can reflect the current scroll position.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MenuScrollbar {
    /// Track top edge in screen pixels (set by the projection system at runtime;
    /// a headless test may set it directly).
    pub track_top_y: f32,
    /// Track height in screen pixels (must be > 0 for the drag to map).
    pub track_height: f32,
}

/// Which pointer (if any) is mid-drag on a menu scrollbar. Held in a RESOURCE,
/// NOT on the scrollbar entity, because changing the scroll position triggers the
/// host's per-step republish, which DESPAWNS + respawns the scrollbar entity each
/// frame — a per-entity held flag would reset to `None` after the very first step
/// and the drag would die. Keyed on the persistent `PointerId`, so the drag
/// survives any number of respawns. Set on `Pointer<Press>`, cleared on
/// `Pointer<Release>`; the manual `scrollbar_press_drag` tracker reads it and
/// emits [`MenuScrollDragged`] for the held pointer's live position each frame.
/// Shared by BOTH renderers (only one menu is active at a time).
#[derive(Resource, Default)]
pub struct ScrollbarDragState {
    pub pressed_by: Option<PointerId>,
    /// Track screen rect (top edge + height, logical px) CACHED at press time.
    /// The track never moves during a drag (only the pointer does), and the grid
    /// track's `ComputedNode`/`GlobalTransform` read as ZERO on the frame it is
    /// respawned by the per-step republish — so the manual tracker maps the live
    /// pointer against this cached rect rather than the (possibly just-respawned)
    /// entity's geometry. Set on press, when the pressed track's geometry is valid.
    pub track_top_y: f32,
    pub track_height: f32,
}

/// Non-generic marker on each cube face plus the face's base ring placement.
///
/// Stored at build time so the per-frame OoT page-fold can recompute each face's
/// transform from its (immutable) base without corrupting it. A non-generic
/// component lets the fold/animation systems query faces without being generic
/// over the host's `PageId`.
#[derive(Component)]
pub struct CubeFace {
    /// Index of this face on the ring.
    pub index: usize,
    /// The face's ring angle (radians), source of the position-derived fold axis.
    pub angle: f32,
    /// The face's base translation on the ring (no fold applied).
    pub base_translation: Vec3,
    /// The face's base rotation on the ring (no fold applied).
    pub base_rotation: Quat,
    /// The face's base scale (carries the inside-of-cube X flip).
    pub base_scale: Vec3,
    /// Half-height of the face, for the bottom-edge hinge.
    pub half_height: f32,
}

/// Upper bound on the per-frame delta used to ease the open/close fold (~2 frames
/// at 60 Hz). Caps how much a single hitchy frame (e.g. the host un-pausing the
/// game on close) can advance the exponential ease, so the fold can never collapse
/// into one frame and snap shut. See [`animate_cube_ring`].
const MAX_FOLD_EASE_DT: f32 = 1.0 / 30.0;

/// Eased open amount for the cube menu (0 = folded shut, 1 = laid flat/open).
///
/// The host sets [`KaleidoscopeOpenState::target`]; [`animate_cube_ring`] eases `amount`
/// toward it each frame and folds the faces accordingly. The host also reads
/// `amount` to drive camera/visibility so the close animation is visible.
#[derive(Resource, Default)]
pub struct KaleidoscopeOpenState {
    pub amount: f32,
    pub target: f32,
}

/// All tuning knobs for the canonical cube, shared by the demo and the game.
///
/// The plugin inserts [`KaleidoscopeMenuConfig::default`] if absent. A host that wants
/// the demo's exact look (e.g. the mock demo itself) inserts its own values
/// before adding the plugin.
#[derive(Resource, Clone, Debug)]
pub struct KaleidoscopeMenuConfig {
    /// Cube/page geometry (radius, face size, camera placement).
    pub geometry: MenuCubeGeometry,
    /// How far a face folds away from the ring when fully closed (radians).
    pub fold_radians: f32,
    /// Ease speed for the open/close fold.
    pub open_close_speed: f32,
    /// Multiplier applied to [`open_close_speed`] while CLOSING (`target == 0`) so
    /// the cube folds away faster than it opens (OoT subscreen feel; the open keeps
    /// the gentle ease). `1.0` = symmetric. Default `2.0`.
    pub close_speed_scale: f32,
    /// OoT opening SPIN: how many ring page-steps the cube starts rotated toward the
    /// viewer-RIGHT neighbour at the start of an OPEN, spinning around to the active
    /// page as the fold completes (synced to the eased open `amount`). `0.0` disables
    /// the spin (no opening rotation); `1.0` = one page-step. Close never spins.
    /// Default `1.0`.
    pub open_spin_faces: f32,
    /// Ease speed for the active-page ring rotation snap.
    pub page_rotate_speed: f32,
    /// Open/close presentation: page-fold (OoT) or a simple scale.
    pub open_close_style: MenuOpenCloseStyle,
    /// Inside-of-cube horizontal flip so face content reads correctly (-1.0).
    pub inside_x_flip: f32,
    /// Minimum ring scale at fully-closed when using [`MenuOpenCloseStyle::SmoothScale`].
    pub min_open_scale: f32,
    /// Draw the bright cube-edge frame around each face (demo look).
    pub draw_edge_frame: bool,
    /// Draw white selection corner-brackets around the selected control (demo look).
    pub draw_selection_corners: bool,
    /// Draw the left/right page-navigation affordance buttons on each face (the
    /// L/R "switch subscreen" arrows). Decorative-only in the lib (the host owns
    /// the actual page cycling via input); they communicate the affordance and
    /// match the demo's look. Default `true` so both the demo and the game get them.
    pub draw_nav_arrows: bool,
    /// Camera `order` for the cube's `Camera3d`.
    pub camera_order: isize,
    /// Whether the cube camera clears the screen (game overlay wants `None`).
    pub camera_clears: bool,
    /// Whether the cube camera starts active. The game gates this off and toggles
    /// it itself; a standalone demo can start it on.
    pub camera_starts_active: bool,
    /// Whether the ring starts visible. The game gates this off; the demo shows it.
    pub ring_starts_visible: bool,
    /// Whether interactive controls are spawned as Bevy-pickable (so `Pointer<*>`
    /// events fire on them). Hosts that drive their own manual world→screen
    /// hit-test (the mock demo) set this `false` to keep controls `Pickable::IGNORE`
    /// and avoid double-handling. The game sets it `true` to use Bevy picking.
    /// Default `true`.
    pub pickable_controls: bool,
}

impl Default for KaleidoscopeMenuConfig {
    fn default() -> Self {
        Self {
            geometry: MenuCubeGeometry::default(),
            fold_radians: 1.60,
            open_close_speed: 8.0,
            close_speed_scale: 2.0,
            // >1.0 starts the open spin further into the neighbour page so more of
            // the rotation is visible (1.5 = ~135° sweep on a 4-page cube).
            open_spin_faces: 1.5,
            page_rotate_speed: 5.2,
            open_close_style: MenuOpenCloseStyle::OotPageFold,
            inside_x_flip: -1.0,
            min_open_scale: 0.64,
            draw_edge_frame: true,
            draw_selection_corners: true,
            draw_nav_arrows: true,
            // Game-overlay defaults (see module docs in `oot_cube_app.rs`): the
            // cube camera must NOT clear, must NOT start active, and the ring must
            // start hidden — the host gates them on when the menu opens.
            camera_order: 8,
            camera_clears: false,
            camera_starts_active: false,
            ring_starts_visible: false,
            pickable_controls: true,
        }
    }
}

/// Plugin: spawns the cube camera + ring and rebuilds faces from
/// `ActiveMenuPages<PageId, Action>`. Add once with the host's page/action types.
pub struct KaleidoscopeMenuPlugin<PageId, Action> {
    _marker: PhantomData<fn() -> (PageId, Action)>,
}

impl<PageId, Action> Default for KaleidoscopeMenuPlugin<PageId, Action> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

// fontdb::load_system_fonts() is explicitly a no-op on Android. This runs once
// (via resource_added condition) on the frame TextRenderer is first available.
#[cfg(target_os = "android")]
fn seed_text3d_android_fonts(mut renderer: ResMut<TextRenderer>) {
    let mut guard = renderer.lock();
    guard.db_mut().load_fonts_dir("/system/fonts");
    bevy::log::info!(
        "android: seeded Text3d fontdb with {} face(s) from /system/fonts",
        guard.db().faces().count()
    );
}

impl<PageId, Action> Plugin for KaleidoscopeMenuPlugin<PageId, Action>
where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins)
            .init_resource::<KaleidoscopeOpenState>();
        // fontdb::load_system_fonts() is a no-op on Android (explicitly excluded
        // by fontdb via `not(target_os = "android")`). Seed /system/fonts/ on the
        // frame TextRenderer first becomes available so cosmic-text never sees an
        // empty database and panics with "no default font found".
        #[cfg(target_os = "android")]
        app.add_systems(
            Update,
            seed_text3d_android_fonts.run_if(resource_added::<TextRenderer>),
        );
        if !app.world().contains_resource::<KaleidoscopeMenuConfig>() {
            app.insert_resource(KaleidoscopeMenuConfig::default());
        }
        // The cube is framed by a PERSPECTIVE `Camera3d`; bevy_lunex's stock
        // `lunex_2d_picking` backend only raycasts orthographic cameras, so it never
        // generates hits for the cube. When the host wants Bevy picking on the cube
        // controls (`pickable_controls`), install a dedicated 3D picking backend that
        // raycasts the cube camera against the controls' Lunex planes and emits
        // `PointerHits` — that's what makes `Pointer<Over>`/`Pointer<Click>` fire on
        // the cube. Hosts with their own manual hit-test (the demo) leave it off.
        if app
            .world()
            .resource::<KaleidoscopeMenuConfig>()
            .pickable_controls
        {
            app.add_systems(
                PreUpdate,
                cube_3d_picking
                    .in_set(PickingSystems::Backend)
                    .in_set(KaleidoscopeRenderPre),
            );
            // Make ECS-driven focus / hover visible (the host moves focus in ECS
            // without rebuilding the face). The demo drives its own look + rebuilds
            // on nav, so this is gated to the Bevy-picking (game) configuration.
            //
            // Both readers live in the public [`KaleidoscopeFocusVisuals`] set, ordered
            // AFTER `rebuild_cube_faces` so a republish that respawns the controls can't
            // wipe the host writer's focus flags after they're set. The host runs its
            // `MenuVisualState` writer `.before(KaleidoscopeFocusVisuals)`; without that
            // edge the `Changed` readers could run before the writer (and a rebuild
            // could reset the flags afterwards) — the "cursor highlight gone" bug.
            app.configure_sets(
                Update,
                KaleidoscopeFocusVisuals
                    .after(rebuild_cube_faces::<PageId, Action>)
                    // Member of the gateable render set: when the host gates
                    // `KaleidoscopeRender` off (another backend is active) the focus-
                    // visual readers stop too, while the `.after(rebuild_cube_faces)`
                    // edge above is preserved (membership composes with ordering).
                    .in_set(KaleidoscopeRender),
            );
            app.add_systems(
                Update,
                (sync_control_focus_visuals, sync_selection_corner_visuals)
                    .in_set(KaleidoscopeFocusVisuals),
            );
            // Feature C: draggable scrollbar. Keep each scrollbar track's screen
            // extent fresh (projection), and observe pointer drags on a scrollbar
            // to emit the neutral `MenuScrollDragged` fraction the host applies.
            app.add_message::<MenuScrollDragged>();
            app.init_resource::<ScrollbarDragState>();
            app.add_systems(Update, project_scrollbar_tracks.in_set(KaleidoscopeRender));
            app.add_observer(scrollbar_drag_start);
            app.add_observer(scrollbar_drag);
            // Fix 1: the manual press+move tracker (the robust path for the cube's
            // custom picking backend). `Pointer<Press>`/`Pointer<Release>` mark the
            // track pressed; `scrollbar_press_drag` then emits `MenuScrollDragged`
            // for the pointer's live position each frame while pressed — so dragging
            // the thumb scrolls even where `Pointer<Drag>` continuity doesn't reach.
            app.add_observer(scrollbar_press);
            app.add_observer(scrollbar_release);
            app.add_systems(
                Update,
                scrollbar_press_drag
                    .after(project_scrollbar_tracks)
                    .in_set(KaleidoscopeRender),
            );
        }
        // Feature B: cross-fade the whole cube (faces/controls/text/icons) with the
        // open/close fold `amount`, so it fades in/out like the scrim instead of
        // popping. Runs in PostUpdate AFTER `sync_control_focus_visuals` (which can
        // swap a control's material handle this frame) so the fade always lands on
        // the live material; ordered after the animate step (Update) that advances
        // `amount`. Cheap: it only mutates the base-color alpha on existing assets.
        app.add_systems(
            PostUpdate,
            fade_kaleidoscope_materials.in_set(KaleidoscopeRender),
        );
        app.add_systems(Startup, setup_cube)
            .add_systems(
                Update,
                (
                    rebuild_cube_faces::<PageId, Action>,
                    animate_cube_ring::<PageId, Action>,
                )
                    .chain()
                    .in_set(KaleidoscopeRender),
            )
            // In-place dynamic text (the host-filled detail panel). Runs after a
            // rebuild so freshly spawned dynamic lines pick up the host's content
            // the same frame, and on every host content change thereafter — no face
            // rebuild needed for cursor-dependent text.
            .add_systems(
                Update,
                apply_dynamic_text
                    .after(rebuild_cube_faces::<PageId, Action>)
                    .in_set(KaleidoscopeRender),
            );
    }
}

fn setup_cube(mut commands: Commands, config: Res<KaleidoscopeMenuConfig>) {
    let geo = config.geometry;
    commands.spawn((
        Name::new("Cube pause camera"),
        KaleidoscopePauseCamera,
        Camera3d::default(),
        Camera {
            order: config.camera_order,
            // Host-gated by default: OFF until the host activates the menu. An
            // active higher-order camera otherwise clears the whole screen every
            // frame, hiding the lower-order game cameras.
            is_active: config.camera_starts_active,
            // Transparent clear (Option 1 overlay) keeps the live game world
            // visible behind the cube. A standalone demo flips `camera_clears` on.
            clear_color: if config.camera_clears {
                ClearColorConfig::default()
            } else {
                ClearColorConfig::None
            },
            ..default()
        },
        RenderLayers::layer(0),
        // NO explicit Msaa: a Camera3d overlaying a Camera2d on the same window must
        // share its sample count or it renders its clear but drops all geometry. The
        // host's Camera2d uses the default (Msaa::Sample4); omitting Msaa here
        // inherits that same default so they match.
        Transform::from_translation(Vec3::new(0.0, geo.camera_y, -geo.camera_distance))
            .looking_at(Vec3::new(0.0, geo.look_y, 0.0), Vec3::Y),
    ));
    commands.spawn((
        Name::new("Cube menu ring"),
        AmbitionMenuRoot,
        MenuRing,
        UiRoot3d,
        Transform::default(),
        if config.ring_starts_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        RenderLayers::layer(0),
    ));
}

/// 3D picking backend for the cube's perspective camera.
///
/// bevy_lunex's stock `lunex_2d_picking` only raycasts ORTHOGRAPHIC cameras, so it
/// never produces hits for the cube (a perspective `Camera3d`). This backend
/// raycasts the [`KaleidoscopePauseCamera`] against every hoverable Lunex `Dimension`
/// plane (the live controls) and writes `PointerHits` so the picking core can
/// dispatch `Pointer<Over>` / `Pointer<Click>` to the cube controls.
///
/// Only hoverable nodes are considered: controls that opted out of picking
/// (`Pickable::IGNORE` — disabled controls, panels, text, decoration) are skipped,
/// so the ray lands on the actual interactive controls.
fn cube_3d_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    camera_query: Query<
        (
            Entity,
            &Camera,
            &bevy::camera::RenderTarget,
            &GlobalTransform,
        ),
        With<KaleidoscopePauseCamera>,
    >,
    // Only INTERACTIVE control planes are pick candidates: the query REQUIRES
    // `KaleidoscopeControlStyle`, which `spawn_control` puts on every interactive control and
    // nothing else (the full-face background, the per-page `UiRoot3d` face plane —
    // which carries a face-sized `Dimension` — decorative panels, text, and the
    // selection corners all lack it). Without this gate those non-control planes are
    // valid `Dimension` candidates too; a face-spanning plane that wins the depth
    // sort silently swallows the click (it has no `AmbitionMenuControl`, so the host
    // observer's `controls.get(hit)` returns `Err` and the click is dropped). Gating
    // to real controls makes every spawned, enabled control reliably clickable.
    nodes: Query<
        (
            Entity,
            &Dimension,
            &GlobalTransform,
            Option<&Pickable>,
            &ViewVisibility,
        ),
        With<KaleidoscopeControlStyle>,
    >,
    mut output: MessageWriter<PointerHits>,
) {
    // The gated cube camera is only active while the menu is open; bail otherwise.
    let Some((cam_entity, camera, render_target, cam_transform)) =
        camera_query.iter().find(|(_, c, _, _)| c.is_active)
    else {
        return;
    };
    let primary_window = primary_window.single().ok();

    // Hoverable control planes only (skip IGNORE: disabled controls). Decoration /
    // panels / text / the face root are already excluded by the `KaleidoscopeControlStyle`
    // query filter above.
    let candidates: Vec<_> = nodes
        .iter()
        .filter(|(_, _, transform, pickable, vis)| {
            vis.get()
                && !transform.affine().is_nan()
                && pickable.map(|p| p.is_hoverable).unwrap_or(true)
        })
        .map(|(entity, dimension, transform, pickable, _)| (entity, dimension, transform, pickable))
        .collect();

    for (pointer, location) in pointers
        .iter()
        .filter_map(|(pointer, loc)| loc.location().map(|l| (pointer, l)))
    {
        // Only handle pointers on this camera's render target.
        let on_target = render_target
            .normalize(primary_window)
            .is_some_and(|t| t == location.target);
        if !on_target {
            continue;
        }

        let viewport_pos = camera
            .logical_viewport_rect()
            .map(|v| v.min)
            .unwrap_or_default();
        let pos_in_viewport = location.position - viewport_pos;
        let Ok(ray) = camera.viewport_to_world(cam_transform, pos_in_viewport) else {
            continue;
        };

        let mut picks: Vec<(Entity, HitData)> = Vec::new();
        for (entity, dimension, node_transform, _pickable) in candidates.iter().copied() {
            // Intersect the cursor ray with the node's local Z=0 plane.
            let world_to_node = node_transform.affine().inverse();
            let ray_origin_node = world_to_node.transform_point3(ray.origin);
            let ray_dir_node = world_to_node.transform_vector3(*ray.direction);
            if ray_dir_node.z.abs() < 1e-6 {
                continue; // parallel to the plane
            }
            let t = -ray_origin_node.z / ray_dir_node.z;
            if t < 0.0 {
                continue; // behind the camera
            }
            let hit_node = ray_origin_node + ray_dir_node * t;
            let rect = Rect::from_center_size(Vec2::ZERO, **dimension);
            if !rect.contains(hit_node.xy()) {
                continue;
            }
            let hit_world = node_transform.transform_point(hit_node.xy().extend(0.0));
            // Depth = distance from the camera along the ray (nearer = smaller).
            let depth = (hit_world - ray.origin).length();
            picks.push((
                entity,
                HitData::new(
                    cam_entity,
                    depth,
                    Some(hit_world),
                    Some(*node_transform.back()),
                ),
            ));
        }
        // Nearest plane first so the picking core's hover/click resolves the
        // front-most control.
        picks.sort_by(|a, b| a.1.depth.total_cmp(&b.1.depth));
        let order = camera.order as f32;
        output.write(PointerHits::new(*pointer, picks, order));
    }
}

/// Recolor each control's material from its live [`MenuVisualState`] so keyboard /
/// gamepad focus and pointer hover are VISIBLE. Without this, the lib only colours
/// the selected cell once at build time, so a host that moves focus purely in ECS
/// (the game) sees no on-screen cursor movement — the "arrow keys do nothing" bug.
///
/// Non-generic (keyed off [`KaleidoscopeControlStyle`]) so it doesn't need the host's
/// `Action`. Only changed states write a new material handle (cheap, idempotent).
pub fn sync_control_focus_visuals(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut controls: Query<
        (
            &KaleidoscopeControlStyle,
            &MenuVisualState,
            &mut MeshMaterial3d<StandardMaterial>,
            // Feature B: keep this control's recorded design alpha in sync with its
            // recoloured (focused/hover) material so `fade_kaleidoscope_materials`
            // fades the new highlight colour at the right base alpha.
            Option<&mut KaleidoscopeFade>,
        ),
        Changed<MenuVisualState>,
    >,
) {
    for (style, vis, mut material, fade) in &mut controls {
        let highlight = vis.focused || vis.selected || vis.hovered;
        let color = if style.disabled {
            disabled_control_color()
        } else {
            control_color(style.kind, highlight, style.important)
        };
        let base_alpha = color.alpha();
        if let Some(mut fade) = fade {
            fade.base_alpha = base_alpha;
        }
        // Blend (not Opaque) so `fade_kaleidoscope_materials` can fade the control
        // in/out with the open fold (Feature B).
        *material = MeshMaterial3d(materials.add(StandardMaterial {
            base_color: fade_color(color, base_alpha),
            alpha_mode: AlphaMode::Blend,
            cull_mode: None,
            unlit: true,
            ..default()
        }));
    }
}

/// Reveal the FOCUSED control's selection corner-brackets and hide everyone else's,
/// in place, from each control's live [`MenuVisualState`]. This is the prominent
/// cursor indicator (the recolor alone is too subtle): pre-click-fix the corners
/// were baked from `selected` at build time, but the build is now cursor-independent
/// (so clicks survive press->release), so the cursor visual is applied at runtime.
/// Reacts to `Changed<MenuVisualState>` like [`sync_control_focus_visuals`].
pub fn sync_selection_corner_visuals(
    controls: Query<(&MenuVisualState, &Children), Changed<MenuVisualState>>,
    mut corners: Query<&mut Visibility, With<SelectionCorner>>,
) {
    for (vis, children) in &controls {
        let target = if vis.focused || vis.selected || vis.hovered {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for &child in children {
            if let Ok(mut visibility) = corners.get_mut(child) {
                if *visibility != target {
                    *visibility = target;
                }
            }
        }
    }
}

/// Rebuild the ring's faces whenever the host's published pages change.
pub fn rebuild_cube_faces<PageId, Action>(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    config: Res<KaleidoscopeMenuConfig>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    ring_query: Query<Entity, With<MenuRing>>,
    faces: Query<Entity, With<AmbitionMenuPage<PageId>>>,
    mut last_version: Local<Option<u64>>,
    mut dirty: Local<bool>,
) where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + Send + Sync + 'static,
{
    let Some(pages) = pages else {
        return;
    };
    // Rebuild on version bump (host republish) or first run. Cheap: page models
    // are small and rebuilt only when the host changes them.
    if !pages.is_changed() && !*dirty && *last_version == Some(pages.version) {
        return;
    }
    *dirty = false;
    *last_version = Some(pages.version);

    for face in &faces {
        commands.entity(face).despawn();
    }
    let Ok(ring) = ring_query.single() else {
        warn!("cube: ring entity not found yet — deferring face rebuild");
        *dirty = true;
        return;
    };
    debug!(
        "cube: rebuilding {} face(s) (active page present: {})",
        pages.pages.len(),
        pages.active.is_some()
    );
    let geo = config.geometry;
    let n = pages.pages.len().max(1) as f32;
    let flip = config.inside_x_flip;
    commands.entity(ring).with_children(|ring| {
        for (i, model) in pages.pages.iter().enumerate() {
            let active = pages.active.as_ref() == Some(&model.id);
            let angle = (i as f32) * std::f32::consts::TAU / n;
            let pos = Vec3::new(
                angle.sin() * geo.page_radius,
                0.0,
                angle.cos() * geo.page_radius,
            );
            let rot = Quat::from_rotation_y(angle);
            let scale = Vec3::new(flip, 1.0, 1.0);
            let mut face = ring.spawn((
                Name::new("Cube face"),
                AmbitionMenuPage {
                    id: model.id.clone(),
                    active,
                },
                CubeFace {
                    index: i,
                    angle,
                    base_translation: pos,
                    base_rotation: rot,
                    base_scale: scale,
                    half_height: geo.page_height * 0.5,
                },
                UiRoot3d,
                // bevy_lunex needs a layout root + a Dimension on each face for the
                // child UiLayout::window() planes to resolve their Rl/Rh sizes.
                UiLayoutRoot::new_3d(),
                Dimension::from((geo.page_width, geo.page_height)),
                Transform::from_translation(pos)
                    .with_rotation(rot)
                    .with_scale(scale),
                Visibility::Visible,
                RenderLayers::layer(0),
            ));
            face.with_children(|ui| {
                render_page_model(ui, &mut materials, &asset_server, &config, model, active)
            });
        }
    });
}

/// Drive the whole ring per frame: ease the open amount, snap the ring rotation
/// to the active face, apply the open/close presentation, and (in OoT style)
/// fold every face about its bottom edge.
///
/// Ported from the demo's `animate_menu_ring` + `apply_oot_open_fold`,
/// generalized over N pages.
fn animate_cube_ring<PageId, Action>(
    time: Res<Time>,
    config: Res<KaleidoscopeMenuConfig>,
    mut state: ResMut<KaleidoscopeOpenState>,
    pages: Option<Res<ActiveMenuPages<PageId, Action>>>,
    mut ring: Query<&mut Transform, (With<MenuRing>, Without<CubeFace>)>,
    mut faces: Query<(&CubeFace, &mut Transform), Without<MenuRing>>,
) where
    PageId: PartialEq + Send + Sync + 'static,
    Action: Send + Sync + 'static,
{
    let Ok(mut ring_t) = ring.single_mut() else {
        return;
    };
    let Some(pages) = pages else {
        return;
    };
    let n = pages.pages.len().max(1) as f32;

    // Detect open vs close from the host's target: >0.5 = opening, else closing.
    let opening = state.target > 0.5;

    // Ease the open amount toward the host's target (demo's exp ease). The CLOSE
    // uses a faster rate (`close_speed_scale`×) so the cube folds away snappily
    // without the lingering tail, while the OPEN keeps its gentle ease.
    let rate = if opening {
        config.open_close_speed
    } else {
        config.open_close_speed * config.close_speed_scale
    };
    let new_amount = ease_fold_amount(state.amount, state.target, rate, time.delta_secs());
    let open = smoothstep(new_amount.clamp(0.0, 1.0));

    // OoT opening SPIN: while opening, start the ring rotated one page-step toward
    // the viewer-RIGHT neighbour and spin around so the active page swings to the
    // front, synced to the eased open `amount` (finishes aligned as the fold-in
    // completes). The ring formula `from_rotation_y(-idx * TAU/n)` brings the
    // viewer-LEFT neighbour (`idx+1`) to front for a positive step; the viewer-RIGHT
    // neighbour is `idx-1`, so the spin offset starts NEGATIVE and eases to 0.
    // (Sign note: if this spins the wrong way, flip the leading `-` below.)
    let spin_offset = if opening {
        -config.open_spin_faces * (1.0 - open)
    } else {
        0.0 // close never spins — it just folds away facing the active page.
    };

    // Snap the ring so the active face turns to the camera (OoT page turn).
    let active_idx = pages
        .active
        .as_ref()
        .and_then(|a| pages.pages.iter().position(|p| &p.id == a))
        .unwrap_or(0) as f32;
    let target = Quat::from_rotation_y(-(active_idx + spin_offset) * std::f32::consts::TAU / n);
    let rotate_step = (time.delta_secs() * config.page_rotate_speed).clamp(0.0, 1.0);
    let spin = ring_t.rotation.slerp(target, rotate_step);

    // PERF (2026-06-10): SETTLE EARLY-OUT. Once the fold amount has reached its
    // target (`ease_fold_amount` snaps within 0.002) AND the ring is already at the
    // active-face rotation, the cube is fully open and still. Writing the ring +
    // every face `Transform` then would needlessly re-propagate `GlobalTransform`
    // over all face children (panels/text/icons/corners) EVERY frame — pure churn
    // while the menu just sits open. Skip the writes while nothing actually moves;
    // any retarget (open/close, page rotate) moves `target`/`state.target` and
    // re-arms the animation next frame.
    let amount_settled =
        new_amount == state.target && (new_amount - state.amount).abs() <= f32::EPSILON;
    let rotation_settled = ring_t.rotation.angle_between(target) < 1.0e-4;
    if amount_settled && rotation_settled {
        return;
    }
    state.amount = new_amount;

    match config.open_close_style {
        MenuOpenCloseStyle::SmoothScale => {
            let scale = config.min_open_scale + (1.0 - config.min_open_scale) * open;
            ring_t.rotation = spin;
            ring_t.scale = Vec3::splat(scale);
            ring_t.translation = Vec3::new(0.0, -0.05 * (1.0 - open), -0.42 * (1.0 - open));
            for (face, mut t) in &mut faces {
                reset_face_transform(face, &mut t);
            }
        }
        MenuOpenCloseStyle::OotPageFold => {
            ring_t.rotation = spin;
            ring_t.scale = Vec3::ONE;
            ring_t.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
            let fold = config.fold_radians * (1.0 - open);
            for (face, mut t) in &mut faces {
                apply_face_fold(face, fold, &mut t);
            }
        }
    }
}

/// Restore a face to its unfolded base placement (used by the scale style).
fn reset_face_transform(face: &CubeFace, transform: &mut Transform) {
    transform.translation = face.base_translation;
    transform.rotation = face.base_rotation;
    transform.scale = face.base_scale;
}

/// Generalized port of the demo's `apply_oot_open_fold`.
///
/// The demo's n=4 cardinal mapping folds each face about a horizontal axis in
/// *ring space* (the parent frame), pinning the face's bottom edge as a hinge:
///
/// | page   | ring angle θ | demo fold axis | `(cosθ, 0, -sinθ)` |
/// |--------|--------------|----------------|---------------------|
/// | Items  | 0°           | +X             | (1, 0, 0)           |
/// | Map    | 90°          | -Z             | (0, 0, -1)          |
/// | Quest  | 180°         | -X             | (-1, 0, 0)          |
/// | System | 270°         | +Z             | (0, 0, 1)           |
///
/// So the fold axis is exactly the ring-space tangent `(cosθ, 0, -sinθ)` — the
/// horizontal direction along the bottom edge of the face — with a single
/// positive `fold`. This reproduces the demo for n=4 AND generalizes to any N
/// (the axis is derived from the face's own ring angle, not a hardcoded enum).
/// The fold is pre-multiplied (`fold_rotation * base_rotation`) so it acts in
/// ring space, exactly like the demo.
fn apply_face_fold(face: &CubeFace, fold: f32, transform: &mut Transform) {
    let axis = Vec3::new(face.angle.cos(), 0.0, -face.angle.sin());
    let fold_rotation = Quat::from_axis_angle(axis, fold);
    let rotation = fold_rotation * face.base_rotation;
    // Pin the bottom edge of the page (hinge), exactly like the demo.
    let hinge_local = Vec3::new(0.0, -face.half_height, 0.0);
    let hinge_world = face.base_translation + face.base_rotation * hinge_local;
    let translation = hinge_world - rotation * hinge_local;
    transform.translation = translation;
    transform.rotation = rotation;
    transform.scale = face.base_scale;
}

/// OoT-style smoothstep ease (matches the demo's `smoothstep`).
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One frame of the open/close fold ease: exponentially advance `amount` toward
/// `target` at `rate` over `dt` seconds, snapping when within `0.002`.
///
/// The `dt` is CLAMPED to [`MAX_FOLD_EASE_DT`]. The exponential ease
/// `1 - exp(-rate*dt)` saturates to ~1.0 for a large `dt`, which would collapse the
/// WHOLE fold into a single frame (`amount` jumps straight to `target`). That is
/// exactly what an embedding host hits on CLOSE — closing the menu typically
/// un-pauses the game and the resume frame carries a big delta (a one-frame hitch
/// from un-suspending the sim / re-acquiring render state). Unclamped, that hitch
/// eases `amount` 1.0 -> ~0 in one frame, so a host that gates the cube camera /
/// visibility on `amount` (to keep the fold on-screen) cuts it the very next frame
/// and the close reads as an instant SNAP instead of a fold. The standalone demo
/// never hitches (no pause/resume), so it never snapped — this was the
/// demo-vs-host regression. Capping `dt` keeps the ease frame-rate independent for
/// normal frames while making one spiky frame cost at most ~2 frames of progress.
fn ease_fold_amount(amount: f32, target: f32, rate: f32, dt: f32) -> f32 {
    let dt = dt.min(MAX_FOLD_EASE_DT);
    let open_step = 1.0 - (-rate * dt).exp();
    let mut next = amount + (target - amount) * open_step;
    if (next - target).abs() < 0.002 {
        next = target;
    }
    next
}

/// Feature B: drive every ring material's base-color alpha to
/// `base_alpha * KaleidoscopeOpenState::amount` so the cube cross-fades with the
/// open/close fold (matching the scrim) instead of popping. Cheap: mutates the
/// alpha channel on existing `StandardMaterial` assets in place (the spawn sites
/// each `materials.add(..)` a unique handle per entity, so this never aliases).
fn fade_kaleidoscope_materials(
    state: Res<KaleidoscopeOpenState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    faded: Query<(&KaleidoscopeFade, &MeshMaterial3d<StandardMaterial>)>,
    // Faded planes whose material handle was (re)spawned or swapped THIS frame:
    // `rebuild_cube_faces` (new planes, always Blend) and `sync_control_focus_visuals`
    // (focus recolor, new Blend handle). `Changed<MeshMaterial3d>` covers both adds
    // and replacements.
    touched: Query<
        (),
        (
            Changed<MeshMaterial3d<StandardMaterial>>,
            With<KaleidoscopeFade>,
        ),
    >,
    mut last_amount: Local<f32>,
) {
    let amount = state.amount.clamp(0.0, 1.0);
    // PERF (2026-06-10): SETTLE EARLY-OUT. Each plane's target (mode + alpha)
    // depends only on `amount` and its static `base_alpha`/textured-ness, so when
    // the fold amount is unchanged AND no faded plane's material was respawned or
    // recolored this frame, the whole sweep is a guaranteed no-op — skip it instead
    // of doing a `materials.get()` for every one of the ~hundreds of menu planes
    // every frame while the menu just sits open.
    if (amount - *last_amount).abs() <= f32::EPSILON && touched.is_empty() {
        return;
    }
    *last_amount = amount;
    // Restore the proven pre-Feature-B per-element alpha scheme once the fold is
    // fully open, so the cube stops z-fighting WITHOUT turning text/icons into
    // squares:
    //   * SOLID planes (panels, lines, borders, selection corners, scrollbar) ->
    //     `Opaque`. Opaque writes depth, so the per-face depth bands
    //     (DEPTH_BACKGROUND..DEPTH_SELECTION) are resolved by the GPU depth test
    //     instead of an unstable back-to-front transparent sort — no flicker.
    //   * TEXTURED planes (the Text3d glyph atlas, item icons — anything with a
    //     `base_color_texture`) -> `Blend`, alpha = `base_alpha * amount` so they
    //     cross-fade with the fold. Their texture is mostly transparent; drawing it
    //     Opaque renders the transparent texels as the base-colour box (the "text is
    //     just squares" / "icons look weird" regression). They stay Blend and
    //     depth-TEST against the opaque panels behind them (Bevy's transparent pass
    //     tests depth even though it doesn't write it), so the few transparent layers
    //     sort correctly over the solid background.
    // Fix 3 (open/close flicker): the per-element rule applies at ALL amounts — there
    // is no "everything Blend while folding" branch any more. During the open/close
    // fold, coplanar SOLID panels/lines drawn Blend z-fight as the cube rotates in
    // (the remaining flicker). Keeping solids Opaque (depth-writing) the WHOLE time
    // resolves the depth bands by the GPU depth test, so they never z-fight. Solids
    // therefore do NOT fade — they pop in/out with the fold geometry + the scrim,
    // which already carry the transition. Only TEXTURED planes cross-fade by `amount`.
    for (fade, material) in &faded {
        // Copy the few fields out so the immutable read ends before any get_mut.
        let Some((textured, cur_mode, cur_alpha)) = materials.get(&material.0).map(|m| {
            (
                m.base_color_texture.is_some(),
                m.alpha_mode,
                m.base_color.alpha(),
            )
        }) else {
            continue;
        };
        let (target_mode, target_alpha) = if textured {
            (AlphaMode::Blend, fade.base_alpha * amount)
        } else {
            (AlphaMode::Opaque, fade.base_alpha)
        };
        // Only invalidate (and re-extract) the asset when the mode or alpha actually
        // needs to change — avoids thrashing every material every frame while the
        // menu sits open or shut, and corrects planes freshly (re)spawned THIS frame
        // by rebuild_cube_faces / sync_control_focus_visuals (which always create
        // Blend): PostUpdate runs after them, so a republish-while-open settles the
        // new planes with no one-frame flicker.
        if cur_mode != target_mode || (cur_alpha - target_alpha).abs() > 1.0e-4 {
            if let Some(mat) = materials.get_mut(&material.0) {
                mat.alpha_mode = target_mode;
                mat.base_color.set_alpha(target_alpha);
            }
        }
    }
}

/// Feature C: keep each scrollbar TRACK's screen-space extent fresh by projecting
/// the track plane's top + bottom edges through the pause camera, so the drag
/// observer can map a pointer's vertical screen position into the neutral `0..=1`
/// fraction. Runs while the cube camera is active; bails otherwise (the menu is
/// shut). A headless test can skip this and set [`MenuScrollbar`] fields directly.
fn project_scrollbar_tracks(
    camera_query: Query<(&Camera, &GlobalTransform), With<KaleidoscopePauseCamera>>,
    mut scrollbars: Query<(&mut MenuScrollbar, &Dimension, &GlobalTransform)>,
) {
    let Some((camera, cam_transform)) = camera_query.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    for (mut bar, dimension, node_transform) in &mut scrollbars {
        let half_h = dimension.y * 0.5;
        // Top + bottom edge of the track in node-local space → world → screen.
        let top_world = node_transform.transform_point(Vec3::new(0.0, half_h, 0.0));
        let bottom_world = node_transform.transform_point(Vec3::new(0.0, -half_h, 0.0));
        let (Ok(top), Ok(bottom)) = (
            camera.world_to_viewport(cam_transform, top_world),
            camera.world_to_viewport(cam_transform, bottom_world),
        ) else {
            continue;
        };
        let top_y = top.y.min(bottom.y);
        let height = (bottom.y - top.y).abs();
        if height > f32::EPSILON {
            bar.track_top_y = top_y;
            bar.track_height = height;
        }
    }
}

/// Feature C: map a pointer position over a scrollbar track into the neutral
/// `0..=1` drag fraction (0 = top, 1 = bottom). `None` if the track has no
/// measured height yet (the projection has not run). Shared by the DragStart +
/// Drag observers so a press and a drag map identically.
fn scrollbar_fraction(bar: &MenuScrollbar, pointer_y: f32) -> Option<f32> {
    crate::scrollbar_fraction_from_rect(bar.track_top_y, bar.track_height, pointer_y)
}

/// Feature C: a press that lands on the scrollbar immediately jumps the scroll to
/// the pressed position (emits the neutral fraction), so a tap on the track moves
/// the thumb there — exactly like a desktop scrollbar.
fn scrollbar_drag_start(
    drag: On<Pointer<DragStart>>,
    bars: Query<&MenuScrollbar>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    if let Ok(bar) = bars.get(drag.entity) {
        if let Some(fraction) = scrollbar_fraction(bar, drag.pointer_location.position.y) {
            out.write(MenuScrollDragged { fraction });
        }
    }
}

/// Feature C: while dragging on the scrollbar, emit the neutral fraction for the
/// pointer's current position so the host updates the scroll position live (mouse
/// OR touch — both arrive as the same `Pointer<Drag>`).
fn scrollbar_drag(
    drag: On<Pointer<Drag>>,
    bars: Query<&MenuScrollbar>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    if let Ok(bar) = bars.get(drag.entity) {
        if let Some(fraction) = scrollbar_fraction(bar, drag.pointer_location.position.y) {
            out.write(MenuScrollDragged { fraction });
        }
    }
}

/// Fix 1: a press on the scrollbar marks the track as held by that pointer (and
/// jumps the scroll to the pressed position). `Pointer<Press>` fires through the
/// cube's custom picking backend exactly like `Pointer<Click>` does, so this is the
/// reliable entry the manual drag tracker keys off of (the `Pointer<Drag>`
/// observers above only fire when the picking core's drag continuity reaches the
/// cube, which the custom backend doesn't guarantee — both paths emit the same
/// `MenuScrollDragged` fraction, so having both is safe).
fn scrollbar_press(
    press: On<Pointer<Press>>,
    bars: Query<&MenuScrollbar>,
    mut drag: ResMut<ScrollbarDragState>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    if let Ok(bar) = bars.get(press.entity) {
        // Mark the held pointer + CACHE the track geometry in the RESOURCE so the
        // drag survives the per-step republish that respawns this entity.
        drag.pressed_by = Some(press.pointer_id);
        drag.track_top_y = bar.track_top_y;
        drag.track_height = bar.track_height;
        if let Some(fraction) = scrollbar_fraction(bar, press.pointer_location.position.y) {
            out.write(MenuScrollDragged { fraction });
        }
    }
}

/// Releasing the pointer ends the manual scrollbar drag (clears the resource).
fn scrollbar_release(release: On<Pointer<Release>>, mut drag: ResMut<ScrollbarDragState>) {
    if drag.pressed_by == Some(release.pointer_id) {
        drag.pressed_by = None;
    }
}

/// While a pointer is held on a scrollbar (`ScrollbarDragState.pressed_by`), emit
/// the neutral `MenuScrollDragged` fraction for that pointer's LIVE position each
/// frame against the CURRENT scrollbar entity (re-found by component, so it works
/// even after the per-step republish respawns the entity). This is what makes
/// click-and-drag work on the cube (mouse AND touch — both arrive as a
/// `PointerLocation`) where the custom 3D backend doesn't drive `Pointer<Drag>`.
fn scrollbar_press_drag(
    pointers: Query<(&PointerId, &PointerLocation)>,
    drag: Res<ScrollbarDragState>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    let Some(held) = drag.pressed_by else {
        return;
    };
    let Some(loc) = pointers
        .iter()
        .find(|(id, _)| **id == held)
        .and_then(|(_, loc)| loc.location())
    else {
        return;
    };
    // Map the live pointer onto the CACHED track rect (constant during a drag, and
    // valid across the respawn that zeroes a fresh entity's geometry).
    if let Some(fraction) =
        crate::scrollbar_fraction_from_rect(drag.track_top_y, drag.track_height, loc.position.y)
    {
        out.write(MenuScrollDragged { fraction });
    }
}

fn panel_depth(rect: MenuRect, actionable: bool) -> f32 {
    if actionable {
        return DEPTH_ACTION;
    }
    let near_full_page = rect.w > 98.0 && rect.h > 98.0;
    let edge_bar = rect.w < 1.5 || rect.h < 1.5;
    if near_full_page {
        DEPTH_BACKGROUND
    } else if edge_bar {
        DEPTH_EDGE
    } else if rect.w > 40.0 || rect.h > 35.0 {
        DEPTH_LARGE_PANEL
    } else {
        DEPTH_CARD
    }
}

/// True for the narrow, vertically-centred flanking page-turn buttons (the L/R
/// "switch subscreen" controls). Matched by shape (narrow + tall + near a left or
/// right edge) so any host that places edge buttons at the conventional rect gets
/// the dedicated depth band, independent of the host's exact pixel rect.
fn is_edge_button_rect(rect: MenuRect) -> bool {
    let narrow = rect.w <= 12.0;
    let tall = rect.h >= 8.0;
    let near_edge = rect.x <= 10.0 || (rect.x + rect.w) >= 90.0;
    narrow && tall && near_edge
}

fn control_color(kind: MenuControlKind, selected: bool, important: bool) -> Color {
    if selected {
        Color::srgba(0.98, 0.76, 0.26, 0.96)
    } else if important {
        Color::srgba(0.13, 0.34, 0.28, 0.96)
    } else {
        match kind {
            MenuControlKind::Item => Color::srgba(0.055, 0.074, 0.155, 0.96),
            MenuControlKind::Scrollbar => Color::srgba(0.42, 0.32, 0.08, 0.92),
            MenuControlKind::Action => Color::srgba(0.09, 0.12, 0.26, 0.96),
            _ => Color::srgba(0.055, 0.070, 0.145, 0.96),
        }
    }
}

fn disabled_control_color() -> Color {
    Color::srgba(0.040, 0.045, 0.075, 0.72)
}

/// Fix 1: the DIM scrollbar track (the full-height channel the thumb rides in). Much
/// darker than the old solid yellow so the bright thumb reads as the grab handle.
fn scrollbar_track_color() -> Color {
    Color::srgba(0.10, 0.09, 0.04, 0.55)
}

/// Fix 1: the BRIGHT scrollbar thumb (the grab handle / scroll-position indicator).
fn scrollbar_thumb_color() -> Color {
    Color::srgba(0.98, 0.82, 0.36, 0.95)
}

fn menu_color(color: MenuColor) -> Color {
    Color::srgba(color.r, color.g, color.b, color.a)
}

fn menu_srgba(color: MenuColor) -> Srgba {
    Srgba::new(color.r, color.g, color.b, color.a)
}

fn menu_align(align: MenuTextAlign) -> TextAlign {
    match align {
        MenuTextAlign::Left => TextAlign::Left,
        MenuTextAlign::Center => TextAlign::Center,
        MenuTextAlign::Right => TextAlign::Right,
    }
}

#[cfg(test)]
mod fade_tests;
#[cfg(test)]
mod fold_ease_tests;
#[cfg(test)]
mod scrollbar_tests;
#[cfg(test)]
mod scrollbar_thumb_tests;

mod page;
use page::{apply_dynamic_text, fade_color, render_page_model};
