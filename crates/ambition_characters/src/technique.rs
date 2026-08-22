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
