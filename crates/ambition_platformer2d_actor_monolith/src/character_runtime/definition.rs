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
//!         |  prepare_character(...)      validates + flattens
//!         v
//! PreparedCharacterOverrides   PARTIAL. `None` still means "ask the catalog",
//!         |                    and this module is the only one that can say it
//!         |  Plugin::finish              folds the catalog in, once, for the
//!         v                              whole cast, transactionally
//! PreparedCharacterDefinition  COMPLETE, immutable, no inheritance left, no
//!                              string search in authoritative gameplay paths
//! ```
//!
//! The middle row is the 2026-07-29 change and the reason the arrows are two.
//! Preparation used to publish the partial value directly, so each construction
//! path resolved `None` against the catalog itself — and the SEATED path could
//! not, because the workspace policy `engine.character-authority-is-app-local`
//! puts the catalog beyond an engine-side projection's reach. A worn player and
//! a seated fighter wearing the same character therefore disagreed about that
//! character's kit. See `docs/planning/character-preparation-finalization-plan.md`.
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
//! [binding resolution boundary](ambition_platformer2d_shared_tangle::binding), so a
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
use ambition_platformer2d_shared_tangle::binding::{
    BindingLedger, BindingReport, Namespace, Resolver,
};

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

/// The ranged payload an authored `ranged` move needs to throw.
///
/// Not a lookup namespace like the others — there is no table of payloads to
/// misspell. It is here because a `ranged` move whose action set supplies no
/// projectile is the same CLASS of failure every other resolver reports (content
/// disagreeing with content, silently, until a playtest), and reporting it
/// through the same channel means one place to read and one guard to watch.
pub struct RangedPayload;

impl Namespace for RangedPayload {
    const NAME: &'static str = "ranged payload";
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
///
/// ⚠ **both fields are `Option` and that is load-bearing**, for the same reason
/// every other kit field on a definition is: `None` means *the author said
/// nothing*, which is a question, and `Some` is an answer that outranks whatever
/// would have answered it.
///
/// They used to be flat, with `max_health` defaulting to `1`. That default was
/// indistinguishable from an authored one-hit glass cannon — so the value could
/// not be applied to a body without nerfing every character that had never
/// thought about health, which is why only the SEATING path ever read it and the
/// worn path read the catalog instead. Two construction paths, two answers, one
/// character (GPT 5.6, 2026-07-29). Making the absence expressible is what lets
/// one applier serve every path.
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
    /// Reaches a body as [`Mass`](crate::features::Mass), which drives the mount
    /// pair's mass-weighted centre of gravity (ADR 0020): a heavy mount keeps the
    /// COG near itself, so the lighter rider orbits it on a gravity flip.
    ///
    /// `None` leaves the body's own mass alone — which for a seated fighter is
    /// the one its roster archetype set. Authoring `Some(1.0)` and saying nothing
    /// are different claims even though 1.0 is the ambient default, and only the
    /// first one may overwrite an archetype.
    ///
    /// ⚠ this said "AUTHORED AND UNCONSUMED — no production code reads this,
    /// verified by grep". The grep was right about the FIELD and wrong about the
    /// concept: `Mass` already existed, already rewound, and was populated from
    /// the ROSTER archetype and never from here. So this was not a dead field, it
    /// was a second declaration of a fact only the roster could state — and
    /// "delete it" was very nearly the recommendation (2026-07-29).
    pub mass: Option<f32>,
    /// **How hard this body is to LAUNCH** — the knockback weight, reaching a
    /// body as [`CombatTuning::weight`](crate::combat::CombatTuning). `1.0` is
    /// the reference body; a heavy fighter authors more and takes less of the
    /// growth term (`scaled_knockback` divides by it).
    ///
    /// ⚠ **distinct from [`Self::mass`], which is the mount pair's centre of
    /// gravity.** They are the same word in physics and two different mechanics
    /// here, and conflating them would make a heavy mount hard to knock about
    /// as a side effect of how its rider orbits it.
    ///
    /// `None` leaves the body's own — for a clustered actor that is its roster
    /// archetype's, which is the ONLY place a weight could be stated until now.
    /// Two characters seated from one archetype therefore weighed the same and
    /// could not differ, which is a per-character fact in every platform fighter
    /// that has one (D73 phase 1).
    pub knockback_weight: Option<f32>,
}

/// Where a body's collision geometry comes from (§4.11, §5).
///
/// Both variants are CONSUMED, and each by the authority that owns the fact
/// (wired 2026-07-29 — until then this field had no reader anywhere, so a
/// provider could author a body and receive some other size entirely):
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
/// ⚠ the split is not arbitrary. A projection that resized a LIVE body would be
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

/// **One authored character.** Sections may be inline or referenced.
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
    /// ⚠ **preparation RESOLVES this reference; no runtime consumes it**, and the
    /// reason is structural rather than an oversight (checked 2026-07-29).
    ///
    /// This declares a portrait TARGET — a name. Dialogue resolves a speaker's
    /// portrait through `CharacterCatalog::portrait_ref`, which yields concrete
    /// `{ image, manifest, default_clip }` paths. There is no
    /// target → portrait-art resolver anywhere, so an authored target has nothing
    /// to resolve THROUGH and reaches nothing.
    ///
    /// Either that resolver gets built — the sheet path is exactly this shape,
    /// `SheetTarget` resolving a name to a manifest — or this field goes and the
    /// catalog owns portraits outright. ⛔ what it must NOT become is a copy of
    /// the catalog's concrete paths: two places declaring the same art is the
    /// split this campaign exists to remove.
    pub portrait: Option<String>,
    /// **Lines this character says when nothing more specific does.**
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
    /// pedestals (Jon, 2026-07-29).
    ///
    /// ⚠ the LOWEST-precedence voice, not a dialogue system: a yarn node wins,
    /// then the catalog row's situation pool, then its `fallback_dialogue`, then
    /// this. It exists so the floor is "says something in character" rather than
    /// silence.
    pub voice: Vec<String>,
    pub body: Option<BodySource>,
    pub hurtboxes: Option<HurtboxDoc>,
    pub vitals: Vitals,
    /// **What this body does when it DIES, and what it drops** — explode,
    /// divide, crash, or refuse to die at all.
    ///
    /// D73 phase 1. These are properties of the creature, and until now the ONLY
    /// producer of [`crate::combat::CombatCapabilities`] in the workspace was
    /// `ArchetypeSpecExt::combat_capabilities` — so a mite that splits when
    /// killed could say so as an archetype and a registered character could not
    /// say it at all. A seated fighter and a worn player simply had no death
    /// traits, whatever they were.
    ///
    /// `None` means the author said nothing, and nothing is inserted — today's
    /// behaviour for every character in the repo. ⚠ absence RETRACTS on a
    /// re-wear, like every other physical fact a persona claims: wearing a
    /// sandbag and then a duelist must not leave the duelist unkillable.
    pub death_traits: Option<ambition_characters::actor::CharacterDeathTraits>,
    /// **What this character normally DOES when nothing overrides it** — the
    /// name of an autonomous-controller profile (a catalog `brain_presets` key).
    ///
    /// ⚠ **not the current controller, and the distinction is the whole rule.**
    /// Jon, 2026-08-10: *"a character definition may name a default autonomous
    /// controller profile … that does not mean the controller is intrinsic
    /// identity. Possessing a Goblin changes who drives the Goblin. It does not
    /// change what a Goblin is."* A human, a CPU, a replay or a policy may drive
    /// this body; this only says what happens when none of them does.
    ///
    /// Precedence, resolved by `resolve_initial_brain`: an authored placement
    /// override wins, then this, then the catalog row's `default_brain`. `None`
    /// leaves the row in charge, which is every character in the repo today.
    ///
    /// ⚠ **a provider-relative REFERENCE, not a resolved catalog key.**
    /// `resolve_initial_brain` qualifies it into the character's own provider
    /// namespace exactly as it qualifies an authored placement override, so a
    /// definition and a placement cannot mean different things by the same word.
    /// The type says which of the two it is: [`BrainProfileRef`] is what content
    /// writes, [`BrainPresetId`] is the canonical key it resolves to.
    ///
    /// [`BrainProfileRef`]: ambition_characters::actor::character_catalog::BrainProfileRef
    /// [`BrainPresetId`]: ambition_characters::actor::character_catalog::BrainPresetId
    pub default_brain_profile:
        Option<ambition_characters::actor::character_catalog::BrainProfileRef>,
    pub moveset: Option<MovesetContract>,
    /// What this character CAN do — melee, ranged, special, locomotion style.
    ///
    /// `None` means "the author said nothing", and the catalog row's
    /// `default_action_set` stands. `Some(ActionSet::default())` means "the
    /// author said NOTHING APPLIES", which is a different statement and has to
    /// survive as one: Sanic's kit is the momentum ride and the ball dash, and
    /// a resolver that treats an authored-empty set as unauthored hands him a
    /// punch (queue C3 / architecture campaign X3, R-b).
    ///
    /// Splitting this from [`Self::moveset`] is the identity split C3 is about.
    /// A moveset says what the MOVES are; an action set says what the body and
    /// the AI believe the body can reach for. Leaving the second exclusively in
    /// the catalog means a definition can author moves the brain does not know
    /// exist — and it makes a ranged move depend on a projectile specification
    /// from a different authority entirely (GPT 5.6, 2026-07-28).
    pub action_set: Option<ambition_characters::brain::ActionSet>,
    /// How this character MOVES — the state-free movement policy.
    ///
    /// The third leg of an identity's kit, beside the action set and the moveset,
    /// and the last one that was still exclusively the catalog's. A provider
    /// authoring a character that runs on momentum rather than swept axes had to
    /// say so in a catalog row even when it authored everything else on the
    /// definition (queue C3 / campaign R-a, deferred from the first slice
    /// deliberately and landed 2026-07-28).
    ///
    /// `None` means the catalog row stands, which is every character that has not
    /// authored one.
    pub motion_model: Option<ambition_platformer2d_core::MotionModelSpec>,
    /// Per-character axis FEEL — run accel, jump speed, coyote time, the rest.
    ///
    /// Distinct from [`Self::motion_model`], which picks the SOLVER. This is that
    /// solver's numbers, and it is the last kit-adjacent field that was still
    /// exclusively the catalog's.
    ///
    /// ⚠ `None` and `Some` are load-bearing here in a way they are not elsewhere:
    /// the marker component's PRESENCE means "this body's tuning is authored,
    /// not the shared dev tuning", so an unauthored character must end up with no
    /// marker rather than with a defaulted one. A re-wear from an authored feel
    /// back to the sandbox protagonist has to return the body to the live
    /// inspector sliders.
    pub movement_tuning: Option<ambition_platformer2d_core::MovementTuning>,
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
            default_brain_profile: None,
            moveset: None,
            action_set: None,
            motion_model: None,
            movement_tuning: None,
        }
    }

    /// Author what this character normally does when nothing overrides it.
    /// See [`Self::default_brain_profile`].
    pub fn with_default_brain_profile(
        mut self,
        profile: impl Into<ambition_characters::actor::character_catalog::BrainProfileRef>,
    ) -> Self {
        self.default_brain_profile = Some(profile.into());
        self
    }

    /// Author what this character does when it dies. See the field.
    pub fn with_death_traits(
        mut self,
        traits: ambition_characters::actor::CharacterDeathTraits,
    ) -> Self {
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
    pub fn with_action_set(mut self, action_set: ambition_characters::brain::ActionSet) -> Self {
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

    /// **Give this character a face**, by naming a portrait TARGET.
    ///
    /// ⛔ **the field existed for months and was unauthorable**: there was no
    /// builder, so `portrait: None` in the constructor was the only value it
    /// ever held, and nothing read it either. Both halves are closed now — see
    /// `character_sprites::assets::portrait_for_declared_character` for what a
    /// target resolves THROUGH.
    ///
    /// ⚠ a NAME (`"alice"`), not a path. Paths are what the catalog derives from
    /// the gameplay sheet's own name; a definition naming concrete paths would
    /// be the second declaration of the same art that this field's doc forbids.
    /// A character that authors nothing here keeps the catalog's answer, which
    /// is how every character in the repo resolves today.
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.portrait = Some(portrait.into());
        self
    }

    /// **Hand this character's body geometry to its spritesheet.**
    ///
    /// `world_per_pixel` is the ONE number: how much world one sheet pixel
    /// covers. The collision box, the sprite quad and the quad's offset all
    /// follow from the art at that scale, so none of the three can drift from
    /// the other two. See [`BodySource`].
    pub fn with_sprite_authored_body(mut self, world_per_pixel: f32) -> Self {
        self.body = Some(BodySource::SpriteAuthored { world_per_pixel });
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

/// **What one authored definition OVERRIDES, before the catalog is folded in.**
///
/// The output of [`prepare_character`], and the input to finalization — never a
/// runtime value. Every kit field here is an `Option` whose `None` means *the
/// author said nothing*, which is a question and not an answer: the body cannot
/// act on it without also consulting the catalog row.
///
/// # This type is deliberately unnameable outside this module
///
/// Not `pub`, not `pub(crate)`, and that visibility is the entire mechanism.
/// Preparation used to publish this partial value AS the runtime authority, so
/// both body-construction paths had to re-resolve `None` against the catalog
/// themselves — and only one of them did. A seated fighter and a worn player
/// wearing the same character disagreed about that character's kit for a day
/// (campaign H1, 2026-07-28).
///
/// Two types with the same visibility would just be that bug with a longer name.
/// Because `presentation`, `seating`, and `avatar::starting_character` are
/// siblings of this module rather than children, they *cannot* read a partial
/// value: the phase split is checked by the compiler instead of by review.
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
    death_traits: Option<ambition_characters::actor::CharacterDeathTraits>,
    /// See [`CharacterDefinition::default_brain_profile`]. Carried through
    /// unchanged — the catalog is not consulted, because the FOLD's job is to
    /// answer what a character IS and this is a default the resolver applies at
    /// spawn, where the placement's own override is also visible.
    default_brain_profile: Option<ambition_characters::actor::character_catalog::BrainProfileRef>,
    moveset: Option<MovesetContract>,
    /// The authored action set, carried through preparation unchanged.
    ///
    /// `None` and `Some(empty)` mean different things all the way to the body —
    /// see [`CharacterDefinition::action_set`].
    action_set: Option<ambition_characters::brain::ActionSet>,
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

/// **Where a prepared character's fighting kit comes from.**
///
/// The one honest answer to "what does this character reach for", decided ONCE
/// at finalization instead of re-decided by each construction path.
///
/// Two variants and not one, because exactly one case is genuinely undecidable
/// before a body exists: the host's code-side protagonist kit is built from that
/// body's own persisted `AbilitySet`, so no per-character value can hold it.
/// Naming that case is the point — the alternative is a "complete" definition
/// that quietly is not, which is the bug class this whole split exists to close.
#[derive(Debug, Clone, PartialEq)]
pub enum PreparedKit {
    /// Content decided: this action set, these moves. Whether the decision came
    /// from the definition or from the catalog row it inherited is finalization's
    /// business and nobody else's.
    ///
    /// The moveset is never `None` here. An authored one wins; otherwise it is
    /// DERIVED from the winning action set, which is what a body that authored
    /// capabilities and no explicit timeline needs in order to swing at all.
    Authored {
        action_set: ambition_characters::brain::ActionSet,
        moveset: MovesetContract,
    },
    /// The host's code-side kit, rebuilt per body from its `AbilitySet`.
    ///
    /// `authored_moveset` is still honoured: a character may take the host kit's
    /// capabilities and bring its own timelines.
    HostCode {
        authored_moveset: Option<MovesetContract>,
    },
}

impl PreparedKit {
    /// The action set content decided on, or `None` when only a body can say.
    pub fn action_set(&self) -> Option<&ambition_characters::brain::ActionSet> {
        match self {
            Self::Authored { action_set, .. } => Some(action_set),
            Self::HostCode { .. } => None,
        }
    }

    /// The moveset to put on a body that is not building the host kit itself.
    pub fn projectable_moveset(&self) -> Option<&MovesetContract> {
        match self {
            Self::Authored { moveset, .. } => Some(moveset),
            Self::HostCode { authored_moveset } => authored_moveset.as_ref(),
        }
    }
}

/// **A prepared character: flat, immutable, and COMPLETE.**
///
/// The session consumes resolved values. That is the real invariant behind §4.3 —
/// not "sharing must live in a generator", but that nothing downstream re-derives
/// a character from parents, patches, or a string search.
///
/// Complete is the word that changed on 2026-07-29. This value used to be the
/// output of [`prepare_character`], carrying `Option` kit fields whose `None`
/// meant "ask the catalog" — so every construction path had to hold the catalog
/// and perform the same fold, and the seated path could not (the workspace policy
/// `engine.character-authority-is-app-local` puts the catalog out of an
/// engine-side projection's reach). Now the fold happens ONCE, at the
/// finalization barrier, and what a body reads has no questions left in it.
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
    pub death_traits: Option<ambition_characters::actor::CharacterDeathTraits>,
    /// The autonomous profile this character normally runs, if it named one —
    /// **RESOLVED**, as a canonical [`BrainPresetId`] rather than the authored
    /// [`BrainProfileRef`] the definition carries.
    ///
    /// ⭐ that difference is what "prepared" is supposed to mean. Preparation
    /// qualifies the authored reference into its namespace once, so nothing
    /// downstream has to consult the catalog to find out what the character
    /// already said about itself. `None` still leaves the catalog row's
    /// `default_brain` in charge.
    ///
    /// [`BrainPresetId`]: ambition_characters::actor::character_catalog::BrainPresetId
    /// [`BrainProfileRef`]: ambition_characters::actor::character_catalog::BrainProfileRef
    pub default_brain_profile: Option<ambition_characters::actor::character_catalog::BrainPresetId>,
    /// What this character fights with — resolved, not inherited.
    pub kit: PreparedKit,
    /// The movement policy, resolved. Every body already carries exactly one
    /// explicit model, so this is a value rather than a question.
    pub motion_model: ambition_platformer2d_core::MotionModelSpec,
    /// The movement feel, resolved.
    ///
    /// ⚠ still an `Option`, and its `None` is now an ANSWER rather than a
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

    /// **A line this character says when nothing more specific does.**
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

    /// Every line this character brought.
    pub fn voice(&self) -> impl ExactSizeIterator<Item = &str> {
        self.voice.iter().map(String::as_str)
    }

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
    // ⭐ the CONTRACT's crate, not the runtime's: a verb name is authoring
    // vocabulary. One of the couplings that kept this file out of
    // `ambition_characters` (D73 appendix C).
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
        // **The moveset and the action set have to agree about RANGED.** (C3)
        //
        // A move on the `ranged` verb needs a projectile to throw, and the
        // projectile specification lives on the ACTION SET, not on the move. So
        // a character authoring both — the case the C3 precedence work makes
        // possible — can now author a ranged move and an action set with no
        // ranged payload, and the two are individually valid: the verb is real,
        // the move is real, the set is real. The button does nothing.
        //
        // This is preparation's job precisely because neither half is wrong on
        // its own; only the PAIR is, and preparation is the only place both are
        // in hand (GPT 5.6, 2026-07-28). Reported as a binding failure so it
        // travels the same route every other content disagreement does — named,
        // non-fatal, and visible to the guard.
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
                    ledger.record(
                        ambition_platformer2d_shared_tangle::binding::UnresolvedRef {
                            namespace: RangedPayload::NAME,
                            id: target.clone(),
                            declared_by: format!("{declared_by} verb `{verb}`"),
                            // Nothing WAS available, and saying so is the whole
                            // report: the character authored an action set and left
                            // `ranged` empty, so there is no candidate to suggest
                            // and no typo to find. The fix is authoring, not
                            // spelling.
                            available: Vec::new(),
                            did_you_mean: None,
                        },
                    );
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
        default_brain_profile: definition.default_brain_profile,
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

/// **Fold one character's overrides against the catalog into a complete value.**
///
/// The whole campaign in one function. Precedence, unchanged from what the worn
/// path used to do inline: an explicitly authored value outranks the catalog row;
/// `None` means the author said nothing and the row stands; and `Some(empty)` is
/// an authoring DECISION that outranks the row exactly as a filled one does.
///
/// What CHANGED is where it happens. This used to run per body, in
/// `apply_worn_character_kit`, which meant a construction path without a catalog
/// could not do it — and `project_prepared_character_definitions`, the path that
/// serves every SEATED fighter, is exactly such a path. So a seated fighter
/// inherited nothing while a worn player wearing the same character inherited
/// everything.
///
/// `catalog: None` is a real composition, not a degraded one: a bare engine App
/// that registers characters and installs no catalog has nothing to inherit FROM,
/// which is the same answer as "this id is not in the catalog" — the case the
/// runtime already handled by installing the host compatibility kit.
fn finalize_character(
    overrides: PreparedCharacterOverrides,
    catalog: Option<&ambition_characters::actor::character_catalog::CharacterCatalog>,
) -> PreparedCharacterDefinition {
    use ambition_characters::actor::character_catalog::PlayableKitSource;
    use ambition_characters::brain::ActionSet;

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
        default_brain_profile,
        moveset,
        action_set,
        motion_model,
        movement_tuning,
        cue_dependencies,
        vfx_dependencies,
        checked,
        unresolved,
    } = overrides;

    // THE KIT. Three outcomes, and which one a character gets is decided here
    // once rather than by whichever construction path reaches it first.
    let kit = match action_set {
        // The definition authored capabilities. Nothing else gets a vote.
        Some(set) => PreparedKit::Authored {
            moveset: derive_moveset(&set, moveset),
            action_set: set,
        },
        None => match catalog.and_then(|catalog| catalog.playable_kit_source(&id)) {
            Some(PlayableKitSource::Authored) => {
                let set = catalog
                    .and_then(|catalog| catalog.build_default_action_set(&id))
                    .unwrap_or_else(|| {
                        // A known Authored row whose preset does not resolve is
                        // malformed content. Reported ONCE here rather than every
                        // time a body wears it, and the body still gets a safe
                        // peaceful kit rather than silent host privileges.
                        bevy::log::error!(
                            "character `{id}` declares an Authored playable kit but its \
                             default_action_set does not resolve; preparing a safe peaceful kit"
                        );
                        ActionSet::peaceful()
                    });
                PreparedKit::Authored {
                    moveset: derive_moveset(&set, moveset),
                    action_set: set,
                }
            }
            // A `HostCode` row, or an id the catalog does not know, or no catalog
            // at all. All three mean the same thing to a body: build the host kit
            // from what this body can do.
            // ⚠ **AND THE ONE CONTRADICTION THE PLAN SAID DID NOT EXIST.**
            //
            // A character can take the host-code kit — whose action set the HOST
            // builds, so the definition authors none — and still bring its own
            // timelines; `authored_moveset` exists precisely for that. If that
            // moveset declares the `ranged` verb, the same press is owned twice:
            // by the legacy charge-projectile path this kit installs, and by the
            // moveset's ranged verb. That is the exact double-ownership
            // `RangedExecution::HostCharge` exists to prevent, arriving through
            // the one door it does not watch.
            //
            // The finalization plan recorded that there was no contradictory
            // authored HostCode configuration to reject, and that a validator
            // would therefore be a test of itself. There is one (GPT 5.6,
            // 2026-07-29).
            //
            // Decided HERE and not in `prepare_character`'s binding ledger,
            // because deciding this needs the CATALOG and the catalog is
            // deliberately not in scope until finalization — the Phase A split.
            //
            // ⚠ **and REPORTING it was not enough**, which is the second half of
            // the same finding. A diagnostic left the contradictory kit intact and
            // published it, so runtime still installed both owners and the log line
            // was a description of a bug rather than a fix for one. Invalid
            // ownership must not reach a body at all (GPT 5.6, second pass).
            _ => PreparedKit::HostCode {
                authored_moveset: moveset.map(|mut moveset| {
                    let revoked = revoke_host_owned_ranged(&mut moveset);
                    if !revoked.is_empty() {
                        bevy::log::error!(
                            "character `{id}` takes the host-code kit AND authored the ranged \
                             verb(s) {revoked:?}. The host kit owns the ranged press through its \
                             charge-projectile path, so one press would have fired both; those \
                             verb bindings are DROPPED and the charge path keeps the press. To own \
                             the verb from content instead, author an action set — that makes the \
                             character `Authored`, and its moveset owns ranged outright"
                        );
                    }
                    moveset
                }),
            },
        },
    };

    PreparedCharacterDefinition {
        // **HEALTH FOLDS LIKE EVERY OTHER KIT FIELD**, and it is the last one
        // that did not. The catalog row has carried `max_health: Option<i32>`
        // with exactly this `None`-means-unauthored meaning since it was added,
        // and `session::setup` read it directly — so a registered character's
        // authored pool and a catalog row's authored pool were two authorities
        // that never met. Folding here is what lets ONE applier serve the worn
        // player and the seated fighter (GPT 5.6, 2026-07-29).
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
            Some(catalog) => {
                crate::avatar::starting_character::motion_model_spec_for_character_id(catalog, &id)
            }
            None => ambition_platformer2d_core::MotionModelSpec::AxisSwept(Default::default()),
        }),
        movement_tuning: movement_tuning.or_else(|| catalog?.axis_tuning(&id)),
        death_traits,
        // **RESOLVED HERE, not at spawn.** A prepared definition should hold a
        // canonical identity, not an authored reference someone still has to
        // interpret — otherwise "prepared" means "partly prepared", and the
        // catalog stays in the loop for a fact the character owns.
        //
        // ⭐ **the namespace is the DEFINITION's provider, and preparation no
        // longer consults the catalog to learn it.** It used to read the
        // character's catalog row and borrow the namespace off a neighbouring
        // key (`entry.default_brain`), which meant a character needed a parallel
        // catalog row to be told its own provider — the last thing keeping this
        // fact in the catalog's hands.
        //
        // ⚠ **the earlier refusal to do this was right at the time and is now
        // discharged.** Synthesising `test::patrol_peaceful` from a provider
        // whose presets nobody had namespaced produced a key that existed
        // nowhere, so the two id spaces stayed "assumed equal, never checked".
        // They are checked now:
        // `character_definitions_and_catalog_fragments_share_one_provider_namespace`
        // asserts every registered definition's provider is a provider the
        // catalog registry assembled under, for the shipped composition. A
        // fixture that hits the old trap is a fixture that skipped assembly.
        default_brain_profile: default_brain_profile.map(|reference| {
            ambition_characters::actor::character_catalog::BrainPresetId::new(
                ambition_characters::actor::character_catalog::qualify_in_provider(
                    &provider,
                    reference.as_str(),
                ),
            )
        }),
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

/// **Take the ranged press away from a moveset whose body wears the host kit.**
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
/// ⚠ **`special` used to be deliberately NOT folded in, and that was H2 again one
/// field over.** The reasoning said it is a capability marker on the host compat
/// kit (`bubble_shield`) with no authored move behind it, and that an authored
/// persona drives its special through its own path — true only when that persona
/// authored a MOVESET. The public API does not require one: a definition may
/// carry `action_set.special = Some(..)` and no moveset at all, and `ActionSet
/// ::special`'s own doc says the brain reads `special.is_some()` to decide
/// whether to press it while the execution "is a data-driven move in the body's
/// `ActorMoveset`". So that character advertised a signature move with no
/// timeline behind it — the exact defect H2 closed for `ranged` (GPT 5.6,
/// 2026-07-29).
///
/// Folding it here cannot double-fire: this branch runs ONLY when there is no
/// authored moveset, so there is no second declaration to collide with. A
/// persona that authored its moves still overrides everything derived.
fn derive_moveset(
    action_set: &ambition_characters::brain::ActionSet,
    authored: Option<MovesetContract>,
) -> MovesetContract {
    authored.unwrap_or_else(|| {
        crate::combat::moveset::build_actor_moveset(
            None,
            action_set.melee.as_ref(),
            action_set.ranged.as_ref(),
            action_set.special.as_ref(),
        )
        .unwrap_or_default()
    })
}

/// One character, prepared and finalized, outside an `App`.
#[cfg(test)]
pub(crate) struct FinalizedCharacter {
    pub(crate) prepared: PreparedCharacterDefinition,
    pub(crate) report: BindingReport,
    pub(crate) checked: Vec<&'static str>,
}

#[cfg(test)]
impl FinalizedCharacter {
    /// True when every reference preparation COULD check did resolve.
    pub(crate) fn is_clean(&self) -> bool {
        self.report.is_empty()
    }
}

/// Run the whole pipeline on one definition, with no composition around it.
///
/// Deliberately NOT available to production. The barrier exists because the
/// catalog is not knowable at registration time, so a production caller able to
/// fold early would be choosing to inherit from whatever happened to be
/// installed so far — which is the ordering hazard `Plugin::finish` removes.
#[cfg(test)]
pub(crate) fn prepare_and_finalize_for_test(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
) -> FinalizedCharacter {
    prepare_and_finalize_against_for_test(definition, bindings, None)
}

/// The same, with a catalog to inherit from — the case the barrier exists for.
#[cfg(test)]
pub(crate) fn prepare_and_finalize_against_for_test(
    definition: CharacterDefinition,
    bindings: &CharacterBindings,
    catalog: Option<&ambition_characters::actor::character_catalog::CharacterCatalog>,
) -> FinalizedCharacter {
    let PreparedCharacter {
        prepared, report, ..
    } = prepare_character(definition, bindings);
    let checked = prepared.checked.clone();
    FinalizedCharacter {
        prepared: finalize_character(prepared, catalog),
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
///   presentation source, and the authored MOVESET, HURTBOX DOC, ACTION SET and
///   MOTION MODEL of any body whose identity resolves to a registered character.
///   Both construction paths honour all four —
///   `project_prepared_character_definitions` for a seated or spawned body, and
///   `avatar::starting_character`'s one construction for a worn one — because
///   wiring only the worn path left seated fighters without their action set for
///   a day (2026-07-28).
/// * **Still the catalog's, and legitimately:** display names, sheet targets,
///   default brains, tiers, tags, aggro and attack ranges, and the remaining
///   movement TUNING. Those are not the KIT, which is what C3 was about — the
///   catalog is the right authority for what a character is CALLED and what it
///   LOOKS like, and it stays the fallback for every kit field a definition did
///   not author, which is most characters.
///
/// **Precedence, in one sentence:** an explicitly authored value on the
/// definition outranks the catalog row; `None` means the author said nothing and
/// the row stands; and `Some(empty)` is an authoring DECISION that outranks the
/// row exactly as a filled one does. That last distinction is the load-bearing
/// one — collapsing it hands an intentionally weaponless character a punch.
///
/// This comment previously said registering a character "does not yet cause a
/// production-spawned body to receive what that character authored — the §7.10
/// fight test projects it by hand". That stopped being true when the projection
/// landed and the hand-projection was deleted; nothing re-read the comment,
/// because a doc comment describing an ABSENCE has no citation to rot (queue W1).
/// Tracked as C3.
#[derive(bevy::prelude::Resource, Debug, Clone, Default)]
pub struct PreparedCharacterRegistry {
    by_id: BTreeMap<ambition_entity_catalog::CharacterId, PreparedCharacterDefinition>,
    generation: CharacterCatalogGeneration,
}

/// Which version of the cast a value was built from.
///
/// Nothing downstream could say WHICH cast a body's kit came from, so "this body
/// was built before the cast changed" was not a question the code could ask — it
/// could only compare the values and guess. That is the shape of every
/// stale-derivation bug in this repo.
///
/// ⚠ this used to open "the registry is a live resource: a room transition
/// builds a fresh one, and registration mutates it in place". That stopped being
/// true when the finalization barrier landed (2026-07-29): the registry is
/// PUBLISHED once, whole, at `Plugin::finish` or the `PreStartup` backstop, and a
/// registration arriving after the barrier closes PANICS rather than mutating it.
/// One generation per publication, carried across rebuilds — a counter that
/// restarted would republish generation 1 over a body stamped with generation 1
/// from the previous cast, and every staleness check would read "still current".
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
    /// `pub(crate)` and named for what it is: the registration seam is the door
    /// (it rejects duplicate ids and ambiguous display names), and this is the
    /// hatch a focused test uses when it wants a registry without an `App`.
    #[cfg(test)]
    pub(crate) fn insert_prepared(&mut self, prepared: PreparedCharacterDefinition) {
        let generation = self.generation;
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

    /// **Stamp this registry as the publication that follows `previous`.**
    ///
    /// One generation per PUBLICATION, not one per character — the barrier
    /// assembles the whole cast and publishes it once, so "how many characters
    /// were inserted" stopped being a meaningful clock the day the fold moved
    /// (2026-07-29).
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
        if definition.id.as_str().trim().is_empty() {
            return Err(CharacterRegistrationError::BlankId);
        }
        // Registration installs its own barrier. Deliberately not a composition
        // requirement: `register_character` is an App extension anybody may call
        // on a bare App, and a finalizer that only runs when the caller also
        // remembered a plugin is a finalizer most callers will not have — which
        // is the same shape as every other "the app forgot the step" defect this
        // module exists because of.
        if !self.is_plugin_added::<CharacterPreparationPlugin>() {
            self.add_plugins(CharacterPreparationPlugin);
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
                 transaction (see docs/planning/character-preparation-finalization-plan.md)"
            );
        }
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

/// **What providers have authored, before the catalog exists to fold against.**
///
/// The preparation-phase half of the registry. Holds partial values and is
/// consumed by [`CharacterPreparationPlugin::finish`]; nothing downstream can
/// read one, because [`PreparedCharacterOverrides`] does not escape this module.
#[derive(bevy::prelude::Resource, Debug, Clone, Default)]
struct StagedCharacterOverrides {
    by_id: BTreeMap<String, PreparedCharacterOverrides>,
    /// Set when the barrier closes, so a late contribution is a panic rather than
    /// a value nobody will ever fold.
    finalized: bool,
}

impl StagedCharacterOverrides {
    fn id_for_display_name(&self, display_name: &str) -> Option<&str> {
        self.by_id
            .values()
            .find(|staged| staged.display_name == display_name)
            .map(|staged| staged.id.as_str())
    }

    /// Returns the previous author when the id was already spoken for.
    fn insert(&mut self, staged: PreparedCharacterOverrides) -> Option<String> {
        self.by_id
            .insert(staged.id.clone(), staged)
            .map(|previous| previous.provider)
    }
}

/// **The finalization barrier.** (H1, 2026-07-29)
///
/// Bevy runs every plugin's `build` during registration and every `finish` once
/// all of them are ready. That ordering is the whole reason this is a plugin
/// rather than a startup system or an eager fold at registration time: a provider
/// registering its cast before the App installs `CharacterCatalog` would otherwise
/// inherit an empty row and bake the absence in permanently. Which provider goes
/// first is a composition detail no provider can see.
///
/// Installed automatically by [`CharacterDefinitionAppExt::try_register_character`].
///
/// ⚠ **`App::update` does not run `finish`** — Bevy's runners do. A hand-driven
/// App (every headless test, every fixture, every tool in this repository) must
/// call [`ambition_platformer2d_runtime::finalize`] or it will have a staged cast and no
/// published one. That is not silent: `PreparedCharacterRegistry` is absent
/// rather than empty, and absent already means "no registered characters" to
/// every consumer.
pub struct CharacterPreparationPlugin;

impl bevy::prelude::Plugin for CharacterPreparationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<StagedCharacterOverrides>();
        // THE BACKSTOP, for the apps Bevy's runner never touches.
        //
        // `App::update` does not run `finish` — runners do — and this repository
        // drives `update` by hand almost everywhere: every headless test, the
        // external-consumer fixture, the rollback harnesses, the tools. Without
        // this, all of them would register a cast and publish none, and the
        // symptom is the worst kind: every character silently falls back to the
        // host's compatibility kit, so a consumer's peaceful wanderer comes out
        // swinging the protagonist's sword. That is not hypothetical — it is
        // what the outlander fixture reported within an hour of the barrier
        // landing.
        //
        // Not a second authority: it calls the SAME finalizer, guarded by the
        // same `finalized` flag, so whichever trigger fires first wins and the
        // other is a no-op. And it is not a weaker barrier — `PreStartup` runs
        // after every plugin's `build`, which is the entire ordering hazard
        // `finish` exists to remove. What `finish` still buys is that the
        // registry exists before ANY system runs, including `Startup`.
        app.add_systems(bevy::prelude::PreStartup, close_preparation_barrier);
    }

    fn finish(&self, app: &mut bevy::prelude::App) {
        finalize_prepared_cast(app.world_mut());
    }
}

/// The `PreStartup` half of [`CharacterPreparationPlugin`]'s backstop.
fn close_preparation_barrier(world: &mut bevy::prelude::World) {
    finalize_prepared_cast(world);
}

/// **Fold the staged cast and publish it.** Idempotent; runs at most once.
fn finalize_prepared_cast(world: &mut bevy::prelude::World) {
    let Some(mut staged) = world.get_resource_mut::<StagedCharacterOverrides>() else {
        return;
    };
    // ⚠ **`App::finish` re-runs EVERY plugin's `finish`, every time it is
    // called.** It does not track which ones already ran — it walks the whole
    // registry and sets `plugins_state = Finished` (read in `bevy_app` 0.18.1
    // after this bit us, 2026-07-29). The `PreStartup` backstop is a second
    // trigger on top of that.
    //
    // Without this flag, a second call republished an EMPTY registry: the staged
    // overrides had already been consumed, so the whole cast silently vanished on
    // the fixture's second step. The barrier has to be idempotent itself;
    // nothing upstream makes it so.
    if staged.finalized {
        return;
    }
    staged.finalized = true;
    let staged = std::mem::take(&mut staged.by_id);
    let catalog = world
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        .cloned();
    // TRANSACTIONAL: the whole cast is folded and only then published, so a
    // reader can never observe a registry that holds half of one generation.
    let previous = world
        .get_resource::<PreparedCharacterRegistry>()
        .map(PreparedCharacterRegistry::generation)
        .unwrap_or_default();
    let mut registry = PreparedCharacterRegistry::default();
    for (_, overrides) in staged {
        registry.insert(finalize_character(overrides, catalog.as_ref()));
    }
    registry.stamp_after(previous);
    world.insert_resource(registry);
}

// A CHILD of the preparation module, not a sibling. Its subject is the partial
// phase — `PreparedCharacterOverrides`, the fold, the barrier — and a sibling
// could not name any of them. Making the tests reach the same way runtime does
// would have meant widening the visibility that IS the design.
#[cfg(test)]
#[path = "definition_tests.rs"]
mod definition_tests;
