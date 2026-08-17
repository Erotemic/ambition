//! **THE AUTHORED CHARACTER** — what content writes down, before anything
//! resolves it.
//!
//! ⭐⭐ **moved out of `ambition_platformer2d_actor_monolith` on 2026-08-12**
//! (D73 checklist item 2, appendix C ruling 4). It had lived beside the
//! PREPARATION that consumes it, in the monolith, which made the crate that
//! happens to build a character look like the owner of what a character IS.
//!
//! Item 1 settled where the cut goes and the file settled it, not a preference:
//! `derive_moveset` — the single reach into `ambition_combat` — is a private
//! preparation function rather than a method on this type, and resolving a kit
//! is runtime work. So the AUTHORED half moves and
//! `PreparedCharacterDefinition` stays above.
//!
//! ⚠ **this half already had no `crate::` references at all**, which is how a
//! 600-line move became a cut rather than a refactor. The one monolith type it
//! did name — `avatar::RangedExecution` — moved here first, for the same reason
//! and in its own commit.

use ambition_entity_catalog::{HurtboxDoc, MovesetContract};

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
/// **THE POOL A BODY GETS WHEN NO AUTHORITY DESCRIBES IT.**
///
/// ⭐ this number used to be spelled `DEFAULT_PROVOKED_HEALTH`, and it lived in
/// the monolith's brain builders because the only place that supplied it was
/// generic PROVOCATION: a peaceful placement spawned at `1`, so being struck
/// replaced the body's whole `BodyHealth` with a fresh 4-point pool. That is a
/// body mutation dressed as a mood change, and it was the last one left in
/// `provoked_projection` (ledger D101).
///
/// ⇒ the repair is one level up, exactly where the `1` was: an undescribed body
/// is undescribed whether or not anybody has hit it yet. Being provoked is not
/// an argument that a creature has a different body, so the default moved to the
/// two places that answer *how tough is a body nobody has authored* — the
/// character body blueprint and the peaceful-NPC seed — and provocation stopped
/// writing health at all.
///
/// ⚠ **the NUMBER is still D96 item 7 and still Jon's.** What changed is the
/// AUTHORITY, not the value: 4 before, 4 after, and answering the ledger row is
/// an edit to this one constant plus whichever characters state their own.
///
/// ⚠ a peaceful body takes no health damage at all (`actor_hit` accumulates
/// strikes and explicitly does not damage a talkable NPC), so raising the
/// peaceful default from 1 to this is inert until the body is hostile — which is
/// why the move is a refactor rather than a rebalance.
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
    /// Reaches a body as `Mass` (the monolith's `features::ecs::mount`), which drives the mount
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
    /// body as `CombatTuning::weight` (`ambition_combat`). `1.0` is
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
    /// producer of `CombatCapabilities` (`ambition_combat`) in the workspace was
    /// `ArchetypeSpecExt::combat_capabilities` — so a mite that splits when
    /// killed could say so as an archetype and a registered character could not
    /// say it at all. A seated fighter and a worn player simply had no death
    /// traits, whatever they were.
    ///
    /// `None` means the author said nothing, and nothing is inserted — today's
    /// behaviour for every character in the repo. ⚠ absence RETRACTS on a
    /// re-wear, like every other physical fact a persona claims: wearing a
    /// sandbag and then a duelist must not leave the duelist unkillable.
    pub death_traits: Option<crate::actor::CharacterDeathTraits>,
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
    pub action_set: Option<crate::brain::ActionSet>,
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
    /// **The verbs this BODY has** — jump, double jump, dash, dodge, shield,
    /// ledge grab, blink, fly, glide, swim.
    ///
    /// ⭐ **a capability is the character's, never the controller's and never
    /// the ruleset's.** The archetype has always been able to state a movement
    /// kit (`ArchetypeSpecExt::movement_kit`, four flags); a registered
    /// character could not state one at all, which is why a match seat had to be
    /// handed a flat set by the MATCH — *"every fighter in this match has the
    /// same verbs"* — and why the Smash demo's fighters do not use the shield,
    /// dodge and ledge machinery that already exists underneath them. Nothing
    /// had granted them the capability, because nothing could.
    ///
    /// ⚠ **`None` means the author said nothing**, and the migration bridge
    /// stands: a seat whose character authors no verbs still takes the match's
    /// declared set, exactly as today. That bridge is what a character authoring
    /// its own kit removes, one character at a time.
    ///
    /// ⛔ **a ruleset may only take verbs away.** `AbilitySet::intersect` is the
    /// operation a mode is allowed: Smash may say *"no flying in this match"*
    /// and may not say *"everyone can jump"*, because forcing a jump onto a body
    /// that cannot jump is the engine manufacturing a capability — the exact
    /// thing that makes Puppy Slug in a fighter seat indistinguishable from a
    /// generic humanoid.
    pub abilities: Option<ambition_platformer2d_core::AbilitySet>,
    /// **How this body moves under its own power** — top speed, gait, surface
    /// cling. See [`crate::actor::CharacterLocomotion`].
    ///
    /// `None` means the character said nothing, and a construction path with a
    /// legacy source (the archetype's `run_speed`/`move_style`, a match's
    /// fighter default) still uses it. A crawler that authors this is a crawler
    /// wherever it is spawned — including a fighter seat, which is Jon's
    /// compositional acceptance test.
    pub locomotion: Option<crate::actor::CharacterLocomotion>,
    /// **Whether touching this body hurts, and how much.** `None` = it does
    /// not, which is most characters.
    pub contact_damage: Option<crate::actor::ContactDamage>,
    /// **The POLICY this character runs when nothing else drives it** — the
    /// controller authority, carried as a value rather than as a name.
    ///
    /// ⭐⭐ **and it is the ONLY half now** (2026-08-12, ledger D97). This field's
    /// doc used to describe a fork: a sibling `default_brain_profile` named a
    /// catalog `brain_presets` key for the NPC road while this carried a
    /// `BrainProfile` for the enemy road, *"the same idea in two vocabularies"*,
    /// and the note promised a convergence. The convergence turned out to be a
    /// deletion: the preset half had **zero authors in the entire repo** and one
    /// consumer, and its absence was what produced the empty-string default that
    /// crashed two shipped rooms. A character states its policy HERE, or it
    /// leaves the catalog row in charge; there is no third place.
    ///
    /// ⚠ **the two vocabularies still exist, one authority apart.** A catalog ROW
    /// may name a `brain_presets` key and many still do (D81 counts ~125
    /// adopters); a character DEFINITION states a `BrainProfile`. What is gone is
    /// a definition being able to say the same thing in the row's words.
    ///
    /// `None` leaves the archetype's projection in charge, which is every
    /// character that has not migrated.
    pub autonomous_profile: Option<crate::brain::BrainProfile>,
    /// **The SHARED policy this character names**, resolved out of the catalog's
    /// `autonomous_profiles` map at preparation into [`Self::autonomous_profile`].
    ///
    /// ⭐ **this is what makes a policy reusable** (ledger D80). Carrying a
    /// profile by value says what ONE character does; naming one says several
    /// characters fight alike — which is the whole reason `medium_striker`
    /// exists as a whole-body archetype worn by five goblins, a lab raider and a
    /// skitter. A named profile lets those five keep their own bodies and share
    /// the decision-making, which is the Group-B/Group-C split.
    ///
    /// ⛔⛔ **INLINE XOR NAMED, and authoring both is a REFUSAL** (Jon's
    /// redirect §9). The precedence this used to document — inline wins, named
    /// is the fallback — was whole-value REPLACEMENT wearing the word
    /// "specialization", and nothing merged. Documenting replacement as
    /// specialization is misleading API on the day it ships; if a real patch is
    /// ever wanted it gets a real `BrainProfilePatch` with explicit semantics.
    ///
    /// ⛔ **and a name nobody authored is a PREPARATION FAILURE**, not a silent
    /// `None`. Falling back would reproduce the explicit-`CharacterId` mistake
    /// one layer down: the author said which policy this creature uses, the
    /// lookup missed, and the archetype quietly stayed in charge — green
    /// everywhere, wrong in play.
    pub autonomous_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// **The policy this creature adopts when PROVOKED**, by provider-relative
    /// name.
    ///
    /// ⛔ **provocation picks an enemy ARCHETYPE by substring-matching a display
    /// name today** (`hostile_brain_id_for_actor`: *does the id or the name or
    /// the dialogue node contain "pirate"*). That is the fused ontology at its
    /// most literal — a peaceful pirate that gets struck is handed a different
    /// BODY, not a different attitude — and it is the only thing keeping three
    /// archetype rows alive that no level places.
    ///
    /// ⭐ what provocation actually is: the same body, a different driver, and a
    /// changed relationship. This is the driver half, stated by the creature
    /// that has one.
    ///
    /// `None` = this character has nothing to say about being provoked, which
    /// leaves the legacy name-match in charge — every character today.
    pub provoked_profile_ref: Option<crate::brain::BrainProfileRef>,
    /// **What this character's PROJECTILE looks like** — the cosmetic id its
    /// ranged verb spawns (`"hadouken"`).
    ///
    /// ⛔ **it had no home outside an enemy ARCHETYPE row** (ledger D83).
    /// `ActorTuning::ranged_visual` carries it at runtime and the archetype road
    /// filled it; the character-first constructor wrote an empty string, so a
    /// migrated robot fired an unadorned rock. The melee side of the same
    /// question has been a character fact for a long time (the catalog's
    /// `attack_vfx`), which is what makes the absence a gap rather than a design.
    ///
    /// `None` = this character's ranged verb draws whatever the projectile
    /// itself authors, which is every character that has never had one.
    pub ranged_vfx: Option<String>,
    /// **HOW this character's ranged attack is executed** — a charged projectile
    /// (hold to build, release to fire) or an ordinary moveset verb.
    ///
    /// ⭐ **an authored CHARACTER fact since 2026-08-11** (GPT 5.6 §4). It was
    /// derived from `PlayableKitSource::HostCode`, which made a gameplay property
    /// of the protagonist's attack look like a property of which crate built it —
    /// and so made *delete HostCode* read as *delete the charge*. Jon's product
    /// rule is the opposite: Player Robot v3 is the same character with the same
    /// repertoire in Ambition and in Smash, and a mode changes interpretation and
    /// restrictions rather than silently replacing its moves.
    ///
    /// ⚠ the DEFAULT is `MovesetVerb`, which is what every character that has
    /// never had a charge already does.
    pub ranged_execution: crate::brain::RangedExecution,
    /// **This body is a PRACTICE TARGET** — a training dummy, not a
    /// participant.
    ///
    /// ⛔ **the last fact keeping the sandbags on `character_archetypes.ron`**
    /// (ledger D77). `ArchetypeSpec::is_sandbag` has four live consumers — the
    /// save sync excludes it from the file, the path assignment skips it, and
    /// two sprite reads select on it — and `new_character_in` wrote `false` via
    /// `..Default::default()`, so a migrated sandbag would silently have joined
    /// the save file and changed its sprite.
    ///
    /// ⚠ **on the definition, not read off a catalog tag.** The plane-swarm
    /// lesson: a body that reads an intrinsic from a catalog row it cannot see
    /// gets the wrong answer in a standalone demo that borrowed the character.
    #[doc(alias = "is_sandbag")]
    pub practice_target: bool,
    /// **The weapon this character carries**, by id, resolved through the same
    /// held-item registry the archetype's `held_item` uses.
    ///
    /// ⭐ **a fact about the creature, not about the placement.** A cove raider
    /// carries a gun-sword wherever it stands; the item is what it drops when it
    /// dies and what its swing looks like. It was reachable only through an
    /// archetype row, so a migrated raider lost its weapon — which is most of
    /// what a raider IS.
    ///
    /// ⚠ **it grants no VERBS here.** The archetype path folds a held item's
    /// melee/ranged into the resolved `ActionSet`; a character authors its verbs
    /// on [`Self::action_set`] directly, so this states what the body HOLDS and
    /// the action set states what it DOES. Authoring an item and forgetting the
    /// verb gives a body a weapon it never swings — visible, rather than a
    /// silently different creature.
    pub held_item: Option<String>,
    /// **What this body can be RIDDEN as, and what it can ride** (ADR 0020).
    /// `None` = neither. See
    /// [`crate::actor::CharacterMount`].
    pub mount: Option<crate::actor::CharacterMount>,
    /// **Deep-dream visual jitter seed** — this character's participation in the
    /// psychedelic shader pass, and how it differs from its neighbours.
    ///
    /// ⚠ **presentation, and on the definition for the same reason the sheet
    /// is**: it is a fact about what this creature LOOKS like, true of every
    /// instance, and it was reachable only through an enemy archetype row. The
    /// puppy slug is the live case — `dream_seed: Some(0.271828)` is the only
    /// thing between a migrated slug and the psychedelic pass it has always had.
    ///
    /// `None` = does not participate, which is nearly everything.
    pub dream_seed: Option<f32>,
    /// **Two of this character, told the same things, think the same thoughts —
    /// so a mirror match plays as a reflection.**
    ///
    /// ⭐ **an authored TRAIT, not the default CPU policy.** Ordinarily an
    /// autonomous participant's deterministic decision/noise stream is derived
    /// from WHICH PARTICIPANT it is, so two CPUs wearing one character diverge
    /// within a few decisions — that is what a viewer expects of two opponents.
    /// A character that authors this asks for the opposite: every equally
    /// configured twin begins on the SAME cognitive stream.
    ///
    /// ⛔⛔ **it does NOT synchronise their actions, and must never be
    /// implemented that way.** The property is *identical cognition + symmetric
    /// information → symmetric behaviour*, which is an emergent consequence of
    /// sharing one stream, not a canned mirror animation. The moment two of them
    /// see different worlds — different damage, different position, a different
    /// foe — they decide differently, and that is correct. A mirror that survived
    /// asymmetric observations would be a puppet show.
    ///
    /// ⚠ so the only thing this authorises is the CHOICE OF STREAM, made once at
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
    /// ⭐ pass the LOCAL name (`medium_striker`). Whether the assembled catalog
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

#[cfg(test)]
mod authority_tests {
    use super::*;

    /// **WHAT A CHARACTER IS ALLOWED TO KNOW** — D73's first failure mode,
    /// guarded from the destination side.
    ///
    /// The brief's warning is *"do not migrate `ArchetypeSpec` into
    /// `CharacterDefinition` wholesale — it holds THREE authorities and they
    /// must separate"*. `ArchetypeSpec` now has the same exhaustive destructure
    /// saying where each of its 49 fields goes. This is the other half: a field
    /// arriving HERE has to be justified as something a body may state.
    ///
    /// ⇒ **the failure mode now requires an explicit lie rather than an
    /// omission.** Carrying `aggro_radius` across would not quietly widen a
    /// struct; it would stop this crate compiling until somebody filed a
    /// controller fact under one of the headings below.
    ///
    /// ⚠ **the DEFAULT CONTROLLER group is the subtle one and is deliberately
    /// not empty.** A character may state the policy it comes with — the goblin
    /// names `medium_striker`, the shark riders carry one inline — and that is
    /// the campaign's own design, not a leak: *"one adopter does not earn the
    /// indirection."* What must stay true is that a DEFAULT is replaceable.
    /// **Changing the controller does not change the body**; a body that could
    /// not be driven by another mind would be the failure this group is watched
    /// for.
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

            // ── DEFAULT CONTROLLER (4) — see the ⚠ above ────────────────────
            //
            // A policy this character COMES WITH, by name or inline. Not the
            // controller itself, and never a reason for a body fact to live in
            // a profile or the reverse.
            autonomous_profile: _,
            autonomous_profile_ref: _,
            provoked_profile_ref: _,
            // ⭐ **filed HERE and not under BODY**, and the group's own ⚠ is the
            // reason: it states something about this character's AUTONOMOUS
            // drivers — two of them share one deterministic cognitive stream —
            // and it says nothing about the body. It passes the group's test
            // exactly: **changing the controller does not change the body.** Put
            // a person on the sticks and this field means nothing at all.
            //
            // ⛔ it is not on `BrainProfile` because a profile is reusable across
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
