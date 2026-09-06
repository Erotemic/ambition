//! Room metadata, music request, and visual profile.

use bevy_ecs::prelude::Component;

/// Track the music identifier the active room would like to play.
///
/// Written by `sync_room_music_request` from `ActiveRoomMetadata`,
/// consumed by the music-intent adapter as the "default track" when no
/// encounter override is active. The encounter system retains
/// priority — a `Some(...)` from `EncounterMusicRequest::desired_track()`
/// overrides this component the same way it overrides the sandbox-wide
/// default music track. Empty/absent room music falls back to
/// the music registry's `default_track`.
#[derive(Component, Clone, Debug, Default)]
pub struct RoomMusicRequest {
    pub desired_track: Option<String>,
}

/// Focused active-room metadata on the same canonical session root as `RoomSet`.
///
/// Updated by `sync_active_room_metadata` when the active room changes. Consumers
/// (room music selection, ambient layer selection,
/// renderer palette swaps) can subscribe via `ambition_platformer2d_shared_tangle::lifecycle::SessionWorldRef<ActiveRoomMetadata>`
/// + change detection without importing the larger `RoomSet` type.
#[derive(Component, Clone, Debug, Default)]
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
/// cheaper than adding a separate session component per consumer.
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
    /// Does a body SOMEBODY IS DRIVING get a nameplate here?
    ///
    /// ⭐⭐ THE DEFAULT (`None` ⇒ no) IS AN EXPLORATION RULE, and it stops being
    /// right the moment a room holds more than one driven body. A plate exists
    /// to name a body you are not inhabiting, so hiding it over the one you are
    /// is honest in a game with a single driven body — and in a four-fighter
    /// match it renders as "everyone is labelled except the human", which is
    /// exactly the player-centrism this engine keeps removing. Jon, 2026-08-24:
    /// *"This is player 1 centric behavior, and we should have none of it."*
    ///
    /// ⇒ a room with a CAST declares `Some(true)` and every fighter is labelled
    /// the same way. `Some(false)` labels none of them, which is the other
    /// uniform answer and is one value away.
    pub label_driven_bodies: Option<bool>,
}

impl RoomNameplatePolicy {
    pub fn is_empty(&self) -> bool {
        self.full_opacity_count.is_none()
            && self.fade_out_count.is_none()
            && self.label_driven_bodies.is_none()
    }

    pub fn merge(&mut self, other: RoomNameplatePolicy) {
        if self.full_opacity_count.is_none() {
            self.full_opacity_count = other.full_opacity_count;
        }
        if self.fade_out_count.is_none() {
            self.fade_out_count = other.fade_out_count;
        }
        if self.label_driven_bodies.is_none() {
            self.label_driven_bodies = other.label_driven_bodies;
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
    /// `ambition_platformer2d_runtime::in_mode("sanic")` rather than on a global state, so
    /// Ambition can host several demos' rulesets in one binary and the rules
    /// only run inside the rooms that opted into them. `None` is the base game.
    ///
    /// Authored as the LDtk level string field `mode`; merged first-`Some`-wins
    /// across an active area's member levels, like every other string field
    /// here.
    ///
    /// The field is declared in each LDtk project, so an authored room can select
    /// its own ruleset.
    pub mode: Option<String>,
    /// How far past this room's bounds, ALONG the fall direction, a body may
    /// drift before the world declares it gone
    /// ([`WorldEdgeMargins::fall`](ambition_platformer2d_core::WorldEdgeMargins::fall)).
    ///
    /// A platformer's pit depth and a platform fighter's blast zone are the
    /// same number, and it belongs to the room. `None` takes the engine
    /// default, which is what every room got when the number was a literal
    /// inside the movement kernel — a stage could not disagree with it, which
    /// is why a fighting stage could not be authored at all.
    ///
    /// Authored as the LDtk level integer field `fall_out_margin`, in whole
    /// pixels, merged first-`Some`-wins like every other field here. Integer
    /// because `RoomMetadata` is `Eq` and a distance in pixels has no business
    /// being fractional; the composer widens it for the engine.
    ///
    /// ⛔ FLAT here, and grouped on the engine's `World`. The three are merged
    /// independently first-`Some`-wins across metadata sources, so a struct
    /// would have to merge field-by-field anyway and would only hide that.
    pub fall_out_margin: Option<i32>,
    /// ACROSS the fall direction, in whole pixels. `None` — the default — means
    /// the sides are not a loss condition: walking off the left edge of a
    /// corridor is a room transition, not a death. A fighting stage authors a
    /// number; a platformer room never does.
    ///
    /// Authored as the LDtk level integer field `side_out_margin`.
    pub side_out_margin: Option<i32>,
    /// AGAINST the fall direction, in whole pixels. `None` (the default) lets a
    /// body rise forever.
    ///
    /// Authored as the LDtk level integer field `rise_out_margin`.
    pub rise_out_margin: Option<i32>,
    /// Where finishing this room leads — the id of the room its goal sends
    /// the player to. `None` means the room has no successor and loops in
    /// place, which is the classic arcade answer and a real destination rather
    /// than the absence of one.
    ///
    /// Keeping this in room metadata lets authored content define progression
    /// without a room-id dispatch table in Rust.
    ///
    ///  the engine does not check that the named room EXISTS. It cannot:
    /// a level file states an id and only the loaded `RoomSet` knows which
    /// rooms a session holds, so a room that names a destination it does not
    /// have is a WARNING at the consumer, not a load-time refusal. Keeping it
    /// a bare id rather than a resolved handle is what lets a room name a
    /// sibling in another world file.
    ///
    /// Authored as the LDtk level string field `next_room`, merged
    /// first-`Some`-wins like every other string field here.
    pub next_room: Option<String>,
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
            && self.fall_out_margin.is_none()
            && self.side_out_margin.is_none()
            && self.rise_out_margin.is_none()
            && self.next_room.is_none()
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
        if self.fall_out_margin.is_none() {
            self.fall_out_margin = other.fall_out_margin;
        }
        if self.side_out_margin.is_none() {
            self.side_out_margin = other.side_out_margin;
        }
        if self.rise_out_margin.is_none() {
            self.rise_out_margin = other.rise_out_margin;
        }
        if self.next_room.is_none() {
            self.next_room = other.next_room;
        }
        // A multi-level area is a gallery if ANY member level marks it one.
        self.gallery = self.gallery || other.gallery;
        self.visual_profile.merge(other.visual_profile);
        self.nameplate_policy.merge(other.nameplate_policy);
    }
}
