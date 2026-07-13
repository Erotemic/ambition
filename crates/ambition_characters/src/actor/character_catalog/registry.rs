//! App-local composition of independently authored character catalogs.
//!
//! Each experience provider registers one fragment. The registry rebuilds the
//! assembled [`CharacterCatalog`] deterministically after every registration,
//! so plugin order is not authority. Local preset names are namespaced during
//! assembly; character ids remain the cross-provider identity and therefore
//! must be globally unique within one `App`.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::{App, Resource};

use super::{try_parse_catalog, validator, CharacterCatalog, CharacterCatalogData};

/// One provider's immutable character definitions.
#[derive(Clone, Debug)]
pub struct CharacterCatalogFragment {
    provider_id: String,
    default_character_id: Option<String>,
    catalog: CharacterCatalogData,
    source_ron: String,
}

impl CharacterCatalogFragment {
    pub fn from_ron(
        provider_id: impl Into<String>,
        default_character_id: Option<impl Into<String>>,
        catalog_ron: &str,
    ) -> Result<Self, CharacterCatalogAssemblyError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(CharacterCatalogAssemblyError::EmptyProviderId);
        }
        let catalog = try_parse_catalog(catalog_ron).map_err(|message| {
            CharacterCatalogAssemblyError::MalformedFragment {
                provider_id: provider_id.clone(),
                message,
            }
        })?;
        let validation = validator::validate(&catalog);
        if !validation.is_empty() {
            return Err(CharacterCatalogAssemblyError::InvalidFragment {
                provider_id,
                errors: validation,
            });
        }
        let default_character_id = default_character_id.map(Into::into);
        if let Some(default_id) = default_character_id.as_deref() {
            if !catalog.characters.contains_key(default_id) {
                return Err(CharacterCatalogAssemblyError::MissingDefaultCharacter {
                    provider_id,
                    character_id: default_id.to_string(),
                });
            }
        }
        Ok(Self {
            provider_id,
            default_character_id,
            catalog,
            source_ron: catalog_ron.to_string(),
        })
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn default_character_id(&self) -> Option<&str> {
        self.default_character_id.as_deref()
    }

    pub fn catalog(&self) -> &CharacterCatalogData {
        &self.catalog
    }

    fn validate(&self) -> Result<(), CharacterCatalogAssemblyError> {
        if self.provider_id.trim().is_empty() {
            return Err(CharacterCatalogAssemblyError::EmptyProviderId);
        }
        let errors = validator::validate(&self.catalog);
        if !errors.is_empty() {
            return Err(CharacterCatalogAssemblyError::InvalidFragment {
                provider_id: self.provider_id.clone(),
                errors,
            });
        }
        if let Some(default_id) = self.default_character_id.as_deref() {
            if !self.catalog.characters.contains_key(default_id) {
                return Err(CharacterCatalogAssemblyError::MissingDefaultCharacter {
                    provider_id: self.provider_id.clone(),
                    character_id: default_id.to_string(),
                });
            }
        }
        Ok(())
    }
}

/// All linked provider fragments for one Bevy `App`.
#[derive(Resource, Clone, Debug, Default)]
pub struct CharacterCatalogRegistry {
    fragments: BTreeMap<String, CharacterCatalogFragment>,
}

impl CharacterCatalogRegistry {
    pub fn register(
        &mut self,
        fragment: CharacterCatalogFragment,
    ) -> Result<(), CharacterCatalogAssemblyError> {
        fragment.validate()?;
        if let Some(existing) = self.fragments.get(&fragment.provider_id) {
            if existing.default_character_id == fragment.default_character_id
                && existing.source_ron == fragment.source_ron
            {
                return Ok(());
            }
            return Err(CharacterCatalogAssemblyError::DuplicateProvider {
                provider_id: fragment.provider_id,
            });
        }
        self.fragments
            .insert(fragment.provider_id.clone(), fragment);
        Ok(())
    }

    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.fragments.keys().map(String::as_str)
    }

    pub fn assemble(&self) -> Result<AssembledCharacterCatalog, CharacterCatalogAssemblyError> {
        let mut brain_presets = BTreeMap::new();
        let mut action_set_presets = BTreeMap::new();
        let mut characters = BTreeMap::new();
        let mut defaults = BTreeMap::new();
        let mut owners: BTreeMap<String, String> = BTreeMap::new();

        for (provider_id, fragment) in &self.fragments {
            let brain_names: BTreeMap<String, String> = fragment
                .catalog
                .brain_presets
                .keys()
                .map(|name| (name.clone(), namespaced(provider_id, name)))
                .collect();
            let action_names: BTreeMap<String, String> = fragment
                .catalog
                .action_set_presets
                .keys()
                .map(|name| (name.clone(), namespaced(provider_id, name)))
                .collect();

            for (local_name, preset) in &fragment.catalog.brain_presets {
                brain_presets.insert(brain_names[local_name].clone(), preset.clone());
            }
            for (local_name, preset) in &fragment.catalog.action_set_presets {
                action_set_presets.insert(action_names[local_name].clone(), preset.clone());
            }

            for (character_id, entry) in &fragment.catalog.characters {
                if let Some(existing_provider) = owners.get(character_id) {
                    return Err(CharacterCatalogAssemblyError::DuplicateCharacter {
                        character_id: character_id.clone(),
                        first_provider: existing_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                let mut entry = entry.clone();
                entry.default_brain = brain_names
                    .get(&entry.default_brain)
                    .expect("fragment validation guarantees the brain preset")
                    .clone();
                entry.default_action_set = action_names
                    .get(&entry.default_action_set)
                    .expect("fragment validation guarantees the action-set preset")
                    .clone();
                owners.insert(character_id.clone(), provider_id.clone());
                characters.insert(character_id.clone(), entry);
            }

            if let Some(default_id) = &fragment.default_character_id {
                defaults.insert(provider_id.clone(), default_id.clone());
            }
        }

        let catalog = CharacterCatalog::from_data(CharacterCatalogData {
            brain_presets,
            action_set_presets,
            characters,
        });
        let validation = validator::validate(catalog.data());
        if !validation.is_empty() {
            return Err(CharacterCatalogAssemblyError::InvalidAssembly(validation));
        }
        Ok(AssembledCharacterCatalog {
            catalog,
            defaults: CharacterCatalogDefaults(defaults),
            owners: CharacterCatalogOwners(owners),
        })
    }
}

fn namespaced(provider_id: &str, local_name: &str) -> String {
    format!("{provider_id}::{local_name}")
}

/// Provider-specific default character ids.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterCatalogDefaults(pub BTreeMap<String, String>);

impl CharacterCatalogDefaults {
    pub fn for_provider(&self, provider_id: &str) -> Option<&str> {
        self.0.get(provider_id).map(String::as_str)
    }
}

/// Which provider authored each globally visible character id.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterCatalogOwners(pub BTreeMap<String, String>);

impl CharacterCatalogOwners {
    pub fn provider_for(&self, character_id: &str) -> Option<&str> {
        self.0.get(character_id).map(String::as_str)
    }
}

#[derive(Clone, Debug)]
pub struct AssembledCharacterCatalog {
    pub catalog: CharacterCatalog,
    pub defaults: CharacterCatalogDefaults,
    pub owners: CharacterCatalogOwners,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterCatalogAssemblyError {
    EmptyProviderId,
    DuplicateProvider {
        provider_id: String,
    },
    MalformedFragment {
        provider_id: String,
        message: String,
    },
    InvalidFragment {
        provider_id: String,
        errors: Vec<String>,
    },
    MissingDefaultCharacter {
        provider_id: String,
        character_id: String,
    },
    DuplicateCharacter {
        character_id: String,
        first_provider: String,
        second_provider: String,
    },
    InvalidAssembly(Vec<String>),
}

impl fmt::Display for CharacterCatalogAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "character catalog provider id must not be empty"),
            Self::DuplicateProvider { provider_id } => {
                write!(f, "character catalog provider '{provider_id}' registered twice")
            }
            Self::MalformedFragment {
                provider_id,
                message,
            } => write!(
                f,
                "character catalog fragment '{provider_id}' is malformed RON: {message}"
            ),
            Self::InvalidFragment {
                provider_id,
                errors,
            } => write!(
                f,
                "character catalog fragment '{provider_id}' is invalid:\n  - {}",
                errors.join("\n  - ")
            ),
            Self::MissingDefaultCharacter {
                provider_id,
                character_id,
            } => write!(
                f,
                "character catalog fragment '{provider_id}' names missing default character '{character_id}'"
            ),
            Self::DuplicateCharacter {
                character_id,
                first_provider,
                second_provider,
            } => write!(
                f,
                "character id '{character_id}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::InvalidAssembly(errors) => write!(
                f,
                "assembled character catalog is invalid:\n  - {}",
                errors.join("\n  - ")
            ),
        }
    }
}

impl std::error::Error for CharacterCatalogAssemblyError {}

/// App build-time registration seam used by experience providers.
pub trait CharacterCatalogAppExt {
    fn try_register_character_catalog_fragment(
        &mut self,
        fragment: CharacterCatalogFragment,
    ) -> Result<&mut Self, CharacterCatalogAssemblyError>;

    fn register_character_catalog_fragment(
        &mut self,
        fragment: CharacterCatalogFragment,
    ) -> &mut Self {
        self.try_register_character_catalog_fragment(fragment)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl CharacterCatalogAppExt for App {
    fn try_register_character_catalog_fragment(
        &mut self,
        fragment: CharacterCatalogFragment,
    ) -> Result<&mut Self, CharacterCatalogAssemblyError> {
        let (registry, assembled) = {
            let mut candidate = self
                .world()
                .get_resource::<CharacterCatalogRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(fragment)?;
            let assembled = candidate.assemble()?;
            (candidate, assembled)
        };
        self.insert_resource(registry)
            .insert_resource(assembled.catalog)
            .insert_resource(assembled.defaults)
            .insert_resource(assembled.owners);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "alpha": (
                display_name: "Alpha", spritesheet: "a.png", manifest: "a.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;
    const B: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "beta": (
                display_name: "Beta", spritesheet: "b.png", manifest: "b.ron",
                tier: MainHall, body_kind: Standard, composition: None,
                default_brain: "idle", default_action_set: "peaceful", tags: [],
            ),
        },
    )"#;

    fn fragment(provider: &str, default_id: &str, ron: &str) -> CharacterCatalogFragment {
        CharacterCatalogFragment::from_ron(provider, Some(default_id), ron).unwrap()
    }

    #[test]
    fn malformed_ron_is_a_structured_error() {
        let error = CharacterCatalogFragment::from_ron("broken", None::<String>, "not ron")
            .expect_err("malformed provider data must not panic");
        assert!(matches!(
            error,
            CharacterCatalogAssemblyError::MalformedFragment { provider_id, .. }
                if provider_id == "broken"
        ));
    }

    #[test]
    fn provider_order_does_not_change_the_assembly() {
        let mut first = CharacterCatalogRegistry::default();
        first.register(fragment("a", "alpha", A)).unwrap();
        first.register(fragment("b", "beta", B)).unwrap();
        let first = first.assemble().unwrap();

        let mut second = CharacterCatalogRegistry::default();
        second.register(fragment("b", "beta", B)).unwrap();
        second.register(fragment("a", "alpha", A)).unwrap();
        let second = second.assemble().unwrap();

        assert_eq!(first.catalog, second.catalog);
        assert_eq!(first.defaults, second.defaults);
        assert_eq!(first.owners, second.owners);
        assert!(first.catalog.data().brain_presets.contains_key("a::idle"));
        assert!(first.catalog.data().brain_presets.contains_key("b::idle"));
    }

    #[test]
    fn duplicate_character_ids_fail_with_stable_provider_names() {
        let mut registry = CharacterCatalogRegistry::default();
        registry.register(fragment("a", "alpha", A)).unwrap();
        registry.register(fragment("b", "alpha", A)).unwrap();
        assert_eq!(
            registry.assemble().unwrap_err(),
            CharacterCatalogAssemblyError::DuplicateCharacter {
                character_id: "alpha".to_string(),
                first_provider: "a".to_string(),
                second_provider: "b".to_string(),
            }
        );
    }

    #[test]
    fn failed_registration_leaves_the_previous_assembly_intact() {
        let mut app = App::new();
        app.register_character_catalog_fragment(fragment("a", "alpha", A));
        let before = app
            .world()
            .resource::<CharacterCatalog>()
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();

        let error = app
            .try_register_character_catalog_fragment(fragment("b", "alpha", A))
            .err()
            .expect("registration should fail");
        assert!(matches!(
            error,
            CharacterCatalogAssemblyError::DuplicateCharacter { .. }
        ));
        let after = app
            .world()
            .resource::<CharacterCatalog>()
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        assert_eq!(before, after);
        assert_eq!(
            app.world()
                .resource::<CharacterCatalogRegistry>()
                .providers()
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    fn separate_apps_hold_independent_catalogs() {
        let mut app_a = App::new();
        app_a.register_character_catalog_fragment(fragment("a", "alpha", A));
        let mut app_b = App::new();
        app_b.register_character_catalog_fragment(fragment("b", "beta", B));

        let catalog_a = app_a.world().resource::<CharacterCatalog>();
        let catalog_b = app_b.world().resource::<CharacterCatalog>();
        assert!(catalog_a.get("alpha").is_some());
        assert!(catalog_a.get("beta").is_none());
        assert!(catalog_b.get("beta").is_some());
        assert!(catalog_b.get("alpha").is_none());
    }
}
