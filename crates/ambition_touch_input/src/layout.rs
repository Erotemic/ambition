//! Layout values are bound to the visible circle, not to the square `Node`
//! bounds. Adjacent diamond buttons can have overlapping squares while their
//! circles do not overlap; [`touch_action_at_position`] tests circle distance
//! so multitouch matches what the user sees.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::gameplay_presentation::ScreenRect;

/// Marker and identity for touch action buttons. Each is a Bevy `Button`
/// whose `Interaction` is collected into its `TouchInputState` field each
/// frame; the virtual-device input kinds (`crate::virtual_device`) then
/// resolve it through the participant's bindings like a physical button.
/// Leafwing user inputs need the reflect/serde derives.
#[derive(
    Component,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    bevy::prelude::Reflect,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum TouchActionButton {
    Jump,
    Attack,
    Special,
    /// The shared dodge/dash press. Named for the channel, not the outcome:
    /// the body decides what a press does, and the label comes from the live
    /// `ControlPrompt`.
    Burst,
    Blink,
    Interact,
    Projectile,
    FlyToggle,
    Shield,
    /// Capture attempt. Availability-gated like Special: only a body granted
    /// the verb that also authors a grab shows the button.
    Grab,
    /// Sustained-technique slot. Held, not tapped; content decides what
    /// holding it does (a locomotion mode, a stance).
    Modifier,
    Start,
    Reset,
}

/// Uniform scale for every touch-control dimension (action cluster, button
/// positions and sizes, menu row). Smaller than 1.0 so the HUD takes less of
/// a phone screen; change the size here and keep the shape.
pub(super) const TOUCH_SCALE: f32 = 0.7;
/// Fonts shrink less than geometry, so labels stay legible at phone DPI.
pub(super) const TOUCH_FONT_SCALE: f32 = 0.85;
pub(crate) const ACTION_CLUSTER_MARGIN: f32 = 10.0;
pub(crate) const ACTION_BEZEL_PAD: f32 = 8.0;
pub(crate) const ACTION_CLUSTER_W: f32 = 310.0 * TOUCH_SCALE;
pub(crate) const ACTION_CLUSTER_H: f32 = 312.0 * TOUCH_SCALE;
pub(crate) const ACTION_BEZEL_W: f32 = ACTION_CLUSTER_W + ACTION_BEZEL_PAD * 2.0;
pub(crate) const ACTION_BEZEL_H: f32 = ACTION_CLUSTER_H + ACTION_BEZEL_PAD * 2.0;
/// Inset for the movement stick from the lower-left corner.
/// A larger gap keeps the thumb control away from the screen edge and its
/// gesture navigation.
pub(crate) const JOYSTICK_MARGIN: f32 = 64.0 * TOUCH_SCALE;
/// Generous movement-stick footprint reserved from menu drag-scroll gestures.
pub(crate) const JOYSTICK_EXCLUSION_SIZE: f32 = 300.0 * TOUCH_SCALE;
pub(crate) const MENU_ROW_MARGIN: f32 = 12.0;
pub(crate) const MENU_ROW_W: f32 = 198.0 * TOUCH_SCALE;
pub(crate) const MENU_W: f32 = 88.0 * TOUCH_SCALE;
pub(crate) const MENU_H: f32 = 44.0 * TOUCH_SCALE;
/// 88px button plus 4px margin each side, scaled like the menu buttons so the
/// hit test matches the overlay.
pub(crate) const MENU_CELL: f32 = 96.0 * TOUCH_SCALE;

#[derive(Clone, Copy, Debug)]
pub struct TouchActionSpec {
    pub action: TouchActionButton,
    pub label: &'static str,
    pub left: f32,
    pub top: f32,
    pub size: f32,
    pub font_size: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchJoystickLayout {
    pub margin: f32,
    pub base_size: f32,
    pub knob_size: f32,
    pub exclusion_size: f32,
}

impl TouchJoystickLayout {
    /// Top-left of the drawn stick (base ring, knob, U/R/L/D glyphs) within
    /// the reserved `exclusion_size` footprint, in root-local y-down px.
    ///
    /// The footprint is a gesture-exclusion region flush to the screen
    /// corner. The art is inset by `margin` from its left and bottom edges,
    /// clear of edge swipes. The stick and the glyphs both derive from here.
    pub fn art_origin(&self) -> Vec2 {
        Vec2::new(
            self.margin,
            self.exclusion_size - self.base_size - self.margin,
        )
    }

    /// Root-local center of the drawn stick: where the knob rests and what the
    /// glyphs orbit.
    pub fn art_center(&self) -> Vec2 {
        self.art_origin() + Vec2::splat(self.base_size * 0.5)
    }
}

pub fn movement_joystick_layout() -> TouchJoystickLayout {
    TouchJoystickLayout {
        margin: JOYSTICK_MARGIN,
        base_size: 200.0 * TOUCH_SCALE,
        knob_size: 100.0 * TOUCH_SCALE,
        exclusion_size: JOYSTICK_EXCLUSION_SIZE,
    }
}

/// The lower-right action layout for both the rendered UI and raw multitouch
/// hit testing. All positions live here, so the overlay and the touch path
/// cannot drift.
pub fn touch_action_layout() -> [TouchActionSpec; 11] {
    // Authored at 1.0 scale; `scaled` applies TOUCH_SCALE / TOUCH_FONT_SCALE.
    let scaled = |action, label, left, top, size, font_size| TouchActionSpec {
        action,
        label,
        left: left * TOUCH_SCALE,
        top: top * TOUCH_SCALE,
        size: size * TOUCH_SCALE,
        font_size: font_size * TOUCH_FONT_SCALE,
    };
    // Four bands, read as a controller face. Each band is a full row, so the
    // arrangement holds when a character shows only some buttons (missing
    // slots are hidden, not repacked).
    //
    //   shoulder   Blink    Fly      Shot
    //   utility    Shield   Interact Special
    //   face       Attack            Burst
    //   primary        Run      Jump
    //
    // The primary band is the thumb's home. Run and Jump sit next to each
    // other, closer than to anything else, so a thumb can hold Run while
    // tapping Jump. `no_touch_button_overlaps_another` checks the spacing.
    [
        // Shoulder band.
        scaled(TouchActionButton::Blink, "Blink", 14.0, 4.0, 62.0, 13.0),
        scaled(TouchActionButton::FlyToggle, "Fly", 124.0, 0.0, 62.0, 14.0),
        scaled(
            TouchActionButton::Projectile,
            "Shot",
            234.0,
            4.0,
            62.0,
            13.0,
        ),
        // Utility band.
        scaled(TouchActionButton::Shield, "Shield", 8.0, 84.0, 60.0, 12.0),
        scaled(
            TouchActionButton::Interact,
            "Interact",
            125.0,
            76.0,
            60.0,
            12.0,
        ),
        // Signature slot. Hidden when the scheme has no Special (the
        // availability check gates visibility and the hit test).
        scaled(
            TouchActionButton::Special,
            "Special",
            242.0,
            84.0,
            60.0,
            12.0,
        ),
        // Face band.
        scaled(TouchActionButton::Attack, "Attack", 30.0, 158.0, 70.0, 14.0),
        // Smaller than its neighbors (58 vs 70) to keep the gap.
        scaled(TouchActionButton::Grab, "Grab", 125.0, 158.0, 58.0, 12.0),
        scaled(TouchActionButton::Burst, "Burst", 210.0, 158.0, 70.0, 14.0),
        // Primary band: the pair.
        scaled(TouchActionButton::Modifier, "Run", 82.0, 240.0, 66.0, 14.0),
        scaled(TouchActionButton::Jump, "Jump", 162.0, 240.0, 66.0, 14.0),
    ]
}

/// The drawn centre and radius of one action button, in screen pixels, for a
/// cluster resolved at `cluster`.
///
/// The one projection from authored layout space to screen space. The
/// rendered `Node` and the raw hit test both use it, so the circle and its
/// touch target agree, also when the cluster is compacted.
pub fn touch_action_circle(spec: TouchActionSpec, cluster: ScreenRect) -> (Vec2, f32) {
    let scale = action_cluster_scale(cluster);
    let center = cluster.min
        + Vec2::new(
            (spec.left + spec.size * 0.5) * scale,
            (spec.top + spec.size * 0.5) * scale,
        );
    (center, spec.size * 0.5 * scale)
}

/// How much the authored action layout was scaled to reach `cluster`.
pub fn action_cluster_scale(cluster: ScreenRect) -> f32 {
    if ACTION_CLUSTER_W <= 0.0 {
        return 1.0;
    }
    (cluster.width() / ACTION_CLUSTER_W).max(0.0)
}

/// Hit-test a `pos` against the visible action button circles and the menu row.
/// Touch positions use the same top-left-origin logical coordinate space as
/// Bevy window cursor positions.
///
/// Action buttons are drawn as circles, so they are hit-tested as circles;
/// diagonal square bounds may overlap.
///
/// Both rectangles come from the resolved [`TouchControlPlacement`], never
/// from the window, so a cluster in a surround column is tappable where it
/// is drawn.
/// [`TouchControlPlacement`]: crate::placement::TouchControlPlacement
pub fn touch_action_at_position(
    pos: Vec2,
    cluster: Option<ScreenRect>,
    menu_row: Option<ScreenRect>,
) -> Option<TouchActionButton> {
    if let Some(cluster) = cluster {
        for spec in touch_action_layout() {
            let (center, radius) = touch_action_circle(spec, cluster);
            if pos.distance(center) <= radius {
                return Some(spec.action);
            }
        }
    }

    if let Some(menu_row) = menu_row {
        let scale = if MENU_ROW_W > 0.0 {
            (menu_row.width() / MENU_ROW_W).max(0.0)
        } else {
            1.0
        };
        for (action, col) in [
            (TouchActionButton::Start, 0usize),
            (TouchActionButton::Reset, 1),
        ] {
            let min = menu_row.min + Vec2::new((col as f32 * MENU_CELL + 4.0) * scale, 4.0 * scale);
            let size = Vec2::new(MENU_W, MENU_H) * scale;
            if pos.x >= min.x
                && pos.x <= min.x + size.x
                && pos.y >= min.y
                && pos.y <= min.y + size.y
            {
                return Some(action);
            }
        }
    }

    None
}

#[cfg(test)]
mod layout_tests {
    //! Touch HUD hit-testing. The layout feeds both the rendered overlay and
    //! the multitouch path, so every button's drawn center must hit-test back
    //! to itself.
    use super::*;

    #[test]
    fn every_touch_button_is_distinct_and_drawable() {
        let layout = touch_action_layout();
        for spec in layout {
            assert!(spec.size > 0.0, "{:?} has non-positive size", spec.action);
            assert!(!spec.label.is_empty());
        }
        for (i, a) in layout.iter().enumerate() {
            for b in &layout[i + 1..] {
                assert_ne!(a.action, b.action, "duplicate touch action {:?}", a.action);
            }
        }
    }

    /// No two buttons crowd each other. `each_button_center_hit_tests_back_to_itself`
    /// catches only overlap, not a gap too small for a finger.
    ///
    /// The bound is in authored space (before `TOUCH_SCALE`), where the layout
    /// is edited. Adding a button to a full cluster trips this, so the layout
    /// gets redesigned instead of squeezed.
    #[test]
    fn no_touch_button_overlaps_another() {
        /// Authored-space minimum gap between two buttons' visible circles.
        const MIN_GAP: f32 = 10.0;

        let layout = touch_action_layout();
        for (i, a) in layout.iter().enumerate() {
            for b in &layout[i + 1..] {
                // Undo the uniform scale, so values are in the table's units.
                let center = |s: &TouchActionSpec| {
                    Vec2::new(s.left + s.size * 0.5, s.top + s.size * 0.5) / TOUCH_SCALE
                };
                let radius = |s: &TouchActionSpec| s.size * 0.5 / TOUCH_SCALE;
                let gap = center(a).distance(center(b)) - radius(a) - radius(b);
                assert!(
                    gap >= MIN_GAP,
                    "{:?} and {:?} are {gap:.1}px apart in authored space \
                     (minimum {MIN_GAP}); re-lay the cluster out rather than \
                     shrinking the gap",
                    a.action,
                    b.action,
                );
            }
        }
    }

    /// Run and Jump are the pair: the thumb must hold Run while tapping Jump,
    /// so they must be nearer each other than to any other button. Checks the
    /// relationship, not coordinates.
    #[test]
    fn the_sustain_and_jump_buttons_are_each_others_nearest_neighbour() {
        let layout = touch_action_layout();
        let spec = |a: TouchActionButton| {
            *layout
                .iter()
                .find(|s| s.action == a)
                .expect("every button is laid out")
        };
        let center = |s: TouchActionSpec| Vec2::new(s.left + s.size * 0.5, s.top + s.size * 0.5);
        let run = spec(TouchActionButton::Modifier);
        let jump = spec(TouchActionButton::Jump);
        let pair_distance = center(run).distance(center(jump));

        for other in layout {
            if other.action == TouchActionButton::Modifier
                || other.action == TouchActionButton::Jump
            {
                continue;
            }
            let d = center(other).distance(center(run));
            assert!(
                d > pair_distance,
                "{:?} is nearer Run than Jump is; the hold-and-tap pair must stay \
                 adjacent or hold-to-run is unusable",
                other.action,
            );
        }
    }

    /// A cluster resolved at an arbitrary rectangle.
    fn cluster_at(min: Vec2, scale: f32) -> ScreenRect {
        ScreenRect::from_min_size(min, Vec2::new(ACTION_CLUSTER_W, ACTION_CLUSTER_H) * scale)
    }

    fn menu_at(min: Vec2) -> ScreenRect {
        ScreenRect::from_min_size(min, Vec2::new(MENU_ROW_W, crate::placement::MENU_ROW_H))
    }

    /// Every button's drawn center hit-tests back to itself at any placement
    /// and scale, including compacted into a surround column.
    #[test]
    fn each_button_center_hit_tests_back_to_itself() {
        for (name, cluster) in [
            (
                "bottom-right overlay",
                cluster_at(Vec2::new(1050.0, 500.0), 1.0),
            ),
            (
                "reserved left column",
                cluster_at(Vec2::new(8.0, 800.0), 1.0),
            ),
            ("compacted column", cluster_at(Vec2::new(12.0, 820.0), 0.9)),
        ] {
            for spec in touch_action_layout() {
                let (center, _) = touch_action_circle(spec, cluster);
                assert_eq!(
                    touch_action_at_position(center, Some(cluster), None),
                    Some(spec.action),
                    "{name}: centre of {:?} should hit itself (overlay/touch drift)",
                    spec.action,
                );
            }
        }
    }

    #[test]
    fn a_point_away_from_every_control_hits_nothing() {
        let cluster = cluster_at(Vec2::new(1050.0, 500.0), 1.0);
        assert_eq!(
            touch_action_at_position(Vec2::new(200.0, 200.0), Some(cluster), None),
            None,
        );
    }

    /// With no resolved rectangles nothing hits: a hidden HUD is not tappable.
    #[test]
    fn an_unplaced_cluster_is_not_tappable() {
        assert_eq!(
            touch_action_at_position(Vec2::new(1100.0, 560.0), None, None),
            None,
        );
    }

    #[test]
    fn menu_row_buttons_are_hittable_at_their_resolved_rect() {
        let menu = menu_at(Vec2::new(900.0, 12.0));
        let start_center = menu.min + Vec2::new(4.0 + MENU_W * 0.5, 4.0 + MENU_H * 0.5);
        assert_eq!(
            touch_action_at_position(start_center, None, Some(menu)),
            Some(TouchActionButton::Start),
        );
    }

    /// Visible circles, not square bounds: a point inside a button's square
    /// but outside its circle hits nothing. Each button's own corner is such
    /// a point, so spacing changes cannot make this vacuous.
    #[test]
    fn touch_action_hit_test_uses_visible_circle_not_square_bounds() {
        let cluster = cluster_at(Vec2::new(1050.0, 500.0), 1.0);
        for spec in touch_action_layout() {
            let (center, radius) = touch_action_circle(spec, cluster);
            // Just inside the square's corner, comfortably outside the circle
            // (the corner sits `r*sqrt(2)` from the centre).
            let corner = center + Vec2::splat(radius * 0.78);
            assert!(
                corner.distance(center) > radius,
                "{:?}: the probe must be outside the circle for this to mean anything",
                spec.action,
            );
            assert_eq!(
                touch_action_at_position(corner, Some(cluster), None),
                None,
                "{:?}: a point in the square corner but outside the visible circle \
                 must not register a press",
                spec.action,
            );
        }
    }
}
