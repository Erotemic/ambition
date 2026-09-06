//! Authored character data before runtime preparation.
//! Kit resolution remains runtime work; this module contains body-owned authored facts.

use ambition_entity_catalog::{HurtboxDoc, MovesetContract};

/// Non-authoritative reproducibility metadata for generated variants.
/// Derived characters have independent stable ids; lineage is not inheritance or balance policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lineage {
    pub derived_from: Option<String>,
    pub generator_revision: Option<String>,
    pub source_fingerprint: Option<String>,
}

/// Default health when no character-authored health pool is provided.
pub const DEFAULT_UNAUTHORED_BODY_HEALTH: i32 = 4;

/// Optional authored physical limits; `None` leaves construction-time state authoritative.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vitals {
    /// Authored health pool. `None` leaves the construction-time pool authoritative.
    pub max_health: Option<i32>,
    /// Reaches the body as `Mass`, which drives the mount pair's mass-weighted
    /// center of gravity. `None` preserves the body's existing mass; `Some(1.0)`
    /// is an explicit authored override even though `1.0` is the ambient default.
    pub mass: Option<f32>,
    /// Knockback weight used by combat launch scaling. `1.0` is the reference body.
    /// This is independent of [`Self::mass`], which controls mount-pair physics.
    /// `None` preserves the body or roster value.
    pub knockback_weight: Option<f32>,
    /// The standing height an author BUILT THIS DEFINITION FROM, in world
    /// pixels. `collision_scale` remains independent crop/footprint compensation.
    ///
    /// ⛔⛔ A RECORD, NOT A RUNTIME AUTHORITY, and this doc said otherwise until
    /// 2026-08-31. It read *"used to scale sprite-authored geometry consistently
    /// across characters"*, which is true of
    /// [`world_per_pixel_for_height`] and false of this field: the two callers
    /// that set it (`player_robot_lineage`, Mary-O's forms) compute the scale
    /// from it at AUTHORING time and store the OUTPUT on
    /// `BodySource::SpriteAuthored`. Measured: outside tests, nothing in
    /// gameplay reads this field — the only reader in the tree is
    /// `moveset_export`'s JSON dump, which is what a record is for.
    ///
    /// ⚠ NOT a duplicate of the catalog row's `standing_height` either, which IS
    /// read (`ambition_sprite_sheet::character::catalog_join`) and sizes 18
    /// characters. The two populations are DISJOINT: neither caller of
    /// `with_canonical_height` appears among the catalog rows that author a
    /// standing height. Two mechanisms, not two live truths for one fact.
    pub canonical_height: Option<f32>,
}

/// Compute the art-pixel to world-unit scale for an authored canonical height.
/// Returns `None` when the sheet reports no positive body height.
pub fn world_per_pixel_for_height(canonical_height: f32, sheet_pixel_height: f32) -> Option<f32> {
    (sheet_pixel_height > 0.0).then(|| canonical_height / sheet_pixel_height)
}

/// Authority for body collision geometry.
/// `SpriteAuthored` follows per-pose sheet geometry; `Explicit` is a spawn-time constant.
/// Runtime projections must not become a second live-body geometry authority.
#[derive(Debug, Clone, PartialEq)]
pub enum BodySource {
    /// The sheet authors it, per pose (`SpritePosedBody`).
    SpriteAuthored { world_per_pixel: f32 },
    /// Explicit authored half-extents.
    Explicit { half_extents: (f32, f32) },
}

/// One authored character. Control assignment belongs to session authority, not character identity.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterDefinition {
    pub id: ambition_entity_catalog::CharacterId,
    pub display_name: String,
    /// Attribution and asset namespace; not gameplay-rule authority.
    pub provider: String,
    pub lineage: Option<Lineage>,
    /// The sheet manifest target this character's art resolves through.
    pub sheet: Option<String>,
    /// Logical portrait target resolved independently of the full sheet.
    pub portrait: Option<String>,
    /// Lowest-precedence fallback voice lines.
    /// Yarn and catalog situation/fallback dialogue take precedence.
    pub voice: Vec<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    /// Body death and drop behavior. `None` publishes no character-specific override.
    /// Changing characters must retract any previously projected death traits.
    pub death_traits: Option<crate::actor::CharacterDeathTraits>,
    pub moveset: Option<MovesetContract>,
    /// Actions this character may choose, separate from the moveset that defines the moves.
    pub action_set: Option<crate::brain::ActionSet>,
    /// How this character MOVES — the state-free movement policy.
    ///
    /// `None` leaves the catalog row's movement policy in force.
    pub motion_model: Option<ambition_platformer2d_core::MotionModelSpec>,
    /// Per-character solver parameters. `None` leaves live/shared tuning authoritative.
    /// Presence of this field marks tuning as authored rather than inspector-controlled.
    pub movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    /// Body-authored ability baseline; controller kind never grants body capabilities.
    /// Match `MatchAbilities` may add a declared floor and apply a ceiling:
    /// `effective = (authored union granted) intersect permitted`.
    /// `None` contributes no character-authored abilities.
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// How this body moves under its own power — top speed, gait, surface
    /// cling. See [`crate::actor::CharacterLocomotion`].
    pub locomotion: Option<crate::actor::CharacterLocomotion>,
    /// Whether touching this body hurts, and how much. `None` = it does
    /// not, which is most characters.
    pub contact_damage: Option<crate::actor::ContactDamage>,
    /// Inline autonomous policy for this character.
    /// `None` leaves the catalog/archetype projection authoritative.
    pub autonomous_profile: Option<crate::brain::BrainProfile>,
    /// Provider-relative name of a shared autonomous policy resolved during preparation.
    /// Inline and named policies are mutually exclusive; an unresolved name is a preparation error.
    pub autonomous_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// Provider-relative autonomous policy adopted by the same body when provoked.
    /// `None` leaves the existing fallback behavior in charge.
    pub provoked_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// Cosmetic projectile id for this character. `None` uses projectile-authored presentation.
    pub ranged_vfx: Option<String>,
    /// Execution mode for ranged attacks; defaults to `MovesetVerb`.
    pub ranged_execution: crate::brain::RangedExecution,
    /// Whether this body is a practice target rather than a participant.
    #[doc(alias = "is_sandbag")]
    pub practice_target: bool,
    /// Weapon carried by this character.
    /// Held items do not grant verbs; [`Self::action_set`] states what the body can do.
    pub held_item: Option<String>,
    /// What this body can ride or be ridden as. `None` means neither.
    pub mount: Option<crate::actor::CharacterMount>,
    /// Presentation seed for deep-dream visual jitter. `None` excludes this character from the pass.
    pub dream_seed: Option<f32>,
    /// If true, equally configured CPU twins start from the same deterministic cognitive stream.
    /// This does not synchronize actions: differing observations must still produce divergence.
    /// The choice is made at construction, not per tick.
    pub preserves_mirror_symmetry: bool,
}

impl CharacterDefinition {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            id: ambition_entity_catalog::CharacterId::new(id),
            display_name: display_name.into(),
            provider: provider.into(),
            lineage: None,
            sheet: None,
            portrait: None,
            voice: Vec::new(),
            body: None,
            hurtboxes: None,
            vitals: Vitals::default(),
            death_traits: None,
            moveset: None,
            action_set: None,
            motion_model: None,
            movement_tuning: None,
            abilities: None,
            locomotion: None,
            contact_damage: None,
            autonomous_profile: None,
            autonomous_profile_ref: None,
            provoked_profile_ref: None,
            ranged_vfx: None,
            ranged_execution: crate::brain::RangedExecution::MovesetVerb,
            practice_target: false,
            held_item: None,
            mount: None,
            dream_seed: None,
            preserves_mirror_symmetry: false,
        }
    }

    /// Author the verbs this body has. See [`Self::abilities`].
    pub fn with_abilities(mut self, abilities: ambition_platformer2d_core::AbilitySet) -> Self {
        self.abilities = Some(abilities);
        self
    }

    /// Author how this body moves. See [`Self::locomotion`].
    pub fn with_locomotion(mut self, locomotion: crate::actor::CharacterLocomotion) -> Self {
        self.locomotion = Some(locomotion);
        self
    }

    /// Author what this character can ride and be ridden as. See
    /// [`Self::mount`].
    pub fn with_mount(mut self, mount: crate::actor::CharacterMount) -> Self {
        self.mount = Some(mount);
        self
    }

    /// Author this character's deep-dream seed. See [`Self::dream_seed`].
    pub fn with_dream_seed(mut self, seed: f32) -> Self {
        self.dream_seed = Some(seed);
        self
    }

    /// Use one initial cognitive stream for equally configured CPU twins.
    pub fn preserving_mirror_symmetry(mut self) -> Self {
        self.preserves_mirror_symmetry = true;
        self
    }

    /// Author this body as a training dummy. See [`Self::practice_target`].
    pub fn as_practice_target(mut self) -> Self {
        self.practice_target = true;
        self
    }

    /// Author the weapon this character carries. See [`Self::held_item`].
    pub fn with_held_item(mut self, id: impl Into<String>) -> Self {
        self.held_item = Some(id.into());
        self
    }

    /// Name a shared provider-relative policy by its local key.
    pub fn with_autonomous_profile_named(mut self, key: impl Into<String>) -> Self {
        self.autonomous_profile_ref = Some(crate::brain::BrainProfileRef::new(key));
        self
    }

    /// Name the policy this creature adopts when provoked. See
    /// [`Self::provoked_profile_ref`].
    pub fn with_provoked_profile_named(mut self, key: impl Into<String>) -> Self {
        self.provoked_profile_ref = Some(crate::brain::BrainProfileRef::new(key));
        self
    }

    /// Author how this character executes ranged attacks.
    pub fn with_ranged_execution(mut self, execution: crate::brain::RangedExecution) -> Self {
        self.ranged_execution = execution;
        self
    }

    /// Author this character's projectile presentation id.
    pub fn with_ranged_vfx(mut self, id: impl Into<String>) -> Self {
        self.ranged_vfx = Some(id.into());
        self
    }

    /// Author the policy this character runs by default. See
    /// [`Self::autonomous_profile`].
    pub fn with_autonomous_profile(mut self, profile: crate::brain::BrainProfile) -> Self {
        self.autonomous_profile = Some(profile);
        self
    }

    /// Author what touching this body costs. See [`Self::contact_damage`].
    pub fn with_contact_damage(mut self, contact: crate::actor::ContactDamage) -> Self {
        self.contact_damage = Some(contact);
        self
    }

    /// Author what this character does when it dies. See the field.
    pub fn with_death_traits(mut self, traits: crate::actor::CharacterDeathTraits) -> Self {
        self.death_traits = Some(traits);
        self
    }

    pub fn with_moveset(mut self, moveset: MovesetContract) -> Self {
        self.moveset = Some(moveset);
        self
    }

    /// Author this character's action set, outranking the catalog row.
    ///
    /// Passing `ActionSet::default()` is a real authoring decision — "this
    /// character reaches for nothing" — and is preserved as such. See the field.
    pub fn with_action_set(mut self, action_set: crate::brain::ActionSet) -> Self {
        self.action_set = Some(action_set);
        self
    }

    /// Author how this character moves, outranking the catalog row.
    pub fn with_motion_model(mut self, spec: ambition_platformer2d_core::MotionModelSpec) -> Self {
        self.motion_model = Some(spec);
        self
    }

    /// Author this character's movement feel, outranking the catalog row.
    pub fn with_movement_tuning(
        mut self,
        tuning: ambition_platformer2d_core::MovementTuning,
    ) -> Self {
        self.movement_tuning = Some(tuning);
        self
    }

    pub fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    /// Name this character's portrait target. This is a logical target name, not
    /// an asset path; asset resolution derives the concrete path. Omitting it
    /// preserves the catalog-provided portrait choice.
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.portrait = Some(portrait.into());
        self
    }

    /// Use sheet-authored body geometry at one `world_per_pixel` scale.
    pub fn with_sprite_authored_body(mut self, world_per_pixel: f32) -> Self {
        self.body = Some(BodySource::SpriteAuthored { world_per_pixel });
        self
    }

    /// Author standing height in world pixels; sheet-aware callers derive art scale from it.
    pub fn with_canonical_height(mut self, height: f32) -> Self {
        self.vitals.canonical_height = Some(height);
        self
    }

    pub fn with_hurtboxes(mut self, doc: HurtboxDoc) -> Self {
        self.hurtboxes = Some(doc);
        self
    }

    /// Give this character a voice: lines it says when nothing more specific
    /// does. See [`Self::voice`].
    pub fn with_voice<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.voice = lines.into_iter().map(Into::into).collect();
        self
    }
}

#[cfg(test)]
mod authority_tests {
    use super::*;

    /// A character definition may state only body-owned facts. A default
    /// controller is allowed as replaceable policy; changing the controller must
    /// not change body identity.
    #[allow(dead_code)]
    fn a_character_states_only_what_a_body_may_state(definition: &CharacterDefinition) {
        let CharacterDefinition {
            // ── IDENTITY & PRESENTATION BINDING (7) ─────────────────────────
            id: _,
            display_name: _,
            provider: _,
            lineage: _,
            sheet: _,
            portrait: _,
            voice: _,

            // ── BODY (15) — what this creature IS ───────────────────────────
            body: _,
            hurtboxes: _,
            vitals: _,
            death_traits: _,
            moveset: _,
            action_set: _,
            motion_model: _,
            movement_tuning: _,
            abilities: _,
            locomotion: _,
            contact_damage: _,
            held_item: _,
            mount: _,
            practice_target: _,
            ranged_execution: _,

            // ── DEFAULT CONTROLLER (4) — see the  above ────────────────────
            //
            // A policy this character COMES WITH, by name or inline. Not the
            // controller itself, and never a reason for a body fact to live in
            // a profile or the reverse.
            autonomous_profile: _,
            autonomous_profile_ref: _,
            provoked_profile_ref: _,
            //  filed HERE and not under BODY, and the group's own  is the
            // reason: it states something about this character's AUTONOMOUS
            // drivers — two of them share one deterministic cognitive stream —
            // and it says nothing about the body. It passes the group's test
            // exactly: changing the controller does not change the body. Put
            // a person on the sticks and this field means nothing at all.
            //
            //  it is not on `BrainProfile` because a profile is reusable across
            // characters, and this is one character's identity rather than a
            // difficulty rung's. See [`Self::preserves_mirror_symmetry`].
            preserves_mirror_symmetry: _,

            // ── PRESENTATION PROJECTED FROM THE BODY (2) ────────────────────
            //
            // The same pair that rides in `ActorTuning` and `ArchetypeSpec`.
            // Presentation observes a body; it is not a fourth authority.
            ranged_vfx: _,
            dream_seed: _,
        } = definition;
    }
}
