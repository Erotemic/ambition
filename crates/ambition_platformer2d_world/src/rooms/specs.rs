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
/// ⚠ **`character_id` is OPTIONAL, and that is not laziness.** Every level in the
/// tree authors a display name today and resolves through
/// `CharacterCatalog::id_for_authored_identity`, which tries an id first and a
/// display name second. Making the field required would break every existing
/// world file to fix a hazard none of them has hit yet. Both roads must keep
/// working, so both roads are tested.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnemySpawnSpec {
    /// What it DOES: the roster brain key the archetype is selected by.
    pub brain: ambition_entity_catalog::placements::CharacterBrain,
    /// **Which `CharacterDefinition` this spawn instantiates** — the body's
    /// gameplay identity.
    ///
    /// A character is a reusable authored template (body, vitals, movement,
    /// repertoire) and presentation is a projection of it. Which sprite a body
    /// wears therefore never determines which character it is.
    ///
    /// ⚠ **optional only during the migration.** A spawn that authors no id
    /// still resolves ART through [`Self::presentation_identity`]'s display-name
    /// fallback; that road is presentation compatibility and must never answer
    /// the gameplay question. Ask [`Self::gameplay_character_id`] for that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_id: Option<ambition_entity_catalog::CharacterId>,
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
    /// `None` = "this placement did not say", which keeps today's answer: the
    /// archetype's policy. Every existing level is that.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub respawn: Option<ambition_entity_catalog::placements::RespawnPolicy>,
    /// **How this body feels about the player when it spawns.**
    ///
    /// ⭐ the last spawn-context fact an enemy ARCHETYPE owned: a row could say
    /// `is_hostile: false`, which made "ambient wildlife that never aggros"
    /// a property of the creature rather than of this placement of it.
    ///
    /// `None` = the archetype's answer, which is every level authored so far.
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
    pub fn new(brain: ambition_entity_catalog::placements::CharacterBrain) -> Self {
        Self {
            brain,
            character_id: None,
            respawn: None,
            disposition: None,
            brain_profile: None,
        }
    }

    /// **The PRESENTATION identity this spawn wears** — which sheet, portrait
    /// and animation set the renderer should bind.
    ///
    /// `name` is the caller's [`Authored::name`]. Returning the name unchanged
    /// when no id is authored keeps the display-name road intact — the resolver
    /// downstream (`id_for_authored_identity`) accepts either form.
    ///
    /// ⛔ **NOT the gameplay character.** The name fallback is the silent join
    /// this struct's doc describes above; it is tolerable for pixels because a
    /// wrong sheet is visible, and intolerable for health, mass, repertoire or
    /// death traits, where being wrong is not.
    pub fn presentation_identity<'a>(&'a self, name: &'a str) -> &'a str {
        self.character_id
            .as_ref()
            .map(ambition_entity_catalog::CharacterId::as_str)
            .unwrap_or(name)
    }

    /// **Which `CharacterDefinition` this spawn instantiates**, when it says.
    ///
    /// ⭐ **no fallback, and the absence of one is the point.** A display name
    /// that happens to match a character is a coincidence the engine must not
    /// act on. `None` means *"this placement did not say"*, which construction
    /// answers with the legacy archetype — a transitional state that must stay
    /// visible rather than be papered over by a name match.
    pub fn gameplay_character_id(&self) -> Option<&ambition_entity_catalog::CharacterId> {
        self.character_id.as_ref()
    }

    pub fn with_character_id(
        mut self,
        character_id: impl Into<ambition_entity_catalog::CharacterId>,
    ) -> Self {
        self.character_id = Some(character_id.into());
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

impl From<ambition_entity_catalog::placements::CharacterBrain> for EnemySpawnSpec {
    fn from(brain: ambition_entity_catalog::placements::CharacterBrain) -> Self {
        Self::new(brain)
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
    use super::EnemySpawnSpec;

    fn spec() -> EnemySpawnSpec {
        EnemySpawnSpec::new(ambition_entity_catalog::placements::CharacterBrain::Custom(
            "medium_striker".to_string(),
        ))
    }

    /// **The two identities answer differently, and the difference is the point.**
    ///
    /// D73's sharpest correction: a body's gameplay character must not be
    /// inferred from the sprite it wears. An unauthored spawn resolves its ART
    /// through its display name — a join this file's own doc calls a documented
    /// silent failure, tolerable only because a wrong sheet is visible — and
    /// that same chain must return NOTHING when asked which character the body
    /// IS. Poison the accessor to fall back to the name and this reds.
    #[test]
    fn an_unauthored_spawn_wears_a_name_but_claims_no_character() {
        let spec = spec();
        assert_eq!(
            spec.presentation_identity("Fretjaw"),
            "Fretjaw",
            "art still falls back to the display name; that road is unchanged",
        );
        assert_eq!(
            spec.gameplay_character_id(),
            None,
            "a display name is not a character claim, even when one matches",
        );
    }

    /// The authored road answers BOTH questions, which is why authoring an id is
    /// the migration: it is the only form in which the two agree by construction.
    #[test]
    fn an_authored_id_answers_both_questions() {
        let spec = spec().with_character_id("fretjaw");
        assert_eq!(spec.presentation_identity("Fretjaw"), "fretjaw");
        assert_eq!(
            spec.gameplay_character_id().map(|id| id.as_str()),
            Some("fretjaw")
        );
    }
}
