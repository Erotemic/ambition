//! Register catalog fragments and install their character preparation barrier.

use crate::actor::character_catalog::{
    CharacterCatalogAssemblyError, CharacterCatalogFragment, CharacterCatalogRegistry,
};
use bevy::prelude::App;

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
            .insert_resource(assembled.declarations)
            .insert_resource(assembled.owners)
            .insert_resource(assembled.brain_profiles);
        // A catalog row is a character, so a fragment brings the preparation
        // barrier that prepares its rows, exactly as a registered definition
        // does (`stage_authored_character`).
        if !self.is_plugin_added::<crate::prepared::CharacterPreparationPlugin>()
            && matches!(
                self.plugins_state(),
                bevy::app::PluginsState::Adding | bevy::app::PluginsState::Ready
            )
        {
            self.add_plugins(crate::prepared::CharacterPreparationPlugin);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::character_catalog::test_fragments::{fragment, A, B};
    use crate::actor::character_catalog::*;

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

    /// A character's barks have one owner (Q52): its catalog row, which one
    /// provider authors. A second provider that authors barks for the same
    /// character is a conflict that names both providers. The lines are not
    /// replaced and not joined.
    #[test]
    fn a_second_provider_cannot_author_the_barks_of_one_character() {
        use crate::actor::character_catalog::BarkSituation;
        let row = |line: &str| {
            format!(
                r#"(
                    brain_presets: {{ "idle": StandStill }},
                    action_set_presets: {{ "peaceful": (move_style: Walk) }},
                    characters: {{
                        "alpha": (
                            display_name: "Alpha", manifest: "a.ron",
                            tier: MainHall, body_kind: Standard, composition: None,
                            default_brain: "idle", default_action_set: "peaceful", tags: [],
                            barks: ( on_hit: ["{line}"] ),
                        ),
                    }},
                )"#
            )
        };
        let mut app = App::new();
        app.register_character_catalog_fragment(fragment("a", "alpha", &row("first")));
        let error = app
            .try_register_character_catalog_fragment(fragment("b", "alpha", &row("second")))
            .err()
            .expect("a second owner of one character's barks is refused");
        assert_eq!(
            error,
            CharacterCatalogAssemblyError::DuplicateCharacter {
                character_id: "alpha".to_string(),
                first_provider: "a".to_string(),
                second_provider: "b".to_string(),
            }
        );
        let report = error.to_string();
        assert!(report.contains("'a'") && report.contains("'b'"), "{report}");
        let catalog = app.world().resource::<CharacterCatalog>();
        for rotation in 0..3 {
            assert_eq!(
                catalog.bark_line("alpha", BarkSituation::OnHit, rotation),
                Some("first"),
                "the refused provider's line was joined to the pool or replaced it"
            );
        }
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
