//! Character registration and preparation.
//!
//! [`register_character`] accepts a decomposable authored [`CharacterDefinition`].
//! Preparation validates references and derives partial overrides; finalization folds in
//! catalog defaults once to produce a complete immutable runtime definition. Runtime
//! gameplay therefore does not re-run inheritance or string searches. Derived cue/art
//! dependencies come from the definition rather than parallel hand-maintained lists.

use std::collections::{BTreeMap, BTreeSet};

use crate::actor::definition::{BodySource, CharacterDefinition, Lineage, Vitals};
use ambition_binding::{BindingLedger, BindingReport, Namespace, Resolver};
use ambition_entity_catalog::{HurtboxDoc, MoveEventKind, MovesetContract};

pub use crate::binding_namespaces::{
    MoveId, PortraitTarget, RangedPayload, SfxCueId, SheetTarget, VerbId, VfxTag,
};

/// What one authored definition OVERRIDES, before the catalog is folded in.
///
/// The output of [`prepare_character`], and the input to finalization — never a
/// runtime value. Every kit field here is an `Option` whose `None` means *the
/// author said nothing*, which is a question and not an answer: the body cannot
/// act on it without also consulting the catalog row.
///
/// # This type is deliberately unnameable outside this module
///
/// Not `pub`, not `pub(crate)`, and that visibility is the entire mechanism. A seated fighter and a
/// worn player wearing the same character disagreed about that character's kit for a day.
#[derive(Debug, Clone, PartialEq)]
struct PreparedCharacterOverrides {
    id: String,
    display_name: String,
    provider: String,
    lineage: Option<Lineage>,
    sheet: Option<String>,
    portrait: Option<String>,
    /// The authored voice, carried through preparation unchanged.
    voice: Vec<String>,
    body: Option<BodySource>,
    hurtboxes: Option<HurtboxDoc>,
    vitals: Vitals,
    /// See [`CharacterDefinition::death_traits`]. No catalog counterpart
    /// exists to fold against, so it carries straight through.
    death_traits: Option<crate::actor::CharacterDeathTraits>,
    /// See [`CharacterDefinition::abilities`]. No catalog counterpart exists —
    /// a catalog row has never been able to state a body's verbs — so it
    /// carries straight through.
    abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// See [`CharacterDefinition::locomotion`]. Carried; no catalog counterpart.
    locomotion: Option<crate::actor::CharacterLocomotion>,
    /// See [`CharacterDefinition::contact_damage`]. Carried; no counterpart.
    contact_damage: Option<crate::actor::ContactDamage>,
    /// See [`CharacterDefinition::autonomous_profile`]. Carried.
    autonomous_profile: Option<crate::brain::BrainProfile>,
    /// See [`CharacterDefinition::autonomous_profile_ref`]. RESOLVED at
    /// preparation, so nothing downstream ever sees the name.
    autonomous_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// See [`CharacterDefinition::ranged_vfx`]. Carried.
    ranged_vfx: Option<String>,
    /// See [`CharacterDefinition::ranged_execution`]. Carried.
    ranged_execution: crate::brain::RangedExecution,
    /// See [`CharacterDefinition::provoked_profile_ref`]. RESOLVED at finalize.
    provoked_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// See [`CharacterDefinition::practice_target`]. Carried.
    practice_target: bool,
    /// See [`CharacterDefinition::held_item`]. Carried.
    held_item: Option<String>,
    /// See [`CharacterDefinition::dream_seed`]. Carried.
    dream_seed: Option<f32>,
    /// See [`CharacterDefinition::preserves_mirror_symmetry`]. Carried.
    preserves_mirror_symmetry: bool,
    /// See [`CharacterDefinition::mount`]. Carried.
    mount: Option<crate::actor::CharacterMount>,
    moveset: Option<MovesetContract>,
    /// The authored action set, carried through preparation unchanged.
    ///
    /// `None` and `Some(empty)` mean different things all the way to the body —
    /// see [`CharacterDefinition::action_set`].
    action_set: Option<crate::brain::ActionSet>,
    /// The authored movement policy, carried through preparation unchanged.
    /// `None` leaves the catalog row in charge.
    motion_model: Option<ambition_platformer2d_core::MotionModelSpec>,
    /// The authored movement feel, carried through preparation unchanged.
    movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    /// DERIVED (§4.6): every cue this character can emit, read off its moves.
    /// Sorted, so two peers assemble byte-identical inventories.
    cue_dependencies: BTreeSet<String>,
    /// DERIVED: every vfx tag its moves request.
    vfx_dependencies: BTreeSet<String>,
    /// Namespaces preparation actually RESOLVED, carried on the published value.
    ///
    /// This lived only on the transient [`PreparedCharacter`] and was dropped on
    /// the floor by registration, so once a character was published there was no
    /// way to ask whether its cues had been checked against a real vocabulary or
    /// whether nobody had looked. Those must never read the same — that confusion
    /// is the entire reason the binding boundary exists — and a distinction that
    /// survives only until the value is stored is not a distinction.
    checked: Vec<&'static str>,
    /// References a resolver was supplied for and REJECTED, formatted for a reader.
    ///
    /// [`Self::checked`] says a vocabulary was consulted; it says nothing about the
    /// verdict, so a typo'd sheet target read as "verified" — which is how four
    /// shipped characters declared `<name>_spritesheet` (the sheet FILE) instead of
    /// the sheet TARGET and drew placeholders while every check stayed green.
    /// Registration still publishes (a placeholder beats a session that refuses to
    /// boot); carrying the failures onto the published value is what lets a guard
    /// be red about them without making the runtime fatal.
    unresolved: Vec<String>,
}

/// Where a prepared character's fighting kit comes from.
///
/// The one honest answer to "what does this character reach for", decided ONCE
/// at finalization instead of re-decided by each construction path.
///
/// Two variants and not one, because exactly one case is genuinely undecidable before a body
/// exists: the host's code-side protagonist kit is built from that body's own persisted
/// `AbilitySet`, so no per-character value can hold it.
#[derive(Debug, Clone, PartialEq)]
pub enum PreparedKit {
    /// Content decided: this action set, these moves.
    ///
    /// The moveset is never `None` here. An authored one wins; otherwise it is
    /// DERIVED from the winning action set, which is what a body that authored
    /// capabilities and no explicit timeline needs in order to swing at all.
    Authored {
        action_set: crate::brain::ActionSet,
        moveset: MovesetContract,
    },
    /// No catalog action set exists, so the body's runtime `AbilitySet` builds
    /// the host-side kit. This remains valid for hosts whose protagonist kit
    /// depends on runtime progression rather than a catalog row; a shipped
    /// catalog character reaching this arm indicates missing content.
    ///
    /// `authored_moveset` is still honored when timelines exist without an
    /// authored action set.
    Unauthored {
        authored_moveset: Option<MovesetContract>,
    },
}

/// Everything construction needs to build this character's body, gathered
/// once so a constructor never has to rediscover what a character is.
///
///  presentation and authoring metadata are deliberately absent (sheet,
/// portrait, voice, cue/vfx dependency inventories, the checked/unresolved
/// report). They belong to a prepared definition and not to a body, and a
/// constructor that could see them would eventually read one.
#[derive(Clone, Copy, Debug)]
pub struct CharacterBodyBlueprint<'a> {
    pub character_id: &'a str,
    pub display_name: &'a str,
    /// The pool this body spawns with. Resolved rather than borrowed as
    /// `Vitals`, because a MATCH may legitimately overrule it (a seat's pool is
    /// the match's business) and an overridable field is honest about that where
    /// a borrowed authored value would not be.
    pub max_health: i32,
    ///  not `Option` — this is what completeness MEANS. A body that cannot
    /// say how it moves is not a body, and the whole point of
    /// [`PreparedCharacterDefinition::body_blueprint`] is that the question is
    /// answered once, at the boundary, rather than by every reader unwrapping.
    pub locomotion: crate::actor::CharacterLocomotion,
    pub contact_damage: Option<crate::actor::ContactDamage>,
    pub dream_seed: Option<f32>,
    /// Do this character's autonomous twins share one cognitive stream? See
    /// [`CharacterDefinition::preserves_mirror_symmetry`].
    ///
    ///  carried on the blueprint rather than looked up later, for the reason
    /// this whole type exists: the brain is chosen at construction on three
    /// separate roads (a seat, a room spawn, a rewind rebuild), and a fact one
    /// road reads from a registry the others cannot reach is a fact that goes
    /// missing on two of them.
    pub preserves_mirror_symmetry: bool,
    pub practice_target: bool,
    pub autonomous_profile: Option<crate::brain::BrainProfile>,
    pub mount: Option<&'a crate::actor::CharacterMount>,
    pub held_item: Option<&'a str>,
    pub death_traits: Option<&'a crate::actor::CharacterDeathTraits>,
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// What this body's ranged verb LOOKS like. See
    /// [`CharacterDefinition::ranged_vfx`].
    pub ranged_vfx: Option<&'a str>,
}

/// Why this character cannot build a body on its own, named rather than
/// counted.
///
///  this replaces `is_complete_body -> bool`.
///
/// Naming the facts costs one struct and buys the diagnostic: "goblin is not
/// character-complete: locomotion" is a sentence somebody can act on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingCharacterFacts {
    pub character_id: String,
    /// The absent facts, in a stable order.
    pub missing: Vec<&'static str>,
}

impl std::fmt::Display for MissingCharacterFacts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "character `{}` cannot build a body on its own; it has not authored: {}",
            self.character_id,
            self.missing.join(", ")
        )
    }
}

impl PreparedCharacterDefinition {
    /// Return the body blueprint only when every fact required by
    /// character-first construction is authored.
    ///
    /// Required construction facts belong in this checklist. Missing facts are
    /// reported explicitly so callers can choose their compatibility path rather
    /// than treating a character id alone as sufficient.
    pub fn body_blueprint(&self) -> Result<CharacterBodyBlueprint<'_>, MissingCharacterFacts> {
        let mut missing = Vec::new();
        if self.locomotion.is_none() {
            missing.push("locomotion");
        }
        if !missing.is_empty() {
            return Err(MissingCharacterFacts {
                character_id: self.id.as_str().to_string(),
                missing,
            });
        }
        Ok(self.blueprint_with_locomotion(self.locomotion.expect("checked above")))
    }

    /// The blueprint a MATCH SEAT builds, with the stage supplying anything
    /// the character has not stated.
    ///
    ///  this is the one legitimate caller of a default top speed, and it is
    /// separated from [`Self::body_blueprint`] so it cannot be reached by
    /// accident. A stage has to give a body that never said how fast it is
    /// SOMETHING or it stands still on the platform; a ROOM must not, because
    /// there the honest answer is that the character is not migrated yet and the
    /// archetype road still owns it.
    pub fn seat_blueprint(&self, fallback_run_speed: f32) -> CharacterBodyBlueprint<'_> {
        self.blueprint_with_locomotion(self.locomotion.unwrap_or(
            crate::actor::CharacterLocomotion {
                run_speed: fallback_run_speed,
                ..Default::default()
            },
        ))
    }

    fn blueprint_with_locomotion(
        &self,
        locomotion: crate::actor::CharacterLocomotion,
    ) -> CharacterBodyBlueprint<'_> {
        CharacterBodyBlueprint {
            locomotion,
            character_id: self.id.as_str(),
            display_name: &self.display_name,
            //  the ONE default for a body no authority describes, shared with
            // the peaceful-NPC seed. It was `1` here and `1` there and `4` inside
            // generic provocation, which is how "being hit changes your HP pool"
            // got to look reasonable. See `DEFAULT_UNAUTHORED_BODY_HEALTH`.
            max_health: self
                .vitals
                .max_health
                .unwrap_or(crate::actor::DEFAULT_UNAUTHORED_BODY_HEALTH),
            contact_damage: self.contact_damage,
            dream_seed: self.dream_seed,
            preserves_mirror_symmetry: self.preserves_mirror_symmetry,
            practice_target: self.practice_target,
            autonomous_profile: self.autonomous_profile,
            mount: self.mount.as_ref(),
            held_item: self.held_item.as_deref(),
            death_traits: self.death_traits.as_ref(),
            abilities: self.abilities,
            ranged_vfx: self.ranged_vfx.as_deref(),
        }
    }
}

impl PreparedKit {
    /// The action set content decided on, or `None` when only a body can say.
    pub fn action_set(&self) -> Option<&crate::brain::ActionSet> {
        match self {
            Self::Authored { action_set, .. } => Some(action_set),
            Self::Unauthored { .. } => None,
        }
    }

    /// The moveset to put on a body that is not building the host kit itself.
    pub fn projectable_moveset(&self) -> Option<&MovesetContract> {
        match self {
            Self::Authored { moveset, .. } => Some(moveset),
            Self::Unauthored { authored_moveset } => authored_moveset.as_ref(),
        }
    }
}

/// A prepared character: flat, immutable, and COMPLETE.
///
/// The session consumes resolved values. That is the real invariant behind §4.3 —
/// not "sharing must live in a generator", but that nothing downstream re-derives
/// a character from parents, patches, or a string search.
///
/// Now the fold happens ONCE, at the finalization barrier, and what a body reads has no questions
/// left in it.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCharacterDefinition {
    pub id: ambition_entity_catalog::CharacterId,
    pub display_name: String,
    pub provider: String,
    pub lineage: Option<Lineage>,
    pub sheet: Option<String>,
    pub portrait: Option<String>,
    /// See [`CharacterDefinition::voice`]. Empty means this character brought no
    /// lines of its own, which is different from "it has nothing to say" — the
    /// catalog may still speak for it.
    voice: Vec<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    /// What this body does when it dies, if it authored anything. See
    /// [`CharacterDefinition::death_traits`] — `None` stays `None`
    /// through the fold, because the catalog has no counterpart for it.
    pub death_traits: Option<crate::actor::CharacterDeathTraits>,
    /// The verbs this body has, as the character authored them — see
    /// [`CharacterDefinition::abilities`]. `None` means the character stated
    /// none, and a construction path that has a legacy source for verbs (an
    /// archetype's movement kit, a match's declared set) still uses it.
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// How this body moves, as the character authored it. `None` leaves a
    /// legacy source (an archetype row, a construction default) in charge.
    pub locomotion: Option<crate::actor::CharacterLocomotion>,
    /// What touching this body costs, as the character authored it.
    pub contact_damage: Option<crate::actor::ContactDamage>,
    /// The POLICY this character runs when nothing else drives it — the
    /// controller authority, carried as a value rather than as a name.
    ///
    /// Every road that needs to know what this body does when nobody drives it reads THIS, and
    /// lowers it against the body (`enemy_default_brain`): spawn, rewind and live restore all
    /// make the same call, which is why they cannot disagree.
    ///
    /// `None` leaves the archetype's projection in charge, which is every
    /// character that has not migrated.
    ///
    /// [`BrainProfile`]: crate::brain::BrainProfile
    pub autonomous_profile: Option<crate::brain::BrainProfile>,
    /// See [`CharacterDefinition::ranged_vfx`].
    pub ranged_vfx: Option<String>,
    /// HOW this character fires — see
    /// [`CharacterDefinition::ranged_execution`]. Read by the persona derive so
    /// the charge is a fact about the CHARACTER rather than about which arm of
    /// `PlayableKitSource` built it.
    pub ranged_execution: crate::brain::RangedExecution,
    /// The policy this creature adopts when provoked, RESOLVED — see
    /// [`CharacterDefinition::provoked_profile_ref`].
    pub provoked_profile: Option<crate::brain::BrainProfile>,
    /// The same policy's CANONICAL ID, kept beside the value.
    ///
    ///  the value drives the body at the moment of the provoke; the id is what
    /// a REWIND resolves later (`AutonomousSource::ProvokedProfile`). Resolving
    /// both from one preparation is what stops them disagreeing — a provoke that
    /// installed one policy and restored another would be a desync nobody could
    /// read.
    pub provoked_profile_id: Option<ambition_entity_catalog::BrainProfileId>,
    /// See [`CharacterDefinition::practice_target`].
    pub practice_target: bool,
    /// See [`CharacterDefinition::held_item`].
    pub held_item: Option<String>,
    /// Deep-dream visual jitter seed. See
    /// [`CharacterDefinition::dream_seed`] — presentation, true of every
    /// instance, and until now reachable only through an archetype row.
    pub dream_seed: Option<f32>,
    /// Do this character's autonomous twins share one cognitive stream? See
    /// [`CharacterDefinition::preserves_mirror_symmetry`] for the trait and for
    /// what it deliberately does not do.
    pub preserves_mirror_symmetry: bool,
    /// Mount and pilot capabilities. See [`CharacterDefinition::mount`].
    pub mount: Option<crate::actor::CharacterMount>,
    /// What this character fights with — resolved, not inherited.
    pub kit: PreparedKit,
    /// The move timelines the CHARACTER ITSELF stated, if it stated any.
    ///
    ///  distinct from `kit`'s moveset, and the difference decides a real
    /// question. `kit` always carries a moveset — derived from the action set
    /// when the character authored no timelines — so it cannot answer *"did
    /// this character say what its moves ARE?"*. A match that grants a borrowed
    /// cast a fighter's action set (`MatchParticipant::action_set`) must
    /// override a DERIVED moveset and must not override an authored one: the
    /// stage may say *"you may attack on this stage"* and may not say what the
    /// attack is. Without this field the two cases are indistinguishable and
    /// the grant wins over both.
    pub authored_moveset: Option<MovesetContract>,
    /// The movement policy, resolved. Every body already carries exactly one
    /// explicit model, so this is a value rather than a question.
    pub motion_model: ambition_platformer2d_core::MotionModelSpec,
    /// The movement feel, resolved.
    ///
    ///  still an `Option`, and its `None` is now an ANSWER rather than a
    /// question: "this character has no authored feel, so a body wearing it runs
    /// on the shared dev tuning and must NOT carry the authored-tuning marker".
    /// Before the fold, `None` here meant the catalog might still have one.
    pub movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
    /// DERIVED (§4.6): every cue this character can emit, read off its moves.
    /// Sorted, so two peers assemble byte-identical inventories.
    cue_dependencies: BTreeSet<String>,
    /// DERIVED: every vfx tag its moves request.
    vfx_dependencies: BTreeSet<String>,
    /// Namespaces preparation actually RESOLVED, carried on the published value.
    ///
    /// This lived only on the transient [`PreparedCharacter`] and was dropped on
    /// the floor by registration, so once a character was published there was no
    /// way to ask whether its cues had been checked against a real vocabulary or
    /// whether nobody had looked. Those must never read the same — that confusion
    /// is the entire reason the binding boundary exists — and a distinction that
    /// survives only until the value is stored is not a distinction.
    checked: Vec<&'static str>,
    /// References a resolver was supplied for and REJECTED, formatted for a reader.
    ///
    /// [`Self::checked`] says a vocabulary was consulted; it says nothing about the
    /// verdict, so a typo'd sheet target read as "verified" — which is how four
    /// shipped characters declared `<name>_spritesheet` (the sheet FILE) instead of
    /// the sheet TARGET and drew placeholders while every check stayed green.
    /// Registration still publishes (a placeholder beats a session that refuses to
    /// boot); carrying the failures onto the published value is what lets a guard
    /// be red about them without making the runtime fatal.
    unresolved: Vec<String>,
}

impl PreparedCharacterDefinition {
    /// Every reference preparation could check and rejected. Empty is the only
    /// acceptable state for shipped content.
    pub fn unresolved_references(&self) -> impl ExactSizeIterator<Item = &str> {
        self.unresolved.iter().map(String::as_str)
    }
}

impl PreparedCharacterDefinition {
    /// Every cue this character can emit. Derived at preparation, never authored.
    pub fn cue_dependencies(&self) -> impl ExactSizeIterator<Item = &str> {
        self.cue_dependencies.iter().map(String::as_str)
    }

    pub fn vfx_dependencies(&self) -> impl ExactSizeIterator<Item = &str> {
        self.vfx_dependencies.iter().map(String::as_str)
    }

    //  `checked_namespaces()` was here — the plural twin of `was_checked()`
    // below, written for symmetry and never called (compiler-verified,
    // ). `was_checked` asks the question consumers actually have.

    /// Was this namespace actually verified for this character?
    ///
    /// `false` means NOT CHECKED — no resolver was supplied — and says nothing
    /// about whether the references are good.
    pub fn was_checked(&self, namespace: &str) -> bool {
        self.checked.iter().any(|name| *name == namespace)
    }

    /// A line this character says when nothing more specific does.
    ///
    /// `rotation` cycles the pool so a repeated bark varies. `None` means this
    /// character brought no voice — the caller stays with whatever it had, which
    /// is the engine-generic line or silence.
    ///
    /// Deliberately situation-BLIND, unlike the catalog's pools. A definition's
    /// voice is the floor, and a floor that only covers some moments is not one.
    pub fn voice_line(&self, rotation: u32) -> Option<&str> {
        if self.voice.is_empty() {
            return None;
        }
        Some(self.voice[(rotation as usize) % self.voice.len()].as_str())
    }

    //  `voice()` was here — the plural twin of `voice_line()` above, same
    // shape and same fate. Nothing ever wanted the whole pool; the bark road
    // asks for one line at a rotation.

    /// The token the engine materializer demands for this character's art.
    ///
    /// Its own id: the sheet table is keyed by catalog id and display name, and a
    /// character that shares a sibling's sheet by reference still demands under
    /// its own id so the load ledger names the right character.
    pub fn art_load_token(&self) -> &str {
        self.id.as_str()
    }
}

/// Derive the cue and vfx inventory from a moveset.
///
/// §4.6: a hand-maintained cue list beside the moves it describes will drift, so
/// this reads the moves themselves — event cues, and the strike sound a hit volume
/// carries. Blank cues are skipped: an empty cue is authored silence, not a
/// reference to a cue named "".
fn derive_presentation_dependencies(
    moveset: &MovesetContract,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut cues = BTreeSet::new();
    let mut vfx = BTreeSet::new();
    for spec in &moveset.moves {
        for event in &spec.events {
            match &event.kind {
                MoveEventKind::Sfx { cue } if !cue.is_empty() => {
                    cues.insert(cue.clone());
                }
                MoveEventKind::Vfx { effect, .. } if !effect.is_empty() => {
                    vfx.insert(effect.clone());
                }
                _ => {}
            }
        }
        for window in &spec.windows {
            for volume in &window.volumes {
                if let Some(strike) = volume.hit_sfx.as_ref().filter(|s| !s.is_empty()) {
                    cues.insert(strike.clone());
                }
                if let Some(tag) = volume.vfx.as_ref().filter(|s| !s.is_empty()) {
                    vfx.insert(tag.clone());
                }
            }
        }
    }
    (cues, vfx)
}

/// What preparation had available to check against.
///
/// An ABSENT resolver means NOT CHECKED, and [`PreparedCharacter::checked`] says
/// which namespaces were — "we did not look" must never read like "we looked and
/// it was fine", which is the failure the binding boundary exists to prevent.
#[derive(Default)]
pub struct CharacterBindings {
    cues: Option<Resolver<SfxCueId>>,
    sheets: Option<Resolver<SheetTarget>>,
    portraits: Option<Resolver<PortraitTarget>>,
    vfx: Option<Resolver<VfxTag>>,
}

impl CharacterBindings {
    /// Check authored cues against the session's authorized cue set.
    pub fn with_authorized_cues<I, S>(mut self, cues: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.cues = Some(Resolver::new(cues));
        self
    }

    /// Check authored sheet targets against what the composition can resolve.
    pub fn with_available_sheets<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.sheets = Some(Resolver::new(targets));
        self
    }

    /// It is the ONLY way to populate `self.portraits`, so the field is `None`
    /// in every composition, `PortraitTarget::NAME` never joins
    /// `checked_namespaces`, and preparation's report says *"we did not look"*
    /// about portraits — permanently, and correctly, which is what makes it
    /// invisible. A character naming a portrait nobody authored is a fault
    /// nothing can currently raise.
    ///
    ///  kept deliberately. Deleting it would delete the only road to the
    /// check rather than the reason it is unused: `with_available_sheets` beside
    /// it IS wired, so the pattern works and this one was simply never
    /// connected.
    pub fn with_available_portraits<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.portraits = Some(Resolver::new(targets));
        self
    }

    /// Whether a caller already supplied a SHEET vocabulary.
    ///
    ///  this exists because the two `with_engine_*_vocabulary` methods had to stop being
    /// INHERENT (P1.7 sub-case (a)). They filled these resolvers from
    /// `ambition_sprite_sheet`, and that crate DEPENDS ON `ambition_characters` — so an
    /// inherent method on a type living there would be a cycle the moment the model moves down.
    ///
    ///  they are free functions at the REGISTRATION SEAM now
    /// (`with_engine_vocabularies`), which is also where the doc always said
    /// they belonged: *"this is the registration seam's job, because
    /// registration is where the engine is"*. What they need from the type is
    /// only this question — "did the caller already say?" — which is a QUERY and
    /// not a policy, so it is the part that stays.
    pub fn has_sheet_vocabulary(&self) -> bool {
        self.sheets.is_some()
    }

    /// Whether a caller already supplied a PORTRAIT vocabulary. See
    /// [`Self::has_sheet_vocabulary`] for why this is a predicate rather than a
    /// filler.
    pub fn has_portrait_vocabulary(&self) -> bool {
        self.portraits.is_some()
    }

    /// Check the DERIVED vfx inventory against the tags renderers know.
    pub fn with_known_vfx_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.vfx = Some(Resolver::new(tags));
        self
    }

    fn checked(&self) -> Vec<&'static str> {
        // `MoveId` is always checkable: a verb and a hurtbox override resolve
        // against the character's OWN moves, so no session vocabulary is needed.
        // Everything else is only checked when a resolver was supplied, and its
        // absence from this list is the honest report of "we did not look".
        //
        // NOT listed, deliberately: `BodySource` is an inline enum, not a reference —
        // `SpriteAuthored { world_per_pixel }` and `Explicit { half_extents }` name nothing outside
        // the character — so there is no "body" namespace to resolve.
        let mut out = vec![MoveId::NAME];
        if self.cues.is_some() {
            out.push(SfxCueId::NAME);
        }
        if self.sheets.is_some() {
            out.push(SheetTarget::NAME);
        }
        if self.portraits.is_some() {
            out.push(PortraitTarget::NAME);
        }
        if self.vfx.is_some() {
            out.push(VfxTag::NAME);
        }
        out
    }
}

/// A prepared character plus what preparation could and could not verify.
///
/// Module-private for the same reason [`PreparedCharacterOverrides`] is: it
/// carries one, and a type that leaks a partial value leaks the partial phase.
struct PreparedCharacter {
    prepared: PreparedCharacterOverrides,
    report: BindingReport,
    /// Namespaces that were actually resolved.
    #[allow(dead_code)]
    checked: Vec<&'static str>,
}

impl PreparedCharacter {
    /// True when every reference preparation COULD check did resolve.
    #[allow(dead_code)]
    fn is_clean(&self) -> bool {
        self.report.is_empty()
    }
}

/// Validate and flatten one authored definition.
///
/// Not fatal on an unresolved reference: the report is what makes the degradation loud, and a
/// character that draws a placeholder and says why beats a session that refuses to boot. Every
/// input verb the moveset runtime resolves.
///
/// The four bases the trigger path asks for, each with the directional and
/// airborne suffixes `directional_verb_chain` produces. Built rather than
/// listed so it cannot drift from the chain that consumes it: if a fifth base
/// or a fifth direction is added, this is the one place that has to learn about
/// it, and every character's registration starts checking against it for free.
fn runtime_verb_vocabulary() -> Vec<String> {
    //  the CONTRACT's crate, not the runtime's: a verb name is authoring vocabulary.
    use ambition_entity_catalog::{ATTACK_VERB, RANGED_VERB, SMASH_VERB, SPECIAL_VERB};
    let mut vocabulary = Vec::new();
    for base in [ATTACK_VERB, SMASH_VERB, RANGED_VERB, SPECIAL_VERB] {
        for dir in [
            ambition_entity_catalog::AttackDir::Neutral,
            ambition_entity_catalog::AttackDir::Forward,
            ambition_entity_catalog::AttackDir::Up,
            ambition_entity_catalog::AttackDir::Down,
            ambition_entity_catalog::AttackDir::Back,
        ] {
            for grounded in [true, false] {
                vocabulary.extend(ambition_entity_catalog::directional_verb_chain(
                    base, dir, grounded,
                ));
            }
        }
    }
    //  THE REPERTOIRE'S OWN VOCABULARY, ASKED FOR RATHER THAN RESTATED.
    //
    //  the cost of the second list was two shipped defects in three days.
    // `prepare_character` reports an unresolved verb and PUBLISHES ANYWAY, so a verb the table
    // binds and this list has never heard of is a move authored onto a button the runtime says
    // does not exist — visible only in the binding report until somebody presses it.
    //
    //  still FLAT, and that is the repertoire's shape rather than an
    // exemption here. A throw is not `grab_forward` and a taunt has no
    // direction: running either through `directional_verb_chain` would invent
    // `capture_throw_up_air`, and would let a fighter that authored only throws
    // light up its grab slot through the directional matcher. The chain above
    // stays because it is a generative RULE over the four directional bases —
    // which every character has, smash repertoire or not.
    vocabulary.extend(
        crate::smash_repertoire::REPERTOIRE_VERBS
            .iter()
            .copied()
            .map(str::to_owned),
    );
    vocabulary.sort();
    vocabulary.dedup();
    vocabulary
}

fn prepare_character(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
) -> PreparedCharacter {
    let mut ledger = BindingLedger::new();
    let declared_by = format!("character `{}`", definition.id);

    let (cue_dependencies, vfx_dependencies) = definition
        .moveset
        .as_ref()
        .map(derive_presentation_dependencies)
        .unwrap_or_default();

    // Every verb must name a move THIS character declares. A verb pointing at a
    // move id that does not exist resolves to "this character has no attack",
    // which is indistinguishable from a peaceful character at runtime.
    if let Some(moveset) = definition.moveset.as_ref() {
        let moves = Resolver::<MoveId>::new(moveset.moves.iter().map(|m| m.id.as_str()));
        // And every VERB must be one the runtime can actually press. A verb
        // outside the vocabulary binds a perfectly valid move to a button that
        // does not exist, and the move is simply never triggered — the same
        // "this character has no attack" outcome as a dangling move id, reached
        // from the other side and just as silent.
        let verb_vocabulary = runtime_verb_vocabulary();
        let verbs = Resolver::<VerbId>::new(verb_vocabulary.iter().map(String::as_str));
        for (verb, target) in &moveset.verbs {
            if moves.bind(target).is_none() {
                ledger.record(moves.explain(target, format!("{declared_by} verb `{verb}`")));
            }
            if verbs.bind(verb).is_none() {
                ledger.record(verbs.explain(verb, format!("{declared_by} moveset verb")));
            }
        }
        // The moveset and the action set have to agree about RANGED. (C3)
        //
        // A move on the `ranged` verb needs a projectile to throw, and the
        // projectile specification lives on the ACTION SET, not on the move. So
        // a character authoring both — the case the C3 precedence work makes
        // possible — can now author a ranged move and an action set with no
        // ranged payload, and the two are individually valid: the verb is real,
        // the move is real, the set is real. The button does nothing.
        //
        // This is preparation's job precisely because neither half is wrong on its own; only
        // the PAIR is, and preparation is the only place both are in hand.
        //
        // Only checked when the definition authored a set. Falling through to
        // the catalog is the migration path, and its rows are resolved
        // elsewhere; complaining here would be complaining about a value this
        // definition never claimed.
        if let Some(action_set) = definition.action_set.as_ref() {
            if action_set.ranged.is_none() {
                let ranged_verb = ambition_entity_catalog::RANGED_VERB;
                for (verb, target) in &moveset.verbs {
                    if verb.as_str() != ranged_verb {
                        continue;
                    }
                    ledger.record(ambition_binding::UnresolvedRef {
                        namespace: RangedPayload::NAME,
                        id: target.clone(),
                        declared_by: format!("{declared_by} verb `{verb}`"),
                        // Nothing WAS available, and saying so is the whole report: the
                        // character authored an action set and left `ranged` empty, so there is
                        // no candidate to suggest and no typo to find.
                        available: Vec::new(),
                        did_you_mean: None,
                    });
                }
            }
        }
        // Move-time hurtbox overrides must name a declared move too, or the
        // override is dead data nothing will ever sample.
        if let Some(hurtboxes) = definition.hurtboxes.as_ref() {
            for move_id in hurtboxes.moves.keys() {
                if moves.bind(move_id).is_none() {
                    ledger
                        .record(moves.explain(move_id, format!("{declared_by} hurtbox override")));
                }
            }
        }
    }

    // Cues, against the session's authorized set when one was supplied.
    if let Some(cue_resolver) = bindings.cues.as_ref() {
        for cue in &cue_dependencies {
            if cue_resolver.bind(cue).is_none() {
                ledger.record(cue_resolver.explain(cue, declared_by.clone()));
            }
        }
    }

    // The art references. Each is checked only when the composition supplied the
    // vocabulary; `checked` reports which ones it could.
    if let (Some(resolver), Some(sheet)) = (bindings.sheets.as_ref(), definition.sheet.as_deref()) {
        if resolver.bind(sheet).is_none() {
            ledger.record(resolver.explain(sheet, declared_by.clone()));
        }
    }
    if let (Some(resolver), Some(portrait)) =
        (bindings.portraits.as_ref(), definition.portrait.as_deref())
    {
        if resolver.bind(portrait).is_none() {
            ledger.record(resolver.explain(portrait, declared_by.clone()));
        }
    }
    // The DERIVED vfx inventory, resolved the same way the derived cue inventory
    // is. §4.6 derived this list and then nothing looked at it.
    if let Some(resolver) = bindings.vfx.as_ref() {
        for tag in &vfx_dependencies {
            if resolver.bind(tag).is_none() {
                ledger.record(resolver.explain(tag, declared_by.clone()));
            }
        }
    }

    let report = ledger.finish();
    let prepared = PreparedCharacterOverrides {
        id: definition.id.as_str().to_string(),
        display_name: definition.display_name,
        provider: definition.provider,
        lineage: definition.lineage,
        sheet: definition.sheet,
        portrait: definition.portrait,
        voice: definition.voice,
        body: definition.body,
        hurtboxes: definition.hurtboxes,
        vitals: definition.vitals,
        death_traits: definition.death_traits,
        abilities: definition.abilities,
        locomotion: definition.locomotion,
        contact_damage: definition.contact_damage,
        autonomous_profile: definition.autonomous_profile,
        autonomous_profile_ref: definition.autonomous_profile_ref.clone(),
        ranged_vfx: definition.ranged_vfx.clone(),
        ranged_execution: definition.ranged_execution,
        provoked_profile_ref: definition.provoked_profile_ref.clone(),
        practice_target: definition.practice_target,
        held_item: definition.held_item.clone(),
        dream_seed: definition.dream_seed,
        preserves_mirror_symmetry: definition.preserves_mirror_symmetry,
        mount: definition.mount,
        moveset: definition.moveset,
        action_set: definition.action_set,
        motion_model: definition.motion_model,
        movement_tuning: definition.movement_tuning,
        cue_dependencies,
        vfx_dependencies,
        checked: bindings.checked(),
        // The COMPACT form, not the resolver's `Display`. That one appends every
        // available id so a single log line can be acted on without a debugger,
        // which is right for a log and wrong for a value stored per character:
        // one unresolved sheet carries 400 ids, and a guard listing several of
        // them buries its own verdict. The log already printed the long form.
        unresolved: report
            .unresolved()
            .iter()
            .map(|reference| {
                let mut line = format!(
                    "unknown {} `{}` declared by `{}`",
                    reference.namespace, reference.id, reference.declared_by
                );
                if let Some(suggestion) = &reference.did_you_mean {
                    line.push_str(&format!(" — did you mean `{suggestion}`?"));
                }
                line
            })
            .collect(),
    };
    let checked = prepared.checked.clone();
    PreparedCharacter {
        prepared,
        report,
        checked,
    }
}

/// Fold one character's overrides against the catalog into a complete value.
///
/// `catalog: None` is a real composition, not a degraded one: a bare engine App
/// that registers characters and installs no catalog has nothing to inherit FROM,
/// which is the same answer as "this id is not in the catalog" — the case the
/// runtime already handled by installing the host compatibility kit.
fn finalize_character(
    overrides: PreparedCharacterOverrides,
    catalog: Option<&crate::actor::character_catalog::CharacterCatalog>,
    profiles: Option<&crate::actor::character_catalog::BrainProfileRegistry>,
) -> PreparedCharacterDefinition {
    use crate::brain::ActionSet;

    let PreparedCharacterOverrides {
        id,
        display_name,
        provider,
        lineage,
        sheet,
        portrait,
        voice,
        body,
        hurtboxes,
        vitals,
        death_traits,
        abilities,
        locomotion,
        contact_damage,
        autonomous_profile,
        dream_seed,
        preserves_mirror_symmetry,
        mount,
        moveset,
        action_set,
        motion_model,
        movement_tuning,
        cue_dependencies,
        vfx_dependencies,
        checked,
        unresolved,
        held_item,
        practice_target,
        autonomous_profile_ref,
        ranged_vfx,
        ranged_execution,
        provoked_profile_ref,
    } = overrides;

    // THE KIT. Three outcomes, and which one a character gets is decided here
    // once rather than by whichever construction path reaches it first.
    // Captured BEFORE the fold consumes it: `derive_moveset` substitutes a
    // derivation when this is `None`, and the substitution is exactly the thing
    // downstream needs to tell apart from the real answer.
    let authored_moveset = moveset.clone();
    let kit = match action_set {
        // The definition authored capabilities. Nothing else gets a vote.
        Some(set) => PreparedKit::Authored {
            moveset: derive_moveset(&set, moveset),
            action_set: set,
        },
        //  the question is "does the catalog know this id" (AC6.3). It was
        // `playable_kit_source(&id)`, whose `Option<PlayableKitSource>` had one
        // variant and was therefore already only answering membership.
        None if catalog.is_some_and(|catalog| catalog.knows(&id)) => {
            let set = catalog
                .and_then(|catalog| catalog.build_default_action_set(&id))
                .unwrap_or_else(|| {
                    // A known row whose preset does not resolve is malformed
                    // content. Reported ONCE here rather than every time a body
                    // wears it, and the body still gets a safe peaceful kit
                    // rather than silent host privileges.
                    bevy::log::error!(
                        "character `{id}` has a catalog row whose default_action_set does \
                         not resolve; preparing a safe peaceful kit"
                    );
                    ActionSet::peaceful()
                });
            PreparedKit::Authored {
                moveset: derive_moveset(&set, moveset),
                action_set: set,
            }
        }
        //  AN ID THE CATALOG DOES NOT KNOW, or no catalog at all — the
        // two states that remain now that no row can select the host kit by
        // name. Both mean the same thing to a body: nobody authored a kit, so
        // build one from what this body can do.
        //  AND THE ONE CONTRADICTION THE PLAN SAID DID NOT EXIST.
        //
        // A character can reach this arm — authoring no action set, so the
        // HOST builds one from the body — and still bring its own timelines;
        // `authored_moveset` exists precisely for that. If that
        // moveset declares the `ranged` verb, the same press is owned twice:
        // by the legacy charge-projectile path this kit installs, and by the
        // moveset's ranged verb. That is the exact double-ownership
        // `RangedExecution::ChargedProjectile` exists to prevent, arriving through
        // the one door it does not watch.
        //
        //  and REPORTING it was not enough, which is the second half of the same finding.
        // Invalid ownership must not reach a body at all.
        _ => PreparedKit::Unauthored {
            authored_moveset: moveset.map(|mut moveset| {
                let revoked = revoke_host_owned_ranged(&mut moveset);
                if !revoked.is_empty() {
                    bevy::log::error!(
                        "character `{id}` authored NO action set — so the host builds its \
                             kit from the body — AND authored the ranged verb(s) {revoked:?}. \
                             That host kit owns the ranged press through its \
                             charge-projectile path, so one press would have fired both; those \
                             verb bindings are DROPPED and the charge path keeps the press. To own \
                             the verb from content instead, author an action set — that makes the \
                             character `Authored`, and its moveset owns ranged outright"
                    );
                }
                moveset
            }),
        },
    };

    PreparedCharacterDefinition {
        // HEALTH FOLDS LIKE EVERY OTHER KIT FIELD, and it is the last one
        // that did not. The catalog row has carried `max_health: Option<i32>`
        // with exactly this `None`-means-unauthored meaning since it was added,
        // and `session::setup` read it directly — so a registered character's
        // authored pool and a catalog row's authored pool were two authorities
        // that never met. Folding here is what lets ONE applier serve the worn
        // player and the seated fighter.
        //
        // Mass has no catalog counterpart to fold against; it carries through.
        vitals: Vitals {
            max_health: vitals
                .max_health
                .or_else(|| catalog?.max_health(&id))
                // A pool of zero or less is dead on arrival and no author means
                // it. Clamped once, at the barrier, so no consumer has to.
                .map(|max| max.max(1)),
            ..vitals
        },
        motion_model: motion_model.unwrap_or_else(|| match catalog {
            Some(catalog) => catalog.motion_model_spec(&id),
            None => ambition_platformer2d_core::MotionModelSpec::AxisSwept(Default::default()),
        }),
        movement_tuning: movement_tuning.or_else(|| catalog?.axis_tuning(&id)),
        death_traits,
        // Carried, not folded: nothing else in the engine can state a body's
        // verbs, so there is no second authority to reconcile with.
        abilities,
        // A prepared definition that still needs the catalog to say whether a body flies is only
        // partly prepared.
        //
        // `Option<bool>` still carries the distinction the field's own doc names, and
        // preparation is where silence becomes an ANSWER: `None` resolves to `Some(false)`, so
        // a prepared character is never mute about flight and no constructor downstream has a
        // second authority to consult.
        locomotion: locomotion.map(|locomotion| crate::actor::CharacterLocomotion {
            //  `body_kind` IS NOT LOCOMOTION AUTHORITY (,
            // ). This read
            // `locomotion.baseline_free_flight || body_kind(&id) == Floating`, so a
            // presentation/footprint enum decided whether a body flies — and
            // the character had no way to disagree, because `flies` was a
            // bare `bool` whose `false` meant "did not say".
            //
            //  `Floating` still answers a real question, and keeping that
            // straight is the whole fix: it supplies no
            // `default_standing_height`, meaning *the SHEET decides how tall
            // this is* — which is why the PCA's body is 68px and not the 48px
            // `Standard` hands out. Geometry and locomotion were coupled
            // through this one enum; only the locomotion edge is cut.
            //
            //  silence now resolves to GROUNDED, and the three characters
            // that genuinely fly say so on their own definitions (the parrot,
            // the burning shark, and both plane swarms).
            baseline_free_flight: Some(locomotion.baseline_free_flight.unwrap_or(false)),
            ..locomotion
        }),
        contact_damage,
        // See `resolve_autonomous_profile`: provider-relative, inline XOR named, and a name nobody
        // authored is a refusal.
        autonomous_profile: resolve_autonomous_profile(
            &id,
            &provider,
            autonomous_profile,
            autonomous_profile_ref.as_ref(),
            profiles,
        ),
        ranged_vfx,
        ranged_execution,
        provoked_profile: resolve_autonomous_profile(
            &id,
            &provider,
            None,
            provoked_profile_ref.as_ref(),
            profiles,
        ),
        provoked_profile_id: provoked_profile_ref
            .as_ref()
            .map(|reference| reference.resolve_in(&provider)),
        practice_target,
        held_item,
        dream_seed,
        preserves_mirror_symmetry,
        mount,
        authored_moveset,
        // Resolve canonical identity during preparation from the definition's provider. Spawn
        // consumes the prepared identity and does not reinterpret authored references.
        id: ambition_entity_catalog::CharacterId::new(id),
        display_name,
        provider,
        lineage,
        sheet,
        portrait,
        voice,
        body,
        hurtboxes,
        kit,
        cue_dependencies,
        vfx_dependencies,
        checked,
        unresolved,
    }
}

/// Resolve the character's autonomous policy.
///
/// Inline and named policies are mutually exclusive. Named references are
/// provider-qualified and an explicit missing reference is an error. If no
/// profile registry is assembled, shared policy resolution is unavailable.
fn resolve_autonomous_profile(
    id: &str,
    provider: &str,
    inline: Option<crate::brain::BrainProfile>,
    named: Option<&crate::brain::BrainProfileRef>,
    // Autonomous policy resolves from the profile registry, not the character
    // catalog; sharing a provider fragment does not merge those authorities.
    profiles: Option<&crate::actor::character_catalog::BrainProfileRegistry>,
) -> Option<crate::brain::BrainProfile> {
    match (inline, named) {
        (Some(_), Some(named)) => panic!(
            "character `{id}` authors an inline autonomous profile AND names the \
             shared profile `{named}`. Those do not merge — one would silently \
             replace the other — so authoring both is refused. State one, or ask \
             for a real patch type"
        ),
        (Some(inline), None) => Some(inline),
        (None, None) => None,
        (None, Some(named)) => {
            // An explicitly named profile must resolve. No registry is valid only
            // for characters that name no shared profile; treating an unresolved
            // reference as absence would contradict the authored definition.
            let profiles = profiles.filter(|profiles| !profiles.is_empty());
            let Some(profiles) = profiles else {
                panic!(
                    "character `{id}` (provider `{provider}`) names the shared \
                     autonomous profile `{named}`, and this composition published no \
                     profile registry for it to live in. An explicitly named policy \
                     needs an authority to resolve it — resolving it to nothing \
                     would leave this body on its archetype while the definition \
                     says otherwise. Publish the provider's `BrainProfileRegistry`, \
                     or author the profile inline"
                )
            };
            let resolved = named.resolve_in(provider);
            match profiles.get(&resolved) {
                Some(profile) => Some(*profile),
                None => panic!(
                    "character `{id}` (provider `{provider}`) names the autonomous \
                     profile `{named}`, which resolves to `{resolved}` and is not \
                     published. An explicitly named policy that does not exist is a \
                     content error, not an absence — resolving it to nothing would \
                     leave this body on its archetype while the definition says \
                     otherwise. Published: [{}]",
                    profiles.ids().collect::<Vec<_>>().join(", ")
                ),
            }
        }
    }
}

/// Take the ranged press away from a moveset whose body wears the host kit.
///
/// Returns the verb bindings removed, in sorted order, for the caller to name.
///
/// The whole ranged FAMILY, not the base verb alone. `directional_verb_chain`
/// resolves a press through `ranged_air_forward` → `ranged_forward` →
/// `ranged_air` → `ranged`, so a moveset binding any suffixed form owns the press
/// for that direction exactly as the base form owns the neutral one. A guard
/// that watched only `"ranged"` would have let `ranged_air` through — the same
/// double-fire, one direction over, and invisible until somebody shot while
/// jumping.
///
/// Only the VERB bindings go. The move itself stays: a timeline nothing presses
/// is inert, and pruning it would delete authored content on the strength of a
/// reachability argument, which is the more expensive mistake. It keeps its cues
/// in the derived inventory, so the session loads a sound it will not play —
/// cheap, and honest about what the author wrote.
fn revoke_host_owned_ranged(moveset: &mut MovesetContract) -> Vec<String> {
    let base = ambition_entity_catalog::RANGED_VERB;
    let prefix = format!("{base}_");
    let revoked: Vec<String> = moveset
        .verbs
        .keys()
        .filter(|verb| verb.as_str() == base || verb.starts_with(&prefix))
        .cloned()
        .collect();
    for verb in &revoked {
        moveset.verbs.remove(verb);
    }
    revoked
}

/// An authored moveset if there is one, otherwise the moves the action set implies.
///
/// Deriving rather than leaving it empty is the other half of H1: a character can
/// legitimately author *what it can reach for* and leave the timelines to the
/// prefab builder, and a body that got the capability without the timeline
/// advertises an attack it cannot perform.
///
/// The public API does not require one: a definition may carry `action_set.special = Some(..)`
/// and no moveset at all, and `ActionSet ::special`'s own doc says the brain reads
/// `special.is_some()` to decide whether to press it while the execution "is a data-driven move
/// in the body's `ActorMoveset`".
///
/// Folding it here cannot double-fire: this branch runs ONLY when there is no
/// authored moveset, so there is no second declaration to collide with. A
/// persona that authored its moves still overrides everything derived.
fn derive_moveset(
    action_set: &crate::brain::ActionSet,
    authored: Option<MovesetContract>,
) -> MovesetContract {
    //  THE LOW CRATE'S, not this monolith's. This read
    // `crate::combat::moveset::build_actor_moveset`, and `ambition_combat`
    // depends on `ambition_characters` — so preparation calling UP was the last
    // thing keeping the authoritative character model from following the model
    // down. The derivation lives in `crate::moveset_prefabs` now;
    // `ambition_combat` re-exports it, so its own call sites are unchanged.
    //
    // The ranged preset IS the ranged verb here, whatever the definition's
    // `ranged_execution` says: a body that fires through the host's charge path
    // is worn through `ambition_combat::worn_kit`, which selects by execution.
    let derived = crate::moveset_prefabs::build_actor_moveset(
        None,
        action_set.melee.as_ref(),
        action_set.ranged.as_ref(),
        action_set.special.as_ref(),
    )
    .unwrap_or_default();
    overlay_authored_moves(derived, authored)
}

/// AUTHORED MOVES OVERLAY THE KIT'S, they do not REPLACE it.
///
/// Authored wins on collision, in both halves: naming a move id or a verb the
/// kit also produces is a deliberate statement about that one thing. A character
/// that authors an `"attack"` move means that swing rather than the derived one;
/// a character that authors none keeps whatever the kit folded.
///
/// This is the ONE statement of the rule. Preparation (above) and the worn-kit
/// compiler in `ambition_combat::worn_kit` each carried their own copy, and the
/// second was written after the first had "learned" the overlay — so the rule
/// was once true in one of them and silently total replacement in the other.
pub fn overlay_authored_moves(
    derived: MovesetContract,
    authored: Option<MovesetContract>,
) -> MovesetContract {
    let Some(authored) = authored else {
        return derived;
    };
    let authored_ids: std::collections::BTreeSet<&str> =
        authored.moves.iter().map(|mv| mv.id.as_str()).collect();
    let mut merged = MovesetContract {
        moves: authored.moves.clone(),
        verbs: derived.verbs,
    };
    merged.moves.extend(
        derived
            .moves
            .into_iter()
            .filter(|mv| !authored_ids.contains(mv.id.as_str())),
    );
    merged.verbs.extend(authored.verbs);
    merged
}

/// One character, prepared and finalized, outside an `App`.
#[cfg(any(test, feature = "test-support"))]
pub struct FinalizedCharacter {
    pub prepared: PreparedCharacterDefinition,
    pub report: BindingReport,
    pub checked: Vec<&'static str>,
}

#[cfg(any(test, feature = "test-support"))]
impl FinalizedCharacter {
    /// True when every reference preparation COULD check did resolve.
    pub fn is_clean(&self) -> bool {
        self.report.is_empty()
    }
}

/// Test-only finalization seam. Production finalization waits until the composition's catalog is
/// complete; exposing this path to production would permit order-dependent early folding.
#[cfg(any(test, feature = "test-support"))]
pub fn prepare_and_finalize_for_test(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
) -> FinalizedCharacter {
    prepare_and_finalize_against_for_test(definition, bindings, None)
}

/// The same, with a catalog to inherit from — the case the barrier exists for.
#[cfg(any(test, feature = "test-support"))]
pub fn prepare_and_finalize_against_for_test(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
    catalog: Option<&crate::actor::character_catalog::CharacterCatalog>,
) -> FinalizedCharacter {
    //  a fixture that hands over a catalog is modelling a composition that
    // assembled one, and assembly publishes the policy registry beside it — so
    // the profiles come from the SAME source rather than being absent, which
    // would silently exercise the no-registry branch.
    let profiles = catalog.map(|catalog| {
        crate::actor::character_catalog::BrainProfileRegistry::from_catalog_for_test(
            //  the provider a fixture's own characters name, so a
            // provider-relative policy reference resolves the way it will in
            // production rather than only when the key happens to be bare.
            &definition.provider,
            catalog,
        )
    });
    let PreparedCharacter {
        prepared, report, ..
    } = prepare_character(definition, bindings);
    let checked = prepared.checked.clone();
    FinalizedCharacter {
        prepared: finalize_character(prepared, catalog, profiles.as_ref()),
        report,
        checked,
    }
}

/// Prepared character authority keyed by stable id.
///
/// Authored definition values override catalog fallbacks for kit fields; `None`
/// means use the catalog value, while `Some(empty)` is an explicit empty value.
/// Sprite declarations, cue authorization, presentation source, moveset,
/// hurtbox, action set, and motion model are derived from this registry. Display
/// metadata and remaining catalog tuning stay catalog-owned.
#[derive(bevy::prelude::Resource, Debug, Clone, Default)]
pub struct PreparedCharacterRegistry {
    by_id: BTreeMap<ambition_entity_catalog::CharacterId, PreparedCharacterDefinition>,
    generation: CharacterCatalogGeneration,
}

/// Which version of the cast a value was built from.
///
/// One generation per publication, carried across rebuilds — a counter that restarted would
/// republish generation 1 over a body stamped with generation 1 from the previous cast, and every
/// staleness check would read "still current".
///
/// A monotonic counter, not a hash: two registries with identical contents
/// assembled at different times are legitimately different generations, and a
/// consumer caching against a hash would silently keep a value across a
/// replacement that happened to produce the same cast. Cheap enough to stamp,
/// and it is rollback-safe because it only ever moves forward within a session
/// and is rebuilt with the registry on a load.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharacterCatalogGeneration(u64);

impl CharacterCatalogGeneration {
    pub fn get(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for CharacterCatalogGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cast generation {}", self.0)
    }
}

impl PreparedCharacterRegistry {
    /// Which version of the cast this is.
    ///
    /// Increments on every published change. A consumer that derived something
    /// from the registry can hold this beside it and know, cheaply and without
    /// comparing contents, whether its derivation is still about the cast that
    /// exists now.
    pub fn generation(&self) -> CharacterCatalogGeneration {
        self.generation
    }

    pub fn get(&self, id: &str) -> Option<&PreparedCharacterDefinition> {
        self.by_id.get(id)
    }

    pub fn ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.by_id
            .keys()
            .map(ambition_entity_catalog::CharacterId::as_str)
    }

    /// The stable id of the character presented under `display_name`.
    ///
    /// The registration seam is the ONLY place a registered-only character's
    /// display name is written down, and rooms, LDtk entities and roster entries
    /// all legitimately name characters that way. Without this the alias is
    /// resolvable exclusively through the sprite table — which forgets it once the
    /// sheet decodes, and does not exist at all in an art-free build.
    ///
    /// A linear scan: the registry is a per-composition cast list (single digits
    /// to low tens), and a second index would be a second thing to keep true.
    pub fn id_for_display_name(&self, display_name: &str) -> Option<&str> {
        self.by_id
            .values()
            .find(|prepared| prepared.display_name == display_name)
            .map(|prepared| prepared.id.as_str())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &PreparedCharacterDefinition)> {
        self.by_id.iter().map(|(id, def)| (id.as_str(), def))
    }

    /// The union of every prepared character's derived cue inventory.
    ///
    /// §4.6: this is the CAST's contribution to a session's authorized set, not
    /// the whole of it — stage, ruleset, announcer, world-object, UI and shell
    /// dependencies join it at session level.
    pub fn cast_cue_dependencies(&self) -> BTreeSet<&str> {
        self.by_id
            .values()
            .flat_map(PreparedCharacterDefinition::cue_dependencies)
            .collect()
    }

    /// Publish a prepared definition directly.
    ///
    /// Named for what it is: the registration seam is the door (it rejects
    /// duplicate ids and ambiguous display names), and this is the hatch a
    /// focused test uses when it wants a registry without an `App`.
    ///
    ///  behind `test-support`, and that feature is what keeps it off the
    /// production surface now that the callers are in another crate. It was
    /// `#[cfg(test)] pub(crate)`, which is stronger — but `#[cfg(test)]` items
    /// do not cross a crate boundary, and the alternative was making the hatch
    /// an ordinary `pub` method, which is the mechanical relocation deciding the
    /// public architecture (P1.7 sub-case (b)).
    #[cfg(any(test, feature = "test-support"))]
    pub fn insert_prepared(&mut self, prepared: PreparedCharacterDefinition) {
        let generation = self.generation;
        // ⛔⛔ THIS ONE REPLACES, AND THAT IS THE RULING RATHER THAN AN
        // OVERSIGHT — measured 2026-09-05 by making it refuse and counting what
        // fell. FOUR tests, and their names are the whole argument:
        // `deleting_an_override_in_a_hot_reload_gives_the_body_its_own_numbers_back`,
        // `a_new_cast_generation_refreshes_a_seated_fighters_kit`,
        // `a_character_that_stops_authoring_hurtboxes_has_them_retracted`,
        // `replacing_the_cast_reprojects_a_body_wearing_the_same_character`.
        // ⇒ Every one of them re-registers ONE id on purpose, because that is
        // what a hot reload IS, and refusing the second write would pin the
        // registry to the pre-reload definition — the opposite of the defect a
        // silent-overwrite audit is looking for.
        //
        // ⭐ THE TWO ROADS ANSWER DIFFERENTLY AND BOTH ARE RIGHT. The PRODUCTION
        // door (`register_character`) refuses a duplicate as
        // `CharacterRegistrationError::DuplicateId`, because there a second
        // write means two PROVIDERS claimed one stable id and somebody has to
        // lose. Here a second write means the SAME author published again, which
        // is a republication and is the line directly below.
        //
        // ⚠ What that costs, said in place: a fixture that registers two
        // genuinely different characters under one id keeps the second silently.
        // Nothing distinguishes that from a reload, and it cannot — the hatch
        // sees one definition, not an intent.
        self.insert(prepared);
        // Each hatched insert is its own publication: a test using the hatch has
        // no barrier to publish for it.
        self.stamp_after(generation);
    }

    fn insert(&mut self, prepared: PreparedCharacterDefinition) -> Option<String> {
        let id = prepared.id.clone();
        match self.by_id.insert(id.clone(), prepared) {
            Some(previous) => Some(previous.provider),
            None => None,
        }
    }

    /// Stamp this registry as the publication that follows `previous`.
    ///
    /// It takes the previous generation rather than starting from its own zero,
    /// and that is the load-bearing part: a rebuilt registry is a fresh
    /// `Default`, so without this a hot reload would republish generation 1 over
    /// a body stamped with generation 1 from the PREVIOUS cast, and every
    /// staleness check would read "still current". A monotonic counter that
    /// restarts is worse than no counter, because it looks like one.
    fn stamp_after(&mut self, previous: CharacterCatalogGeneration) {
        self.generation = CharacterCatalogGeneration(previous.0 + 1);
    }
}

/// Why a character could not be published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterRegistrationError {
    BlankId,
    /// Two providers authored the same stable id. Not a merge: a stable id is the
    /// thing saves, replays, and the network key on, so a silent last-one-wins
    /// would make two sessions disagree about who a character is.
    DuplicateId {
        character_id: String,
        first_provider: String,
        second_provider: String,
    },
    /// Two DIFFERENT characters present under the same display name.
    ///
    /// Rejected rather than disambiguated, for the same reason as a duplicate id:
    /// picking a winner means two authorities can pick differently.
    AmbiguousDisplayName {
        display_name: String,
        first_id: String,
        second_id: String,
    },
}

impl std::fmt::Display for CharacterRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlankId => write!(f, "a character definition has a blank id"),
            Self::DuplicateId {
                character_id,
                first_provider,
                second_provider,
            } => write!(
                f,
                "character `{character_id}` is authored by both `{first_provider}` and \
                 `{second_provider}`; a stable id is what saves, replays, and peers key on, \
                 so one of them must be renamed (a crossover variant is its own product with \
                 its own id — see §4.3)"
            ),
            Self::AmbiguousDisplayName {
                display_name,
                first_id,
                second_id,
            } => write!(
                f,
                "`{first_id}` and `{second_id}` both present as `{display_name}`. Content                  addresses characters by display name — a room's `enemy.name`, an                  interactable's `character_id`, a roster entry — and the registry, the                  catalog, and the sprite alias table each resolve that name their own way,                  so an ambiguous one can stage one character and decode another's art.                  Give one of them a distinct display name"
            ),
        }
    }
}

impl std::error::Error for CharacterRegistrationError {}

/// Prepared character state held behind the registration/finalization barrier.
///
/// Production callers contribute definitions and read the finished
/// [`PreparedCharacterRegistry`]; the private lifecycle controls when staged
/// definitions are folded. The accessors below expose only identity needed for
/// duplicate validation before staging completes.
#[derive(Debug, Clone)]
struct StagedCharacter {
    inner: PreparedCharacterOverrides,
}

impl StagedCharacter {
    /// The stable id this registration will publish under.
    fn id(&self) -> &str {
        &self.inner.id
    }

    /// The display name this registration claims.
    fn display_name(&self) -> &str {
        &self.inner.display_name
    }

    /// Which provider authored it — the answer a rejected duplicate id owes its
    /// caller ("`x` was already registered by `y`").
    fn provider(&self) -> &str {
        &self.inner.provider
    }
}

/// What [`prepare_for_registration`] hands back: the staged value and what
/// preparation could and could not verify.
struct StagedRegistration {
    /// The partial. Opaque — see [`StagedCharacter`].
    staged: StagedCharacter,
    /// Every reference a resolver was supplied for and could not resolve.
    report: BindingReport,
}

/// Validate and flatten one authored definition, without folding it.
///
/// The first half of registration. It is a pure function of its arguments — no
/// catalog, no baked globals — which is what lets the same definition prepare
/// identically in every build. What it CANNOT do is inherit, because the catalog
/// is not knowable at registration time; that is [`finalize_cast`]'s job and the
/// whole reason the two are separate.
fn prepare_for_registration(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
) -> StagedRegistration {
    let PreparedCharacter {
        prepared, report, ..
    } = prepare_character(definition, bindings);
    StagedRegistration {
        staged: StagedCharacter { inner: prepared },
        report,
    }
}

/// Fold a whole staged cast against the assembled catalog, transactionally.
///
/// The second half. It takes the cast rather than one character on purpose: the
/// registry it returns is published in one write, so a reader can never observe
/// a registry holding half of one generation.
///
/// `previous` is the generation the outgoing registry was stamped with, so the
/// new one sorts after it — a cast hot reload must be distinguishable from the
/// boot cast by more than its contents.
fn finalize_cast(
    staged: impl IntoIterator<Item = StagedCharacter>,
    catalog: Option<&crate::actor::character_catalog::CharacterCatalog>,
    profiles: Option<&crate::actor::character_catalog::BrainProfileRegistry>,
    previous: CharacterCatalogGeneration,
) -> PreparedCharacterRegistry {
    let mut registry = PreparedCharacterRegistry::default();
    for character in staged {
        registry.insert(finalize_character(character.inner, catalog, profiles));
    }
    registry.stamp_after(previous);
    registry
}

// ─────────────────────────────────────────────────────────────────────────────
// THE LIFECYCLE. Staging, the barrier, and the fold it closes over.
//
// The only structure that makes early folding unspellable by ordinary production code is the one
// where the fold and the thing that decides WHEN to fold are the same module's private business.
//
// What crosses the boundary instead is intent: `stage_authored_character` (a
// contribution) and `PreparedCharacterRegistry` (the finished read). A host that
// wants to enrich a registration — the engine's baked sheet and portrait
// vocabularies, say — does that to the `CharacterBindings` it passes IN, which is
// why `ambition_platformer2d_actor_monolith::character_runtime` still owns
// `with_engine_vocabularies` and this crate has never heard of a sprite sheet.
// ─────────────────────────────────────────────────────────────────────────────

/// The single registration seam. (§4.1)
///
/// Prepares the definition and stages it for the barrier. A provider makes ONE
/// call and does not have to know that sheets, cues, and gameplay numbers are
/// consumed by different subsystems.
///
/// # Registration is DECLARATIVE — it does not load anything
///
/// The binding report is logged rather than returned as an error: see
/// `prepare_character` for why an unresolved reference degrades loudly instead of
/// refusing.
pub fn stage_authored_character(
    app: &mut bevy::app::App,
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
) -> Result<(), CharacterRegistrationError> {
    if definition.id.as_str().trim().is_empty() {
        return Err(CharacterRegistrationError::BlankId);
    }
    if !app.is_plugin_added::<CharacterPreparationPlugin>() {
        app.add_plugins(CharacterPreparationPlugin);
    }
    let provider = definition.provider.clone();
    let StagedRegistration {
        staged: prepared,
        report,
    } = prepare_for_registration(definition, bindings);
    let id = prepared.id().to_string();

    // Transactional: assemble the candidate, and only publish if the id is
    // free. A rejected registration leaves the previous authority active.
    let mut candidate = app
        .world()
        .get_resource::<StagedCharacterOverrides>()
        .cloned()
        .unwrap_or_default();
    if candidate.finalized {
        panic!(
            "character `{id}` was registered after the preparation barrier closed. \
             Authoring is a `Plugin::build` operation: a contribution that arrives \
             after finalization would be folded against a catalog the published cast \
             was already built without, so half the session would know a character the \
             other half does not. A later cast change is a separate explicit \
             transaction (see docs/archive/planning-superseded/2026-08-13/character-preparation-finalization-plan.md)"
        );
    }
    // A display name already spoken for by a DIFFERENT id is rejected before the
    // insert, so the registry can never hold the ambiguity that
    // `id_for_display_name` would then have to resolve arbitrarily.
    if let Some(first_id) = candidate.id_for_display_name(prepared.display_name()) {
        if first_id != prepared.id() {
            return Err(CharacterRegistrationError::AmbiguousDisplayName {
                display_name: prepared.display_name().to_string(),
                first_id: first_id.to_string(),
                second_id: prepared.id().to_string(),
            });
        }
    }
    if let Some(first_provider) = candidate.insert(prepared) {
        return Err(CharacterRegistrationError::DuplicateId {
            character_id: id,
            first_provider,
            second_provider: provider,
        });
    }
    app.insert_resource(candidate);

    if !report.is_empty() {
        report.log(&format!("preparing character `{id}`"));
    }
    Ok(())
}

/// What providers have authored, before the catalog exists to fold against.
///
/// The preparation-phase half of the registry. Holds partial values and is
/// consumed by [`CharacterPreparationPlugin::finish`].
///
///  PRIVATE, and that is the barrier's second half. A resource this crate does not export
/// cannot be read, taken, or reconstructed by a host — so there is no route to a
/// `StagedCharacter` at all outside this module, let alone to folding one.
#[derive(bevy::ecs::resource::Resource, Debug, Clone, Default)]
struct StagedCharacterOverrides {
    by_id: BTreeMap<ambition_entity_catalog::CharacterId, StagedCharacter>,
    /// Set when the barrier closes, so a late contribution is a panic rather than
    /// a value nobody will ever fold.
    finalized: bool,
}

impl StagedCharacterOverrides {
    fn id_for_display_name(&self, display_name: &str) -> Option<&str> {
        self.by_id
            .values()
            .find(|staged| staged.display_name() == display_name)
            .map(StagedCharacter::id)
    }

    /// Returns the previous author when the id was already spoken for.
    fn insert(&mut self, staged: StagedCharacter) -> Option<String> {
        self.by_id
            .insert(staged.id().to_string().into(), staged)
            .map(|previous| previous.provider().to_string())
    }
}

/// Bevy runs every plugin's `build` during registration and every `finish` once
/// all of them are ready. That ordering is the whole reason this is a plugin
/// rather than a startup system or an eager fold at registration time: a provider
/// registering its cast before the App installs `CharacterCatalog` would otherwise
/// inherit an empty row and bake the absence in permanently. Which provider goes
/// first is a composition detail no provider can see.
///
/// Installed automatically by [`stage_authored_character`].
///
///  `App::update` does not run `finish` — Bevy's runners do. A hand-driven
/// App (every headless test, every fixture, every tool in this repository) must
/// call `ambition_platformer2d_runtime::finalize` or it will have a staged cast
/// and no published one. That is not silent: `PreparedCharacterRegistry` is
/// absent rather than empty, and absent already means "no registered characters"
/// to every consumer.
pub struct CharacterPreparationPlugin;

impl bevy::app::Plugin for CharacterPreparationPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<StagedCharacterOverrides>();
        // THE BACKSTOP, for the apps Bevy's runner never touches.
        //
        // `App::update` does not run `finish` — runners do — and this repository drives
        // `update` by hand almost everywhere: every headless test, the external-consumer
        // fixture, the rollback harnesses, the tools.
        //
        // Not a second authority: it calls the SAME finalizer, guarded by the
        // same `finalized` flag, so whichever trigger fires first wins and the
        // other is a no-op. And it is not a weaker barrier — `PreStartup` runs
        // after every plugin's `build`, which is the entire ordering hazard
        // `finish` exists to remove. What `finish` still buys is that the
        // registry exists before ANY system runs, including `Startup`.
        app.add_systems(bevy::app::PreStartup, close_preparation_barrier);
    }

    fn finish(&self, app: &mut bevy::app::App) {
        finalize_prepared_cast(app.world_mut());
    }
}

/// The `PreStartup` half of [`CharacterPreparationPlugin`]'s backstop.
fn close_preparation_barrier(world: &mut bevy::ecs::world::World) {
    finalize_prepared_cast(world);
}

/// Fold the staged cast and publish it. Idempotent; runs at most once.
fn finalize_prepared_cast(world: &mut bevy::ecs::world::World) {
    let Some(mut staged) = world.get_resource_mut::<StagedCharacterOverrides>() else {
        return;
    };
    // Without a guard, a second call republished an EMPTY registry: the staged
    // overrides had already been consumed, so the whole cast silently vanished on
    // the fixture's second step. The barrier has to be idempotent itself;
    // nothing upstream makes it so.
    if staged.finalized {
        return;
    }
    staged.finalized = true;
    let staged = std::mem::take(&mut staged.by_id);
    let catalog = world
        .get_resource::<crate::actor::character_catalog::CharacterCatalog>()
        .cloned();
    // The POLICY authority, published beside the catalog by assembly.
    let profiles = world
        .get_resource::<crate::actor::character_catalog::BrainProfileRegistry>()
        .cloned();
    // TRANSACTIONAL: the whole cast is folded and only then published, so a
    // reader can never observe a registry that holds half of one generation.
    let previous = world
        .get_resource::<PreparedCharacterRegistry>()
        .map(PreparedCharacterRegistry::generation)
        .unwrap_or_default();
    world.insert_resource(finalize_cast(
        staged.into_values(),
        catalog.as_ref(),
        profiles.as_ref(),
        previous,
    ));
}

#[cfg(test)]
#[path = "prepared_tests.rs"]
mod prepared_tests;
