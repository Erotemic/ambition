//! Authored room content specs (props, items, portals, shrines, gravity zones).
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use ambition_platformer2d_core as ae;

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
    /// `gun_sword`. Resolved via `ambition_characters::brain::held_item_by_id`.
    ///
    /// An unregistered id REFUSES construction (`UnknownHeldItem`) — it does not
    /// skip at spawn, whatever this comment used to say. The planned-construction
    /// campaign made it a hard boundary check and left the note behind; the room
    /// binding sweep found the contradiction.
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

/// Authored/runtime portal channel color — now owned by the Tier-0 catalog and
/// re-exported here so `rooms::PortalChannelColorSpec` paths stay stable (fable
/// audit F9.2). See [`ambition_entity_catalog::placements::PortalChannelColorSpec`].
pub use ambition_entity_catalog::placements::PortalChannelColorSpec;

/// LDtk-authored static portal — the runtime-facing spec carrying kernel `Vec2`
/// (`pos`/`normal`). The Tier-0 MIRROR carried on the `placements` channel is
/// [`ambition_entity_catalog::placements::PortalSchema`]; the actor portal
/// lowering reconstructs this from a placement record (fable audit F9.2). Pure
/// room IR; the Ambition portal adapter lowers it to a runtime `PlacedPortal` at
/// room load.
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

/// **An authored encounter's trigger volume**, in the room IR.
///
/// ⭐⭐ **this exists so an encounter can be read off a ROOM instead of off an
/// `LdtkProject`** (D136). `EncounterTrigger` and `LockWall` were the two
/// markers the converter deliberately dropped — *"read by their own consumers
/// off the raw `LdtkProject`; they never join the emission stream"* — and that
/// sentence is why five production files still need the LDtk crate, which is
/// what stands between the workspace and its capability-footprint number.
///
/// ⚠ **the IR type and `ambition_encounter`'s domain type are deliberately
/// separate, and that is layering rather than a fork.** The world crate is
/// below the encounter crate and must not learn what an encounter IS; it
/// carries WHERE one was authored, and the loader converts. The conversion is
/// stated at both ends so a reader meets the pair on purpose.
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

/// **A `Switch`'s authored `on_activate` line**, in the room IR.
///
/// ⭐⭐ **a typed family and NOT a field on `InteractableSpec`, deliberately**
/// (D136). The switch already emits an interactable — but `InteractableSpec` is
/// a variant of the CLOSED Tier-0 `PlacementSchema`, whose kinds are *"an
/// explicit compatibility contract [that] may only change with a
/// fingerprint-schema bump"*. Widening it would put a netcode/replay schema
/// event behind a load-time authored string that exactly one consumer reads.
///
/// ⚠ **most switches emit none.** `on_activate` is optional: a switch without
/// one is driven by its `action`/`target_encounter` pair as before, and
/// `authored_switch_commands` is the only reader.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SwitchCommandSpec {
    /// The switch's authored `id` — the key `SwitchActivation` joins on.
    pub switch_id: String,
    /// The authored command line, already trimmed and known non-empty.
    pub line: String,
}

/// **An authored encounter's lock wall**, in the room IR. One per area at most.
///
/// See [`EncounterTriggerSpec`] for why this lives here rather than being read
/// off the project, and for why it is not `ambition_encounter::LockWallSpec`.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EncounterLockWallSpec {
    /// The wall's authored `id`, or empty when it has none. Only the GATED
    /// reader needs it — an encounter's own wall is identified by its area.
    pub id: String,
    /// The authored `gated_by` condition, when this wall is a gate rather than
    /// an encounter's. ⚠ absent is meaningful and common: a `LockWall` with no
    /// `gated_by` belongs to the encounter whose phase drives it, or is inert.
    pub gated_by: Option<String>,
    /// World-space minimum corner of the wall.
    pub min: ae::Vec2,
    /// World-space size of the wall.
    pub size: ae::Vec2,
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
    /// ⭐ **`impl Into<T>`, not `T`.** Every existing call passes the payload
    /// exactly, and the blanket `impl From<T> for T` keeps those compiling — but
    /// it also lets a family GROW its payload from a bare value into a struct
    /// without rewriting two dozen call sites. [`EnemySpawnSpec`] is the first to
    /// use that: it wraps the `CharacterBrain` every caller used to pass.
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
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
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

/// **An authored enemy's BEHAVIOUR and its ART are two different identities.**
///
/// Before this existed the payload was a bare `CharacterBrain` and the art was
/// joined by matching `Authored::name` — a human-readable display name — against
/// the character catalog. That join has a documented, silent failure: renaming a
/// character un-arts every level that placed it, because nothing connects the two
/// but a string a human typed twice. Two demos carried a hand-written
/// `name_enemies_for_render` pass to patch the name back in after conversion, and
/// a third was about to be written.
///
/// ⭐ **the shape is borrowed from next door.** `NpcSpawn` already authors a
/// `character_id`, and `MovingPlatform` was given an authored `id` on 2026-08-05
/// for the same reason — *a name is presentation, and this repo has twice paid
/// for keying gameplay on one*. `EnemySpawn` was the remaining placement that
/// takes an identity and does not read one.
///
/// ⭐ **the display-name road is GONE (2026-08-14).** This doc used to say the
/// id was optional because every level resolved through
/// `CharacterCatalog::id_for_authored_identity` — an id first, a display name
/// second — and that requiring it would break existing world files. The census
/// refuted the premise: 184 authored `EnemySpawn` entities, 0 without an id. So
/// the field is required, the lowering REFUSES a placement that authors none,
/// and there is no second road left to test.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnemySpawnSpec {
    /// What it DOES: which driver policy plays this placement of the character.
    /// ⚠ it selects nothing about the BODY — that is [`Self::character_id`]'s
    /// job, and was the archetype road's confusion.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// **Which `CharacterDefinition` this spawn instantiates** — the body's
    /// gameplay identity.
    ///
    /// A character is a reusable authored template (body, vitals, movement,
    /// repertoire) and presentation is a projection of it. Which sprite a body
    /// wears therefore never determines which character it is.
    ///
    /// ⭐ **REQUIRED, since 2026-08-14.** It was `Option` "only during the
    /// migration", and the migration is over: measured across every `.ldtk` in
    /// the repo — the content worlds, both demos, and the `ambition_map_assets`
    /// submodule — **184 `EnemySpawn` entities, 0 without an id.** AC6.1 deleted
    /// the archetype road an absent id used to fall back to, so the last thing
    /// absence could mean was already gone.
    ///
    /// ⛔ the lowering REFUSES an authored entity with no id rather than
    /// defaulting one. Defaulting is what made "which character is this" a
    /// question with two answers, and the point of the type is that absence
    /// stops being representable.
    pub character_id: ambition_entity_catalog::CharacterId,
    /// Which way this occurrence initially faces.
    ///
    /// `Right` is the compatibility default because character-first actor
    /// construction historically initialized `BodyKinematics::facing` to
    /// `+1.0`. LDtk and other authoring surfaces may state `Left` explicitly
    /// without teaching the character or its controller about stage direction.
    #[serde(default, skip_serializing_if = "SpawnFacing::is_default")]
    pub facing: SpawnFacing,
    /// **When this body comes back after it dies** (ADR 0022).
    ///
    /// ⭐ **a PLACEMENT fact with nowhere to be authored, until now.** Respawn
    /// is the one thing in an enemy archetype row that is neither the
    /// character's nor the controller's — the same creature is a permanent
    /// casualty in a story room and a repopulating trash mob in a corridor, and
    /// the row could only say one. It lived there because a placement had no
    /// field for it, which is exactly the shape a migrated character exposes:
    /// with the mites' rows deleted, their respawn policy arrives through the
    /// `combatant` FALLBACK, and that is luck rather than authorship.
    ///
    /// `None` = "this placement did not say", and the engine answers with the
    /// NAMED default `UNDESCRIBED_BODY_RESPAWN` (`OnRoomReenter` today). ⚠ it
    /// used to say *"the archetype's policy"* — there is no archetype, and the
    /// lookup that supplied one could not fail, so every body reached it and
    /// none of them chose it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub respawn: Option<ambition_entity_catalog::placements::RespawnPolicy>,
    /// **How this body feels about the player when it spawns.**
    ///
    /// ⭐ the last spawn-context fact an enemy ARCHETYPE owned: a row could say
    /// `is_hostile: false`, which made "ambient wildlife that never aggros"
    /// a property of the creature rather than of this placement of it.
    ///
    /// `None` = whatever CHARACTER CONSTRUCTION resolved — the creature's own
    /// answer, kept. An authored disposition overrules it, which is the only
    /// thing a placement is entitled to say here. ⚠ it used to say *"the
    /// archetype's answer"*, and that was the defect: the fallback read the
    /// generic `combatant` row's `hostile_by_default: true` and handed the giant
    /// GNU — a mount whose profile states it seeks nobody — its hostility back
    /// one line after construction had resolved it correctly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<ambition_entity_catalog::placements::SpawnDisposition>,
    /// **WHO DRIVES THIS ONE** — the shared controller policy this placement
    /// wants, by provider-relative name.
    ///
    /// ⭐⭐ **the third authority's missing half.** A character states what a
    /// body IS and a `BrainProfile` states how a driver decides, but until now
    /// only the CHARACTER could name a profile — so one creature had exactly one
    /// way to be played everywhere it appeared. That is the enemy-archetype
    /// ontology surviving one level down: body and driver fused, just at a
    /// finer grain.
    ///
    /// What it buys is the demonstration Jon asked for in place of the
    /// one-of-each archetype museum: *the same controller policy can drive
    /// distinct bodies, and the same body can use distinct policies.* A goblin
    /// that patrols a corridor and a goblin that guards a door are one creature
    /// and two placements, not two creatures.
    ///
    /// `None` = the character's own profile, which is every level authored so
    /// far. ⛔ a name that resolves to nothing is a construction ERROR, the same
    /// contract `CharacterDefinition::autonomous_profile_ref` carries — an
    /// explicit reference that misses must never read as silence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brain_profile: Option<ambition_entity_catalog::BrainProfileRef>,
}

impl EnemySpawnSpec {
    /// ⛔ **the character is a CONSTRUCTOR argument, not something added later.**
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

    /// **The PRESENTATION identity this spawn wears** — which sheet, portrait
    /// and animation set the renderer should bind.
    ///
    /// ⛔⛔ **the `name` parameter and the display-name fallback are GONE.** This
    /// took `name: &str` and returned it unchanged when no id was authored,
    /// which made a display name that happens to match a character into a
    /// gameplay-adjacent join — tolerable for pixels, because a wrong sheet is
    /// visible, and the exact silent coincidence the id exists to replace. With
    /// the id required there is nothing to fall back FROM.
    ///
    /// Kept as a named accessor rather than inlined because presentation and
    /// gameplay asking the same question through two names is what made the
    /// divergence expressible in the first place; now they demonstrably agree.
    pub fn presentation_identity(&self) -> &str {
        self.character_id.as_str()
    }

    /// **Which `CharacterDefinition` this spawn instantiates.**
    ///
    /// ⭐ no fallback and no `Option`: a placement states its character or it is
    /// not a placement. `None` used to mean *"this placement did not say"*, which
    /// construction answered with the legacy archetype — a road AC6.1 deleted, so
    /// the last thing absence could mean went with it.
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

// ⛔ **`From<CharacterBrain>` is DELETED.** It said a brain alone is a
// placement, which is the exact claim the required `character_id` refutes: a
// controller is not a creature, and a spec built from one named no body at all.

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
/// here so `rooms::PickupSpec` / `rooms::PickupKind` paths stay stable.
pub use ambition_entity_catalog::placements::PickupSpec;
pub use ambition_entity_catalog::PickupKind;

/// Authored chest payload — now owned by the Tier-0 catalog and carried through
/// the single `PlacementRecord` channel (fable audit F9.2). Re-exported here so
/// `rooms::ChestSpec` / `rooms::ChestStateSpec` paths stay stable.
pub use ambition_entity_catalog::placements::{ChestSpec, ChestStateSpec};

/// Authored breakable payload + its state/trigger/collision enums — now owned by
/// the Tier-0 catalog and carried through the single `PlacementRecord` channel
/// (fable audit F9.2). Re-exported here so `rooms::Breakable*` paths stay stable.
pub use ambition_entity_catalog::placements::{
    BreakableCollisionSpec, BreakableSpec, BreakableStateSpec, BreakableTriggerSpec,
};

#[cfg(test)]
mod enemy_spawn_identity_tests {
    use super::{EnemySpawnSpec, SpawnFacing};

    /// Authored room data uses a semantic direction while the body keeps its
    /// existing signed runtime representation. Silence preserves the old +1
    /// construction behavior for non-migrated worlds.
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

    /// **The two identities agree BY CONSTRUCTION, and that is the whole change.**
    ///
    /// ⛔⛔ this module used to assert the opposite and was right to:
    /// `an_unauthored_spawn_wears_a_name_but_claims_no_character` pinned that art
    /// fell back to the display name while the gameplay accessor returned
    /// `None` — a documented silent join, tolerable only because a wrong sheet
    /// is visible. That state is now unrepresentable: `character_id` is
    /// required, `presentation_identity` takes no name to fall back to, and
    /// `gameplay_character_id` has no `None` to return. A test demanding a value
    /// the type can no longer hold is a product decided against, not a
    /// regression, so it is deleted rather than weakened.
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
