//! Data-driven sandbox room-set and loading-zone graph.
//!
//! Rooms are runtime graph nodes built from LDtk-authored runtime data. This
//! module owns transition graph assembly and arrival validation, while LDtk owns
//! sandbox world authoring.
//! Loading-zone links point at destination zones by name, so authoring no longer
//! requires brittle hand-written spawn coordinates.

use ambition_engine as ae;
use bevy::prelude::{Message, Resource};
use petgraph::graph::{Graph, NodeIndex};

mod graph;
mod spawn;
mod systems;
#[cfg(test)]
mod tests;

pub use spawn::validated_spawn;
pub use systems::{
    hide_portal_loading_zone_visuals, sync_active_room_metadata, sync_portal_ring_rotation_system,
    sync_portal_sprite_animation, sync_portal_sprite_visibility, sync_room_music_request,
    tick_portal_phases_system, PortalSprite,
};

/// How a loading zone should be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingZoneActivation {
    /// Walk-off-the-edge transition. Validator requires the zone to
    /// touch a level edge so the player physically walks off the
    /// screen into it. Arrival on the target side is 92px inset
    /// from the matching edge.
    EdgeExit,
    /// Interact-to-enter door. Doesn't require an edge; the player
    /// presses Interact while overlapping the zone to fire the
    /// transition. Arrival on the target side is centered on the
    /// target zone, bottom-26px.
    Door,
    /// Walk-into-the-zone trigger. Like `EdgeExit` (overlap = fire)
    /// but NOT required to touch a level edge — used for portals
    /// and other mid-room walk-through transitions where the
    /// player just steps inside the rectangle and the transition
    /// fires. Arrival uses the same centered-bottom rule as `Door`.
    Walk,
}

impl LoadingZoneActivation {
    pub fn label(self) -> &'static str {
        match self {
            Self::EdgeExit => "edge exit",
            Self::Door => "door",
            Self::Walk => "walk",
        }
    }
}

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub id: String,
    pub name: String,
    pub activation: LoadingZoneActivation,
    pub aabb: ae::Aabb,
}

impl LoadingZone {
    pub fn is_ready(&self, wants_interact: bool) -> bool {
        match self.activation {
            LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => true,
            LoadingZoneActivation::Door => wants_interact,
        }
    }

    pub fn hint(&self, _flying: bool) -> String {
        match self.activation {
            LoadingZoneActivation::EdgeExit | LoadingZoneActivation::Walk => {
                format!("{}: {}", self.activation.label(), self.name)
            }
            LoadingZoneActivation::Door => {
                format!(
                    "{}: {} (Interact / double-tap up)",
                    self.activation.label(),
                    self.name
                )
            }
        }
    }
}

/// Portal lifecycle phase. A portal's traversal readiness lives in
/// the *portal*, not in its controlling switch — the switch only
/// commands open/close; the portal runs the boot/shutdown sequence.
///
/// Sprite mapping (gate_portal_spritesheet rows):
/// - `Off`          → no portal sprite visible (only the ring)
/// - `Opening`      → opening animation (one-shot, ~0.64s)
/// - `On`           → stable animation (looping; traversal allowed)
/// - `Closing`      → closing animation (one-shot, ~0.64s)
///
/// Switch-flip behavior:
/// - off → on: Off→Opening, or Closing→Opening (resumes mid-close)
/// - on → off: On→Closing, or Opening→Closing (interrupts mid-open)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PortalPhase {
    Off,
    Opening { elapsed: f32 },
    On,
    Closing { elapsed: f32 },
}

impl Default for PortalPhase {
    fn default() -> Self {
        Self::Off
    }
}

impl PortalPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Opening { .. } => "opening",
            Self::On => "on",
            Self::Closing { .. } => "closing",
        }
    }

    pub fn portal_sprite_visible(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub fn allows_traversal(self) -> bool {
        matches!(self, Self::On)
    }
}

/// One portal's configuration + live phase.
#[derive(Clone, Debug)]
pub struct PortalConfig {
    /// The switch whose on/off state commands this portal's boot /
    /// shutdown sequence. Read from `save.data().switch(switch_id)`.
    pub switch_id: String,
    /// LDtk display name of the portal sprite entity (NpcSpawn
    /// name). The visibility system matches this against
    /// `FeatureName` to hide the portal sprite when phase == Off.
    pub portal_sprite_name: String,
    /// LDtk display name of the ring sprite entity. Used by the
    /// ring-spin visual flourish during `Opening`.
    pub ring_sprite_name: String,
    pub phase: PortalPhase,
}

/// Per-portal registry mapping `LoadingZone.id` → portal lifecycle.
/// `detect_room_transition_system` consults the registry before
/// writing a `RoomTransitionRequested`: if the zone is a portal,
/// traversal is allowed only while `phase == On`. Empty by default
/// — populated by story-content plugins.
///
/// Replaces the earlier `GatedZoneRegistry` (which only tracked
/// the switch and treated the zone as a thin switch-gate). The
/// portal's *own* state is what gates traversal — the switch just
/// drives the boot/shutdown sequence — so the readiness check lives
/// here, not in the switch system.
#[derive(Resource, Default, Debug, Clone)]
pub struct PortalRegistry {
    pub portals: std::collections::HashMap<String, PortalConfig>,
}

impl PortalRegistry {
    pub fn register(
        &mut self,
        zone_id: impl Into<String>,
        switch_id: impl Into<String>,
        portal_sprite_name: impl Into<String>,
        ring_sprite_name: impl Into<String>,
    ) {
        self.portals.insert(
            zone_id.into(),
            PortalConfig {
                switch_id: switch_id.into(),
                portal_sprite_name: portal_sprite_name.into(),
                ring_sprite_name: ring_sprite_name.into(),
                phase: PortalPhase::default(),
            },
        );
    }

    pub fn phase(&self, zone_id: &str) -> PortalPhase {
        self.portals
            .get(zone_id)
            .map(|c| c.phase)
            .unwrap_or(PortalPhase::Off)
    }

    pub fn is_portal(&self, zone_id: &str) -> bool {
        self.portals.contains_key(zone_id)
    }

    pub fn allows_traversal(&self, zone_id: &str) -> bool {
        self.portals
            .get(zone_id)
            .map(|c| c.phase.allows_traversal())
            .unwrap_or(true)
    }
}

/// 8 frames × 80ms = 640ms. Mirrors the `opening` row duration in
/// `interdimensional_gate_portal_spritesheet.yaml`.
pub const PORTAL_OPENING_DURATION_SECS: f32 = 0.640;
/// Mirrors the `closing` row duration.
pub const PORTAL_CLOSING_DURATION_SECS: f32 = 0.640;

/// Advance a portal phase one tick. Pure function — exposed so a
/// system can call it without holding `&mut PortalConfig`.
pub fn tick_portal_phase(phase: &mut PortalPhase, switch_on: bool, dt: f32) {
    match phase {
        PortalPhase::Off => {
            if switch_on {
                *phase = PortalPhase::Opening { elapsed: 0.0 };
            }
        }
        PortalPhase::Opening { elapsed } => {
            *elapsed += dt;
            if !switch_on {
                // Interrupted mid-open — start closing from the same
                // visual progress (so the player sees a smooth reverse,
                // not a snap back to fully-open).
                let opened_frac = (*elapsed / PORTAL_OPENING_DURATION_SECS).clamp(0.0, 1.0);
                *phase = PortalPhase::Closing {
                    elapsed: PORTAL_CLOSING_DURATION_SECS * (1.0 - opened_frac),
                };
            } else if *elapsed >= PORTAL_OPENING_DURATION_SECS {
                *phase = PortalPhase::On;
            }
        }
        PortalPhase::On => {
            if !switch_on {
                *phase = PortalPhase::Closing { elapsed: 0.0 };
            }
        }
        PortalPhase::Closing { elapsed } => {
            *elapsed += dt;
            if switch_on {
                let closed_frac = (*elapsed / PORTAL_CLOSING_DURATION_SECS).clamp(0.0, 1.0);
                *phase = PortalPhase::Opening {
                    elapsed: PORTAL_OPENING_DURATION_SECS * (1.0 - closed_frac),
                };
            } else if *elapsed >= PORTAL_CLOSING_DURATION_SECS {
                *phase = PortalPhase::Off;
            }
        }
    }
}

/// Track the music identifier the active room would like to play.
///
/// Written by `sync_room_music_request` from `ActiveRoomMetadata`,
/// consumed by `audio::apply_encounter_music` as the "default track"
/// when no encounter override is active. The encounter system retains
/// priority — `EncounterMusicRequest::desired_track = Some(...)`
/// overrides this resource the same way it overrides the sandbox-wide
/// default music track. Empty/absent room music falls back to
/// `sandbox_data.audio.default_music_track`.
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
/// `visual_theme` plus explicit room-visual-profile fields land here.
/// Every field is optional so existing levels keep working
/// without a value. The first non-empty value among an active area's
/// member levels wins; future systems can refine this if needed
/// (e.g. dominant-vote, level-position weighted).
///
/// Consumers: room music selection, ambient layer selection,
/// renderer palette/theme variants. This struct is intentionally
/// non-exhaustive — adding a metadata seam is cheaper than adding a
/// new resource per consumer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomMetadata {
    pub biome: Option<String>,
    pub music_track: Option<String>,
    pub ambient_profile: Option<String>,
    pub visual_theme: Option<String>,
    pub visual_profile: RoomVisualProfile,
}

impl RoomMetadata {
    pub fn is_empty(&self) -> bool {
        self.biome.is_none()
            && self.music_track.is_none()
            && self.ambient_profile.is_none()
            && self.visual_theme.is_none()
            && self.visual_profile.is_empty()
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
        self.visual_profile.merge(other.visual_profile);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq)]
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
}

impl CameraZoneSpec {
    pub const LEGACY_BREATH_ZOOM: f32 = 1.15;

    pub fn effective_zoom(&self) -> f32 {
        self.zoom.unwrap_or(Self::LEGACY_BREATH_ZOOM).max(1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KinematicPathSpec {
    /// Stable authored lookup id. LDtk may not have an explicit `id` field yet,
    /// so conversion falls back to the entity `name` and finally the LDtk iid.
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub path: ae::KinematicPath,
}

impl KinematicPathSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        path: ae::KinematicPath,
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
        self.aliases().any(|alias| alias == query)
    }
}

/// Static decorative prop authored as the `Prop` LDtk entity.
///
/// Props render a sprite at a fixed location with no Interactable
/// (so an Interact press near a prop does NOT pop a dialogue) and
/// no AI / combat / save state. Sheet lookup goes through
/// [`crate::presentation::character_sprites::PropRegistry`] keyed by `kind`.
///
/// Props are kept off `World::objects` (which is the engine-side
/// authored-object list — every entry there grows runtime behavior).
/// They live on `RoomSpec.props` instead so the sandbox can iterate
/// them once at room load to spawn presentation entities without
/// the engine ever seeing them.
#[derive(Clone, Debug, PartialEq)]
pub struct PropSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name. Authors edit this; the renderer uses it
    /// only for entity naming / debug overlay.
    pub name: String,
    /// Registry key for sprite lookup, e.g. `intro_cart`,
    /// `lab_genesis_vat`, `gate_ring`, `gate_portal`. Story-content
    /// plugins populate `PropRegistry` with the corresponding sheet.
    pub kind: String,
    /// World-space center of the prop's bounding box.
    pub pos: ae::Vec2,
    /// Authored bounding-box size. The renderer treats this as the
    /// nominal collision footprint when computing render size from
    /// the sheet's `collision_scale`.
    pub size: ae::Vec2,
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
    pub metadata: RoomMetadata,
    pub camera_zones: Vec<CameraZoneSpec>,
    /// LDtk-authored path index for platforms, hazards, NPC patrols, camera
    /// rails, and future scripted room beats. `World::objects` still mirrors
    /// these as `RoomObjectKind::KinematicPath` for older consumers, but new
    /// systems should use this typed area-local index.
    pub kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored moving platforms for this area. This is the complete
    /// platform set for gameplay: if the vector is empty, the room has no
    /// moving platforms.
    pub moving_platforms: Vec<crate::world::platforms::MovingPlatformState>,
    /// LDtk-authored decorative props. Render-only — see [`PropSpec`].
    pub props: Vec<PropSpec>,
}

#[derive(Clone, Debug)]
struct TransitionEdge {
    from_zone: String,
    to_zone: String,
}

/// Authored directed connection between loading zones in runtime rooms.
///
/// This is intentionally independent from the retired RON world manifest so
/// LDtk and future generators can build `RoomSet` directly from runtime room
/// data.
#[derive(Clone, Debug)]
pub struct RoomLink {
    pub from_room: String,
    pub from_zone: String,
    pub to_room: String,
    pub to_zone: String,
    pub bidirectional: bool,
}

/// Resolved transition from the active room to a graph-linked destination room.
#[derive(Clone, Debug)]
pub struct RoomTransition {
    pub zone: LoadingZone,
    pub target_room: usize,
    pub arrival: ae::Vec2,
}

/// Bevy message emitted when a room transition is triggered (player walks through
/// a loading zone or door). The actual `load_room` call happens in
/// `apply_room_transition_system`, which runs after `sandbox_update` in the
/// `CoreSimulation` chain.
///
/// Carries the resolved `RoomTransition` payload and the optional SFX id for the
/// zone type so the apply system can emit the sound at the correct player position
/// after repositioning.
#[derive(Message, Clone, Debug)]
pub struct RoomTransitionRequested {
    pub transition: RoomTransition,
    /// SFX id to play at the new player position after the room loads.
    pub zone_sfx: Option<ambition_sfx::SfxId>,
}

impl RoomTransitionRequested {
    pub fn new(transition: RoomTransition, zone_sfx: Option<ambition_sfx::SfxId>) -> Self {
        Self {
            transition,
            zone_sfx,
        }
    }
}

/// Small room graph for early loading-zone tests.
#[derive(Resource, Clone, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomSpec>,
    pub active: usize,
    /// Index of the room the player starts in on a fresh sandbox.
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    pub start: usize,
    graph: Graph<String, TransitionEdge>,
    room_nodes: Vec<NodeIndex>,
}
