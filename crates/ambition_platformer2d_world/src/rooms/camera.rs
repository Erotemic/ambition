//! Camera zones, clamp modes, and kinematic path specs.

use ambition_platformer2d_core as ae;
use std::borrow::Cow;

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
    #[serde(default)]
    pub scroll_policy: CameraScrollPolicy,
}

/// How the camera is allowed to travel while a zone is active
/// (`docs/planning/demos/super-mary-o.md`: *"one-way forward scroll + no-backtrack clamp"*).
///
/// Forward-only scrolling uses world/screen `+x` and is independent of gravity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraScrollPolicy {
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
/// The clamp is monotone: it never eases backward toward the watermark.
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
    pub path: ambition_platformer2d_core::KinematicPath,
}

impl KinematicPathSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        path: ambition_platformer2d_core::KinematicPath,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            path,
        }
    }

    /// Every spelling accepted by [`Self::matches_id`]. Validation, conversion,
    /// and runtime lookup must use this same alias set. A path without an
    /// authored id is also reachable by [`kinematic_path_name_slug`] of its name.
    pub fn resolution_aliases(&self) -> impl Iterator<Item = Cow<'_, str>> {
        kinematic_path_aliases(&self.id, &self.name)
    }

    pub fn matches_id(&self, query: &str) -> bool {
        self.resolution_aliases().any(|alias| alias == query)
    }
}

/// [`KinematicPathSpec::resolution_aliases`] for callers that only have the two
/// authored strings. Raw-content validation uses this before world IR exists so
/// validation and runtime lookup share one alias rule.
pub fn kinematic_path_aliases<'a>(
    id: &'a str,
    name: &'a str,
) -> impl Iterator<Item = Cow<'a, str>> {
    [Cow::Borrowed(id), Cow::Borrowed(name)]
        .into_iter()
        .chain(name_slug(name).map(Cow::Owned))
}

/// Canonical conversion from a kinematic path display name to an id. A name
/// slugs to lowercase alphanumerics with each other run collapsed to one `_`.
/// Conversion, validation, and lookup must call this rule rather than reproduce
/// it.
pub fn kinematic_path_name_slug(name: &str) -> Option<String> {
    name_slug(name)
}

/// Every `(spelling, path)` pair a room's authored paths answer to. Generated
/// from [`KinematicPathSpec::resolution_aliases`] so validation and runtime
/// resolution accept the same references. First declaration wins; blank
/// spellings are ignored.
pub fn kinematic_path_lookup(
    specs: &[KinematicPathSpec],
) -> Vec<(String, ambition_platformer2d_core::KinematicPath)> {
    let mut lookup: Vec<(String, ambition_platformer2d_core::KinematicPath)> = Vec::new();
    for spec in specs {
        for alias in spec.resolution_aliases() {
            let alias = alias.into_owned();
            if alias.is_empty() || lookup.iter().any(|(known, _)| known == &alias) {
                continue;
            }
            lookup.push((alias, spec.path.clone()));
        }
    }
    lookup
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
mod kinematic_path_lookup_tests {
    use super::*;

    fn spec(id: &str, name: &str) -> KinematicPathSpec {
        KinematicPathSpec::new(
            id,
            name,
            ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(8.0, 8.0)),
            ae::KinematicPath::line(ae::Vec2::ZERO, ae::Vec2::new(64.0, 0.0), 30.0),
        )
    }

    /// THE INVARIANT: the runtime lookup table accepts exactly what
    /// `matches_id` accepts. Validation resolves through `matches_id`, the
    /// body rides through this table, and any spelling only one of them knows
    /// is a reference reported healthy that does not move anything.
    ///
    /// Covers a path whose id is derived from a spaced authored name and referenced by slug.
    #[test]
    fn every_spelling_matches_id_accepts_is_in_the_lookup_table() {
        let specs = vec![spec("enemy_patrol_path_a", "enemy patrol path A")];
        let lookup = kinematic_path_lookup(&specs);

        for spelling in ["enemy_patrol_path_a", "enemy patrol path A"] {
            assert!(
                specs[0].matches_id(spelling),
                "matches_id must accept `{spelling}`"
            );
            assert!(
                lookup.iter().any(|(alias, _)| alias == spelling),
                "the runtime lookup table must accept `{spelling}` too — \
                 a spelling only validation knows is a patrol that never moves"
            );
        }

        // ...and the poison: a spelling NEITHER accepts stays absent, so this
        // is a shared alias set rather than a table that says yes to anything.
        assert!(!specs[0].matches_id("enemy_patrol_b"));
        assert!(!lookup.iter().any(|(alias, _)| alias == "enemy_patrol_b"));

        // A spelling from a different slug rule must not resolve.
        assert!(!specs[0].matches_id("enemy_patrol_a"));
        assert!(!lookup.iter().any(|(alias, _)| alias == "enemy_patrol_a"));
    }

    /// A bare `Patrol:` reaches lowering as an EMPTY id. It must find nothing —
    /// not collide with a spec whose authored id happens to be blank, which is
    /// how a typo would acquire a working path.
    #[test]
    fn a_blank_spelling_is_registered_by_nobody() {
        let lookup = kinematic_path_lookup(&[spec("", "Ledge Patrol")]);
        assert!(!lookup.iter().any(|(alias, _)| alias.is_empty()));
        assert!(lookup.iter().any(|(alias, _)| alias == "ledge_patrol"));
    }

    /// Two paths answering to one spelling: the first wins, matching the
    /// sweep's "the rest are unreachable" report. A second entry would make
    /// which path you ride depend on iteration order.
    #[test]
    fn a_duplicated_spelling_resolves_to_the_first_declaration() {
        let lookup = kinematic_path_lookup(&[spec("shared", "First"), spec("shared", "Second")]);
        assert_eq!(
            lookup.iter().filter(|(alias, _)| alias == "shared").count(),
            1
        );
    }
}

#[cfg(test)]
mod scroll_policy_tests {
    use super::*;

    /// It never comes back.
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

    /// The camera never eases backward toward its high-water mark.
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

    /// Authored strings, all the ways a level designer might spell it. Anything else is `Free`,
    /// which is what every zone authored before means.
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

    /// It deserializes to `Free` — the behaviour it has always had — which is what
    /// `#[serde(default)]` buys and what this proves by deleting the field from a round-tripped
    /// spec rather than by hand-typing one.
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

        // The serializer's spacing is its own business, so find it rather than assume it.
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
