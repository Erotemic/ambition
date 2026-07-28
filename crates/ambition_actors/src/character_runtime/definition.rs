//! **One registration per character.** (§4.1, §4.6, §5)
//!
//! A character used to be declared through six seams keyed three different ways —
//! a catalog fragment by `character_id`, a roster fragment by `brain_id`, an audio
//! fragment by `provider_id`, world-item art, projectile visuals, and art
//! materialization that was not registered at all. None of them was the character,
//! and nothing checked that they agreed.
//!
//! [`register_character`] is the single seam. What it accepts is **decomposable** —
//! sheets, hurtboxes, movesets, and gameplay numbers have different load times,
//! headless requirements, and replacement frequencies, so any substantial section
//! may be inline or a reference to another typed document — but what it produces is
//! one flat immutable value.
//!
//! ```text
//! CharacterDefinition          authored, decomposable, may reference
//!         |  prepare_character(...)   validates + flattens
//!         v
//! PreparedCharacterDefinition  immutable, no inheritance, no string search
//!                              in authoritative gameplay paths
//! ```
//!
//! ## What is DERIVED rather than registered again
//!
//! ⛔ The failure mode this must not become is *six registries behind one
//! function* — that would improve ergonomics and keep the consistency problem.
//! So preparation derives, from the one definition:
//!
//! * the **cue dependency inventory** (§4.6) — read off `MoveEvent::Sfx`, hit
//!   volumes' strike sounds, and `MoveEvent::Vfx`, never hand-listed beside the
//!   moves it describes, because a hand-maintained list drifts;
//! * the **art load requirement** — the token the engine materializer demands;
//! * a **binding report** over every id in the definition.
//!
//! ## Every string is a cross-layer reference
//!
//! Preparation resolves them through the
//! [binding resolution boundary](ambition_platformer_primitives::binding), so a
//! misspelled cue, move, or verb target is NAMED at load — with namespace,
//! declarer, what was available, and a did-you-mean — instead of going silent
//! until a playtest.
//!
//! ⚠ A resolver proves content agrees with CONTENT. It cannot prove a file
//! exists: a cue id that binds against the authorized set may still name a bank
//! entry no renderer produced. That check belongs to whoever loads the asset, and
//! [`super::CharacterLoadStates`] is where the art half reports.

use std::collections::{BTreeMap, BTreeSet};

use ambition_entity_catalog::{HurtboxDoc, MoveEventKind, MovesetContract};
use ambition_platformer_primitives::binding::{BindingLedger, BindingReport, Namespace, Resolver};

/// The cues a session authorizes. A character's authored cues resolve against
/// this; §4.6 note — a session's authorized set is NOT merely the union over its
/// cast, it also includes stage, ruleset, announcer, world-object, UI and shell
/// dependencies, so the authority is assembled session-level and passed in.
pub struct SfxCueId;

impl Namespace for SfxCueId {
    const NAME: &'static str = "sfx cue";
}

/// The move ids one character's moveset declares. Character-scoped: `swat` in one
/// character has nothing to do with `swat` in another.
pub struct MoveId;

impl Namespace for MoveId {
    const NAME: &'static str = "move";
}

/// The input verbs the moveset runtime can actually press.
///
/// Engine-scoped, unlike [`MoveId`]: a verb is a word content and runtime have to
/// agree on, and the runtime's list is fixed. A moveset binding a verb outside it
/// authors a perfectly valid move onto a button that does not exist.
pub struct VerbId;

impl Namespace for VerbId {
    const NAME: &'static str = "input verb";
}

/// Sheet manifest targets the composition can actually resolve.
///
/// A character's `sheet` is the single most consequential cross-layer reference it
/// makes — get it wrong and the character draws a marked rectangle for the rest of
/// the session. It was never resolved at preparation, so a typo here was reported
/// only later, by the art pipeline, as `NoSheetResolved`: true, but at load time
/// and without a did-you-mean.
pub struct SheetTarget;

impl Namespace for SheetTarget {
    const NAME: &'static str = "sheet target";
}

/// Select-screen portrait targets.
pub struct PortraitTarget;

impl Namespace for PortraitTarget {
    const NAME: &'static str = "portrait target";
}

/// The vfx tags a session's renderers know how to draw.
///
/// §4.6 derives the vfx inventory from the moves that request it, exactly like
/// cues — and then nothing resolved it, so a misspelled `vfx` on a hit volume was
/// derived faithfully into a dependency list nobody checked.
pub struct VfxTag;

impl Namespace for VfxTag {
    const NAME: &'static str = "vfx tag";
}

/// Non-authoritative provenance for a generated crossover variant (§4.3).
///
/// `mary_o` and `mary_o_smash` are two independent, fully-resolved products with
/// distinct stable ids, emitted by one generator from shared source. The engine
/// **never learns what a mode is** — there is no patch layer and no override
/// precedence — and it must not interpret any of this as a balance layer. It
/// exists so a derived character is reproducible.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lineage {
    pub derived_from: Option<String>,
    pub generator_revision: Option<String>,
    pub source_fingerprint: Option<String>,
}

/// Physical limits and vitals. Gameplay numbers, flat.
#[derive(Debug, Clone, PartialEq)]
pub struct Vitals {
    pub max_health: i32,
    pub mass: f32,
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            max_health: 1,
            mass: 1.0,
        }
    }
}

/// Where a body's collision geometry comes from (§4.11, §5).
#[derive(Debug, Clone, PartialEq)]
pub enum BodySource {
    /// The sheet authors it, per pose (`SpritePosedBody`).
    SpriteAuthored { world_per_pixel: f32 },
    /// Explicit authored half-extents.
    Explicit { half_extents: (f32, f32) },
}

/// **One authored character.** Sections may be inline or referenced.
///
/// Note what is NOT here: `default_brain` (§4.7 — control assignment is a session
/// binding on a participant, not an identity trait) and any hand-listed cue
/// vocabulary (§4.6 — derived).
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterDefinition {
    pub id: String,
    pub display_name: String,
    /// Attribution and asset roots. NOT authority: a provider does not own the
    /// right to reinterpret engine rules for its characters.
    pub provider: String,
    pub lineage: Option<Lineage>,
    /// The sheet manifest target this character's art resolves through.
    pub sheet: Option<String>,
    /// Select-screen portrait. Loads WITHOUT the sheet, so an enumeration screen
    /// costs no sheet decode.
    pub portrait: Option<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    pub moveset: Option<MovesetContract>,
}

impl CharacterDefinition {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            provider: provider.into(),
            lineage: None,
            sheet: None,
            portrait: None,
            body: None,
            hurtboxes: None,
            vitals: Vitals::default(),
            moveset: None,
        }
    }

    pub fn with_moveset(mut self, moveset: MovesetContract) -> Self {
        self.moveset = Some(moveset);
        self
    }

    pub fn with_sheet(mut self, sheet: impl Into<String>) -> Self {
        self.sheet = Some(sheet.into());
        self
    }

    pub fn with_hurtboxes(mut self, doc: HurtboxDoc) -> Self {
        self.hurtboxes = Some(doc);
        self
    }
}

/// **A prepared character: flat, immutable, no inheritance left to resolve.**
///
/// The session consumes resolved values. That is the real invariant behind §4.3 —
/// not "sharing must live in a generator", but that nothing downstream re-derives
/// a character from parents, patches, or a string search.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCharacterDefinition {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub lineage: Option<Lineage>,
    pub sheet: Option<String>,
    pub portrait: Option<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    pub moveset: Option<MovesetContract>,
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

    /// Namespaces preparation resolved against a real vocabulary.
    pub fn checked_namespaces(&self) -> impl ExactSizeIterator<Item = &&'static str> {
        self.checked.iter()
    }

    /// Was this namespace actually verified for this character?
    ///
    /// `false` means NOT CHECKED — no resolver was supplied — and says nothing
    /// about whether the references are good.
    pub fn was_checked(&self, namespace: &str) -> bool {
        self.checked.iter().any(|name| *name == namespace)
    }

    /// The token the engine materializer demands for this character's art.
    ///
    /// Its own id: the sheet table is keyed by catalog id and display name, and a
    /// character that shares a sibling's sheet by reference still demands under
    /// its own id so the load ledger names the right character.
    pub fn art_load_token(&self) -> &str {
        &self.id
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
                MoveEventKind::Vfx { effect } if !effect.is_empty() => {
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

    pub fn with_available_portraits<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.portraits = Some(Resolver::new(targets));
        self
    }

    /// Fill in the engine's baked sheet vocabulary unless the caller supplied one.
    ///
    /// Kept OUT of `prepare_character`, which stays a pure function of its
    /// arguments: reaching into a baked global from inside preparation would make
    /// the same definition prepare differently depending on the build. This is the
    /// registration seam's job, because registration is where the engine is.
    pub fn with_engine_sheet_vocabulary(mut self) -> Self {
        if self.sheets.is_none() {
            self.sheets = Some(Resolver::new(
                ambition_sprite_sheet::character::sheets::available_targets(),
            ));
        }
        self
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
        // NOT listed, deliberately: `BodySource` is an inline enum, not a
        // reference — `SpriteAuthored { world_per_pixel }` and
        // `Explicit { half_extents }` name nothing outside the character — so
        // there is no "body" namespace to resolve. GPT-5.6's review listed bodies
        // alongside sheets and portraits; that part of the finding does not apply,
        // and inventing a namespace to satisfy it would be a resolver that always
        // succeeds.
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
pub struct PreparedCharacter {
    pub prepared: PreparedCharacterDefinition,
    pub report: BindingReport,
    /// Namespaces that were actually resolved.
    pub checked: Vec<&'static str>,
}

impl PreparedCharacter {
    /// True when every reference preparation COULD check did resolve.
    pub fn is_clean(&self) -> bool {
        self.report.is_empty()
    }
}

/// Validate and flatten one authored definition.
///
/// Not fatal on an unresolved reference: the report is what makes the degradation
/// loud, and a character that draws a placeholder and says why beats a session
/// that refuses to boot. Where a defect genuinely should refuse publication —
/// malformed movement inheritance (§4.4) — that refusal lives at the seam that
/// owns it and names the whole chain.
/// Every input verb the moveset runtime resolves.
///
/// The four bases the trigger path asks for, each with the directional and
/// airborne suffixes `directional_verb_chain` produces. Built rather than
/// listed so it cannot drift from the chain that consumes it: if a fifth base
/// or a fifth direction is added, this is the one place that has to learn about
/// it, and every character's registration starts checking against it for free.
fn runtime_verb_vocabulary() -> Vec<String> {
    use crate::combat::moveset::{ATTACK_VERB, RANGED_VERB, SMASH_VERB, SPECIAL_VERB};
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
    vocabulary.sort();
    vocabulary.dedup();
    vocabulary
}

pub fn prepare_character(
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
        //
        // The failure this exists for was worse than unreachable: the arena's
        // first hand-authored fighter swung, spawned a hitbox and made a sound,
        // and the hitbox was inert, because a downstream reader inferred
        // melee-ness from the move's ID rather than its verb. That coupling is
        // gone, but the class is not — content and runtime agreeing about a
        // string is exactly what a resolver is for.
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
    let prepared = PreparedCharacterDefinition {
        id: definition.id,
        display_name: definition.display_name,
        provider: definition.provider,
        lineage: definition.lineage,
        sheet: definition.sheet,
        portrait: definition.portrait,
        body: definition.body,
        hurtboxes: definition.hurtboxes,
        vitals: definition.vitals,
        moveset: definition.moveset,
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

/// The prepared authority: one entry per character, keyed by stable id.
///
/// ⚠ §4.1's end state is that subsystem read models are DERIVED from this rather
/// than registered beside it. Where that stands today, checked against the code
/// on 2026-07-28 rather than inherited from the last time somebody wrote it down:
///
/// * **Derived from here:** sprite declarations, cue authorization, each body's
///   presentation source, and — since the projection landed — the authored
///   MOVESET and HURTBOX DOC of any body whose identity resolves to a registered
///   character (`project_prepared_character_definitions`, plus the moveset
///   consulted inside `avatar::starting_character`'s one construction so the
///   action set and the moveset are built together).
/// * **Still the catalog's:** the ACTION SET, the identity baseline, and movement
///   tuning. `CharacterCatalog` remains the authority for those, and a registered
///   character that authors none of them is the ordinary case.
///
/// This comment previously said registering a character "does not yet cause a
/// production-spawned body to receive what that character authored — the §7.10
/// fight test projects it by hand". That stopped being true when the projection
/// landed and the hand-projection was deleted; nothing re-read the comment,
/// because a doc comment describing an ABSENCE has no citation to rot (queue W1).
/// Tracked as C3.
#[derive(bevy::prelude::Resource, Debug, Clone, Default)]
pub struct PreparedCharacterRegistry {
    by_id: BTreeMap<String, PreparedCharacterDefinition>,
}

impl PreparedCharacterRegistry {
    pub fn get(&self, id: &str) -> Option<&PreparedCharacterDefinition> {
        self.by_id.get(id)
    }

    pub fn ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.by_id.keys().map(String::as_str)
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
    /// `pub(crate)` and named for what it is: the registration seam is the door
    /// (it rejects duplicate ids and ambiguous display names), and this is the
    /// hatch a focused test uses when it wants a registry without an `App`.
    #[cfg(test)]
    pub(crate) fn insert_prepared(&mut self, prepared: PreparedCharacterDefinition) {
        self.insert(prepared);
    }

    fn insert(&mut self, prepared: PreparedCharacterDefinition) -> Option<String> {
        let id = prepared.id.clone();
        match self.by_id.insert(id.clone(), prepared) {
            Some(previous) => Some(previous.provider),
            None => None,
        }
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
    /// Rooms, LDtk entities, and roster entries all legitimately name characters
    /// by the label a designer typed, so a display name is an addressing key
    /// whether or not it was meant to be one — and three authorities resolve it
    /// independently. `PreparedCharacterRegistry::id_for_display_name` takes the
    /// first match in id order; `CharacterSpriteAssets::declare` inserts the alias
    /// into a map, so the LAST declaration wins. With `alpha` and `zeta` both
    /// presenting as "Hero", a demand for "Hero" could stage `alpha`, authorize
    /// `alpha`'s provider, and decode `zeta`'s sheet — a fighter with one
    /// character's sounds and another's body (GPT 5.6, 2026-07-26).
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

/// **The single registration seam.** (§4.1)
///
/// Prepares the definition and publishes it into the prepared authority. A
/// provider makes ONE call and does not have to know that sheets, cues, and
/// gameplay numbers are consumed by different subsystems.
///
/// # Registration is DECLARATIVE — it does not load anything
///
/// This used to end by calling `CharacterLoadDemand::request`, on the reasoning
/// that a provider should not need a second call. That reasoning was wrong, and
/// the mistake gets worse the better the plan works: as more characters migrate
/// onto this seam, merely *installing* a provider's plugin would demand every one
/// of its characters' art. That is precisely the startup decode storm §7.1
/// deleted — four privileged ids decoding at boot because someone decided in
/// advance which characters mattered — rebuilt from the other end.
///
/// Loading is driven by a PROJECTION of what a session actually stages: a room
/// plan, a match roster, a startup spec, or a body putting on an identity
/// (`StagesCharacters`). Registration says *what exists*; staging says *what is
/// needed now*. A game with fifty registered fighters and two on screen decodes
/// two sheets.
///
/// The binding report is logged rather than returned as an error: see
/// [`prepare_character`] for why an unresolved reference degrades loudly instead
/// of refusing.
pub trait CharacterDefinitionAppExt {
    fn try_register_character(
        &mut self,
        definition: CharacterDefinition,
        bindings: CharacterBindings,
    ) -> Result<&mut Self, CharacterRegistrationError>;

    fn register_character(&mut self, definition: CharacterDefinition) -> &mut Self {
        self.try_register_character(definition, CharacterBindings::default())
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl CharacterDefinitionAppExt for bevy::prelude::App {
    fn try_register_character(
        &mut self,
        definition: CharacterDefinition,
        bindings: CharacterBindings,
    ) -> Result<&mut Self, CharacterRegistrationError> {
        if definition.id.trim().is_empty() {
            return Err(CharacterRegistrationError::BlankId);
        }
        let provider = definition.provider.clone();
        // The engine supplies its OWN vocabulary. A sheet target resolves against
        // the baked manifest index, which the engine always knows, so every
        // registration gets its sheet reference checked with a did-you-mean whether
        // or not the provider thought to pass a resolver. A boundary that only
        // works when the caller opts in is a boundary most callers will not have.
        let bindings = bindings.with_engine_sheet_vocabulary();
        let PreparedCharacter {
            prepared, report, ..
        } = prepare_character(definition, &bindings);
        let id = prepared.id.clone();

        // Transactional: assemble the candidate, and only publish if the id is
        // free. A rejected registration leaves the previous authority active.
        let mut candidate = self
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .cloned()
            .unwrap_or_default();
        // A display name already spoken for by a DIFFERENT id is rejected before the
        // insert, so the registry can never hold the ambiguity that
        // `id_for_display_name` would then have to resolve arbitrarily.
        if let Some(first_id) = candidate.id_for_display_name(&prepared.display_name) {
            if first_id != prepared.id {
                return Err(CharacterRegistrationError::AmbiguousDisplayName {
                    display_name: prepared.display_name.clone(),
                    first_id: first_id.to_string(),
                    second_id: prepared.id.clone(),
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
        self.insert_resource(candidate);

        if !report.is_empty() {
            report.log(&format!("preparing character `{id}`"));
        }
        Ok(self)
    }
}
