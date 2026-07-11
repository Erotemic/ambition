//! Room metadata, music request, and visual profile.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

/// Track the music identifier the active room would like to play.
///
/// Written by `sync_room_music_request` from `ActiveRoomMetadata`,
/// consumed by the music-intent adapter as the "default track" when no
/// encounter override is active. The encounter system retains
/// priority — a `Some(...)` from `EncounterMusicRequest::desired_track()`
/// overrides this resource the same way it overrides the sandbox-wide
/// default music track. Empty/absent room music falls back to
/// the music registry's `default_track`.
#[derive(Resource, Clone, Debug, Default)]
pub struct RoomMusicRequest {
    pub desired_track: Option<String>,
}

/// Mirrors `RoomSet::active_metadata()` as a standalone Bevy resource.
///
/// Synced by `sync_active_room_metadata` each frame the active room
/// changes. Consumers (room music selection, ambient layer selection,
/// renderer palette swaps) can subscribe via `Res<ActiveRoomMetadata>`
/// + change detection without importing the larger `RoomSet` type.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveRoomMetadata(pub RoomMetadata);

/// Optional declarative room metadata authored on LDtk levels.
///
/// LDtk level fields `biome` / `music_track` / `ambient_profile` /
/// `visual_theme`, explicit room-visual-profile fields, and small
/// presentation-policy overrides land here.
/// Every field is optional so existing levels keep working
/// without a value. The first non-empty value among an active area's
/// member levels wins; future systems can refine this if needed
/// (e.g. dominant-vote, level-position weighted).
///
/// Consumers: room music selection, ambient layer selection,
/// renderer palette/theme variants, nameplate presentation policy. This
/// struct is intentionally non-exhaustive — adding a metadata seam is
/// cheaper than adding a new resource per consumer.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomVisualProfile {
    /// Stable authored profile id (for example `intro_wakeup_room`).
    pub id: Option<String>,
    /// Explicit parallax/background theme. Prefer this over inferring from
    /// biome, music, or loose color-theme strings.
    pub parallax_theme: Option<String>,
    /// Palette / color-grading hint for future renderer passes.
    pub palette: Option<String>,
    /// Lighting mood hint for future post-process / shader passes.
    pub lighting_hint: Option<String>,
    /// Foreground treatment hint for generated atmosphere layers.
    pub foreground_treatment: Option<String>,
}

impl RoomVisualProfile {
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.parallax_theme.is_none()
            && self.palette.is_none()
            && self.lighting_hint.is_none()
            && self.foreground_treatment.is_none()
    }

    pub fn merge(&mut self, other: RoomVisualProfile) {
        if self.id.is_none() {
            self.id = other.id;
        }
        if self.parallax_theme.is_none() {
            self.parallax_theme = other.parallax_theme;
        }
        if self.palette.is_none() {
            self.palette = other.palette;
        }
        if self.lighting_hint.is_none() {
            self.lighting_hint = other.lighting_hint;
        }
        if self.foreground_treatment.is_none() {
            self.foreground_treatment = other.foreground_treatment;
        }
    }

    pub fn label(&self) -> Option<&str> {
        self.id
            .as_deref()
            .or(self.parallax_theme.as_deref())
            .or(self.palette.as_deref())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomNameplatePolicy {
    /// Number of nearest eligible nameplates to draw at full opacity.
    /// `None` falls back to the presentation default.
    pub full_opacity_count: Option<usize>,
    /// Ranked candidate count where nameplate opacity reaches zero.
    /// `None` falls back to the presentation default.
    pub fade_out_count: Option<usize>,
}

impl RoomNameplatePolicy {
    pub fn is_empty(&self) -> bool {
        self.full_opacity_count.is_none() && self.fade_out_count.is_none()
    }

    pub fn merge(&mut self, other: RoomNameplatePolicy) {
        if self.full_opacity_count.is_none() {
            self.full_opacity_count = other.full_opacity_count;
        }
        if self.fade_out_count.is_none() {
            self.fade_out_count = other.fade_out_count;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoomMetadata {
    pub biome: Option<String>,
    pub music_track: Option<String>,
    pub ambient_profile: Option<String>,
    pub visual_theme: Option<String>,
    pub visual_profile: RoomVisualProfile,
    pub nameplate_policy: RoomNameplatePolicy,
    /// This room is a character GALLERY (the Hall of Characters and any future
    /// pedestal room). Engine-generic policy hook (C1): systems switch behavior
    /// on this flag instead of matching a content room id — e.g. the ambient
    /// bark ticker draws each NPC's `Hall` pool here and its `Idle` pool
    /// elsewhere. Authored as the LDtk level bool field `gallery`.
    pub gallery: bool,
    /// The GAME MODE this room belongs to (decomposition D-C, vision §5).
    ///
    /// A hosted demo's rules crate gates its systems on
    /// `ambition_runtime::in_mode("sanic")` rather than on a global state, so
    /// Ambition can host several demos' rulesets in one binary and the rules
    /// only run inside the rooms that opted into them. `None` is the base game.
    ///
    /// Authored as the LDtk level string field `mode`; merged first-`Some`-wins
    /// across an active area's member levels, like every other string field
    /// here.
    pub mode: Option<String>,
}

impl RoomMetadata {
    pub fn is_empty(&self) -> bool {
        self.biome.is_none()
            && self.music_track.is_none()
            && self.ambient_profile.is_none()
            && self.visual_theme.is_none()
            && self.visual_profile.is_empty()
            && self.nameplate_policy.is_empty()
            && !self.gallery
            && self.mode.is_none()
    }

    /// Fold `other` into `self`, preferring values already set.
    /// LDtk active areas can span multiple levels; the first level
    /// with a non-empty value wins so author intent is predictable.
    pub fn merge(&mut self, other: RoomMetadata) {
        if self.biome.is_none() {
            self.biome = other.biome;
        }
        if self.music_track.is_none() {
            self.music_track = other.music_track;
        }
        if self.ambient_profile.is_none() {
            self.ambient_profile = other.ambient_profile;
        }
        if self.visual_theme.is_none() {
            self.visual_theme = other.visual_theme;
        }
        if self.mode.is_none() {
            self.mode = other.mode;
        }
        // A multi-level area is a gallery if ANY member level marks it one.
        self.gallery = self.gallery || other.gallery;
        self.visual_profile.merge(other.visual_profile);
        self.nameplate_policy.merge(other.nameplate_policy);
    }
}
