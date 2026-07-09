//! Authored room content specs (props, items, portals, shrines, gravity zones).
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

/// Static decorative prop authored as the `Prop` LDtk entity.
///
/// Props render a sprite at a fixed location with no Interactable
/// (so an Interact press near a prop does NOT pop a dialogue) and
/// no AI / combat / save state. Sheet lookup goes through the
/// prop registry in
/// `crate::character_sprites::sheets`, keyed by `kind`.
///
/// Props are kept off `World::objects` (which is the engine-side
/// authored-object list — every entry there grows runtime behavior).
/// They live on `RoomSpec.props` instead so the sandbox can iterate
/// them once at room load to spawn presentation entities without
/// the engine ever seeing them.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

/// LDtk-authored held item resting on the ground, pick-up-able with `Attack`.
///
/// Resolved to a [`crate::items::pickup::GroundItem`] at room load by looking
/// `held_item` up in the brain held-item registry
/// (`ambition_characters::brain::held_item_by_id`). This is the authored-placement home for
/// the gauntlet / weapon pickups that the debug `spawn_debug_ground_items_once`
/// table used to drop near the player — kept off `World::objects` for the same
/// reason as [`PropSpec`] (the engine never sees them).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GroundItemSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// Held-item registry id, e.g. `meteor`, `bomb`, `puppy_slug_gun`,
    /// `gun_sword`. Resolved via `ambition_characters::brain::held_item_by_id`; an
    /// unregistered id is skipped at spawn rather than erroring.
    pub held_item: String,
    /// World-space center of the pickup box.
    pub pos: ae::Vec2,
    /// Pickup half-extent, taken from the LDtk entity's box size.
    pub half_extent: ae::Vec2,
}

/// LDtk-authored portal-gun pickup. Pure room IR; the Ambition portal adapter
/// lowers it to a runtime `PortalGunPickup` at room load.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PortalGunSpawnSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// World-space center of the pickup box.
    pub pos: ae::Vec2,
    /// Pickup half-extent, taken from the LDtk entity's box size.
    pub half_extent: ae::Vec2,
}

/// Authored/runtime portal channel color carried by room IR.
///
/// This mirrors the Ambition portal crate's current color vocabulary but keeps
/// `ambition_world` from depending on portal runtime types. Portal lowerings map
/// it back to their runtime channel at the presentation/sim edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum PortalChannelColorSpec {
    Purple,
    Yellow,
    Teal,
    Red,
    Green,
    Magenta,
    Cyan,
    Rose,
    Indexed(u8),
}

impl PortalChannelColorSpec {
    pub fn partner(self) -> Self {
        use PortalChannelColorSpec::*;
        match self {
            Purple => Yellow,
            Yellow => Purple,
            Teal => Red,
            Red => Teal,
            Green => Magenta,
            Magenta => Green,
            Cyan => Rose,
            Rose => Cyan,
            Indexed(n) => Indexed(n ^ 1),
        }
    }

    pub fn name(self) -> String {
        use PortalChannelColorSpec::*;
        match self {
            Purple => "purple".into(),
            Yellow => "yellow".into(),
            Teal => "teal".into(),
            Red => "red".into(),
            Green => "green".into(),
            Magenta => "magenta".into(),
            Cyan => "cyan".into(),
            Rose => "rose".into(),
            Indexed(n) => format!("c{n}"),
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        use PortalChannelColorSpec::*;
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "purple" => Purple,
            "yellow" => Yellow,
            "teal" => Teal,
            "red" => Red,
            "green" => Green,
            "magenta" => Magenta,
            "cyan" => Cyan,
            "rose" => Rose,
            other => Indexed(other.strip_prefix('c')?.parse::<u8>().ok()?),
        })
    }
}

/// LDtk-authored static portal. Pure room IR; the Ambition portal adapter
/// lowers it to a runtime `PlacedPortal` at room load.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PortalSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// Authored channel color (its partner color is the linked exit).
    pub color: PortalChannelColorSpec,
    /// World-space center of the portal face (on the host surface).
    pub pos: ae::Vec2,
    /// Outward surface normal (axis-aligned), pointing into the room.
    pub normal: ae::Vec2,
    /// Explicit link id (LDtk `link` field). When set, the portal pairs with
    /// the OTHER portal carrying the same link — overriding the complementary-
    /// color pairing — and a link that is not exactly two members is closed.
    /// `None` ⇒ legacy color pairing.
    pub link: Option<String>,
    /// Authored along-surface half-length (opening size) from the LDtk box.
    /// `None` ⇒ the fixed default. Both ends of a pair shrink to the minimum.
    pub half_length: Option<f32>,
}

/// LDtk-authored heal/save shrine. Resolves to a [`crate::shrine::HealShrine`]
/// at room load — the authored-placement home for the debug
/// `spawn_debug_shrine_once`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ShrineSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// World-space center of the shrine's interaction box.
    pub pos: ae::Vec2,
    /// Interaction half-extent, taken from the LDtk entity's box size.
    pub half_extent: ae::Vec2,
}

/// LDtk-authored localized-gravity zone (a [`crate::physics::GravityZone`]).
/// `oscillate_amplitude > 0` also attaches a [`crate::physics::OscillatingZone`]
/// so the column slides horizontally. The authored-placement home for the debug
/// `spawn_debug_gravity_zone_once`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GravityZoneSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// World-space center of the zone.
    pub center: ae::Vec2,
    /// Zone half-extent, taken from the LDtk entity's box size.
    pub half_extent: ae::Vec2,
    /// Gravity direction inside the zone (e.g. `(0,-1)` = up).
    pub dir: ae::Vec2,
    /// Horizontal slide amplitude in px; `0` = a static column.
    pub oscillate_amplitude: f32,
    /// Slide frequency (used only when `oscillate_amplitude > 0`).
    pub oscillate_freq: f32,
}

/// Authored entity payload — `(id, name, aabb, payload)`. Per-family typing
/// keeps authored entities out of the engine crate.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Authored<T> {
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub payload: T,
}

impl<T> Authored<T> {
    pub fn new(id: impl Into<String>, name: impl Into<String>, aabb: ae::Aabb, payload: T) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            payload,
        }
    }
}

/// Pure authored damage-volume payload carried by [`RoomSpec`]. Runtime combat
/// crates lower this to their live `DamageVolume`; the world IR only stores
/// plain data.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HazardVolumeSpec {
    pub damage: i32,
    pub knockback: [f32; 2],
    pub kind: ambition_entity_catalog::placements::DamageKind,
    pub team: ambition_entity_catalog::placements::DamageTeam,
    pub hitstop_seconds: f32,
    pub respawn: ambition_entity_catalog::placements::HazardRespawn,
    pub path_id: Option<String>,
    pub motion: Option<ae::KinematicPath>,
    pub enabled: bool,
}

impl HazardVolumeSpec {
    pub fn new(amount: i32) -> Self {
        Self {
            damage: amount,
            knockback: [0.0, 0.0],
            kind: ambition_entity_catalog::placements::DamageKind::Hazard,
            team: ambition_entity_catalog::placements::DamageTeam::Environment,
            hitstop_seconds: 0.0,
            respawn: ambition_entity_catalog::placements::HazardRespawn::Never,
            path_id: None,
            motion: None,
            enabled: true,
        }
    }
}

/// Authored interaction payload — now owned by the Tier-0 catalog and carried
/// through the single `PlacementRecord` channel (fable audit F9.2). Re-exported
/// here so `rooms::InteractableSpec` paths stay stable for authoring/lowering.
pub use ambition_entity_catalog::placements::{InteractableSpec, InteractionKindSpec};

/// Authored pickup payload — now owned by the Tier-0 catalog and carried
/// through the single `PlacementRecord` channel (fable audit F9.2). Re-exported
/// here so `rooms::PickupSpec` / `rooms::PickupKindSpec` paths stay stable.
pub use ambition_entity_catalog::placements::{PickupKindSpec, PickupSpec};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ChestSpec {
    pub state: ChestStateSpec,
    pub reward: Option<PickupKindSpec>,
    pub persistent: bool,
}

impl ChestSpec {
    pub fn new(reward: Option<PickupKindSpec>) -> Self {
        Self {
            state: ChestStateSpec::Closed,
            reward,
            persistent: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ChestStateSpec {
    Closed,
    Opening,
    Opened,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BreakableTriggerSpec {
    #[default]
    OnHit,
    OnStand,
    Either,
}

impl BreakableTriggerSpec {
    pub fn allows_hit(self) -> bool {
        matches!(
            self,
            BreakableTriggerSpec::OnHit | BreakableTriggerSpec::Either
        )
    }

    pub fn allows_stand(self) -> bool {
        matches!(
            self,
            BreakableTriggerSpec::OnStand | BreakableTriggerSpec::Either
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BreakableCollisionSpec {
    #[default]
    None,
    Solid,
    OneWayUp,
}

impl BreakableCollisionSpec {
    pub fn blocks_movement(self) -> bool {
        !matches!(self, BreakableCollisionSpec::None)
    }

    pub fn is_solid(self) -> bool {
        matches!(self, BreakableCollisionSpec::Solid)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BreakableSpec {
    pub state: BreakableStateSpec,
    pub health_current: i32,
    pub health_max: i32,
    pub respawn: ambition_entity_catalog::placements::HazardRespawn,
    pub collision: BreakableCollisionSpec,
    pub trigger: BreakableTriggerSpec,
    pub debris_cue: Option<String>,
    pub pogo_refresh: bool,
}

impl BreakableSpec {
    pub fn new(max_hp: i32) -> Self {
        let max_hp = max_hp.max(1);
        Self {
            state: BreakableStateSpec::Intact,
            health_current: max_hp,
            health_max: max_hp,
            respawn: ambition_entity_catalog::placements::HazardRespawn::Never,
            collision: BreakableCollisionSpec::None,
            trigger: BreakableTriggerSpec::OnHit,
            debris_cue: None,
            pogo_refresh: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BreakableStateSpec {
    Intact,
    Cracking,
    Broken,
    Respawning,
}
