//! The ONE canonical bevy_lunex 3D-cube inventory renderer (#31).
//!
//! Generic over the host's `PageId`/`Action`: the host publishes the ordered
//! faces via [`ActiveMenuPages`], and [`KaleidoscopeMenuPlugin`] spawns a pause
//! `Camera3d` + a ring of bevy_lunex faces, rebuilds them when the pages change,
//! rotates the ring so the active face turns to the camera, and folds the faces
//! open/closed in the OoT "subscreen" style.
//!
//! ## Tuning seam
//! All geometry/speeds/visual knobs live in [`KaleidoscopeMenuConfig`] (a `Resource`).
//! The plugin inserts a default if the host has not; the host (or demo) may
//! insert its own before adding the plugin to match its exact values.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::camera::visibility::RenderLayers;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::picking::backend::{HitData, PointerHits};
use bevy::picking::pointer::{PointerId, PointerLocation};
use bevy::picking::{Pickable, PickingSystems};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_lunex::prelude::*;

use ambition_menu::{
    scrollbar_thumb_layout, ActiveMenuPages, AmbitionMenuControl, AmbitionMenuPage,
    AmbitionMenuRoot, MenuColor, MenuControlKind, MenuCubeGeometry, MenuDynamicText,
    MenuDynamicTextContent, MenuFocusKey, MenuNode, MenuOpenCloseStyle, MenuPageModel, MenuRect,
    MenuScrollDragged, MenuTextAlign, MenuVisualState, ScrollThumb, ScrollbarDragState,
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
#[require(KaleidoscopeOpenState)]
pub struct MenuRing;

/// The publication applied to one ring. The renderer owns this cache.
#[derive(Component)]
pub struct RenderedMenuPages<PageId> {
    version: u64,
    active: Option<PageId>,
}

/// System set for the lib's in-place focus-visual readers
/// ([`sync_control_focus_visuals`] + [`sync_selection_corner_visuals`]), both gated on
/// `Changed<MenuVisualState>`.
///
/// These readers turn the host's `MenuVisualState` (which the host writes from its ECS focus
/// cursor) into the on-screen cursor — the material recolour and the white selection corners.
/// The lib already orders this set AFTER [`rebuild_cube_faces`] so a republish that respawns
/// the controls can't wipe the flags after the writer set them; the host completes the ordering
/// by running its writer `.before(KaleidoscopeFocusVisuals)`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KaleidoscopeFocusVisuals;

/// System set holding the cube's per-frame RENDER + render-adjacent systems
/// ([`rebuild_cube_faces`], [`animate_cube_ring`], [`apply_dynamic_text`],
/// [`fade_kaleidoscope_materials`], the [`KaleidoscopeFocusVisuals`] readers,
/// [`project_scrollbar_tracks`]). The `PreUpdate`-schedule
/// half of cube rendering (the 3D picking backend) lives in [`KaleidoscopeRenderPre`].
/// This set runs every frame by default. A host that uses another presentation
/// may `.run_if(...)` this set and [`KaleidoscopeRenderPre`] to render only the
/// active backend. This only saves CPU: the cube input systems gate themselves.
///
/// [`KaleidoscopeFocusVisuals`] is a member, so gating this set gates it too.
/// Its `.after(rebuild_cube_faces)` ordering stays.
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
    /// Greyed out but focusable (`MenuNode::Control::dimmed`): grey while the
    /// cursor is elsewhere, highlighted when it is here.
    dimmed: bool,
}

/// Marks a control that was spawned on the ACTIVE face (the one turned to the
/// camera). The cube spawns every face's controls at once, so a focus key (an edge
/// page-turn button, a row index) can collide across faces — a host highlight
/// writer that matched purely by focus would light up the same-keyed control on the
/// side/back faces too. Filtering the writer to `With<KaleidoscopeActiveFaceControl>`
/// keeps the cursor highlight on the active face only. Re-stamped every rebuild
/// (a page turn republishes), so it always reflects the current active page.
#[derive(Component, Clone, Copy, Debug)]
pub struct KaleidoscopeActiveFaceControl;

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

/// Everything a live face's children were drawn from, kept on the face so
/// [`rebuild_cube_faces`] can tell what a new publication changed.
///
/// [`ActiveMenuPages`] has one `version` counter for all pages, so any change
/// would otherwise touch all four faces. Equality leaves the face alone; a
/// difference in its nodes or its `active` flag is reconciled in place, node by
/// node against each child's [`RenderedNode`].
///
/// The `config`-derived geometry is not stored: `rebuild_cube_faces` forces a
/// full rebuild when [`KaleidoscopeMenuConfig`] changes.
#[derive(Component, PartialEq)]
pub struct RenderedFace<PageId, Action> {
    /// The page model `render_page_model` was called with.
    pub model: MenuPageModel<PageId, Action>,
    /// Whether it was rendered as the ACTIVE face (baked into depth + control markers).
    pub active: bool,
    /// The face's index on the ring, and the ring size — together these fix the
    /// face's angle and transform.
    pub index: usize,
    pub page_count: usize,
}

/// Caps how much a single hitchy frame (e.g. the host un-pausing the game on close) can advance the
/// exponential ease, so the fold can never collapse into one frame and snap shut. See
/// [`animate_cube_ring`].
const MAX_FOLD_EASE_DT: f32 = 1.0 / 30.0;

/// Eased open amount for the cube menu (0 = folded shut, 1 = laid flat/open).
///
/// The host sets [`KaleidoscopeOpenState::target`]; [`animate_cube_ring`] eases `amount`
/// toward it each frame and folds the faces accordingly. The host also reads
/// `amount` to drive camera/visibility so the close animation is visible.
#[derive(Component, Default)]
#[require(MaterialFadeState)]
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
    /// Whether the cube camera clears the screen. Transparent-overlay hosts (the
    /// game) leave it off so the live world camera shows through; a standalone
    /// demo with no world camera underneath turns it on.
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
    /// The renderer's colours.
    pub look: KaleidoscopeLook,
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
            look: KaleidoscopeLook::default(),
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

/// Redraw every [`Text3d`] when the shared glyph atlas is rebuilt.
///
/// `bevy_rich_text3d` 0.7 rebuilds the atlas in two cases and leaves text that
/// was already drawn wrong in both. Measured on desktop at a phone's raster
/// scale; a phone grows the atlas on the first long page, because its glyphs
/// are about 2.6 times the desktop size.
///
/// - **The window scale factor changes.** The atlas is emptied, and drawn text
///   keeps its old glyph rectangles.
/// - **A full atlas doubles in height.** That makes a new GPU texture. The
///   library tells each text's material by calling `Assets::get_mut` and
///   dropping the result, which Bevy 0.19 no longer counts as a change, so
///   the material keeps the old texture and the text shows squashed pieces of
///   the wrong glyphs. Also, the glyph that did not fit is given a rectangle
///   but never drawn, and stays blank.
///
/// So on either rebuild this lays all text out again and really marks its
/// materials changed. After a growth it first empties the atlas, so the blank
/// glyph is drawn too. The redraw lands one frame after the rebuild.
fn redraw_text3d_when_the_atlas_is_rebuilt(
    settings: Res<Text3dPlugin>,
    mut images: ResMut<Assets<Image>>,
    mut atlases: ResMut<Assets<TextAtlas>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut drawn_in: Local<Option<(f32, UVec2)>>,
    mut texts: Query<(&mut Text3d, Option<&MeshMaterial3d<StandardMaterial>>)>,
) {
    let atlas_size = images
        .get(&TextAtlas::DEFAULT_IMAGE)
        .map_or(UVec2::ZERO, Image::size);
    let now = (settings.scale_factor, atlas_size);
    let Some(before) = drawn_in.replace(now) else {
        return;
    };
    if before == now {
        return;
    }
    if before.1 != now.1 {
        for (_, atlas) in atlases.iter_mut() {
            atlas.clear(&mut images);
        }
    }
    for (mut text, material) in &mut texts {
        text.set_changed();
        if let Some(mut material) = material.and_then(|held| materials.get_mut(&held.0)) {
            // The mutable borrow is what reports the change.
            let _: &mut StandardMaterial = &mut material;
        }
    }
}

impl<PageId, Action> Plugin for KaleidoscopeMenuPlugin<PageId, Action>
where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    // `PartialEq` on the host's action type is what lets `rebuild_cube_faces`
    // compare a published page against the face already drawing it, so only the
    // faces whose content moved get rebuilt.
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins);
        sleep_light_visibility_without_lights(app);
        // fontdb::load_system_fonts() is a no-op on Android (explicitly excluded
        // by fontdb via `not(target_os = "android")`). Seed /system/fonts/ on the
        // frame TextRenderer first becomes available so cosmic-text never sees an
        // empty database and panics with "no default font found".
        #[cfg(target_os = "android")]
        app.add_systems(
            Update,
            seed_text3d_android_fonts.run_if(resource_added::<TextRenderer>),
        );
        app.add_systems(Update, redraw_text3d_when_the_atlas_is_rebuilt);
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
            // Both readers live in the public [`KaleidoscopeFocusVisuals`] set, ordered AFTER
            // `rebuild_cube_faces` so a republish that respawns or rewrites the controls
            // can't wipe the host writer's focus flags after they're set.
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
            ambition_menu::install_menu_scroll_interactions(app);
            app.add_systems(
                Update,
                project_scrollbar_tracks
                    .in_set(KaleidoscopeRender)
                    .in_set(ambition_menu::MenuScrollGeometry::Projection),
            );
            app.add_observer(scrollbar_drag_start);
            app.add_observer(scrollbar_drag);
            // Fix 1: the manual press+move tracker (the robust path for the cube's
            // custom picking backend). `Pointer<Press>`/`Pointer<Release>` mark the
            // track pressed; `scrollbar_press_drag` then emits `MenuScrollDragged`
            // for the pointer's live position each frame while pressed — so dragging
            // the thumb scrolls even where `Pointer<Drag>` continuity doesn't reach.
            app.add_observer(scrollbar_press);
        }
        // Feature B: cross-fade the whole cube (faces/controls/text/icons) with the
        // open/close fold `amount`, so it fades in/out like the scrim instead of
        // popping. Runs in PostUpdate AFTER `sync_control_focus_visuals` (which
        // recolors a control's material in place this frame) so the fade always lands
        // the alpha on top of it; ordered after the animate step (Update) that advances
        // `amount`. Cheap: it only mutates the base-color alpha on existing assets.
        app.add_systems(PostUpdate,
            (assign_scene_layers, bind_scene_layers).chain()
                .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility));
        app.add_systems(PostUpdate, fit_cube_camera_regions
            .after(ambition_menu::render::MenuSurfaceLayout)
            .before(bevy::transform::TransformSystems::Propagate)
            .before(bevy::camera::CameraUpdateSystems));
        app.add_systems(
            PostUpdate,
            fade_kaleidoscope_materials.in_set(KaleidoscopeRender),
        );
        app.add_systems(Update, setup_cube::<PageId, Action>.before(KaleidoscopeRender))
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

fn setup_cube<PageId: Send + Sync + 'static, Action: Send + Sync + 'static>(
    mut commands: Commands,
    config: Res<KaleidoscopeMenuConfig>,
    menus: Query<Entity, (With<ActiveMenuPages<PageId, Action>>, Without<MenuRing>)>,
) {
    let geo = config.geometry;
    for menu in &menus {
        commands.spawn((
            Name::new("Cube pause camera"),
            KaleidoscopePauseCamera,
            ambition_menu::render::MenuPresentationOf(menu),
            ambition_menu::MenuSurface(menu),
            Camera3d::default(),
            // The cube is an unlit UI scene, not an HDR-lit world. Opt out of the
            // default LUT-backed Camera3d tonemapper: it adds measurable frame cost and,
            // when its optional LUT payload is trimmed, Bevy substitutes a magenta
            // diagnostic texture. `None` is both the intended look and the cheap path.
            Tonemapping::None,
            // A Camera3d composited over a Camera2d on the same window MUST share its
            // sample count or it renders its clear but drops all geometry. The game's
            // Camera2d uses Bevy's default (Sample4); pin the same value explicitly.
            // (A/B evidence also showed `Msaa::Off` recovers no frame time, so there
            // is no perf reason to diverge.)
            Msaa::Sample4,
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
            // Set the vertical FOV explicitly from the SAME constant the geometry derives
            // the camera distance from (instead of relying on Bevy's default happening to
            // be 45°), so the framing margin stays correct if that default ever changes.
            Projection::Perspective(PerspectiveProjection {
                fov: ambition_menu::MenuCubeGeometry::CAMERA_FOV_RADIANS,
                ..default()
            }),
            Transform::from_translation(Vec3::new(0.0, geo.camera_y, -geo.camera_distance))
                .looking_at(Vec3::new(0.0, geo.look_y, 0.0), Vec3::Y),
        ));
        commands.entity(menu).insert((
            Name::new("Cube menu ring"),
            AmbitionMenuRoot,
            ambition_menu::MenuSurface(menu),
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
            &ambition_menu::MenuSurface,
        ),
        With<KaleidoscopePauseCamera>,
    >,
    // Only interactive control planes on the active face are pick candidates:
    //
    // * `KaleidoscopeControlStyle`: only `spawn_control` adds it. Without this
    //   gate, a non-control plane (for example, the face-sized `UiRoot3d` plane)
    //   can win the depth sort and drop the click, because it has no
    //   `AmbitionMenuControl`.
    // * `KaleidoscopeActiveFaceControl`: every face's controls exist at once, and
    //   rotated-away faces are still visible to picking. This marker limits
    //   hover, press and drag to the face turned to the camera.
    nodes: Query<
        (
            Entity,
            &Dimension,
            &GlobalTransform,
            Option<&Pickable>,
            &ViewVisibility,
            &ambition_menu::MenuSurface,
        ),
        (
            With<KaleidoscopeControlStyle>,
            With<KaleidoscopeActiveFaceControl>,
        ),
    >,
    mut output: MessageWriter<PointerHits>,
) {
    let primary_window = primary_window.single().ok();
    for (cam_entity, camera, render_target, cam_transform, camera_surface) in camera_query
        .iter()
        .filter(|(_, camera, _, _, _)| camera.is_active)
    {
        // Hoverable control planes only (skip IGNORE: disabled controls). Decoration /
        // panels / text / the face root are already excluded by the `KaleidoscopeControlStyle`
        // query filter above.
        let candidates: Vec<_> = nodes
            .iter()
            .filter(|(_, _, transform, pickable, vis, surface)| {
                surface.0 == camera_surface.0
                    && vis.get()
                    && !transform.affine().is_nan()
                    && pickable.map(|p| p.is_hoverable).unwrap_or(true)
            })
            .map(|(entity, dimension, transform, pickable, _, _)| {
                (entity, dimension, transform, pickable)
            })
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

            if camera
                .logical_viewport_rect()
                .is_some_and(|rect| !rect.contains(location.position))
            {
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
}

/// Recolor each control's material from its live [`MenuVisualState`] so keyboard / gamepad
/// focus and pointer hover are VISIBLE.
///
/// Non-generic (keyed off [`KaleidoscopeControlStyle`]) so it doesn't need the host's
/// `Action`.
///
/// The recolor writes the control's existing material in place, so a cursor
/// step does not create a new asset and bind group. `spawn_control` gives each
/// control its own handle, so nothing else aliases the asset.
///
/// This system owns RGB and republishes `KaleidoscopeFade::base_alpha`.
/// [`fade_kaleidoscope_materials`] owns alpha and `alpha_mode` and runs later
/// in the same frame (PostUpdate). Because the handle does not change, the
/// fade's `touched` filter watches `Changed<MenuVisualState>`.
pub fn sync_control_focus_visuals(
    config: Res<KaleidoscopeMenuConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut controls: Query<
        (
            &KaleidoscopeControlStyle,
            &MenuVisualState,
            &MeshMaterial3d<StandardMaterial>,
            // Feature B: keep this control's recorded design alpha in sync with its
            // recoloured (focused/hover) material so `fade_kaleidoscope_materials`
            // fades the new highlight colour at the right base alpha.
            Option<&mut KaleidoscopeFade>,
        ),
        Changed<MenuVisualState>,
    >,
) {
    for (style, vis, material, fade) in &mut controls {
        let highlight = vis.focused || vis.selected || vis.hovered;
        let color = if style.disabled || (style.dimmed && !highlight) {
            config.look.disabled
        } else {
            config.look.control_color(style.kind, highlight, style.important)
        };
        let base_alpha = color.alpha();
        // A control with no `KaleidoscopeFade` has no one driving its alpha, so this
        // system writes the design alpha itself rather than leaving it at whatever
        // the last state used. `spawn_control` always attaches one; this is the
        // honest answer for a control spawned by something that does not.
        let faded = fade.is_some();
        if let Some(mut fade) = fade {
            fade.base_alpha = base_alpha;
        }
        // Read first and compare, so an unchanged colour never marks the asset
        // changed — a bare `get_mut` costs a re-extract and a GPU re-upload.
        let want = color.to_srgba();
        let Some(current) = materials.get(&material.0).map(|m| m.base_color.to_srgba()) else {
            continue;
        };
        // The alpha channel belongs to the fade when there is one; preserve what it
        // last wrote instead of stomping it back to the design value every frame.
        let alpha = if faded { current.alpha } else { base_alpha };
        if current.red == want.red
            && current.green == want.green
            && current.blue == want.blue
            && current.alpha == alpha
        {
            continue;
        }
        if let Some(mut asset) = materials.get_mut(&material.0) {
            asset.base_color = Color::srgba(want.red, want.green, want.blue, alpha);
        }
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

fn renderer_page_identity_changed<PageId, Action>(
    last_version: Option<u64>,
    last_active: Option<&PageId>,
    pages: &ActiveMenuPages<PageId, Action>,
) -> bool
where
    PageId: PartialEq,
    Action: Send + Sync + 'static,
{
    last_version != Some(pages.version) || last_active != pages.active.as_ref()
}

/// Bring the ring's faces up to the host's published pages whenever they change.
///
/// A face that still draws the same page at the same place on the ring is kept
/// and reconciled in place (`reconcile`): its changed nodes are rewritten on the
/// entities already drawing them. Only a face whose page or ring slot moved, or
/// every face on a config change, is despawned and spawned again.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_cube_faces<PageId, Action>(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    config: Res<KaleidoscopeMenuConfig>,
    rings: Query<(Entity, &ActiveMenuPages<PageId, Action>, Option<&RenderedMenuPages<PageId>>), With<MenuRing>>,
    faces: Query<(Entity, &ChildOf, Option<&RenderedFace<PageId, Action>>), With<AmbitionMenuPage<PageId>>>,
    live: reconcile::LiveFaceNodes<Action>,
) where
    PageId: Clone + PartialEq + Send + Sync + 'static,
    Action: Clone + PartialEq + Send + Sync + 'static,
{
    let config_changed = config.is_changed();
    for (ring, pages, rendered) in &rings {
        // Publication identity is local to this ring. Broad component change
        // ticks do not invalidate content whose values remain the same.
        if !config_changed && !renderer_page_identity_changed(
            rendered.map(|previous| previous.version),
            rendered.and_then(|previous| previous.active.as_ref()),
            pages,
        ) { continue; }
        let geo = config.geometry;
        let page_count = pages.pages.len();
        let n = page_count.max(1) as f32;
        let flip = config.inside_x_flip;
        // Every face's geometry and styling is read out of the config at spawn time, so
        // a config change invalidates all of them at once. This is the rare path, and
        // paying for it wholesale keeps reconciliation to the page's own content.

        // Decide, per PUBLISHED page, whether a live face can be brought to draw it in
        // place: the same page, in the same ring slot, on the same background. What
        // differs beyond that — its nodes, whether it is the active face — is
        // reconciled on the entities already standing.
        let mut kept: Vec<Option<Entity>> = vec![None; page_count];
        let mut retired: Vec<Entity> = Vec::new();
        for (entity, parent, rendered) in &faces {
            if parent.parent() != ring { continue; }
            let keep = match (config_changed, rendered) {
                (false, Some(rendered)) => pages
                    .pages
                    .iter()
                    .position(|model| model.id == rendered.model.id)
                    .filter(|&i| {
                        rendered.index == i
                            && rendered.page_count == page_count
                            && rendered.model.background == pages.pages[i].background
                    }),
                // A face with no `RenderedFace` cannot be proven current, so it is
                // retired and rebuilt rather than trusted.
                _ => None,
            };
            match keep {
                // `is_none()` also retires a duplicate face claiming a page another
                // face already holds; the ring must never carry two of one page.
                Some(i) if kept[i].is_none() => {
                    let model = &pages.pages[i];
                    let active = pages.active.as_ref() == Some(&model.id);
                    let rendered = rendered.expect("a kept face carries its RenderedFace");
                    if rendered.model != *model || rendered.active != active {
                        let mut out = reconcile::FaceWriter {
                            menu: ring,
                            commands: &mut commands,
                            materials: &mut materials,
                            asset_server: &asset_server,
                            config: &config,
                        };
                        reconcile::reconcile_face(&mut out, &live, entity, rendered, model, active);
                        commands.entity(entity).insert((
                            AmbitionMenuPage {
                                id: model.id.clone(),
                                active,
                            },
                            RenderedFace {
                                model: model.clone(),
                                active,
                                index: i,
                                page_count,
                            },
                        ));
                    }
                    kept[i] = Some(entity);
                }
                _ => retired.push(entity),
            }
        }
        for face in &retired {
            commands.entity(*face).despawn();
        }
        debug!(
            target: "ambition_platformer2d::kaleidoscope_rebuild",
            "cube faces version={} page_count={} spawned={} kept={} config_changed={}",
            pages.version,
            page_count,
            page_count - kept.iter().filter(|slot| slot.is_some()).count(),
            kept.iter().filter(|slot| slot.is_some()).count(),
            config_changed,
        );

        let menu = ring;
        commands.entity(ring).with_children(|ring| {
            for (i, model) in pages.pages.iter().enumerate() {
                if kept[i].is_some() {
                    continue;
                }
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
                    ambition_menu::MenuSurface(menu),
                    Name::new("Cube face"),
                    AmbitionMenuPage {
                        id: model.id.clone(),
                        active,
                    },
                    RenderedFace {
                        model: model.clone(),
                        active,
                        index: i,
                        page_count,
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
                    render_page_model(
                        ui,
                        &mut materials,
                        &asset_server,
                        &config,
                        menu,
                        model,
                        active,
                    )
                });
            }
        });
        commands.entity(ring).insert(RenderedMenuPages {
            version: pages.version,
            active: pages.active.clone(),
        });
    }
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
    mut rings: Query<
        (
            Entity,
            &ActiveMenuPages<PageId, Action>,
            &mut KaleidoscopeOpenState,
            &mut Transform,
        ),
        (With<MenuRing>, Without<CubeFace>),
    >,
    mut faces: Query<(&ChildOf, &CubeFace, &mut Transform), Without<MenuRing>>,
) where
    PageId: PartialEq + Send + Sync + 'static,
    Action: Send + Sync + 'static,
{
    for (menu, pages, mut state, mut ring_t) in &mut rings {
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

        // Opening spin: start the ring one page-step toward the viewer-right
        // neighbour and ease to 0 with the open `amount`, so the active page swings
        // to the front. `from_rotation_y(-idx * TAU/n)` brings the viewer-left
        // neighbour (`idx+1`) to front for a positive step, so the offset starts
        // negative.
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

        let amount_settled =
            new_amount == state.target && (new_amount - state.amount).abs() <= f32::EPSILON;
        let rotation_settled = ring_t.rotation.angle_between(target) < 1.0e-4;
        if amount_settled && rotation_settled {
            continue;
        }
        state.amount = new_amount;

        match config.open_close_style {
            MenuOpenCloseStyle::SmoothScale => {
                let scale = config.min_open_scale + (1.0 - config.min_open_scale) * open;
                ring_t.rotation = spin;
                ring_t.scale = Vec3::splat(scale);
                ring_t.translation = Vec3::new(0.0, -0.05 * (1.0 - open), -0.42 * (1.0 - open));
                for (parent, face, mut t) in &mut faces {
                    if parent.parent() != menu {
                        continue;
                    }
                    reset_face_transform(face, &mut t);
                }
            }
            MenuOpenCloseStyle::OotPageFold => {
                ring_t.rotation = spin;
                ring_t.scale = Vec3::ONE;
                ring_t.translation = Vec3::new(0.0, -0.10 * (1.0 - open), 0.0);
                let fold = config.fold_radians * (1.0 - open);
                for (parent, face, mut t) in &mut faces {
                    if parent.parent() != menu {
                        continue;
                    }
                    apply_face_fold(face, fold, &mut t);
                }
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

/// Fold a face around its bottom-edge tangent in ring space.
/// The tangent is `(cos(theta), 0, -sin(theta))`; pre-multiplying the fold
/// rotation applies it in the parent ring frame for any face count.
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
/// `dt` is clamped to [`MAX_FOLD_EASE_DT`]. For a large `dt`,
/// `1 - exp(-rate*dt)` is about 1, and the whole fold happens in one frame.
/// Closing the menu often un-pauses the game, and that resume frame has a
/// large delta, so a host that gates the cube on `amount` would show a snap
/// instead of a fold. With the clamp, one slow frame costs at most about two
/// frames of progress.
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
    mut states: Query<(&KaleidoscopeOpenState, &mut MaterialFadeState)>,
    surfaces: Query<&ambition_menu::MenuSurface>,
    parents: Query<&ChildOf>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    faded: Query<(Entity, &KaleidoscopeFade, &MeshMaterial3d<StandardMaterial>)>,
    // Faded planes that need this sweep this frame. Two causes:
    //
    // * `Changed<MeshMaterial3d>`: a plane `rebuild_cube_faces` just spawned.
    //   Solid planes are born Opaque (`page::solid_plane_alpha_mode`). Textured
    //   planes are born Blend; only their alpha moves to the current fold amount.
    //   Do not flip the mode on the spawn frame: the plane would sit in no render
    //   phase for one frame and flash.
    // * `Changed<MenuVisualState>`: `sync_control_focus_visuals` recolors in
    //   place, so the handle does not change and the first filter cannot see it.
    // * `Changed<KaleidoscopeFade>`: `reconcile` rewrote the material in place at
    //   its design alpha, and the fold amount has to be applied to it again.
    touched: Query<
        (),
        (
            Or<(
                Changed<MeshMaterial3d<StandardMaterial>>,
                Changed<MenuVisualState>,
                Changed<KaleidoscopeFade>,
            )>,
            With<KaleidoscopeFade>,
        ),
    >,
) {
    let any_amount_changed = states
        .iter()
        .any(|(state, previous)| previous.amount != Some(state.amount.clamp(0.0, 1.0)));
    if !any_amount_changed && touched.is_empty() {
        return;
    }
    // Solid planes remain opaque so depth bands resolve without z-fighting.
    // Textured planes remain blended so transparent glyph/icon texels stay
    // transparent; only those planes cross-fade with `amount`.
    for (entity, fade, material) in &faded {
        let Some(owner) = scene_menu(entity, &surfaces, &parents) else {
            continue;
        };
        let Ok((state, previous)) = states.get(owner) else {
            continue;
        };
        let amount = state.amount.clamp(0.0, 1.0);
        if previous.amount == Some(amount) && !touched.contains(entity) {
            continue;
        }
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
        // menu sits open or shut. For planes freshly spawned THIS frame by
        // `rebuild_cube_faces` only a textured plane's alpha can differ (solid ones
        // are born in their final mode); a focus move recolors in place. Alpha
        // alone is a material re-upload, never a pipeline change.
        if cur_mode != target_mode || (cur_alpha - target_alpha).abs() > 1.0e-4 {
            if let Some(mut mat) = materials.get_mut(&material.0) {
                mat.alpha_mode = target_mode;
                mat.base_color.set_alpha(target_alpha);
            }
        }
    }
    for (state, mut previous) in &mut states {
        let amount = Some(state.amount.clamp(0.0, 1.0));
        if previous.amount != amount {
            previous.amount = amount;
        }
    }
}

/// Feature C: keep each scrollbar TRACK's screen-space extent fresh by projecting
/// the track plane's top + bottom edges through the pause camera, so the drag
/// observer can map a pointer's vertical screen position into the neutral `0..=1`
/// fraction. Runs while the cube camera is active; bails otherwise (the menu is
/// shut). A headless test can skip this and set [`MenuScrollbar`] fields directly.
fn project_scrollbar_tracks(
    camera_query: Query<
        (&Camera, &GlobalTransform, &ambition_menu::MenuSurface),
        With<KaleidoscopePauseCamera>,
    >,
    mut scrollbars: Query<(
        &mut MenuScrollbar,
        &Dimension,
        &GlobalTransform,
        &ambition_menu::MenuSurface,
    )>,
    mut states: Query<&mut ScrollbarDragState>,
) {
    for (mut bar, dimension, node_transform, surface) in &mut scrollbars {
        let Some((camera, cam_transform, _)) = camera_query
            .iter()
            .find(|(camera, _, owner)| camera.is_active && owner.0 == surface.0)
        else {
            continue;
        };
        let Ok(mut drag) = states.get_mut(surface.0) else {
            continue;
        };
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
            // Mirror the LAST KNOWN GOOD rect into the shared resource so the press
            // jump + the manual drag tracker map against geometry that is always
            // valid — even on the frame the per-step republish respawns the track
            // entity with a not-yet-projected (zero) transform.
            drag.track_top_y = top_y;
            drag.track_height = height;
        }
    }
}

fn scrollbar_fraction(bar: &MenuScrollbar, pointer_y: f32) -> Option<f32> {
    ambition_menu::scrollbar_fraction_from_rect(bar.track_top_y, bar.track_height, pointer_y)
}

/// Feature C: a press that lands on the scrollbar immediately jumps the scroll to
/// the pressed position (emits the neutral fraction), so a tap on the track moves
/// the thumb there — exactly like a desktop scrollbar.
fn scrollbar_drag_start(
    drag: On<Pointer<DragStart>>,
    bars: Query<(&MenuScrollbar, &ambition_menu::MenuSurface)>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    if let Ok((bar, surface)) = bars.get(drag.entity) {
        if let Some(fraction) = scrollbar_fraction(bar, drag.pointer_location.position.y) {
            out.write(MenuScrollDragged {
                menu: surface.0,
                fraction,
            });
        }
    }
}

/// Feature C: while dragging on the scrollbar, emit the neutral fraction for the
/// pointer's current position so the host updates the scroll position live (mouse
/// OR touch — both arrive as the same `Pointer<Drag>`).
fn scrollbar_drag(
    drag: On<Pointer<Drag>>,
    bars: Query<(&MenuScrollbar, &ambition_menu::MenuSurface)>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    if let Ok((bar, surface)) = bars.get(drag.entity) {
        if let Some(fraction) = scrollbar_fraction(bar, drag.pointer_location.position.y) {
            out.write(MenuScrollDragged {
                menu: surface.0,
                fraction,
            });
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
    bars: Query<&ambition_menu::MenuSurface, With<MenuScrollbar>>,
    mut states: Query<&mut ScrollbarDragState>,
    mut out: MessageWriter<MenuScrollDragged>,
) {
    let Ok(surface) = bars.get(press.entity) else {
        return;
    };
    let Ok(mut drag) = states.get_mut(surface.0) else {
        return;
    };
    drag.pressed_by = Some(press.pointer_id);
    if let Some(fraction) = ambition_menu::scrollbar_fraction_from_rect(
        drag.track_top_y,
        drag.track_height,
        press.pointer_location.position.y,
    ) {
        out.write(MenuScrollDragged {
            menu: surface.0,
            fraction,
        });
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

/// The renderer's colours. A host states its own look in
/// [`KaleidoscopeMenuConfig::look`]; the default is the renderer's own.
#[derive(Clone, Debug, PartialEq)]
pub struct KaleidoscopeLook {
    /// A control under the cursor.
    pub selected: Color,
    /// A control the host marks important.
    pub important: Color,
    /// An item cell.
    pub item: Color,
    /// An action button, and the page-turn buttons.
    pub action: Color,
    /// Any other control.
    pub control: Color,
    /// A control that cannot be used.
    pub disabled: Color,
    /// A scroll bar's track. Much darker than the thumb, so the thumb reads
    /// as the grab handle.
    pub scrollbar_track: Color,
    /// A scroll bar's thumb.
    pub scrollbar_thumb: Color,
    /// The frame around each face.
    pub edge_frame: Color,
    /// The tint of an item's icon that cannot be used, and of one under the
    /// cursor. Any other icon is not tinted.
    pub icon_disabled: Color,
    pub icon_selected: Color,
}

impl Default for KaleidoscopeLook {
    fn default() -> Self {
        Self {
            selected: Color::srgba(0.98, 0.76, 0.26, 0.96),
            important: Color::srgba(0.13, 0.34, 0.28, 0.96),
            item: Color::srgba(0.055, 0.074, 0.155, 0.96),
            action: Color::srgba(0.09, 0.12, 0.26, 0.96),
            control: Color::srgba(0.055, 0.070, 0.145, 0.96),
            disabled: Color::srgba(0.040, 0.045, 0.075, 0.72),
            scrollbar_track: Color::srgba(0.10, 0.09, 0.04, 0.55),
            scrollbar_thumb: Color::srgba(0.98, 0.82, 0.36, 0.95),
            edge_frame: Color::srgba(0.80, 0.92, 1.0, 0.62),
            icon_disabled: Color::srgba(0.55, 0.58, 0.66, 0.55),
            icon_selected: Color::srgb(1.0, 0.95, 0.78),
        }
    }
}

impl KaleidoscopeLook {
    /// The fill of a control that can be used.
    pub(crate) fn control_color(&self, kind: MenuControlKind, selected: bool, important: bool) -> Color {
        if selected {
            self.selected
        } else if important {
            self.important
        } else {
            match kind {
                MenuControlKind::Item => self.item,
                MenuControlKind::Scrollbar => self.scrollbar_track,
                MenuControlKind::Action => self.action,
                _ => self.control,
            }
        }
    }
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

/// Bevy's light visibility checks run only while a light exists.
///
/// This crate's `bevy_pbr` turns on Bevy's `LightPlugin` for the whole host,
/// and the cube is unlit. With no light, `check_dir_light_mesh_visibility`
/// still builds a new `ViewVisibility` query each frame, over every
/// archetype: ~0.45% of the phone's samples in the hub (findings F34, S2b).
/// A light that a game spawns wakes the checks in the same frame.
pub fn sleep_light_visibility_without_lights(app: &mut App) {
    use bevy::light::{DirectionalLight, PointLight, SimulationLightSystems, SpotLight};
    app.configure_sets(
        PostUpdate,
        SimulationLightSystems::CheckLightVisibility.run_if(
            any_with_component::<DirectionalLight>
                .or_else(any_with_component::<PointLight>)
                .or_else(any_with_component::<SpotLight>),
        ),
    );
}

#[cfg(test)]
mod fade_tests;
#[cfg(test)]
mod fold_ease_tests;
#[cfg(test)]
mod light_dormancy_tests;
#[cfg(test)]
mod per_face_rebuild_tests;
#[cfg(test)]
mod rebuild_gate_tests;
#[cfg(test)]
mod reconcile_tests;
#[cfg(test)]
mod scrollbar_tests;
#[cfg(test)]
mod scrollbar_thumb_tests;
#[cfg(test)]
mod tonemapping_tests;

mod page;
mod reconcile;
pub use page::RenderedNode;
// A host that knows when the cube rests puts these layout sets to sleep
// (lunex lays out only the cube).
pub use bevy_lunex::UiSystems;
use page::{apply_dynamic_text, render_page_model};

#[derive(Component, Default)]
struct MaterialFadeState {
    amount: Option<f32>,
}

fn scene_menu(
    mut entity: Entity,
    surfaces: &Query<&ambition_menu::MenuSurface>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    loop {
        if let Ok(surface) = surfaces.get(entity) {
            return Some(surface.0);
        }
        entity = parents.get(entity).ok()?.parent();
    }
}

fn assign_scene_layers(
    mut roots: Query<(Entity, &mut RenderLayers), With<MenuRing>>,
    mut ordered: Local<Vec<Entity>>,
) {
    ordered.clear();
    ordered.extend(roots.iter().map(|(entity, _)| entity));
    ordered.sort_unstable();
    for (ordinal, &entity) in ordered.iter().enumerate() {
        let (_, mut current) = roots.get_mut(entity).unwrap();
        let wanted =
            RenderLayers::from_layers(&[ambition_menu::render::menu_scene_render_layer(ordinal)]);
        if *current != wanted {
            *current = wanted;
        }
    }
}

fn bind_scene_layers(
    mut commands: Commands,
    roots: Query<(Entity, Ref<RenderLayers>), (With<MenuRing>, Without<KaleidoscopePauseCamera>)>,
    mut cameras: Query<
        (&ambition_menu::MenuSurface, &mut RenderLayers),
        (With<KaleidoscopePauseCamera>, Without<MenuRing>),
    >,
    nodes: Query<(Option<&RenderLayers>, Option<&Children>), Without<KaleidoscopePauseCamera>>,
    attached: Query<Entity, Changed<ChildOf>>,
    surfaces: Query<&ambition_menu::MenuSurface>,
    parents: Query<&ChildOf>,
) {
    for (owner, mut current) in &mut cameras {
        let Ok((_, wanted)) = roots.get(owner.0) else {
            continue;
        };
        if *current != *wanted {
            *current = (*wanted).clone();
        }
    }
    let mut apply = |start: Entity, wanted: &RenderLayers| {
        let mut pending = vec![start];
        while let Some(entity) = pending.pop() {
            let Ok((current, children)) = nodes.get(entity) else {
                continue;
            };
            if current != Some(wanted) {
                commands.entity(entity).insert(wanted.clone());
            }
            if let Some(children) = children {
                pending.extend(children.iter());
            }
        }
    };
    for (root, layers) in &roots {
        if layers.is_changed() {
            apply(root, &layers);
        }
    }
    for entity in &attached {
        let Some(menu) = scene_menu(entity, &surfaces, &parents) else {
            continue;
        };
        let Ok((_, layers)) = roots.get(menu) else {
            continue;
        };
        if !layers.is_changed() {
            apply(entity, &layers);
        }
    }
}

fn fit_cube_camera_regions(
    config: Res<KaleidoscopeMenuConfig>,
    mut cameras: Query<(&Camera, &mut Projection), With<KaleidoscopePauseCamera>>,
) {
    let geometry = config.geometry;
    for (camera, mut projection) in &mut cameras {
        let size = camera
            .viewport
            .as_ref()
            .map(|viewport| viewport.physical_size.as_vec2())
            .or_else(|| camera.logical_viewport_size());
        let Some(size) = size.filter(|size| size.cmpgt(Vec2::ZERO).all()) else {
            continue;
        };
        let ratio = (geometry.page_width / geometry.page_height / (size.x / size.y)).max(1.0);
        let fov = 2.0
            * ((ambition_menu::MenuCubeGeometry::CAMERA_FOV_RADIANS * 0.5).tan() * ratio).atan();
        // Keep the camera at the cube's viewing point. Moving back exposes the rear face.
        let Projection::Perspective(current) = &*projection else {
            continue;
        };
        if current.fov != fov {
            if let Projection::Perspective(current) = &mut *projection {
                current.fov = fov;
            }
        }
    }
}
