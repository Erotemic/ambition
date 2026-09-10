//! THE AUTHORED SCHEMAS OF ENGINE TECHNIQUES — the params an `on_hit`
//! effect carries, and nothing that executes one.
//!
//! this lived in `ambition_combat:on_hit` beside the Bevy system that runs it, and the
//! split matters because of who else needs it: the moveset PREFABS name `POGO_BOUNCE_KEY` and
//! call `set_pogo_sfx` while building a contract, and character PREPARATION calls the prefabs.
//!
//! Here the lower fact is *what a `pogo_bounce` effect SAYS*; the rebound itself — the queries,
//! the policies, the message — stays in `ambition_combat` where the bodies are.
//!
//! the cue comes back as a `String`, not an `SfxId`. Wrapping it would
//! mean a new `ambition_characters → ambition_sfx` edge for one newtype, and the
//! layering is better without it: this crate owns the authored TEXT, and
//! deciding that the text names a cue is the consumer's job. `ambition_combat`
//! keeps `pogo_sfx_from` as that adapter.

use ambition_entity_catalog::EffectRef;

/// The techniques this composition installed handlers for.
///
/// ⭐ A KEY IN HERE MEANS SOMETHING INSTALLED ANSWERS IT, because the only way in
/// is the statement that adds the handler system
/// (`combat_schedule::install_technique`). That is the property a metadata
/// registry cannot have, and the reason its predecessor — `ParamSchemaRegistry`,
/// which had zero production callers — could not tell a misspelled effect key
/// from a real one.
///
/// ⛔⛔ **IT LIVES HERE BECAUSE PREPARATION IS WHERE IT IS READ.** It was in
/// `ambition_combat`, beside effect EXECUTION, which was right while the only
/// consumer executed effects. The admission pass runs at the character
/// preparation barrier — in THIS crate — and `ambition_characters` cannot name
/// `ambition_combat` (the dependency runs the other way). A table the check
/// cannot see is a check that cannot run, so the shell moved to the lower crate
/// and `ambition_combat::technique` re-exports it; every existing path still
/// resolves.
///
/// ⚠ Still NOT in `ambition_entity_catalog`, which owns `TechniqueSupport`
/// itself: that crate is engine-free on purpose and this is a bevy `Resource`.
#[derive(bevy::prelude::Resource, Default)]
pub struct InstalledTechniques(pub ambition_entity_catalog::TechniqueSupport);

/// The `on_hit` effect key the engine pogo technique answers.
pub const POGO_BOUNCE_KEY: &str = "pogo_bounce";

/// Params for the `pogo_bounce` technique. `rise` is the gravity-up rebound
/// speed (engine units); omitted → the default pop (matches the flat player
/// `pogo_speed` for feel parity). `sfx` names the contact cue this particular
/// body's rebound makes; omitted → the engine's generic `Pogo` cue.
#[derive(serde::Serialize, serde::Deserialize)]
struct PogoBounceParams {
    #[serde(default = "default_pogo_rise")]
    rise: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sfx: Option<String>,
}

fn default_pogo_rise() -> f32 {
    720.0
}

impl Default for PogoBounceParams {
    fn default() -> Self {
        Self {
            rise: default_pogo_rise(),
            sfx: None,
        }
    }
}

/// The admission check for a `pogo_bounce` effect's params.
///
/// ⭐ THE CHECK IS PUBLIC AND THE TYPE STAYS PRIVATE, deliberately.
/// `PogoBounceParams` is this module's business — every reader goes through
/// `pogo_rise_from` / `pogo_sfx_from`, which is why it was never exported — but
/// a composition that INSTALLS the pogo handler has to declare what the key
/// accepts, and `check_hydrates::<T>` needs `T`. Exposing the function rather
/// than the struct gives the declaration what it needs without reopening the
/// schema to callers who would then hydrate it themselves.
///
/// ⚠ AND IT IS STRICTER THAN THE READER. `pogo_rise_from` below does
/// `.unwrap_or_default()`, so malformed params silently become the default pop
/// at runtime. This refuses them at PREPARATION instead — which is the whole
/// point of the admission contract, and a deliberate tightening rather than a
/// restatement of current behaviour.
pub fn check_pogo_bounce_params(
    params: &ambition_entity_catalog::ParamValue,
) -> Result<(), String> {
    ambition_entity_catalog::check_hydrates::<PogoBounceParams>(params)
}

/// The rebound speed a `pogo_bounce` [`EffectRef`] carries — hydrated from its
/// params, defaulting when absent/malformed. Shared by resolved-body pogo and
/// world-surface pogo.
pub fn pogo_rise_from(effect: &EffectRef) -> f32 {
    effect
        .params
        .hydrate::<PogoBounceParams>()
        .unwrap_or_default()
        .rise
}

/// The contact cue a `pogo_bounce` [`EffectRef`] authored, if any. `None` means
/// "this body has nothing special to say about rebounding" and the caller falls
/// back to the engine's generic pogo cue.
///
/// This is what keeps the pogo sound ATTACK-owned: without it, a body whose
/// blade should clang differently on a rebound could only be told apart by its
/// character id, and the technique's claim to be "a data-authored `on_hit`
/// rather than a hardcoded player branch" would stop being true.
pub fn pogo_sfx_cue_from(effect: &EffectRef) -> Option<String> {
    effect
        .params
        .hydrate::<PogoBounceParams>()
        .ok()
        .and_then(|params| params.sfx)
}

/// Author `cue` as this `pogo_bounce` effect's contact sound, preserving any
/// `rise` already on it. Applied when a body's presentation family is overlaid
/// onto its derived moveset, so the runtime never has to ask WHO bounced.
pub fn set_pogo_sfx(effect: &mut EffectRef, cue: &str) {
    let mut params = effect
        .params
        .hydrate::<PogoBounceParams>()
        .unwrap_or_default();
    params.sfx = Some(cue.to_string());
    // The params are opaque `ron::Value` by design, so this stores exactly the
    // text an author would have written by hand. The value being serialized is
    // this module's own two-field struct, so a failure here is a broken schema,
    // not bad content — and swallowing it would spend the rest of the session
    // playing the generic pogo with nothing to say why.
    effect.params = ambition_entity_catalog::ParamValue::from_typed(&params)
        .expect("PogoBounceParams must round-trip through its own authored RON form");
}
