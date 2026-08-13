//! **THE BEVY `App` LAYER OVER CHARACTER PREPARATION** — registration, the
//! finalization barrier, and the plugin that closes it.
//!
//! ⭐⭐ **the authored MODEL and the preparation PIPELINE are not here any more**
//! (campaign P1.7, 2026-08-12). They are `ambition_characters::prepared`, which
//! is where the reusable character domain belongs: Jon's brief is explicit that
//! a dependency obstacle must not be solved by leaving the authoritative
//! character model inside the actor monolith. What stays is the part that is
//! genuinely an App's — `try_register_character`, `StagedCharacterOverrides`,
//! `CharacterPreparationPlugin`, and the barrier it closes.
//!
//! ⭐ **the cut was clean, and it was MEASURED before it was made**: the moved
//! region held zero `crate::` reach-ins, zero `super::` uses, zero bevy, and
//! five import lines all naming crates `ambition_characters` already had.
//!
//! ⛔⛔ **and the thing that made it hard was not the imports — it was the
//! PRIVACY.** `prepare_character` and `finalize_character` were private module
//! functions, and that privacy IS the finalization barrier: it is what made an
//! early fold unreachable. Splitting the module would have meant publishing
//! them, putting the ordering hazard `CharacterPreparationPlugin::finish` exists
//! to remove back on the production surface. So the barrier became a TYPE
//! instead — `prepared::StagedCharacter` can only be minted by
//! `prepare_for_registration` and only consumed by `finalize_cast`, the
//! `Bound<N>` pattern this repository already runs. Folding early is not
//! prevented now; it is unspellable, and that survives a crate boundary where
//! privacy does not.
//!
//! [`super::CharacterLoadStates`] is where the art half reports.

// ⭐ **every moved name is re-exported**, so `character_runtime::{..}` paths
// across the workspace are unchanged by the relocation. The module a reader
// should EDIT is `ambition_characters::prepared`; this list is the door.
pub use ambition_characters::binding_namespaces::{
    MoveId, PortraitTarget, RangedPayload, SfxCueId, SheetTarget, VerbId, VfxTag,
};
pub use ambition_characters::prepared::{
    finalize_cast, prepare_for_registration, CharacterBindings, CharacterBodyBlueprint,
    CharacterCatalogGeneration, CharacterRegistrationError, MissingCharacterFacts,
    PreparedCharacterDefinition, PreparedCharacterRegistry, PreparedKit, StagedCharacter,
    StagedRegistration,
};

use ambition_characters::actor::definition::CharacterDefinition;

// The barrier-bypassing fixture seams, re-exported so `definition_tests.rs` —
// which is `#[path]`-included as a CHILD of this module and reaches them through
// `use super::*` — is unchanged by the relocation. Behind `ambition_characters`'s
// `test-support` feature, which this crate enables as a DEV-dependency only.
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use ambition_characters::prepared::{
    prepare_and_finalize_against_for_test, prepare_and_finalize_for_test, FinalizedCharacter,
};
use std::collections::BTreeMap;

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
/// **Fill in the engine's baked sheet + portrait vocabularies unless the caller
/// supplied them** — the registration seam's job, and it is a FREE FUNCTION for
/// a reason.
///
/// ⛔ these were inherent methods on `CharacterBindings`
/// (`with_engine_{sheet,portrait}_vocabulary`). The type is moving down into
/// `ambition_characters`, and the vocabularies come from
/// `ambition_sprite_sheet`, which DEPENDS ON `ambition_characters` — so keeping
/// them inherent would be a cycle the compiler finds at the worst moment. The
/// orphan rule adjudicating placement (P1.7 sub-case (a)).
///
/// ⭐ **and it belongs here anyway**, which is what makes this a repair rather
/// than a workaround. The original doc said so: *"kept OUT of
/// `prepare_character`, which stays a pure function of its arguments — reaching
/// into a baked global from inside preparation would make the same definition
/// prepare differently depending on the build. This is the registration seam's
/// job, because registration is where the engine is."*
///
/// ⛔⛔ **portrait targets were checked NOWHERE until the portrait half existed**
/// (ledger D106): `with_available_portraits` was the only road to the resolver
/// and nothing called it, so `portraits` was `None` in every composition,
/// `PortraitTarget::NAME` never joined a prepared character's `checked` list,
/// and preparation reported *"we did not look"* — permanently, correctly, and
/// therefore invisibly. Both halves apply at the ONE seam every registration
/// passes through, rather than at the three call sites, so the next provider
/// cannot forget one.
pub fn with_engine_vocabularies(mut bindings: CharacterBindings) -> CharacterBindings {
    if !bindings.has_sheet_vocabulary() {
        bindings = bindings
            .with_available_sheets(ambition_sprite_sheet::character::sheets::available_targets());
    }
    if !bindings.has_portrait_vocabulary() {
        bindings =
            bindings.with_available_portraits(ambition_sprite_sheet::available_portrait_targets());
    }
    bindings
}

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
        // ⭐ **BOTH vocabularies, at the ONE seam every registration passes
        // through.** Sheets have been checked here since the boundary landed;
        // portraits were not checked anywhere at all (D106), and adding it here
        // rather than at the three call sites is what stops the next provider
        // forgetting one of them.
        let bindings = with_engine_vocabularies(bindings);
        let StagedRegistration {
            staged: prepared,
            report,
        } = prepare_for_registration(definition, &bindings);
        let id = prepared.id().to_string();

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
/// FOLD one, because [`StagedCharacter`] is opaque — it has no public
/// constructor and `finalize_cast` is the only thing that consumes it.
///
/// ⚠ **that used to read "nothing downstream can READ one, because
/// `PreparedCharacterOverrides` does not escape this module"**, and the module
/// it could not escape was in this crate. The partial escapes now — it has to,
/// the model lives in `ambition_characters` — so the guarantee moved from
/// visibility to TYPE. Weaker in what it hides, stronger in what it prevents.
#[derive(bevy::prelude::Resource, Debug, Clone, Default)]
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
        // ⚠ **a `PreUpdate` re-close was TRIED for queue D75 and does not fix
        // it** (2026-08-11). The hypothesis was a cast arriving after the
        // barrier latched; the measurement says otherwise — those hosts read a
        // registry of ZERO at spawn time whether the barrier can re-close or
        // not, so the cast is not late, it is absent from whatever world the
        // spawn is reading. Recorded rather than left as a plausible fix nobody
        // re-measured.
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
    // Without a guard, a second call republished an EMPTY registry: the staged
    // overrides had already been consumed, so the whole cast silently vanished on
    // the fixture's second step. The barrier has to be idempotent itself;
    // nothing upstream makes it so.
    //
    if staged.finalized {
        return;
    }
    staged.finalized = true;
    let staged = std::mem::take(&mut staged.by_id);
    let catalog = world
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        .cloned();
    // The POLICY authority, published beside the catalog by assembly.
    let profiles = world
        .get_resource::<ambition_characters::actor::character_catalog::BrainProfileRegistry>()
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

// A CHILD of the preparation module, not a sibling. Its subject is the partial
// phase — `PreparedCharacterOverrides`, the fold, the barrier — and a sibling
// could not name any of them. Making the tests reach the same way runtime does
// would have meant widening the visibility that IS the design.
#[cfg(test)]
#[path = "definition_tests.rs"]
mod definition_tests;
