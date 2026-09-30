//! Authored character data before runtime preparation.
//! Kit resolution remains runtime work; this module contains body-owned authored facts.

use ambition_entity_catalog::{HurtboxDoc, MovesetContract};

/// Non-authoritative reproducibility metadata for generated variants.
/// Derived characters have independent stable ids; lineage is not inheritance or balance policy.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
pub struct Lineage {
    pub derived_from: Option<String>,
    pub generator_revision: Option<String>,
    pub source_fingerprint: Option<String>,
}

/// Default health when no character-authored health pool is provided.
pub const DEFAULT_UNAUTHORED_BODY_HEALTH: i32 = 4;

/// Optional authored physical limits; `None` leaves construction-time state authoritative.
/// A character's OWN autonomous policy: written out here, or named for lookup.
///
/// ⛔⛔ **AN ENUM RATHER THAN TWO `Option`s, AND THAT IS THE WHOLE POINT.** The
/// contract has always been *"inline and named are mutually exclusive"*. Two
/// optional fields can spell FOUR combinations; the contract wants THREE, and
/// the fourth was refused by a `panic!` inside `resolve_autonomous_profile`
/// whose own message read *"those do not merge — one would silently replace the
/// other — so authoring both is refused"*. That is a correct rule enforced at
/// the wrong layer: a crash, at preparation, in a shipped build, for a state the
/// type handed the author. This spells three and no more.
///
/// ⚠ AUTHORING BOTH IS NOW "CALLING TWO SETTERS", AND THE LAST ONE WINS — the
/// same as every other `with_*` on [`CharacterDefinition`]. Nothing is silently
/// MERGED, which is what the panic actually guarded against; a builder's last
/// call winning is ordinary and visible at the call site.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum AutonomousPolicy {
    /// Written out on this character.
    Inline(crate::brain::BrainProfile),
    /// Provider-relative name of a shared policy, RESOLVED during preparation.
    /// An unresolved name is a preparation error, not an absence: resolving it
    /// to nothing would leave the body on its archetype while the definition
    /// says otherwise.
    Named(crate::brain::BrainProfileRef),
}

impl AutonomousPolicy {
    /// The profile written out here, or `None` when this policy is a NAME.
    ///
    /// ⚠ `None` FROM A `Named` IS NOT "NO POLICY" — the policy exists and lives
    /// in the provider's registry. A caller asking "what does this character
    /// actually do" wants the resolved value on
    /// `PreparedCharacter::autonomous_profile`, after preparation; this is for
    /// callers reading the AUTHORED form.
    pub fn inline(&self) -> Option<&crate::brain::BrainProfile> {
        match self {
            Self::Inline(profile) => Some(profile),
            Self::Named(_) => None,
        }
    }

    /// The shared policy this character names, or `None` when it is inline.
    pub fn named(&self) -> Option<&crate::brain::BrainProfileRef> {
        match self {
            Self::Named(reference) => Some(reference),
            Self::Inline(_) => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
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
}

/// Authority for body collision geometry.
/// `SpriteAuthored` follows per-pose sheet geometry; `Explicit` is a spawn-time constant.
/// Runtime projections must not become a second live-body geometry authority.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum BodySource {
    /// The sheet authors it, per pose (`SpritePosedBody`).
    SpriteAuthored { world_per_pixel: f32 },
    /// Explicit authored half-extents.
    Explicit { half_extents: (f32, f32) },
}

/// One authored character. Control assignment belongs to session authority, not character identity.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
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
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    /// This body's semantic rig: joints, attachment points and hurt parts, in
    /// world units. `None` means the body has no articulated geometry, which is
    /// every character that has not published one. See
    /// [`crate::actor::body_rig`]. Not serialized when absent, so a definition
    /// with no rig renders exactly as it did before rigs existed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_rig: Option<crate::actor::BodyRigDefinition>,
    pub vitals: Vitals,
    /// Body death and drop behavior. `None` publishes no character-specific override.
    /// Changing characters must retract any previously projected death traits.
    pub death_traits: Option<crate::actor::CharacterDeathTraits>,
    pub moveset: Option<MovesetContract>,
    /// The damage its moves deal on a platform-fighter stage, from its
    /// `smash_fighter` facet. Empty: every move keeps its moveset damage there.
    /// Only a match that declares
    /// [`MoveDamageSource::SmashFighterFacet`](crate::smash_fighter::MoveDamageSource)
    /// reads it. See [`crate::smash_fighter::SmashFighterFacet::move_damage`].
    pub fighter_move_damage: crate::smash_fighter::MoveDamage,
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
    /// This character's own autonomous policy, inline or by name.
    ///
    /// `None` leaves the catalog/archetype projection authoritative.
    ///
    /// ⛔⛤ **THIS WAS TWO `Option` FIELDS AND A `panic!`.** `autonomous_profile`
    /// and `autonomous_profile_ref` were separate, the doc said *"inline and
    /// named policies are mutually exclusive"*, and the rule was enforced by
    /// `resolve_autonomous_profile` panicking at preparation on
    /// `(Some(_), Some(_))` — a CRASH in a shipped game for a state the type
    /// invited. Two optionals can spell four combinations and the contract wants
    /// three. ⇒ One `Option<AutonomousPolicy>` spells exactly three, so the
    /// refusal has nothing left to refuse.
    pub autonomous_policy: Option<AutonomousPolicy>,
    /// Provider-relative autonomous policy adopted by the same body when provoked.
    /// `None` leaves the existing fallback behavior in charge.
    pub provoked_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// Cosmetic projectile id for this character. `None` uses projectile-authored presentation.
    pub ranged_vfx: Option<String>,
    /// Execution mode for ranged attacks. `None`: the catalog row's, else
    /// `MovesetVerb`.
    pub ranged_execution: Option<crate::brain::RangedExecution>,
    /// Whether this body is a practice target rather than a participant.
    #[doc(alias = "is_sandbag")]
    pub practice_target: bool,
    /// What wearing this character grants the body, for as long as it wears
    /// it: a super form that cannot be hurt and flattens what it touches.
    /// Joined with the catalog row's at preparation.
    pub empowered: crate::actor::Empowerment,
    /// This body's positive wallet balance absorbs one hit
    /// ([`BodyWalletShield`](crate::actor::BodyWalletShield)) where its game's
    /// rules allow it: Sanic's rings. OR-ed with the catalog row's.
    pub wallet_shield: bool,
    /// Weapon carried by this character.
    /// Held items do not grant verbs; [`Self::action_set`] states what the body can do.
    pub held_item: Option<String>,
    /// What this body can ride or be ridden as. `None` means neither.
    pub mount: Option<crate::actor::CharacterMount>,
    /// This body's two hands, each built from a character of its own. `None`
    /// means it has none.
    pub hands: Option<crate::actor::CharacterHands>,
    /// This body has no left/right variant: turning never mirrors its art, its
    /// hurtboxes or its moves (`ambition_platformer2d_core::Unmirrored`). It is
    /// a fact about the art, so it is true of every placement of the character.
    /// OR-ed with the catalog row's.
    pub unmirrored: bool,
    /// Game components every body of this character carries from the batch
    /// that builds it, at their default values. For state the engine cannot
    /// name, such as a shell phase. See
    /// [`ambition_platformer2d_core::CarriedComponent`].
    pub carries: Vec<ambition_platformer2d_core::CarriedComponent>,
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
            body: None,
            hurtboxes: None,
            body_rig: None,
            vitals: Vitals::default(),
            death_traits: None,
            moveset: None,
            fighter_move_damage: crate::smash_fighter::MoveDamage::new(),
            action_set: None,
            motion_model: None,
            movement_tuning: None,
            abilities: None,
            locomotion: None,
            contact_damage: None,
            autonomous_policy: None,
            provoked_profile_ref: None,
            ranged_vfx: None,
            ranged_execution: None,
            practice_target: false,
            empowered: crate::actor::Empowerment::none(),
            wallet_shield: false,
            held_item: None,
            mount: None,
            hands: None,
            unmirrored: false,
            carries: Vec::new(),
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
    /// Every body of this character carries `C` from the batch that builds
    /// it. See [`Self::carries`].
    pub fn carrying(mut self, carried: ambition_platformer2d_core::CarriedComponent) -> Self {
        if !self.carries.contains(&carried) {
            self.carries.push(carried);
        }
        self
    }

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

    /// Author what wearing this character grants. See [`Self::empowered`].
    pub fn empowered(mut self, traits: crate::actor::Empowerment) -> Self {
        self.empowered = traits;
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
        self.autonomous_policy = Some(AutonomousPolicy::Named(crate::brain::BrainProfileRef::new(
            key,
        )));
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
        self.ranged_execution = Some(execution);
        self
    }

    /// Author this character's projectile presentation id.
    pub fn with_ranged_vfx(mut self, id: impl Into<String>) -> Self {
        self.ranged_vfx = Some(id.into());
        self
    }

    /// Author the policy this character runs by default. See
    /// [`Self::autonomous_policy`], which is the field it writes.
    pub fn with_autonomous_profile(mut self, profile: crate::brain::BrainProfile) -> Self {
        self.autonomous_policy = Some(AutonomousPolicy::Inline(profile));
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

    pub fn with_hurtboxes(mut self, doc: HurtboxDoc) -> Self {
        self.hurtboxes = Some(doc);
        self
    }

    /// Author this body's semantic rig. See [`Self::body_rig`].
    pub fn with_body_rig(mut self, rig: crate::actor::BodyRigDefinition) -> Self {
        self.body_rig = Some(rig);
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
            // ── IDENTITY & PRESENTATION BINDING (6) ─────────────────────────
            id: _,
            display_name: _,
            provider: _,
            lineage: _,
            sheet: _,
            portrait: _,

            // ── BODY (21) — what this creature IS ───────────────────────────
            body: _,
            body_rig: _,
            unmirrored: _,
            carries: _,
            hands: _,
            hurtboxes: _,
            vitals: _,
            death_traits: _,
            moveset: _,
            fighter_move_damage: _,
            action_set: _,
            motion_model: _,
            movement_tuning: _,
            abilities: _,
            locomotion: _,
            contact_damage: _,
            held_item: _,
            mount: _,
            practice_target: _,
            empowered: _,
            wallet_shield: _,
            ranged_execution: _,

            // ── DEFAULT CONTROLLER (3) — see the  above ────────────────────
            //
            // A policy this character COMES WITH, by name or inline. Not the
            // controller itself, and never a reason for a body fact to live in
            // a profile or the reverse.
            autonomous_policy: _,
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
