//! Authored room content specs (props, items, portals, shrines, gravity zones).

use ambition_platformer2d_core as ae;

/// Static decorative prop authored as the `Prop` LDtk entity.
///
/// Sheet lookup goes through the prop registry in `crate::character_sprites::sheets`, keyed by
/// `kind`.
///
/// Props are kept OUT OF THE ENGINE `World` entirely. Its authored collections
/// (`blocks`, `water_regions`, `climbable_regions`, `chains`) each grow runtime
/// behaviour for every entry, and a decoration should grow none. They live on
/// `RoomSpec.props` instead, so the sandbox can iterate them once at room load
/// to spawn presentation entities without the engine ever seeing them.
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
    /// Mirror the sprite vertically when it is drawn.
    ///
    /// Which way a prop POINTS is authored data, not a second asset: a warp
    /// pipe hanging from a ceiling is the same pipe head as one standing on the
    /// ground, upside down. Defaults to `false`, so existing authored data (and
    /// every LDtk prop) is unchanged.
    #[serde(default)]
    pub flip_y: bool,
    /// Whether this prop is scenery or part of the built world. See [`PropDraw`].
    #[serde(default)]
    pub draw: PropDraw,
}

/// What KIND of thing a prop is, which decides how it is drawn.
///
/// The two cases pull in opposite directions, and conflating them is what makes
/// a pipe look wrong: a character's art deliberately overflows its collision box
/// (a 30×48 body wears a much larger sprite) and hangs off a FEET anchor, while a
/// piece of built world has to line up with the geometry a body stands on, to the
/// pixel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PropDraw {
    /// Scenery. Sized and anchored like a character — the art may overflow the
    /// authored box — and drawn BEHIND the cast. Every LDtk prop is this.
    #[default]
    Decoration,
    /// Part of the built world: a flagpole shaft, a girder, a fixture. The art
    /// fills the authored box EXACTLY — that box is the collider a body stands
    /// on or climbs, so art that overflows it puts the world's surface somewhere
    /// the body cannot be — and it still draws BEHIND the cast, because a body
    /// on it must stay visible.
    Structure,
    /// Built world a body goes INSIDE: a warp pipe. Fills its box like
    /// [`Self::Structure`], but draws in FRONT of the cast, so a body within it
    /// is swallowed rather than pasted on top of it.
    Enclosure,
}

impl PropDraw {
    /// Whether the art must fill the authored box exactly, rather than being
    /// sized like a character (which overflows its box on purpose).
    pub fn fills_box(self) -> bool {
        matches!(self, Self::Structure | Self::Enclosure)
    }

    /// Whether the prop draws in FRONT of the cast.
    pub fn occludes_bodies(self) -> bool {
        matches!(self, Self::Enclosure)
    }
}

/// LDtk-authored held item resting on the ground, pick-up-able with `Attack`.
///
/// Resolved to a [`crate::items::pickup::GroundItem`] at room load through the
/// held-item registry. Kept as room placement IR rather than entering the
/// engine `World`'s authored collections (`blocks`, `water_regions`, …), which
/// grow runtime behaviour for every entry.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GroundItemSpec {
    /// LDtk iid — stable across rebuilds for save/debug joins.
    pub id: String,
    /// LDtk display name (editor-facing / entity naming only).
    pub name: String,
    /// Held-item registry id, e.g. `meteor`, `bomb`, `puppy_slug_gun`,
    /// `gun_sword`. Resolved via `ambition_platformer2d::characters::brain::held_item_by_id`.
    ///
    /// An unregistered id refuses construction with `UnknownHeldItem`.
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

/// Portal channel color, re-exported here to keep the room-spec API stable.
pub use ambition_entity_catalog::placements::PortalChannelColorSpec;

/// LDtk-authored static portal in room IR. The portal adapter lowers it to a
/// runtime `PlacedPortal`; placement MIR uses `ambition_entity_catalog::placements::PortalSchema`.
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
    /// `None`  legacy color pairing.
    pub link: Option<String>,
    /// Authored along-surface half-length (opening size) from the LDtk box. Both ends of a pair
    /// shrink to the minimum.
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

/// Authored encounter trigger geometry in the room IR. The world layer carries
/// where the trigger was authored; the encounter loader converts it to the
/// encounter domain type.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EncounterTriggerSpec {
    /// The authored `id` field, or empty when the author left it blank — the
    /// loader falls back to the area id, which is a fact only it knows.
    pub id: String,
    /// World-space minimum corner of the trigger volume.
    pub min: ae::Vec2,
    /// World-space size of the trigger volume.
    pub size: ae::Vec2,
    /// Authored camera zoom for the encounter, defaulted by the loader.
    pub camera_zoom: Option<f32>,
}

/// A switch's authored `on_activate` line in the room IR. It remains a typed
/// room facet instead of widening the closed Tier-0 `PlacementSchema` for a
/// command string consumed only by switch-command lowering. Most switches omit
/// it and use their normal action/target fields.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SwitchCommandSpec {
    /// The switch's authored `id` — the key `SwitchActivation` joins on.
    pub switch_id: String,
    /// The authored command line, already trimmed and known non-empty.
    pub line: String,
}

/// An authored encounter's lock wall, in the room IR. One per area at most.
///
/// See [`EncounterTriggerSpec`] for why this lives here rather than being read
/// off the project, and for why it is not `ambition_encounter::LockWallSpec`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EncounterLockWallSpec {
    /// The wall's authored `id`, or empty when it has none. Only the GATED
    /// reader needs it — an encounter's own wall is identified by its area.
    pub id: String,
    /// The authored `gated_by` condition, when this wall is a gate rather than
    /// an encounter's.  absent is meaningful and common: a `LockWall` with no
    /// `gated_by` belongs to the encounter whose phase drives it, or is inert.
    pub gated_by: Option<String>,
    /// World-space minimum corner of the wall.
    pub min: ae::Vec2,
    /// World-space size of the wall.
    pub size: ae::Vec2,
}

/// LDtk-authored localized-gravity zone (a [`ambition_platformer2d_shared_tangle::gravity::GravityZone`]).
/// `oscillate_amplitude > 0` also attaches a [`ambition_platformer2d_shared_tangle::gravity::OscillatingZone`]
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
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        payload: impl Into<T>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            payload: payload.into(),
        }
    }
}

/// Initial horizontal orientation authored on an enemy placement.
///
/// This is placement context, not a character or brain property: the same
/// character/controller pair may be placed facing either way. Runtime body
/// kinematics still use a signed scalar; this enum keeps that implementation
/// detail out of authored room data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpawnFacing {
    Left,
    #[default]
    Right,
}

impl SpawnFacing {
    /// Signed facing consumed by `BodyKinematics`: left = -1, right = +1.
    pub const fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    /// Whether serialization may omit this value without changing semantics.
    pub const fn is_default(&self) -> bool {
        matches!(self, Self::Right)
    }
}

/// An authored enemy's BEHAVIOUR and its ART are two different identities.
///
/// `brain` selects behavior while `character_id` selects the body. Gameplay
/// identity never depends on the editor-facing display name.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnemySpawnSpec {
    /// What it DOES: which driver policy plays this placement of the character.
    ///  it selects nothing about the BODY — that is [`Self::character_id`]'s
    /// job, and was the archetype road's confusion.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// Which `CharacterDefinition` this spawn instantiates — the body's
    /// gameplay identity.
    ///
    /// A character is a reusable authored template (body, vitals, movement,
    /// repertoire) and presentation is a projection of it. Which sprite a body
    /// wears therefore never determines which character it is.
    ///
    /// Required: every enemy placement names the character it instantiates.
    ///
    ///  the lowering REFUSES an authored entity with no id rather than
    /// defaulting one. Defaulting is what made "which character is this" a
    /// question with two answers, and the point of the type is that absence
    /// stops being representable.
    pub character_id: ambition_entity_catalog::CharacterId,
    /// Which way this occurrence initially faces.
    ///
    /// `Right` preserves the default `+1.0` runtime facing. Authoring surfaces
    /// may state `Left` explicitly without coupling character/controller data to
    /// stage direction.
    #[serde(default, skip_serializing_if = "SpawnFacing::is_default")]
    pub facing: SpawnFacing,
    /// Placement-specific respawn policy. `None` means the placement did not
    /// specify one, so construction uses `UNDESCRIBED_BODY_RESPAWN`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub respawn: Option<ambition_entity_catalog::placements::RespawnPolicy>,
    /// Placement-specific initial disposition. `None` keeps the disposition
    /// resolved by character construction; an authored value overrides it for
    /// this occurrence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<ambition_entity_catalog::placements::SpawnDisposition>,
    /// WHO DRIVES THIS ONE — the shared controller policy this placement
    /// wants, by provider-relative name.
    ///
    /// This separates body identity from controller policy: the same character
    /// can use different profiles at different placements, and one profile can
    /// drive different characters.
    ///
    /// `None` = the character's own profile, which is every level authored so
    /// far.  a name that resolves to nothing is a construction ERROR, the same
    /// contract `CharacterDefinition::autonomous_profile_ref` carries — an
    /// explicit reference that misses must never read as silence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brain_profile: Option<ambition_entity_catalog::BrainProfileRef>,
}

impl EnemySpawnSpec {
    ///  the character is a CONSTRUCTOR argument, not something added later.
    /// This took only a brain and left `character_id: None`, so every call site
    /// was one `.with_character_id(..)` away from a placement that names no
    /// creature — and forgetting it compiled. Taking it here is what makes the
    /// required field mean anything.
    pub fn new(
        brain: ambition_entity_catalog::placements::CharacterBrain,
        character_id: impl Into<ambition_entity_catalog::CharacterId>,
    ) -> Self {
        Self {
            brain,
            character_id: character_id.into(),
            facing: SpawnFacing::default(),
            respawn: None,
            disposition: None,
            brain_profile: None,
        }
    }

    /// The PRESENTATION identity this spawn wears — which sheet, portrait
    /// and animation set the renderer should bind.
    ///
    /// Presentation resolves from the required character id; display names are
    /// not identity fallbacks.
    ///
    /// Kept as a named accessor rather than inlined because presentation and
    /// gameplay asking the same question through two names is what made the
    /// divergence expressible in the first place; now they demonstrably agree.
    pub fn presentation_identity(&self) -> &str {
        self.character_id.as_str()
    }

    /// Which `CharacterDefinition` this spawn instantiates.
    ///
    /// No fallback and no `Option`: every placement states its character.
    pub fn gameplay_character_id(&self) -> &ambition_entity_catalog::CharacterId {
        &self.character_id
    }

    pub fn with_character_id(
        mut self,
        character_id: impl Into<ambition_entity_catalog::CharacterId>,
    ) -> Self {
        self.character_id = character_id.into();
        self
    }

    /// Author how this placement's body feels about the player. See
    /// [`Self::disposition`].
    pub fn with_disposition(
        mut self,
        disposition: ambition_entity_catalog::placements::SpawnDisposition,
    ) -> Self {
        self.disposition = Some(disposition);
        self
    }

    /// Author when this placement's body comes back. See [`Self::respawn`].
    pub fn with_respawn(
        mut self,
        respawn: ambition_entity_catalog::placements::RespawnPolicy,
    ) -> Self {
        self.respawn = Some(respawn);
        self
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

/// Tier-0 authored interaction payload, re-exported here to keep the room
/// authoring/lowering path stable.
pub use ambition_entity_catalog::placements::{InteractableSpec, InteractionKindSpec};

/// Tier-0 authored pickup payload, re-exported here to keep the room
/// authoring/lowering path stable.
pub use ambition_entity_catalog::placements::PickupSpec;
pub use ambition_entity_catalog::PickupKind;

/// Tier-0 authored chest payload, re-exported here to keep the room
/// authoring/lowering path stable.
pub use ambition_entity_catalog::placements::{ChestSpec, ChestStateSpec};

/// Tier-0 authored breakable payload and related enums, re-exported here to
/// keep the room authoring/lowering path stable.
pub use ambition_entity_catalog::placements::{
    BreakableCollisionSpec, BreakableSpec, BreakableStateSpec, BreakableTriggerSpec,
};

#[cfg(test)]
mod enemy_spawn_identity_tests {
    use super::{EnemySpawnSpec, SpawnFacing};

    /// Authored room data uses a semantic direction while the body keeps its existing signed
    /// runtime representation.
    #[test]
    fn spawn_facing_is_semantic_and_backwards_compatible() {
        let spec = EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Passive,
            "fretjaw",
        );
        assert_eq!(spec.facing, SpawnFacing::Right);
        assert_eq!(SpawnFacing::Right.sign(), 1.0);
        assert_eq!(SpawnFacing::Left.sign(), -1.0);
    }

    /// Presentation and gameplay identity are both derived from the required
    /// `character_id`, so they cannot disagree.
    #[test]
    fn presentation_and_gameplay_cannot_disagree_about_the_character() {
        let spec = EnemySpawnSpec::new(
            ambition_entity_catalog::placements::CharacterBrain::Custom(
                "medium_striker".to_string(),
            ),
            "fretjaw",
        );
        assert_eq!(spec.presentation_identity(), "fretjaw");
        assert_eq!(spec.gameplay_character_id().as_str(), "fretjaw");
        assert_eq!(
            spec.presentation_identity(),
            spec.gameplay_character_id().as_str(),
            "one field answers both questions; there is no second source to drift",
        );
    }
}
