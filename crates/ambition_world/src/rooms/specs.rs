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

/// LDtk-authored portal-gun pickup. Resolves to a
/// [`ambition_portal::PortalGunPickup`] (already armed, `arm_timer = 0`) at room
/// load — the authored-placement home for the debug
/// `spawn_debug_portal_gun_pickup_once`.
#[cfg(feature = "portal")]
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

/// LDtk-authored static portal. Resolves to a [`ambition_portal::PlacedPortal`] at room
/// load — pre-placed linked pairs (by complementary color) for the portal test
/// lab, independent of the portal gun. The half-extent is the standard portal
/// opening (derived from the normal), not the LDtk box size.
#[cfg(feature = "portal")]
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PortalSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// Authored channel color (its partner color is the linked exit). Authored
    /// portals are never gun colors — only [`PortalChannelColor`] is authorable.
    ///
    /// [`PortalChannelColor`]: ambition_portal::PortalChannelColor
    pub color: ambition_portal::PortalChannelColor,
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
