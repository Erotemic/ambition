//! Camera zones, clamp modes, and kinematic path specs.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use ambition_engine_core as ae;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraClampMode {
    #[default]
    RoomBounds,
    ZoneBounds,
    None,
}

impl CameraClampMode {
    pub fn from_author_value(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .map(|value| value.to_ascii_lowercase().replace('-', "_"))
            .as_deref()
        {
            Some("zone") | Some("zone_bounds") | Some("camera_zone") => Self::ZoneBounds,
            Some("none") | Some("unclamped") | Some("free") => Self::None,
            _ => Self::RoomBounds,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::RoomBounds => "room_bounds",
            Self::ZoneBounds => "zone_bounds",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CameraZoneSpec {
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub priority: i32,
    /// Requested zoom multiplier while the player overlaps the zone.
    /// `None` preserves the legacy camera-zone breath-out default.
    pub zoom: Option<f32>,
    /// World-space target offset applied after normal look-ahead framing.
    pub target_offset: ae::Vec2,
    /// Optional target-easing override, in hertz.
    pub easing_hz: Option<f32>,
    /// When true, target the zone center instead of the player.
    pub cinematic_lock: bool,
    pub clamp_mode: CameraClampMode,
    /// **M2 — the one-way forward scroll.** `#[serde(default)]`, so every authored
    /// zone that predates it stays exactly as it was.
    #[serde(default)]
    pub scroll_policy: CameraScrollPolicy,
}

/// How the camera is allowed to travel while a zone is active
/// (`docs/planning/demos/super-mary-o.md` M2: *"one-way forward scroll +
/// no-backtrack clamp"*).
///
/// **Narrow on purpose.** There is exactly one shipped need — Mary-O's level
/// scrolls right and never left — so there is exactly one non-default variant.
/// A `ForwardOnly { axis, direction }` generalization waits for a second consumer
/// (grow, don't mint). The axis is SCREEN `+x`, not gravity-relative: a
/// side-scroller's no-backtrack rule is a statement about the level's authored
/// direction of travel, and rotating gravity does not rotate the level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraScrollPolicy {
    /// The camera follows wherever the focus goes. Every zone before M2.
    #[default]
    Free,
    /// The camera never travels back along `-x`. The player may walk left; the
    /// world behind them is gone, and the left edge of the view becomes a wall the
    /// LEVEL enforces, not the camera.
    ForwardOnlyX,
}

impl CameraScrollPolicy {
    pub fn from_author_value(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .map(|v| v.to_ascii_lowercase().replace('-', "_"))
            .as_deref()
        {
            Some("forward_only") | Some("forward_only_x") | Some("no_backtrack") => {
                Self::ForwardOnlyX
            }
            _ => Self::Free,
        }
    }
}

/// The no-backtrack clamp, as a pure function of `(target, watermark)`.
///
/// `watermark` is the furthest the camera has travelled during THIS visit to the
/// zone. The caller clears it on leaving, so re-entering a forward-only zone from
/// the other side is a fresh scroll rather than a camera pinned to wherever it
/// stopped an hour ago.
///
/// Deliberately monotone and stateless-looking: the camera never eases BACKWARD to
/// meet a watermark, it simply refuses to go below it. Easing backward would let a
/// player standing still watch the view creep, which is the bug this clamp exists
/// to prevent.
pub fn apply_forward_only_x(target_x: f32, watermark: &mut Option<f32>) -> f32 {
    // A non-finite target would poison the watermark for the rest of the visit.
    if !target_x.is_finite() {
        return watermark.unwrap_or(target_x);
    }
    let clamped = match *watermark {
        Some(high) => target_x.max(high),
        None => target_x,
    };
    *watermark = Some(clamped);
    clamped
}

impl CameraZoneSpec {
    pub const LEGACY_BREATH_ZOOM: f32 = 1.15;

    pub fn effective_zoom(&self) -> f32 {
        self.zoom.unwrap_or(Self::LEGACY_BREATH_ZOOM).max(1.0)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KinematicPathSpec {
    /// Stable authored lookup id. LDtk may not have an explicit `id` field yet,
    /// so conversion falls back to the entity `name` and finally the LDtk iid.
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub path: ambition_engine_core::KinematicPath,
}

impl KinematicPathSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        path: ambition_engine_core::KinematicPath,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            path,
        }
    }

    pub fn aliases(&self) -> impl Iterator<Item = &str> {
        [self.id.as_str(), self.name.as_str()].into_iter()
    }

    pub fn matches_id(&self, query: &str) -> bool {
        if self.aliases().any(|alias| alias == query) {
            return true;
        }
        // Tolerate `compact_path_name`-style normalization (used by
        // the LDtk path-lookup-id derivation): if the query, after
        // slugifying the spec's `name` field with the same rules
        // *minus* the "_path_" stripping, matches, accept it. This
        // resolves the latent mismatch between authors who reference
        // a path by its raw name-slug (e.g. `enemy_patrol_path_a`)
        // and the runtime id derived from the same name with
        // `_path_` collapsed away (`enemy_patrol_a`). See the comment
        // on `validate_patrol_brain_paths` in `content_validation.rs`.
        if let Some(slug) = name_slug(&self.name) {
            if slug == query {
                return true;
            }
        }
        false
    }
}

fn name_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_sep = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_sep = false;
        } else if !previous_was_sep && !slug.is_empty() {
            slug.push('_');
            previous_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

#[cfg(test)]
mod scroll_policy_tests {
    use super::*;

    /// **The whole of M2.** The camera goes forward. It never comes back.
    #[test]
    fn the_camera_never_travels_back_along_minus_x() {
        let mut w = None;
        assert_eq!(apply_forward_only_x(10.0, &mut w), 10.0);
        assert_eq!(apply_forward_only_x(40.0, &mut w), 40.0);
        // The player walks left. The camera does not follow.
        assert_eq!(apply_forward_only_x(5.0, &mut w), 40.0);
        assert_eq!(apply_forward_only_x(-100.0, &mut w), 40.0);
        // ...and forward progress resumes from where it stopped, not from the
        // player's position: the watermark is the camera's memory, not the level's.
        assert_eq!(apply_forward_only_x(41.0, &mut w), 41.0);
    }

    /// **Never eases backward to meet the watermark.** A camera that crept toward a
    /// high-water mark while the player stood still would be a bug that looked like
    /// a feature for exactly one playtest.
    #[test]
    fn a_standing_player_sees_a_still_camera() {
        let mut w = Some(100.0);
        for _ in 0..60 {
            assert_eq!(apply_forward_only_x(30.0, &mut w), 100.0);
        }
        assert_eq!(w, Some(100.0));
    }

    /// The clamp is per-VISIT. Clearing the watermark on leaving is what lets a
    /// player re-enter a forward-only zone from the other side and scroll it again,
    /// instead of meeting a camera pinned to where it stopped an hour ago.
    #[test]
    fn clearing_the_watermark_restarts_the_scroll() {
        let mut w = None;
        apply_forward_only_x(500.0, &mut w);
        w = None;
        assert_eq!(apply_forward_only_x(20.0, &mut w), 20.0);
    }

    /// A non-finite target must not poison the watermark for the rest of the visit.
    /// One `NaN` frame — a degenerate viewport, a divide by a zero zoom — would
    /// otherwise freeze the camera permanently, and `NaN.max(x)` is not the safe
    /// direction.
    #[test]
    fn a_non_finite_target_never_reaches_the_watermark() {
        let mut w = Some(50.0);
        assert_eq!(apply_forward_only_x(f32::NAN, &mut w), 50.0);
        assert_eq!(w, Some(50.0));
        assert_eq!(apply_forward_only_x(70.0, &mut w), 70.0);

        // ...and on the very first frame, with nothing to fall back on, it passes
        // the NaN through rather than inventing a position.
        let mut fresh = None;
        assert!(apply_forward_only_x(f32::NAN, &mut fresh).is_nan());
        assert_eq!(fresh, None, "and it did not become the watermark");
    }

    /// Authored strings, all the ways a level designer might spell it. Anything else
    /// is `Free`, which is what every zone authored before M2 means.
    #[test]
    fn the_authored_value_parses_the_spellings_a_designer_would_try() {
        for s in [
            "forward_only",
            "Forward-Only",
            "no_backtrack",
            "FORWARD_ONLY_X",
        ] {
            assert_eq!(
                CameraScrollPolicy::from_author_value(Some(s)),
                CameraScrollPolicy::ForwardOnlyX,
                "`{s}`"
            );
        }
        for s in [None, Some("free"), Some(""), Some("nonsense")] {
            assert_eq!(
                CameraScrollPolicy::from_author_value(s),
                CameraScrollPolicy::Free
            );
        }
    }

    /// **Byte-parity.** A zone authored before M2 carries no `scroll_policy` field.
    /// It deserializes to `Free` — the behaviour it has always had — which is what
    /// `#[serde(default)]` buys and what this proves by deleting the field from a
    /// round-tripped spec rather than by hand-typing one.
    #[test]
    fn a_pre_m2_camera_zone_still_parses_and_scrolls_freely() {
        let spec = CameraZoneSpec {
            id: "z".into(),
            name: "z".into(),
            aabb: ae::Aabb::new(ae::Vec2::splat(50.0), ae::Vec2::splat(50.0)),
            priority: 0,
            zoom: None,
            target_offset: ae::Vec2::ZERO,
            easing_hz: None,
            cinematic_lock: false,
            clamp_mode: CameraClampMode::RoomBounds,
            scroll_policy: CameraScrollPolicy::ForwardOnlyX,
        };
        let ron = ron::to_string(&spec).expect("serializes");
        assert!(ron.contains("scroll_policy"));

        // Strip the field, exactly as a pre-M2 file lacks it. The serializer's
        // spacing is its own business, so find it rather than assume it.
        let start = ron.find("scroll_policy").expect("the field is there");
        let end = ron[start..]
            .find(|c| c == ',' || c == ')')
            .map(|i| start + i + usize::from(ron.as_bytes()[start + i] == b','))
            .expect("the field ends");
        let pre_m2 = format!("{}{}", &ron[..start], &ron[end..]);
        let parsed: CameraZoneSpec = ron::from_str(&pre_m2).expect("a pre-M2 zone parses");
        assert_eq!(parsed.scroll_policy, CameraScrollPolicy::Free);
    }
}
