//! THE AUTHORED CHARACTER — what content writes down, before anything
//! resolves it.
//!
//! Item 1 settled where the cut goes and the file settled it, not a preference:
//! `derive_moveset` — the single reach into `ambition_combat` — is a private
//! preparation function rather than a method on this type, and resolving a kit
//! is runtime work. So the AUTHORED half moves and
//! `PreparedCharacterDefinition` stays above.
//!
//! this half already had no `crate::` references at all, which is how a 600-line move became a
//! cut rather than a refactor.

use ambition_entity_catalog::{HurtboxDoc, MovesetContract};

/// Non-authoritative provenance for a generated crossover variant (§4.3).
///
/// `mary_o` and `mary_o_smash` are two independent, fully-resolved products with
/// distinct stable ids, emitted by one generator from shared source. The engine
/// never learns what a mode is — there is no patch layer and no override
/// precedence — and it must not interpret any of this as a balance layer. It
/// exists so a derived character is reproducible.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lineage {
    pub derived_from: Option<String>,
    pub generator_revision: Option<String>,
    pub source_fingerprint: Option<String>,
}

/// Physical limits and vitals. Optional fields distinguish "not authored" from
/// an explicit value, allowing body construction defaults to remain authoritative
/// when the character definition does not override them.
///
/// Default health for a body with no authored health authority. Provocation does
/// not replace the body's health pool.
pub const DEFAULT_UNAUTHORED_BODY_HEALTH: i32 = 4;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vitals {
    /// How much punishment this character's body takes. `None` leaves whatever
    /// pool the body's construction established — for a playable body that is
    /// the host's standard pool.
    ///
    /// The definition-side analogue of the catalog row's `max_health`, which has
    /// carried exactly this `Option` meaning since it was added; finalization
    /// folds the two, definition first.
    pub max_health: Option<i32>,
    /// Reaches the body as `Mass`, which drives the mount pair's mass-weighted
    /// center of gravity. `None` preserves the body's existing mass; `Some(1.0)`
    /// is an explicit authored override even though `1.0` is the ambient default.
    pub mass: Option<f32>,
    /// How hard this body is to LAUNCH — the knockback weight, reaching a
    /// body as `CombatTuning::weight` (`ambition_combat`). `1.0` is
    /// the reference body; a heavy fighter authors more and takes less of the
    /// growth term (`scaled_knockback` divides by it).
    ///
    ///  distinct from [`Self::mass`], which is the mount pair's centre of
    /// gravity. They are the same word in physics and two different mechanics
    /// here, and conflating them would make a heavy mount hard to knock about
    /// as a side effect of how its rider orbits it.
    ///
    /// `None` leaves the body's own — for a clustered actor that is its roster archetype's,
    /// which is the ONLY place a weight could be stated until now.
    pub knockback_weight: Option<f32>,
    /// Standing height in world pixels, used to scale sprite-authored geometry
    /// consistently across characters. `None` preserves the construction-time
    /// size. `collision_scale` remains independent crop/footprint compensation.
    pub canonical_height: Option<f32>,
}

/// Compute the art-pixel to world-unit scale for an authored canonical height.
/// Returns `None` when the sheet reports no positive body height.
pub fn world_per_pixel_for_height(canonical_height: f32, sheet_pixel_height: f32) -> Option<f32> {
    (sheet_pixel_height > 0.0).then(|| canonical_height / sheet_pixel_height)
}

/// Where a body's collision geometry comes from (§4.11, §5).
///
/// * `SpriteAuthored { world_per_pixel }` becomes a
///   [`SpritePosedBody`](ambition_sprite_sheet::character::sheets::SpritePosedBody), installed by
///   `project_prepared_character_definitions` and retracted with the rest of that
///   system's grants. From there the existing per-tick sync derives the collision
///   box, the sprite quad and its offset off the art, so a body that changes
///   SHAPE between poses needs no bespoke per-state boxes.
/// * `Explicit { half_extents }` is a SPAWN-time size, consumed by seating in
///   place of its `SEAT_BODY_PX` placeholder.
///
///  the split is not arbitrary. A projection that resized a LIVE body would be
/// a second geometry authority beside the transit seam (ADR 0024); a spawn-time
/// constant cannot express a silhouette that changes with the pose. Each variant
/// goes to the authority that can actually honour it.
#[derive(Debug, Clone, PartialEq)]
pub enum BodySource {
    /// The sheet authors it, per pose (`SpritePosedBody`).
    SpriteAuthored { world_per_pixel: f32 },
    /// Explicit authored half-extents.
    Explicit { half_extents: (f32, f32) },
}

/// One authored character. Sections may be inline or referenced.
///
/// Note what is NOT here: `default_brain` (§4.7 — control assignment is a session
/// binding on a participant, not an identity trait) and any hand-listed cue
/// vocabulary (§4.6 — derived).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterDefinition {
    pub id: ambition_entity_catalog::CharacterId,
    pub display_name: String,
    /// Attribution and asset roots. NOT authority: a provider does not own the
    /// right to reinterpret engine rules for its characters.
    pub provider: String,
    pub lineage: Option<Lineage>,
    /// The sheet manifest target this character's art resolves through.
    pub sheet: Option<String>,
    /// Select-screen portrait. Loads WITHOUT the sheet, so an enumeration screen
    /// costs no sheet decode.
    ///
    /// This is an unresolved portrait target name. Either resolve it through a
    /// dedicated target resolver or let the catalog own concrete portrait paths;
    /// do not duplicate concrete art paths here.
    pub portrait: Option<String>,
    /// Lines this character says when nothing more specific does.
    ///
    /// A newly registered character has no dialogue graph, no situation pools
    /// and usually no writer — and standing on a pedestal saying nothing is
    /// worse than saying something generic in its own voice. The catalog has
    /// expressed exactly this for a while, on `fallback_dialogue`: *"a character
    /// arrives from the sprite pipeline with a voice long before anyone writes
    /// four separate pools for it … the fallback exists so a newly authored
    /// character is never mute."*
    ///
    /// It could only say that about a character with a CATALOG ROW. A
    /// registered-only character — which is every character another game brings
    /// — had no way to carry a voice at all, so four of them stand mute on Hall
    /// pedestals.
    ///
    ///  the LOWEST-precedence voice, not a dialogue system: a yarn node wins,
    /// then the catalog row's situation pool, then its `fallback_dialogue`, then
    /// this. It exists so the floor is "says something in character" rather than
    /// silence.
    pub voice: Vec<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    /// What this body does when it DIES, and what it drops — explode,
    /// divide, crash, or refuse to die at all.
    ///
    /// These are properties of the creature, and until now the ONLY producer of
    /// `CombatCapabilities` (`ambition_combat`) in the workspace was
    /// `ArchetypeSpecExt:combat_capabilities` — so a mite that splits when killed could say so
    /// as an archetype and a registered character could not say it at all.
    ///
    /// `None` means the author said nothing, and nothing is inserted — today's
    /// behaviour for every character in the repo.  absence RETRACTS on a
    /// re-wear, like every other physical fact a persona claims: wearing a
    /// sandbag and then a duelist must not leave the duelist unkillable.
    pub death_traits: Option<crate::actor::CharacterDeathTraits>,
    pub moveset: Option<MovesetContract>,
    /// What this character CAN do — melee, ranged, special, locomotion style.
    ///
    /// Splitting this from [`Self::moveset`] is the identity split C3 is about.
    /// A moveset says what the MOVES are; an action set says what the body and
    /// the AI believe the body can reach for. Leaving the second exclusively in
    /// the catalog means a definition can author moves the brain does not know
    /// exist — and it makes a ranged move depend on a projectile specification
    /// from a different authority entirely.
    pub action_set: Option<crate::brain::ActionSet>,
    /// How this character MOVES — the state-free movement policy.
    ///
    /// `None` leaves the catalog row's movement policy in force.
    pub motion_model: Option<ambition_platformer2d_core::MotionModelSpec>,
    /// Per-character axis FEEL — run accel, jump speed, coyote time, the rest.
    ///
    /// Distinct from [`Self::motion_model`], which picks the SOLVER. This is that
    /// solver's numbers, and it is the last kit-adjacent field that was still
    /// exclusively the catalog's.
    ///
    ///  `None` and `Some` are load-bearing here in a way they are not elsewhere:
    /// the marker component's PRESENCE means "this body's tuning is authored,
    /// not the shared dev tuning", so an unauthored character must end up with no
    /// marker rather than with a defaulted one. A re-wear from an authored feel
    /// back to the sandbox protagonist has to return the body to the live
    /// inspector sliders.
    pub movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    /// The verbs this BODY has — jump, double jump, dash, dodge, shield,
    /// ledge grab, blink, fly, glide, swim.
    ///
    ///  a capability is the character's, never the controller's and never
    /// the ruleset's. The archetype has always been able to state a movement
    /// kit (`ArchetypeSpecExt::movement_kit`, four flags); a registered
    /// character could not state one at all, which is why a match seat had to be
    /// handed a flat set by the MATCH — *"every fighter in this match has the
    /// same verbs"* — and why the Smash demo's fighters do not use the shield,
    /// dodge and ledge machinery that already exists underneath them. Nothing
    /// had granted them the capability, because nothing could.
    ///
    ///  `None` means the author said nothing, and the migration bridge
    /// stands: a seat whose character authors no verbs still takes the match's
    /// declared set, exactly as today. That bridge is what a character authoring
    /// its own kit removes, one character at a time.
    ///
    ///  a ruleset may only take verbs away. `AbilitySet::intersect` is the
    /// operation a mode is allowed: Smash may say *"no flying in this match"*
    /// and may not say *"everyone can jump"*, because forcing a jump onto a body
    /// that cannot jump is the engine manufacturing a capability — the exact
    /// thing that makes Puppy Slug in a fighter seat indistinguishable from a
    /// generic humanoid.
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// How this body moves under its own power — top speed, gait, surface
    /// cling. See [`crate::actor::CharacterLocomotion`].
    pub locomotion: Option<crate::actor::CharacterLocomotion>,
    /// Whether touching this body hurts, and how much. `None` = it does
    /// not, which is most characters.
    pub contact_damage: Option<crate::actor::ContactDamage>,
    /// The POLICY this character runs when nothing else drives it — the
    /// controller authority, carried as a value rather than as a name.
    ///
    /// and it is the ONLY half now. The convergence turned out to be a deletion: the preset
    /// half had zero authors in the entire repo and one consumer, and its absence was what
    /// produced the empty-string default that crashed two shipped rooms. A character states its
    /// policy HERE, or it leaves the catalog row in charge; there is no third place.
    ///
    /// What is gone is a definition being able to say the same thing in the row's words.
    ///
    /// `None` leaves the archetype's projection in charge, which is every
    /// character that has not migrated.
    pub autonomous_profile: Option<crate::brain::BrainProfile>,
    /// The SHARED policy this character names, resolved out of the catalog's
    /// `autonomous_profiles` map at preparation into [`Self::autonomous_profile`].
    ///
    /// Carrying a profile by value says what ONE character does; naming one says several characters
    /// fight alike — which is the whole reason `medium_striker` exists as a whole-body archetype
    /// worn by five goblins, a lab raider and a skitter. A named profile lets those five keep their
    /// own bodies and share the decision-making, which is the Group-B/Group-C split.
    ///
    /// INLINE XOR NAMED, and authoring both is a REFUSAL. Documenting replacement as
    /// specialization is misleading API on the day it ships; if a real patch is ever wanted it gets
    /// a real `BrainProfilePatch` with explicit semantics.
    ///
    ///  and a name nobody authored is a PREPARATION FAILURE, not a silent
    /// `None`. Falling back would reproduce the explicit-`CharacterId` mistake
    /// one layer down: the author said which policy this creature uses, the
    /// lookup missed, and the archetype quietly stayed in charge — green
    /// everywhere, wrong in play.
    pub autonomous_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// The policy this creature adopts when PROVOKED, by provider-relative
    /// name.
    ///
    ///  provocation picks an enemy ARCHETYPE by substring-matching a display
    /// name today (`hostile_brain_id_for_actor`: *does the id or the name or
    /// the dialogue node contain "pirate"*). That is the fused ontology at its
    /// most literal — a peaceful pirate that gets struck is handed a different
    /// BODY, not a different attitude — and it is the only thing keeping three
    /// archetype rows alive that no level places.
    ///
    ///  what provocation actually is: the same body, a different driver, and a
    /// changed relationship. This is the driver half, stated by the creature
    /// that has one.
    ///
    /// `None` = this character has nothing to say about being provoked, which
    /// leaves the legacy name-match in charge — every character today.
    pub provoked_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// What this character's PROJECTILE looks like — the cosmetic id its
    /// ranged verb spawns (`"hadouken"`).
    ///
    /// `None` = this character's ranged verb draws whatever the projectile
    /// itself authors, which is every character that has never had one.
    pub ranged_vfx: Option<String>,
    /// HOW this character's ranged attack is executed — a charged projectile
    /// (hold to build, release to fire) or an ordinary moveset verb.
    ///
    /// It was derived from `PlayableKitSource::HostCode`, which made a gameplay property of the
    /// protagonist's attack look like a property of which crate built it — and so made *delete
    /// HostCode* read as *delete the charge*.
    ///
    ///  the DEFAULT is `MovesetVerb`, which is what every character that has
    /// never had a charge already does.
    pub ranged_execution: crate::brain::RangedExecution,
    /// This body is a PRACTICE TARGET — a training dummy, not a
    /// participant.
    ///
    ///  on the definition, not read off a catalog tag. The plane-swarm
    /// lesson: a body that reads an intrinsic from a catalog row it cannot see
    /// gets the wrong answer in a standalone demo that borrowed the character.
    #[doc(alias = "is_sandbag")]
    pub practice_target: bool,
    /// The weapon this character carries, by id, resolved through the same
    /// held-item registry the archetype's `held_item` uses.
    ///
    ///  a fact about the creature, not about the placement. A cove raider
    /// carries a gun-sword wherever it stands; the item is what it drops when it
    /// dies and what its swing looks like. It was reachable only through an
    /// archetype row, so a migrated raider lost its weapon — which is most of
    /// what a raider IS.
    ///
    ///  it grants no VERBS here. The archetype path folds a held item's
    /// melee/ranged into the resolved `ActionSet`; a character authors its verbs
    /// on [`Self::action_set`] directly, so this states what the body HOLDS and
    /// the action set states what it DOES. Authoring an item and forgetting the
    /// verb gives a body a weapon it never swings — visible, rather than a
    /// silently different creature.
    pub held_item: Option<String>,
    /// What this body can be RIDDEN as, and what it can ride (ADR 0020).
    /// `None` = neither. See
    /// [`crate::actor::CharacterMount`].
    pub mount: Option<crate::actor::CharacterMount>,
    /// Deep-dream visual jitter seed — this character's participation in the
    /// psychedelic shader pass, and how it differs from its neighbours.
    ///
    ///  presentation, and on the definition for the same reason the sheet
    /// is: it is a fact about what this creature LOOKS like, true of every
    /// instance, and it was reachable only through an enemy archetype row. The
    /// puppy slug is the live case — `dream_seed: Some(0.271828)` is the only
    /// thing between a migrated slug and the psychedelic pass it has always had.
    ///
    /// `None` = does not participate, which is nearly everything.
    pub dream_seed: Option<f32>,
    /// Two of this character, told the same things, think the same thoughts —
    /// so a mirror match plays as a reflection.
    ///
    ///  an authored TRAIT, not the default CPU policy. Ordinarily an
    /// autonomous participant's deterministic decision/noise stream is derived
    /// from WHICH PARTICIPANT it is, so two CPUs wearing one character diverge
    /// within a few decisions — that is what a viewer expects of two opponents.
    /// A character that authors this asks for the opposite: every equally
    /// configured twin begins on the SAME cognitive stream.
    ///
    ///  it does NOT synchronise their actions, and must never be
    /// implemented that way. The property is *identical cognition + symmetric
    /// information → symmetric behaviour*, which is an emergent consequence of
    /// sharing one stream, not a canned mirror animation. The moment two of them
    /// see different worlds — different damage, different position, a different
    /// foe — they decide differently, and that is correct. A mirror that survived
    /// asymmetric observations would be a puppet show.
    ///
    ///  so the only thing this authorises is the CHOICE OF STREAM, made once at
    /// construction. Nothing reads it per tick, and nothing compares two bodies.
    ///
    /// `false` — the default and nearly everything — means this character's CPUs
    /// think for themselves.
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

    /// Author this character's CPU twins onto one cognitive stream, so a mirror
    /// match plays as a reflection. See [`Self::preserves_mirror_symmetry`] —
    /// especially the paragraph on what this deliberately does NOT do.
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

    /// Name a SHARED policy, PROVIDER-RELATIVE. See
    /// [`Self::autonomous_profile_ref`].
    ///
    ///  pass the LOCAL name (`medium_striker`). Whether the assembled catalog
    /// has namespaced its fragments is not something an author should have to
    /// know, and the one who guesses wrong gets a silent miss.
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

    /// Author what this character's projectile looks like. See
    /// [`Self::ranged_vfx`].
    /// See [`Self::ranged_execution`]. A character that charges says so here.
    pub fn with_ranged_execution(mut self, execution: crate::brain::RangedExecution) -> Self {
        self.ranged_execution = execution;
        self
    }

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

    /// Hand this character's body geometry to its spritesheet.
    ///
    /// `world_per_pixel` is the ONE number: how much world one sheet pixel
    /// covers. The collision box, the sprite quad and the quad's offset all
    /// follow from the art at that scale, so none of the three can drift from
    /// the other two. See [`BodySource`].
    pub fn with_sprite_authored_body(mut self, world_per_pixel: f32) -> Self {
        self.body = Some(BodySource::SpriteAuthored { world_per_pixel });
        self
    }

    /// State how tall this character stands, in world pixels — 16 to a tile.
    /// See [`Vitals::canonical_height`] for what the unit is and why it is a
    /// contract rather than a hint.
    ///
    ///  this states the FACT; deriving the art scale from it is
    /// [`world_per_pixel_for_height`], because only the caller holding the sheet
    /// knows how tall the body is in that sheet's own pixels.
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
