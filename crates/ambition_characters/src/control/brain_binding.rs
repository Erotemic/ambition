//! Autonomous controller sources, authored context, and preset validation.

use crate::actor::character_catalog::{
    qualify_preset_like, BossAutonomyId, BrainPresetId, CharacterCatalog,
};
use bevy::prelude::Component;

/// Which *autonomous* actor mode a catalog-backed actor should be in when no
/// temporary controller (player possession / mount) is masking it.
///
/// The mutable half of a [`BrainBinding`]. Every variant is a STABLE,
/// reconstructible source — a snapshot restores this and the whole autonomous
/// configuration (live `Brain`, action set, disposition, archetype tuning) is
/// rebuilt from it deterministically. There is no lossy "some other authority
/// owns the live brain" escape hatch: a provoked actor names its hostile
/// archetype, so a rewind can rerun the roster construction that produced it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutonomousSource {
    /// The character's catalog default preset.
    CatalogDefault,
    /// An explicit catalog preset override (authored `brain_override`, or a
    /// runtime `UsePreset` switch).
    CatalogPreset(BrainPresetId),
    /// A provoked character's autonomous policy, named by stable profile id.
    /// The body remains unchanged; rollback stores the resolvable id rather than
    /// embedding the profile's tuning values.
    ProvokedProfile {
        profile: ambition_entity_catalog::BrainProfileId,
    },
    /// Use the character's own authored autonomous profile as the default.
    /// Carries no payload: body identity identifies the character, and lowering
    /// the profile requires that body's capabilities and movement parameters.
    CharacterProfile,
    /// A boss's authored autonomous mode. The live brain is a `BossPattern`
    /// rebuilt from the boss catalog by this id (never a catalog preset). A boss
    /// carries this so it has a reconstructible autonomous source when a player
    /// possession masks its live brain.
    Boss { archetype: BossAutonomyId },
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BrainBinding {
    /// What restoring this body's autonomous default means — see
    /// [`AutonomousDefault`].
    pub default_preset: AutonomousDefault,
    /// Which autonomous source is live right now.
    pub source: AutonomousSource,
}

impl BrainBinding {
    /// A catalog-backed binding: `default_preset` is the character's catalog
    /// default, always present.
    pub fn new(default_preset: BrainPresetId, source: AutonomousSource) -> Self {
        Self {
            default_preset: AutonomousDefault::Preset(default_preset),
            source,
        }
    }

    /// A binding whose default is the character's own authored policy.
    pub fn from_character_profile() -> Self {
        Self {
            default_preset: AutonomousDefault::CharacterProfile,
            source: AutonomousSource::CharacterProfile,
        }
    }

    /// A boss binding: no catalog default preset; the autonomous source is the
    /// boss's authored [`AutonomousSource::Boss`] mode.
    pub fn boss(archetype: BossAutonomyId) -> Self {
        Self {
            default_preset: AutonomousDefault::None,
            source: AutonomousSource::Boss { archetype },
        }
    }

    /// The catalog preset in effect right now: the override if one is selected,
    /// the character default when on the default, or `None` when the actor's live
    /// brain is not a catalog preset (provoked archetype / boss / no default).
    pub fn active_preset(&self) -> Option<&BrainPresetId> {
        match &self.source {
            AutonomousSource::CatalogPreset(id) => Some(id),
            AutonomousSource::CatalogDefault => self.default_preset.preset(),
            AutonomousSource::ProvokedProfile { .. }
            | AutonomousSource::CharacterProfile
            | AutonomousSource::Boss { .. } => None,
        }
    }

    /// True iff an override preset is currently selected.
    pub fn is_override(&self) -> bool {
        matches!(self.source, AutonomousSource::CatalogPreset(_))
    }

    /// True iff the actor is currently driven by a provoked policy.
    pub fn is_provoked(&self) -> bool {
        matches!(self.source, AutonomousSource::ProvokedProfile { .. })
    }

    /// True iff the actor's autonomous source is a boss mode.
    pub fn is_boss(&self) -> bool {
        matches!(self.source, AutonomousSource::Boss { .. })
    }

    /// The boss archetype id, if this is a boss binding.
    pub fn boss_archetype(&self) -> Option<&BossAutonomyId> {
        match &self.source {
            AutonomousSource::Boss { archetype } => Some(archetype),
            _ => None,
        }
    }

    /// Switch to an override preset. (The caller rebuilds the live `Brain`.)
    pub fn use_preset(&mut self, preset: BrainPresetId) {
        self.source = AutonomousSource::CatalogPreset(preset);
    }

    /// Return to the character default. (The caller rebuilds the live `Brain`.)
    pub fn restore_default(&mut self) {
        // The restored source must match the declared default.
        self.source = match self.default_preset {
            AutonomousDefault::CharacterProfile => AutonomousSource::CharacterProfile,
            AutonomousDefault::Preset(_) | AutonomousDefault::None => {
                AutonomousSource::CatalogDefault
            }
        };
    }
}

/// Per-spawn parameters a *selected* brain preset consumes when instantiated.
///
/// These parameterize a preset that was already chosen; they never SELECT it. A
/// [`Patrol`](crate::brain::state_machine::StateMachineCfg::Patrol) preset
/// consumes `spawn_world_x` for its lane center and, when authored, `patrol_radius`
/// as a lane-radius override. Every non-patrol preset ignores the patrol field.
///
/// This is the transient build INPUT. The persistent, snapshot-safe form an actor
/// carries is [`AuthoredBrainContext`], which produces one of these.
#[derive(Clone, Debug, PartialEq)]
pub struct BrainBuildContext {
    /// The patrol lane center anchor (world X).
    pub spawn_world_x: f32,
    /// Placement lane-radius override for a selected patrol preset. `None` keeps
    /// the preset's authored radius.
    pub patrol_radius: Option<f32>,
}

impl BrainBuildContext {
    /// A context that only anchors the patrol lane center (no radius override).
    pub fn at(spawn_world_x: f32) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: None,
        }
    }

    /// A context carrying a placement's authored patrol radius. A non-positive
    /// `patrol_radius` is treated as "unset" (keep the preset's radius).
    pub fn from_placement(spawn_world_x: f32, patrol_radius: f32) -> Self {
        Self {
            spawn_world_x,
            patrol_radius: (patrol_radius > 0.0).then_some(patrol_radius),
        }
    }
}

/// The authored brain-build context an actor carries for the life of the entity.
///
/// Captured at spawn from the placement (its world anchor + authored patrol
/// radius) and kept through runtime brain switches, so `RestoreDefault` / a
/// snapshot reconcile rebuild a patrol brain around its authored HOME, not
/// wherever the actor happened to wander. A separate component (rather than a
/// field on [`BrainBinding`]) so the binding stays compact and equality-friendly.
///
/// Note on patrol PATHS: a `path_id` is a separate movement attachment
/// (`ActorMotionPath`), not a brain-build parameter — the lane-based patrol
/// preset does not consume one. So it is deliberately absent here.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct AuthoredBrainContext {
    /// The actor's authored world-space spawn X — the patrol lane center anchor.
    pub spawn_anchor_x: f32,
    /// The placement's authored patrol-radius override, if any.
    pub patrol_radius: Option<f32>,
}

impl AuthoredBrainContext {
    /// Capture a placement's authored patrol tuning. A non-positive radius is
    /// treated as "unset".
    pub fn from_placement(spawn_anchor_x: f32, patrol_radius: f32) -> Self {
        Self {
            spawn_anchor_x,
            patrol_radius: (patrol_radius > 0.0).then_some(patrol_radius),
        }
    }

    /// The transient build input a preset resolver consumes.
    pub fn build_context(&self) -> BrainBuildContext {
        BrainBuildContext {
            spawn_world_x: self.spawn_anchor_x,
            patrol_radius: self.patrol_radius,
        }
    }
}

/// Which side of the precedence chain a failing preset came from — for a clear
/// spawn/validation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetSource {
    /// The placement's explicit `brain_override`.
    Override,
    /// The character catalog `default_brain`.
    CharacterDefault,
}

impl std::fmt::Display for PresetSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Override => f.write_str("brain_override"),
            Self::CharacterDefault => f.write_str("catalog default_brain"),
        }
    }
}

/// Why an initial-brain resolution failed. Both variants are content errors: an
/// unknown character id, or a named preset that isn't in the catalog. Neither
/// falls back silently — the spawn/validation site fails loud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrainBuildError {
    /// `character_id` is not in the catalog.
    UnknownCharacter(String),
    /// The selected preset name is not in `brain_presets`. `resolved` is the key
    /// actually looked up after namespace qualification.
    UnknownPreset {
        character_id: String,
        preset: String,
        resolved: String,
        source: PresetSource,
    },
    /// NOBODY NAMES A PRESET, and no placement override was given.
    ///
    ///  it is NOT necessarily an error. A character that states a
    /// [`BrainProfile`] instead is fully authored — this is the signal to ask
    /// THAT authority, which only a caller holding the body can do.
    ///
    /// [`BrainProfile`]: crate::brain::BrainProfile
    NoAutonomousDefault { character_id: String },
}

impl std::fmt::Display for BrainBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCharacter(id) => {
                write!(
                    f,
                    "unknown character_id `{id}` (not in the character catalog)"
                )
            }
            Self::UnknownPreset {
                character_id,
                preset,
                resolved,
                source,
            } => write!(
                f,
                "character `{character_id}`: {source} names unknown brain preset `{preset}` \
                 (resolved to `{resolved}`, not in brain_presets)"
            ),
            Self::NoAutonomousDefault { character_id } => write!(
                f,
                "character `{character_id}`: nothing names an autonomous default — its \
                 definition states no brain profile and its catalog row's `default_brain` \
                 is empty. A caller holding the body must ask the character's own \
                 `BrainProfile`; there is no preset to build."
            ),
        }
    }
}

impl std::error::Error for BrainBuildError {}

/// Prepared-content validation for a placement's `brain_override`, WITHOUT
/// building a brain. Applies the same deterministic namespace rule as spawn
/// ([`qualify_preset_like`]): a raw override resolves within the character's
/// own provider namespace, a fully-qualified override is used exactly.
///
/// `None`/empty means the character default (already validated at catalog
/// assembly) → `Ok(None)`. An unknown character, or an override that does not
/// resolve, is a loud [`BrainBuildError`] — never a silent pass. Returns the
/// resolved preset key so a caller can log what it validated.
pub fn validate_brain_override(
    catalog: &CharacterCatalog,
    character_id: &str,
    authored_override: Option<&str>,
) -> Result<Option<BrainPresetId>, BrainBuildError> {
    let entry = catalog
        .get(character_id)
        .ok_or_else(|| BrainBuildError::UnknownCharacter(character_id.to_string()))?;
    let Some(name) = authored_override.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    // A registered row names its provider. A raw fragment can instead carry
    // that namespace in its preset keys.
    let namespace_carrier = if !entry.provider.is_empty() {
        format!("{}::_", entry.provider)
    } else if !entry.default_action_set.is_empty() {
        entry.default_action_set.clone()
    } else {
        entry.default_brain.clone()
    };
    let key = qualify_preset_like(&namespace_carrier, name);
    if catalog.has_brain_preset(&key) {
        Ok(Some(BrainPresetId::new(key)))
    } else {
        Err(BrainBuildError::UnknownPreset {
            character_id: character_id.to_string(),
            preset: name.to_string(),
            resolved: key,
            source: PresetSource::Override,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_override_needs_no_autonomous_default() {
        use crate::actor::character_catalog::{parse_catalog, test_fragments::A};
        let text = A.replace("default_brain: \"idle\"", "default_brain: \"\"");
        let catalog = CharacterCatalog::from_data(parse_catalog(&text));
        assert!(matches!(
            resolve_initial_binding(&catalog, "alpha", None),
            Err(BrainBuildError::NoAutonomousDefault { .. })
        ));
        let binding = resolve_initial_binding(&catalog, "alpha", Some("idle")).unwrap();
        assert_eq!(binding.default_preset, AutonomousDefault::None);
        assert_eq!(binding.active_preset(), Some(&BrainPresetId::new("idle")));
        assert!(matches!(
            resolve_initial_binding(&catalog, "alpha", Some("unknown")),
            Err(BrainBuildError::UnknownPreset {
                source: PresetSource::Override,
                ..
            })
        ));
    }

    /// A provoked source has no active catalog preset — reconciliation reads
    /// this to rebuild the provoked mode rather than the catalog default.
    #[test]
    fn provoked_source_has_no_active_preset() {
        let mut binding = BrainBinding::new(
            BrainPresetId::new("wanderer_puppy_slug"),
            AutonomousSource::CatalogDefault,
        );
        assert!(!binding.is_provoked());
        binding.source = AutonomousSource::ProvokedProfile {
            profile: ambition_entity_catalog::BrainProfileId::new("cellular_duelist"),
        };
        assert!(binding.is_provoked());
        assert_eq!(binding.active_preset(), None);
    }

    /// A restored controller uses the placement's patrol anchor and radius.
    #[test]
    fn authored_context_reproduces_the_spawn_lane() {
        let authored = AuthoredBrainContext::from_placement(100.0, 200.0);
        let ctx = authored.build_context();
        assert_eq!(ctx.spawn_world_x, 100.0);
        assert_eq!(ctx.patrol_radius, Some(200.0));
        // A non-positive radius is "unset".
        assert_eq!(
            AuthoredBrainContext::from_placement(100.0, 0.0).patrol_radius,
            None
        );
    }
}

#[cfg(test)]

/// Prepared-content validation of `brain_override` against a MULTI-PROVIDER
/// assembled catalog (the real Hall composition: `default`/`sanic`/`mary_o`
/// each own their own `stand_still`). Proves the deterministic namespace rule:
/// a raw override resolves ONLY within the character's own provider namespace,
/// a qualified override is used exactly, and nothing leaks across providers.
mod brain_override_validation {
    use super::*;
    use crate::actor::character_catalog::*;

    fn frag(provider: &str, ron: &str) -> CharacterCatalogFragment {
        CharacterCatalogFragment::from_ron(provider, None::<String>, ron).expect("fragment parses")
    }

    fn character(id: &str, default_brain: &str) -> String {
        format!(
            r#""{id}": (
                    display_name: "{id}", manifest: "x_spritesheet.ron",
                    tier: MainHall, body_kind: Standard, composition: None,
                    default_brain: "{default_brain}", default_action_set: "peaceful", tags: [],
                )"#
        )
    }

    // The assembled host. `default` also owns a `shared_only` preset that the
    // other providers do NOT — the cross-provider-leak probe.
    fn host() -> CharacterCatalog {
        let mut reg = CharacterCatalogRegistry::default();
        reg.register(frag(
            "default",
            &format!(
                r#"(
                    brain_presets: {{ "stand_still": StandStill, "shared_only": StandStill }},
                    action_set_presets: {{ "peaceful": (move_style: Walk) }},
                    characters: {{ {} }},
                )"#,
                character("amb_hero", "stand_still")
            ),
        ))
        .unwrap();
        reg.register(frag(
            "sanic",
            &format!(
                r#"(
                    brain_presets: {{
                        "stand_still": StandStill,
                        "sanic_special": Wanderer(speed: 9.0, aggressiveness: 0.0),
                    }},
                    action_set_presets: {{ "peaceful": (move_style: Walk) }},
                    characters: {{ {} }},
                )"#,
                character("sanic", "stand_still")
            ),
        ))
        .unwrap();
        reg.register(frag(
            "mary_o",
            &format!(
                r#"(
                    brain_presets: {{ "stand_still": StandStill }},
                    action_set_presets: {{ "peaceful": (move_style: Walk) }},
                    characters: {{ {} }},
                )"#,
                character("mary_o", "stand_still")
            ),
        ))
        .unwrap();
        reg.assemble().expect("assembles").catalog
    }

    #[test]
    fn each_provider_resolves_its_own_stand_still() {
        let cat = host();
        // Ambition-owned, Sanic, and Mary-O each resolve the RAW "stand_still"
        // into their OWN provider namespace.
        for (character, provider) in [
            ("amb_hero", "default"),
            ("sanic", "sanic"),
            ("mary_o", "mary_o"),
        ] {
            assert_eq!(
                validate_brain_override(&cat, character, Some("stand_still")).unwrap(),
                Some(BrainPresetId::new(format!("{provider}::stand_still"))),
                "{character}'s raw stand_still resolves in {provider}",
            );
        }
    }

    #[test]
    fn a_fully_qualified_override_is_used_exactly() {
        let cat = host();
        // Sanic authored with a fully-qualified preset from ANOTHER provider —
        // used verbatim (it exists in the assembled catalog).
        assert_eq!(
            validate_brain_override(&cat, "sanic", Some("default::stand_still")).unwrap(),
            Some(BrainPresetId::new("default::stand_still")),
        );
    }

    #[test]
    fn a_provider_local_preset_resolves() {
        let cat = host();
        assert_eq!(
            validate_brain_override(&cat, "sanic", Some("sanic_special")).unwrap(),
            Some(BrainPresetId::new("sanic::sanic_special")),
        );
    }

    #[test]
    fn a_raw_override_does_not_leak_across_providers() {
        let cat = host();
        // `shared_only` exists ONLY under `default`. Sanic authoring it raw must
        // NOT silently resolve to `default::shared_only` — it fails, because a
        // raw override resolves only in the character's own namespace.
        let err = crate::control::brain_binding::validate_brain_override(
            &(cat),
            "sanic",
            Some("shared_only"),
        )
        .unwrap_err();
        assert!(
            matches!(&err, BrainBuildError::UnknownPreset { resolved, .. } if resolved == "sanic::shared_only"),
            "a raw cross-provider preset must fail, not leak: {err:?}",
        );
    }

    #[test]
    fn a_missing_preset_fails_and_empty_is_ok() {
        let cat = host();
        assert!(matches!(
            validate_brain_override(&cat, "amb_hero", Some("no_such")).unwrap_err(),
            BrainBuildError::UnknownPreset { .. }
        ));
        // Absent / empty override is always valid (the character default).
        assert_eq!(
            validate_brain_override(&cat, "amb_hero", None).unwrap(),
            None
        );
        assert_eq!(
            validate_brain_override(&cat, "amb_hero", Some("  ")).unwrap(),
            None
        );
        // An unknown character is a loud error.
        assert!(matches!(
            validate_brain_override(&cat, "nobody", Some("stand_still")).unwrap_err(),
            BrainBuildError::UnknownCharacter(_)
        ));
    }
}
/// Select a reconstructible autonomous preset from the authored override and default.
pub fn resolve_initial_binding(
    catalog: &CharacterCatalog,
    character_id: &str,
    authored_override: Option<&str>,
) -> Result<BrainBinding, BrainBuildError> {
    let entry = catalog
        .get(character_id)
        .ok_or_else(|| BrainBuildError::UnknownCharacter(character_id.to_string()))?;
    let default = Some(BrainPresetId::new(entry.default_brain.clone()))
        .filter(|id| !id.as_str().trim().is_empty());
    let source = match validate_brain_override(catalog, character_id, authored_override)? {
        Some(preset) => AutonomousSource::CatalogPreset(preset),
        None => {
            let preset = default
                .as_ref()
                .ok_or_else(|| BrainBuildError::NoAutonomousDefault {
                    character_id: character_id.to_string(),
                })?;
            if !catalog.has_brain_preset(preset.as_str()) {
                return Err(BrainBuildError::UnknownPreset {
                    character_id: character_id.to_string(),
                    preset: preset.as_str().to_string(),
                    resolved: preset.as_str().to_string(),
                    source: PresetSource::CharacterDefault,
                });
            }
            AutonomousSource::CatalogDefault
        }
    };
    Ok(BrainBinding {
        default_preset: default.map_or(AutonomousDefault::None, AutonomousDefault::Preset),
        source,
    })
}

/// Runtime source to restore when temporary control ends.
///
/// Preset-backed, character-profile, and absent defaults are distinct so restore
/// logic never encodes one source as an empty id or guesses which authority to use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AutonomousDefault {
    /// Nothing to restore. A boss rebuilds its autonomous mode from its own
    /// authority; there is no default to go back to here.
    None,
    /// The character's catalog `default_brain`, captured at spawn. Restoring
    /// rebuilds a fresh brain from THIS preset.
    Preset(BrainPresetId),
    /// The character's own authored [`BrainProfile`] is the default. Nothing
    /// to look up — the body carries the policy its character stated, and
    /// lowering it needs the body anyway.
    ///
    /// [`BrainProfile`]: crate::brain::BrainProfile
    CharacterProfile,
}

impl AutonomousDefault {
    /// The preset to rebuild from, when the default IS a preset.
    pub fn preset(&self) -> Option<&BrainPresetId> {
        match self {
            Self::Preset(id) => Some(id),
            Self::None | Self::CharacterProfile => None,
        }
    }
}
