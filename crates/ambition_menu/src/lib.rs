//! Engine-side unified menu: the renderer-agnostic content model plus two
//! interchangeable presentations of it.
//!
//! This crate is split into host-owned DATA and renderer-owned PRESENTATION.
//! Hosts build generic [`MenuPageModel`] / [`ItemsOnlyPageSpec`] values from
//! their own resources, then translate the [`MenuActionActivated`] /
//! [`MenuClosedRequested`] messages this crate emits back into gameplay events;
//! it never names `OwnedItems`, health, or player components. This crate ships
//! the flat tabbed [`render::bevy_ui`] renderer; the bevy_lunex 3D OoT-style
//! cube renderer is the optional `ambition_menu_kaleidoscope` extension crate
//! (E1e) — both consume the same page model, which is what validates the seam.
//!
//! [`AmbitionInventoryUiPlugin`] installs only the renderer-agnostic
//! resources/messages, so a host can keep it even with no renderer enabled.

use bevy::prelude::{App, Component, Message, Plugin, Resource};

pub mod backend;
pub mod render;

/// A normalized page-space rectangle.
///
/// Coordinates are percentages in the page's local 2D layout space. `(0, 0)` is
/// the top-left corner and `(100, 100)` is the bottom-right corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl MenuRect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub const fn inset(self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            w: self.w - amount * 2.0,
            h: self.h - amount * 2.0,
        }
    }
}

/// Renderer-independent color token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl MenuColor {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);
    pub const DISABLED: Self = Self::rgba(0.28, 0.28, 0.34, 0.72);
    pub const BLUE_PANEL: Self = Self::rgba(0.03, 0.05, 0.16, 0.94);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Text alignment independent of the concrete renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuTextAlign {
    Left,
    Center,
    Right,
}

/// Broad semantic class for controls.
///
/// A renderer may style these differently, and a navigation policy may use this
/// to decide whether a control participates in focus, hover, or scroll.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuControlKind {
    Tab,
    Slot,
    Item,
    Action,
    PopupAction,
    OptionToggle,
    OptionChoice,
    MapMarker,
    Scrollbar,
    PopupPanel,
    Decoration,
}

/// A single page node.
///
/// `Action` is intentionally generic so games can use their own enum instead of
/// stringly typed callbacks. `Control::icon` is an optional asset path, relative
/// to Bevy's asset root.
#[derive(Clone, Debug)]
pub enum MenuNode<Action> {
    Panel {
        rect: MenuRect,
        color: MenuColor,
        action: Option<Action>,
    },
    Text {
        x: f32,
        y: f32,
        size: f32,
        text: String,
        align: MenuTextAlign,
        color: MenuColor,
    },
    /// A text node whose string is filled in place by the host (tagged with
    /// [`MenuDynamicText`]). Spawned empty; the host rewrites it by `slot` on
    /// cursor change, so cursor-dependent text (the detail panel) no longer needs
    /// a face rebuild.
    DynamicText {
        slot: u32,
        x: f32,
        y: f32,
        size: f32,
        align: MenuTextAlign,
        color: MenuColor,
    },
    Control {
        rect: MenuRect,
        kind: MenuControlKind,
        label: String,
        detail: Option<String>,
        icon: Option<String>,
        selected: bool,
        important: bool,
        action: Option<Action>,
        /// Scrollbar thumb geometry, as fractions `0..=1` of the track (Fix 1).
        /// Only meaningful for a [`MenuControlKind::Scrollbar`] control: `start` is
        /// the thumb's top as a fraction of the track height, `size` its height as a
        /// fraction. `None` (every non-scrollbar control) draws no thumb. The host
        /// computes these from its visible/total/window-start; the renderer draws a
        /// dim full-height track with a brighter thumb child at this geometry.
        thumb: Option<ScrollThumb>,
    },
}

/// Scrollbar thumb geometry as track fractions (Fix 1). Both in `0..=1`: `start`
/// is the thumb top (fraction of track height from the top), `size` the thumb
/// height (visible / total). A `size >= 1.0` means the list fits and no thumb is
/// needed; the host only emits a thumb when the list actually scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollThumb {
    pub start: f32,
    pub size: f32,
}

/// Resolve a [`ScrollThumb`] into `(top_fraction, height_fraction)` in `[0, 1]`
/// for a vertical scrollbar track. Single source of truth shared by BOTH menu
/// renderers (the cube + the bevy-UI grid), which previously each carried their
/// own mathematically-equivalent copy and had begun to drift cosmetically. The
/// height is floored grabbable (min 8% of the track) and the thumb travels the
/// remaining `1 - height`.
pub fn scrollbar_thumb_layout(thumb: ScrollThumb) -> (f32, f32) {
    let start = thumb.start.clamp(0.0, 1.0);
    let size = thumb.size.clamp(0.08, 1.0);
    let travel = (1.0 - size).max(0.0);
    (start * travel, size)
}

/// Map a pointer's position along a scrollbar track into the neutral `0..=1`
/// drag fraction (0 = top, 1 = bottom). `None` if the track has no height yet.
/// Single source of truth shared by both renderers (the cube extracts the track
/// rect from its `MenuScrollbar` component; the grid reads its measured node
/// rect) — the division used to be copied into each.
pub fn scrollbar_fraction_from_rect(
    track_top_y: f32,
    track_height: f32,
    pointer_y: f32,
) -> Option<f32> {
    if track_height <= f32::EPSILON {
        return None;
    }
    Some(((pointer_y - track_top_y) / track_height).clamp(0.0, 1.0))
}

/// Backend-agnostic scroll-drag channel (Feature C).
///
/// A renderer emits [`MenuScrollDragged`] carrying a NEUTRAL fraction in
/// `0..=1` (0 = top of the track, 1 = bottom). Neither renderer has any notion
/// of "scroll position" — the host interprets the fraction against its own
/// scrollable range. Both the bevy_ui grid and the `ambition_menu_kaleidoscope`
/// cube publish through this one message, so it lives in the shared model.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct MenuScrollDragged {
    /// Drag position along the track, `0.0` (top) .. `1.0` (bottom).
    pub fraction: f32,
}

/// Which pointer (if any) is mid-drag on a menu scrollbar. Held in a RESOURCE,
/// NOT on the scrollbar entity, because changing the scroll position triggers
/// the host's per-step republish, which DESPAWNS + respawns the scrollbar entity
/// each frame — a per-entity held flag would reset to `None` after the first step
/// and the drag would die. Keyed on the persistent `PointerId`, so the drag
/// survives any number of respawns. Shared by BOTH renderers (only one menu is
/// active at a time), so it lives in the shared model.
#[derive(Resource, Default)]
pub struct ScrollbarDragState {
    pub pressed_by: Option<bevy::picking::pointer::PointerId>,
    /// Track screen rect (top edge + height, logical px) CACHED at press time.
    /// The track never moves during a drag (only the pointer does), and the grid
    /// track's `ComputedNode`/`GlobalTransform` read as ZERO on the frame it is
    /// respawned by the per-step republish — so the manual tracker maps the live
    /// pointer against this cached rect rather than the just-respawned entity's
    /// geometry. Set on press, when the pressed track's geometry is valid.
    pub track_top_y: f32,
    pub track_height: f32,
}

impl<Action> MenuNode<Action> {
    pub fn action(&self) -> Option<&Action> {
        match self {
            Self::Panel { action, .. } | Self::Control { action, .. } => action.as_ref(),
            Self::Text { .. } | Self::DynamicText { .. } => None,
        }
    }

    pub fn action_mut(&mut self) -> Option<&mut Action> {
        match self {
            Self::Panel { action, .. } | Self::Control { action, .. } => action.as_mut(),
            Self::Text { .. } | Self::DynamicText { .. } => None,
        }
    }

    pub fn rect(&self) -> Option<MenuRect> {
        match self {
            Self::Panel { rect, .. } | Self::Control { rect, .. } => Some(*rect),
            Self::Text { .. } | Self::DynamicText { .. } => None,
        }
    }

    pub fn is_actionable(&self) -> bool {
        self.action().is_some()
    }
}

/// Full data description for one visible page/face of the cube menu.
#[derive(Clone, Debug)]
pub struct MenuPageModel<PageId, Action> {
    pub id: PageId,
    pub title: String,
    pub background: MenuColor,
    pub nodes: Vec<MenuNode<Action>>,
}

impl<PageId, Action> MenuPageModel<PageId, Action> {
    pub fn new(id: PageId, title: impl Into<String>, background: MenuColor) -> Self {
        Self {
            id,
            title: title.into(),
            background,
            nodes: Vec::new(),
        }
    }

    pub fn panel(&mut self, rect: MenuRect, color: MenuColor, action: Option<Action>) {
        self.nodes.push(MenuNode::Panel {
            rect,
            color,
            action,
        });
    }

    pub fn text(
        &mut self,
        x: f32,
        y: f32,
        size: f32,
        text: impl Into<String>,
        align: MenuTextAlign,
        color: MenuColor,
    ) {
        self.nodes.push(MenuNode::Text {
            x,
            y,
            size,
            text: text.into(),
            align,
            color,
        });
    }

    /// A host-filled text line (see [`MenuNode::DynamicText`] / [`MenuDynamicText`]).
    /// Spawned empty; the host rewrites it by `slot` so cursor-dependent text needs
    /// no face rebuild.
    pub fn dynamic_text(
        &mut self,
        slot: u32,
        x: f32,
        y: f32,
        size: f32,
        align: MenuTextAlign,
        color: MenuColor,
    ) {
        self.nodes.push(MenuNode::DynamicText {
            slot,
            x,
            y,
            size,
            align,
            color,
        });
    }

    pub fn control(
        &mut self,
        rect: MenuRect,
        kind: MenuControlKind,
        label: impl Into<String>,
        detail: Option<String>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    ) {
        self.control_with_icon(
            rect,
            kind,
            label,
            detail,
            Option::<String>::None,
            selected,
            important,
            action,
        );
    }

    pub fn control_with_icon<I>(
        &mut self,
        rect: MenuRect,
        kind: MenuControlKind,
        label: impl Into<String>,
        detail: Option<String>,
        icon: Option<I>,
        selected: bool,
        important: bool,
        action: Option<Action>,
    ) where
        I: Into<String>,
    {
        self.nodes.push(MenuNode::Control {
            rect,
            kind,
            label: label.into(),
            detail,
            icon: icon.map(Into::into),
            selected,
            important,
            action,
            thumb: None,
        });
    }

    /// Emit a draggable scrollbar control (Fix 1): a [`MenuControlKind::Scrollbar`]
    /// occupying `rect` (the full track), carrying the thumb geometry the renderer
    /// draws on top of the dim track. `thumb_start` / `thumb_size` are track
    /// fractions in `0..=1` (top / height). The host computes them from its own
    /// visible/total/window-start; the renderer owns the track+thumb visuals + the
    /// drag interaction. No `action`: dragging the track IS the interaction.
    pub fn scrollbar(&mut self, rect: MenuRect, thumb_start: f32, thumb_size: f32) {
        self.nodes.push(MenuNode::Control {
            rect,
            kind: MenuControlKind::Scrollbar,
            label: String::new(),
            detail: None,
            icon: None,
            selected: false,
            important: false,
            action: None,
            thumb: Some(ScrollThumb {
                start: thumb_start,
                size: thumb_size,
            }),
        });
    }

    pub fn actionable_nodes(&self) -> impl Iterator<Item = &MenuNode<Action>> {
        self.nodes.iter().filter(|node| node.is_actionable())
    }
}

/// Host-facing lifecycle/effect hook.
///
/// The UI layer queues these events; an Ambition integration can drain the
/// queue to play SFX, pause gameplay, or muffle music.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuShellEffect {
    Opening,
    Opened,
    Closing,
    Closed,
    PageChanged,
    Navigate,
    Activate,
    Cancel,
}

/// Queue of shell effects generated by the menu module.
///
/// This intentionally avoids hard-coding audio or music behavior into the UI.
#[derive(Resource, Default, Clone, Debug)]
pub struct MenuShellEffects {
    pub pending: Vec<MenuShellEffect>,
}

impl MenuShellEffects {
    pub fn push(&mut self, effect: MenuShellEffect) {
        self.pending.push(effect);
    }

    pub fn drain(&mut self) -> impl Iterator<Item = MenuShellEffect> + '_ {
        self.pending.drain(..)
    }
}

/// Coarse lifecycle phase derived from a shell's openness and target state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuShellPhase {
    Closed,
    Opening,
    Open,
    Closing,
}

/// Configurable touch policy for game-friendly menus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchActivationPolicy {
    ActivateOnFirstTap,
    SelectThenTap,
}

/// Pointer/touch gesture affordances supported by the menu shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuGesturePolicy {
    pub swipe_pages: bool,
    pub drag_off_cancels: bool,
    pub drag_scroll_panes: bool,
}

impl Default for MenuGesturePolicy {
    fn default() -> Self {
        Self {
            swipe_pages: true,
            drag_off_cancels: true,
            drag_scroll_panes: true,
        }
    }
}

/// Optional plugin marker for host games that want a single import point.
///
/// The Lunex renderer remains an optional backend. This plugin installs only
/// the renderer-agnostic resources/messages so it is safe for a host to keep
/// even when the Lunex implementation is removed.
pub struct AmbitionInventoryUiPlugin;

impl Plugin for AmbitionInventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuShellEffects>()
            .add_message::<MenuModelChanged>()
            .add_message::<MenuClosedRequested>();
    }
}

/// High-level shell animation style.
///
/// Keep the nostalgic OoT-inspired page fold opt-in at the reusable API level.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MenuOpenCloseStyle {
    #[default]
    SmoothScale,
    OotPageFold,
}

/// Reusable selection rendering hint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuSelectionEffect {
    Fill,
    Outline,
    CornerBrackets {
        corner_len_pct: f32,
        thickness_pct: f32,
    },
}

impl Default for MenuSelectionEffect {
    fn default() -> Self {
        Self::CornerBrackets {
            corner_len_pct: 24.0,
            thickness_pct: 4.0,
        }
    }
}

/// Cube/page geometry shared by renderers that want an OoT-like four-page room.
///
/// `page_width = 2 * page_radius` is the important source-derived relationship:
/// adjacent faces meet at visible cube edges instead of floating apart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MenuCubeGeometry {
    pub page_radius: f32,
    pub page_width: f32,
    pub page_height: f32,
    pub camera_distance: f32,
    pub camera_y: f32,
    pub look_y: f32,
}

impl MenuCubeGeometry {
    /// The cube camera's VERTICAL field of view (radians). Single source of truth:
    /// [`Self::oot_like`] derives the camera distance from it, AND the renderer sets
    /// the camera's `PerspectiveProjection.fov` to it — so the two can't silently
    /// disagree (previously the derivation implicitly assumed Bevy's 45° default).
    pub const CAMERA_FOV_RADIANS: f32 = core::f32::consts::FRAC_PI_4; // 45°

    /// Fraction of the half-screen-height the active face's top edge reaches; the
    /// remainder is the top/bottom margin. `0.80` → ~20% margin. The camera distance
    /// is DERIVED from this (below), so the margin is an explicit, readable knob
    /// rather than an emergent side effect of a magic camera offset — and it survives
    /// changes to the face aspect or page size.
    pub const TARGET_FACE_FILL: f32 = 0.80;

    pub fn oot_like(page_radius: f32) -> Self {
        let page_width = page_radius * 2.0;
        let page_height = page_width * (160.0 / 240.0);
        let face_half_height = page_height * 0.5;
        // The active face is a plane at z = +page_radius; the camera sits at
        // z = −camera_distance looking at the cube centre, so the camera-to-face
        // distance is `page_radius + camera_distance`. With a vertical-FOV perspective
        // camera the face's top edge reaches
        //     fill = face_half_height / (distance · tan(fov/2))
        // of the half-screen-height (aspect- AND page_radius-independent). Solve for
        // the distance that yields `TARGET_FACE_FILL`, then back out the camera offset.
        let distance =
            face_half_height / (Self::TARGET_FACE_FILL * (Self::CAMERA_FOV_RADIANS * 0.5).tan());
        Self {
            page_radius,
            page_width,
            page_height,
            camera_distance: distance - page_radius,
            camera_y: 0.0,
            look_y: 0.0,
        }
    }
}

impl Default for MenuCubeGeometry {
    fn default() -> Self {
        Self::oot_like(2.85)
    }
}

/// Marker for the root entity that owns a menu shell / menu room.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AmbitionMenuRoot;

/// ECS component attached to a rendered menu page/face.
#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct AmbitionMenuPage<PageId> {
    pub id: PageId,
    pub active: bool,
}

/// Stable navigation identity for focusable controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct MenuFocusKey {
    pub row: i32,
    pub col: i32,
    pub order: i32,
}

/// ECS component attached to rendered controls.
///
/// The data-driven builder is still the ergonomic API, but controls that make it
/// into the world should carry their semantic action/kind as components so
/// hover, focus, accessibility, and alternative input can be implemented by ECS
/// systems instead of renderer-private bookkeeping.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AmbitionMenuControl<Action> {
    pub kind: MenuControlKind,
    pub action: Option<Action>,
    pub focus: MenuFocusKey,
}

/// Runtime visual state for a control.
///
/// This belongs in ECS. It changes frequently from hover, focus, touch, and
/// gamepad navigation, while declarative page data can remain stable.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuVisualState {
    pub hovered: bool,
    pub focused: bool,
    pub selected: bool,
    pub pressed: bool,
    pub disabled: bool,
}

/// Marks a text node whose CONTENT is filled in place by the host every frame
/// (or on cursor change) rather than baked into the page data. This is how the
/// cursor-dependent detail panel updates WITHOUT a full face rebuild: the page
/// model stays cursor-independent (so a mouse move does not despawn/respawn the
/// controls and drop a `Pointer<Click>`), and the host rewrites the focused
/// item's / row's description by `slot` via the live `Text3d` on these entities.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuDynamicText {
    /// Stable identifier the host uses to address this text line in place.
    pub slot: u32,
}

/// The live string for a [`MenuDynamicText`] line. The host writes this (a plain
/// `String`, so the host never needs the text backend); a lib system copies it into
/// the entity's `Text3d` on change. This is the in-place channel that lets the host
/// rewrite cursor-dependent text WITHOUT a face rebuild.
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuDynamicTextContent(pub String);

/// ECS metadata for a scrollable viewport.
#[derive(Component, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuScrollPane {
    pub first_visible: usize,
    pub visible_rows: usize,
    pub total_rows: usize,
}

/// Renderer-independent active page set.
///
/// Host games may maintain this resource directly, or keep their own resources
/// and rebuild it only when the menu opens or item data changes. Renderers
/// should treat it as read-only input.
#[derive(Resource, Clone, Debug)]
pub struct ActiveMenuPages<PageId, Action> {
    pub pages: Vec<MenuPageModel<PageId, Action>>,
    pub active: Option<PageId>,
    pub visible: bool,
    pub version: u64,
}

impl<PageId, Action> Default for ActiveMenuPages<PageId, Action> {
    fn default() -> Self {
        Self {
            pages: Vec::new(),
            active: None,
            visible: false,
            version: 0,
        }
    }
}

impl<PageId, Action> ActiveMenuPages<PageId, Action> {
    pub fn replace_pages(&mut self, pages: Vec<MenuPageModel<PageId, Action>>, active: PageId) {
        self.pages = pages;
        self.active = Some(active);
        self.version = self.version.wrapping_add(1);
    }
}

/// A host-defined action was activated by the menu.
///
/// Ambition should map this back to its existing item/use/equip effects. The UI
/// crate deliberately does not know about `OwnedItems`, health, mana, or player
/// components.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct MenuActionActivated<Action> {
    pub action: Action,
}

/// A host-defined action is currently hovered or focused.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct MenuActionPreviewed<Action> {
    pub action: Action,
}

/// The host or renderer requested a model refresh.
#[derive(Message, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuModelChanged;

/// The UI requested that the host close the menu.
#[derive(Message, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MenuClosedRequested;

/// Stable slot identity for an items-only inventory page.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct InventorySlotId(pub usize);

/// Host-provided description of one inventory slot.
///
/// The action is generic and optional. If `owned` is false or `disabled` is
/// true, [`ItemsOnlyPageSpec::into_page_model`] strips the action before the
/// renderer sees it. This keeps renderer backends from accidentally allowing
/// unowned item activation.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryItemNode<Action> {
    pub slot: InventorySlotId,
    pub label: String,
    pub detail: Option<String>,
    pub icon: Option<String>,
    pub count: Option<u32>,
    /// Host-facing verb shown in detail text, e.g. "Use", "Equip", or "Unequip".
    pub action_label: Option<String>,
    /// Human-readable equipment slot name, e.g. "held item" or "body".
    ///
    /// This is display-only. The host game remains the authority for conflicts,
    /// slot capacity, and side effects.
    pub equip_slot_label: Option<String>,
    /// Optional host-computed note such as "will replace Axe".
    pub equip_conflict: Option<String>,
    pub owned: bool,
    pub equipped: bool,
    pub selected: bool,
    pub disabled: bool,
    pub important: bool,
    pub action: Option<Action>,
}

impl<Action> InventoryItemNode<Action> {
    pub fn new(slot: usize, label: impl Into<String>) -> Self {
        Self {
            slot: InventorySlotId(slot),
            label: label.into(),
            detail: None,
            icon: None,
            count: None,
            action_label: None,
            equip_slot_label: None,
            equip_conflict: None,
            owned: true,
            equipped: false,
            selected: false,
            disabled: false,
            important: false,
            action: None,
        }
    }

    pub fn unowned(slot: usize, label: impl Into<String>) -> Self {
        Self {
            owned: false,
            disabled: true,
            ..Self::new(slot, label)
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn count(mut self, count: u32) -> Self {
        self.count = Some(count);
        self
    }

    pub fn action_label(mut self, label: impl Into<String>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    pub fn equip_slot_label(mut self, label: impl Into<String>) -> Self {
        self.equip_slot_label = Some(label.into());
        self
    }

    pub fn equip_conflict(mut self, note: impl Into<String>) -> Self {
        self.equip_conflict = Some(note.into());
        self
    }

    pub fn equipped(mut self, equipped: bool) -> Self {
        self.equipped = equipped;
        self.important |= equipped;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self
    }
}

/// Configuration for building an items-only inventory page.
///
/// This intentionally uses normalized rectangles and host-defined actions so it
/// can feed the Lunex cube renderer, a Bevy UI fallback, or a test-only flat
/// renderer without changing Ambition's item resources.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemsOnlyPageSpec<PageId, Action> {
    pub page_id: PageId,
    pub title: String,
    pub rows: usize,
    pub cols: usize,
    pub grid_rect: MenuRect,
    pub slot_gap_pct: f32,
    pub background: MenuColor,
    pub panel: MenuColor,
    pub selected_slot: Option<InventorySlotId>,
    pub cells: Vec<InventoryItemNode<Action>>,
}

impl<PageId, Action> ItemsOnlyPageSpec<PageId, Action> {
    pub fn new(page_id: PageId, title: impl Into<String>) -> Self {
        Self {
            page_id,
            title: title.into(),
            rows: 4,
            cols: 6,
            grid_rect: MenuRect::new(7.5, 18.0, 85.0, 68.0),
            slot_gap_pct: 1.2,
            background: MenuColor::rgba(0.015, 0.020, 0.055, 0.98),
            panel: MenuColor::BLUE_PANEL,
            selected_slot: None,
            cells: Vec::new(),
        }
    }

    pub fn with_grid(mut self, rows: usize, cols: usize) -> Self {
        self.rows = rows.max(1);
        self.cols = cols.max(1);
        self
    }

    pub fn with_grid_rect(mut self, rect: MenuRect) -> Self {
        self.grid_rect = rect;
        self
    }

    pub fn with_slot_gap(mut self, gap_pct: f32) -> Self {
        self.slot_gap_pct = gap_pct.max(0.0);
        self
    }

    pub fn with_background(mut self, color: MenuColor) -> Self {
        self.background = color;
        self
    }

    pub fn with_panel(mut self, color: MenuColor) -> Self {
        self.panel = color;
        self
    }

    pub fn selected_slot(mut self, slot: Option<InventorySlotId>) -> Self {
        self.selected_slot = slot;
        self
    }

    pub fn push_cell(&mut self, cell: InventoryItemNode<Action>) {
        self.cells.push(cell);
    }

    pub fn with_cell(mut self, cell: InventoryItemNode<Action>) -> Self {
        self.push_cell(cell);
        self
    }

    pub fn into_page_model(self) -> MenuPageModel<PageId, Action> {
        let mut page = MenuPageModel::new(self.page_id, self.title, self.background);
        let title = page.title.clone();
        page.text(
            50.0,
            8.0,
            5.2,
            title,
            MenuTextAlign::Center,
            MenuColor::WHITE,
        );
        page.panel(self.grid_rect, self.panel, None);

        let rows = self.rows.max(1);
        let cols = self.cols.max(1);
        let gap = self.slot_gap_pct.max(0.0);
        let slot_w = (self.grid_rect.w - gap * (cols.saturating_sub(1) as f32)) / cols as f32;
        let slot_h = (self.grid_rect.h - gap * (rows.saturating_sub(1) as f32)) / rows as f32;
        let capacity = rows * cols;

        for cell in self.cells.into_iter() {
            let index = cell.slot.0;
            if index >= capacity {
                continue;
            }
            let row = index / cols;
            let col = index % cols;
            let rect = MenuRect::new(
                self.grid_rect.x + col as f32 * (slot_w + gap),
                self.grid_rect.y + row as f32 * (slot_h + gap),
                slot_w,
                slot_h,
            );

            let selected = cell.selected || self.selected_slot == Some(cell.slot);
            let detail = item_detail(&cell);
            let action = if cell.owned && !cell.disabled {
                cell.action
            } else {
                None
            };
            let important = cell.important || cell.equipped;
            let label = if cell.owned {
                cell.label
            } else {
                format!("{} - ???", cell.label)
            };

            page.control_with_icon(
                rect,
                MenuControlKind::Item,
                label,
                detail,
                cell.icon,
                selected,
                important,
                action,
            );
        }

        page
    }
}

impl<PageId: Clone, Action: Clone> ItemsOnlyPageSpec<PageId, Action> {
    pub fn to_page_model(&self) -> MenuPageModel<PageId, Action> {
        self.clone().into_page_model()
    }
}

fn item_detail<Action>(cell: &InventoryItemNode<Action>) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(detail) = &cell.detail {
        parts.push(detail.clone());
    }
    if let Some(count) = cell.count {
        parts.push(format!("x{count}"));
    }
    if let Some(action_label) = &cell.action_label {
        parts.push(action_label.clone());
    }
    if let Some(slot) = &cell.equip_slot_label {
        parts.push(format!("slot: {slot}"));
    }
    if let Some(conflict) = &cell.equip_conflict {
        parts.push(conflict.clone());
    }
    if cell.equipped {
        parts.push("equipped".to_string());
    }
    if !cell.owned {
        parts.push("not owned".to_string());
    } else if cell.disabled {
        parts.push("disabled".to_string());
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" | "))
    }
}

/// Small trait for host-side adapters that build the items page from gameplay
/// resources.
///
/// Ambition can implement this over a temporary adapter struct that borrows
/// `OwnedItems` and `GridMenuState`; the UI crate does not need to depend on
/// those types.
pub trait ItemsOnlyMenuAdapter {
    type PageId;
    type Action;

    fn items_page_spec(&self) -> ItemsOnlyPageSpec<Self::PageId, Self::Action>;

    fn items_page_model(&self) -> MenuPageModel<Self::PageId, Self::Action> {
        self.items_page_spec().into_page_model()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The active face's vertical framing must hit `TARGET_FACE_FILL` (so the
    /// top/bottom margin is the intended ~20%), independent of `page_radius`. This
    /// recomputes the on-screen fill from the DERIVED camera distance + the shared
    /// FOV, locking the camera_distance derivation in `oot_like` against drift.
    #[test]
    fn cube_face_vertical_fill_matches_target_margin() {
        for radius in [1.0_f32, 2.85, 7.5] {
            let geo = MenuCubeGeometry::oot_like(radius);
            // The face plane sits at +page_radius; the camera at −camera_distance.
            let distance = geo.page_radius + geo.camera_distance;
            let face_half_height = geo.page_height * 0.5;
            let fill =
                face_half_height / (distance * (MenuCubeGeometry::CAMERA_FOV_RADIANS * 0.5).tan());
            assert!(
                (fill - MenuCubeGeometry::TARGET_FACE_FILL).abs() < 1.0e-4,
                "radius {radius}: face fills {fill} of the half-screen, want {} \
                 (~{:.0}% top/bottom margin)",
                MenuCubeGeometry::TARGET_FACE_FILL,
                (1.0 - MenuCubeGeometry::TARGET_FACE_FILL) * 100.0,
            );
        }
        // 0.80 fill ⇒ a 20%-of-half-height margin, as requested.
        assert!((MenuCubeGeometry::TARGET_FACE_FILL - 0.80).abs() < 1.0e-6);
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Page {
        Items,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Action {
        UseHealth,
        EquipAxe,
    }

    #[test]
    fn items_page_builds_actionable_owned_cells() {
        let page = ItemsOnlyPageSpec::new(Page::Items, "Items")
            .with_cell(InventoryItemNode::new(0, "Health Cell").action(Action::UseHealth))
            .with_cell(
                InventoryItemNode::new(1, "Axe")
                    .equipped(true)
                    .action(Action::EquipAxe),
            )
            .into_page_model();

        let actions: Vec<_> = page
            .actionable_nodes()
            .filter_map(MenuNode::action)
            .copied()
            .collect();
        assert_eq!(actions, vec![Action::UseHealth, Action::EquipAxe]);
    }

    #[test]
    fn unowned_cells_are_displayed_but_not_actionable() {
        let page = ItemsOnlyPageSpec::new(Page::Items, "Items")
            .with_cell(InventoryItemNode::unowned(0, "Bow").action(Action::UseHealth))
            .into_page_model();

        assert_eq!(page.actionable_nodes().count(), 0);
        match &page.nodes[2] {
            MenuNode::Control { detail, .. } => {
                assert!(detail
                    .as_ref()
                    .is_some_and(|text| text.contains("not owned")));
            }
            node => panic!("expected item control, got {node:?}"),
        }
    }
}
