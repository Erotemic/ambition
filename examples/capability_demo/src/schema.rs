//! The capability's AUTHORED CONTENT: pulse profiles, as a registered schema.
//!
//! This is the third of the four halves. `ambition_content_pack` never heard of
//! pulses; the schema is registered by the capability that owns it, exactly as
//! `ambition_characters` registers the character catalog. Nobody edits a central
//! content enum, which is the property the row is testing.

use std::sync::Arc;

use ambition_content_pack::{
    CapabilityId, ContentSchemaHandler, DiagnosticCode, FacetOutcome, FacetSource,
    RuntimeDisposition, SchemaId, SchemaRegistration, SchemaVersion,
};
use bevy::prelude::Resource;

/// The authored file kind.
pub const PULSE_SCHEMA: &str = "pulse_profile";

/// One authored profile.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
// An authored field nothing consumes must not be dropped in silence — the
// mechanic would simply never feel the way it was written.
#[serde(deny_unknown_fields)]
pub struct PulseProfile {
    pub name: String,
    /// How far the shockwave reaches, in pixels.
    pub radius: f32,
    /// Impulse at the centre; linear falloff to nothing at the rim.
    pub force: f32,
    /// Ticks before this body may pulse again.
    pub cooldown_ticks: u32,
}

impl Default for PulseProfile {
    fn default() -> Self {
        Self {
            name: "default".into(),
            radius: 96.0,
            force: 420.0,
            cooldown_ticks: 45,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct PulseProfileFile {
    pub profiles: Vec<PulseProfile>,
}

/// The profiles a composition installed, and which one is live.
///
/// A resource rather than a constant, because that is the whole point: the
/// numbers come from authored content, and a game tunes its pulse without
/// touching this crate.
#[derive(Resource, Clone, Debug)]
pub struct PulseProfiles {
    profiles: Vec<PulseProfile>,
    active: usize,
}

impl Default for PulseProfiles {
    fn default() -> Self {
        Self {
            profiles: vec![PulseProfile::default()],
            active: 0,
        }
    }
}

impl PulseProfiles {
    /// Adopt what the content compiler prepared.
    pub fn from_prepared(profiles: Vec<PulseProfile>) -> Self {
        if profiles.is_empty() {
            return Self::default();
        }
        Self {
            profiles,
            active: 0,
        }
    }

    pub fn active(&self) -> PulseProfile {
        self.profiles.get(self.active).cloned().unwrap_or_default()
    }

    /// Choose the active profile by authored name, at COMPOSITION time.
    ///
    /// Freezing it is the cheaper of the two honest answers — the other being to
    /// make the selection rollback-owned — and nothing called `select`, so no
    /// behaviour was lost. A game that needs to switch profiles mid-match should
    /// make that a rollback-owned choice deliberately, not inherit one by
    /// accident.
    ///
    /// `None` when no profile carries that name; the caller decides whether that
    /// is a refusal or a fallback, because a capability cannot know which.
    #[must_use]
    pub fn with_active(mut self, name: &str) -> Option<Self> {
        let index = self.profiles.iter().position(|p| p.name == name)?;
        self.active = index;
        Some(self)
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.profiles.iter().map(|p| p.name.as_str())
    }
}

struct PulseSchema;

impl ContentSchemaHandler for PulseSchema {
    fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
        let file: PulseProfileFile = match ron::from_str(facet.text) {
            Ok(file) => file,
            Err(error) => {
                let code = match error.code {
                    ron::Error::NoSuchStructField { .. } => DiagnosticCode::UnknownField,
                    _ => DiagnosticCode::MalformedSource,
                };
                out.report(facet.diagnostic(code, format!("{error}")));
                return;
            }
        };
        if file.profiles.is_empty() {
            out.report(facet.diagnostic(
                DiagnosticCode::MalformedProviderBinding,
                "a pulse profile file with no profiles in it — the mechanic would fall back to \
                 its built-in numbers and the authored file would look applied",
            ));
            return;
        }
        for (index, profile) in file.profiles.iter().enumerate() {
            let id = facet.content_id(&profile.name);
            // A zero radius is a pulse that reaches nothing and a zero cooldown
            // is one that fires every tick. Both are almost certainly a typo,
            // and both LOOK authored.
            if profile.radius <= 0.0 {
                out.report(
                    facet
                        .diagnostic(
                            DiagnosticCode::MalformedProviderBinding,
                            format!(
                                "pulse `{}` has a radius of {}",
                                profile.name, profile.radius
                            ),
                        )
                        .about(id.clone())
                        .at_field("radius")
                        .fix("a pulse with no radius pushes nothing; give it one or delete it"),
                );
            }
            // THE INDEX IS PART OF THE CANONICAL FORM, because this file is
            // POSITIONAL. `PulseProfiles::from_prepared` pins `active: 0`, so
            // the FIRST profile is the one the mechanic runs — and the pack
            // fingerprint sorts definitions by content id, so a row canonical
            // keyed only by name made SWAPPING two whole profiles a no-op for
            // identity while changing the live pulse's radius, force and
            // cooldown. Measured, not argued: two packs differing only in
            // profile order produced byte-identical canonical bytes and
            // fingerprint `e060c5b64b5a0b78`, with `active` reading `gentle`
            // in one and `cannon` in the other.
            //
            // the rule, since this is the THIRD time: if the lowered artifact is a sequence
            // and the runtime reads it BY POSITION, the position is part of the canonical form.
            out.define(
                id,
                format!(
                    "index={index}\n{}",
                    ron::ser::to_string(profile).unwrap_or_else(|e| format!("<{e}>"))
                ),
            );
        }
        if !out.failed() {
            out.lower(file.profiles);
        }
    }
}

/// The capability's schema, for a composition to install into its registry.
pub fn pulse_schema() -> SchemaRegistration {
    SchemaRegistration {
        id: SchemaId::new(PULSE_SCHEMA),
        version: SchemaVersion(1),
        capability: CapabilityId::new(crate::PULSE_CAPABILITY),
        disposition: RuntimeDisposition::Runtime,
        doc: "Shockwave pulse tuning: radius, force and cooldown, by name.",
        handler: Arc::new(PulseSchema),
    }
}
