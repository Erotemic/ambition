use super::*;

pub use crate::cue_spec::{
    BOSS_PHASE_KEYS, BossMusicBinding, BossPhaseState, EncounterMusicBinding, MusicCueMedia,
    MusicCueSpec, MusicLayerGainSpec, MusicLayerSourceSpec, MusicLayerSpec, MusicPhasePolicy,
    MusicProductSet, MusicSectionSpec, MusicStateBalanceOverride, MusicStateSpec, RoomMusicBinding,
    RoomPhaseTrack,
};

pub use crate::cue_program::PreparedMusicCues as MusicCueCatalog;

#[derive(Resource, Clone, Default)]
pub struct LoadedMusicCueAssets {
    /// Each handle with the asset path it was requested for. The path is part
    /// of what was loaded: a content reload can point the same cue, section and
    /// layer at another file, and a handle that outlives its path would keep
    /// playing the file the author replaced.
    pub(super) sources: HashMap<MusicSourceKey, (String, Handle<KiraAudioSource>)>,
}

impl LoadedMusicCueAssets {
    pub(super) fn get(
        &self,
        provider_id: &str,
        generation: u64,
        cue_id: &str,
        section_id: &str,
        layer_id: &str,
    ) -> Option<Handle<KiraAudioSource>> {
        self.sources
            .get(&MusicSourceKey::new(
                provider_id,
                generation,
                cue_id,
                section_id,
                layer_id,
            ))
            .map(|(_, handle)| handle.clone())
    }

    pub(super) fn retire_generations(
        &mut self,
        catalogs: &AdaptiveMusicCatalogRegistry,
        retained: &[(&str, u64)],
    ) {
        self.sources.retain(|key, _| {
            key.generation == catalogs.generation(&key.provider_id)
                || retained.iter().any(|(provider, generation)| {
                    *provider == key.provider_id && *generation == key.generation
                })
        });
    }

    /// Does the cache hold `key` for exactly this asset path? A different path
    /// under the same key is a stale entry, not a hit.
    pub(super) fn holds(&self, key: &MusicSourceKey, path: &str) -> bool {
        self.sources
            .get(key)
            .is_some_and(|(loaded, _)| loaded == path)
    }

    /// Request the selected state and its immediate continuation.
    pub(super) fn ensure_state_loaded(
        &mut self,
        provider_id: &str,
        generation: u64,
        cue: &PreparedMusicCue,
        state: &PreparedMusicState,
        asset_server: &AssetServer,
    ) {
        for state in
            std::iter::once(state).chain(state.next_state.as_deref().and_then(|id| cue.state(id)))
        {
            let section = cue
                .section(&state.section_id)
                .expect("a prepared state has its section");
            for source in &section.sources {
                let key = MusicSourceKey::new(
                    provider_id,
                    generation,
                    &cue.id,
                    &section.id,
                    &source.layer_id,
                );
                let rel = source.path.clone();
                if !self.holds(&key, &rel) {
                    let handle = asset_server.load(rel.clone());
                    self.sources.insert(key, (rel, handle));
                }
            }
        }
    }

    pub(super) fn section_ready(
        &self,
        provider_id: &str,
        generation: u64,
        cue: &PreparedMusicCue,
        section_id: &str,
        decoded: &Assets<KiraAudioSource>,
    ) -> bool {
        cue.section(section_id).is_some_and(|section| {
            section.sources.iter().all(|source| {
                self.get(
                    provider_id,
                    generation,
                    &cue.id,
                    &section.id,
                    &source.layer_id,
                )
                .is_some_and(|handle| decoded.contains(handle.id()))
            })
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct MusicSourceKey {
    provider_id: String,
    generation: u64,
    cue_id: String,
    section_id: String,
    layer_id: String,
}

impl MusicSourceKey {
    pub(super) fn new(
        provider_id: &str,
        generation: u64,
        cue_id: &str,
        section_id: &str,
        layer_id: &str,
    ) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            generation,
            cue_id: cue_id.to_string(),
            section_id: section_id.to_string(),
            layer_id: layer_id.to_string(),
        }
    }
}

/// App-local adaptive music definitions contributed by linked providers.
///
/// Storage and authority remain distinct: this registry may cache definitions
/// for every linked provider, while `ActiveAudioSelection` chooses the one
/// provider whose catalog may drive the director for the current shell context.
#[derive(Resource, Clone, Debug, Default)]
pub struct AdaptiveMusicCatalogRegistry {
    providers: std::collections::BTreeMap<String, MusicCueCatalog>,
    generations: std::collections::BTreeMap<String, u64>,
    next_generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptiveMusicCatalogError {
    EmptyProviderId,
    DuplicateProvider { provider_id: String },
}

impl std::fmt::Display for AdaptiveMusicCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "adaptive music provider id must not be empty"),
            Self::DuplicateProvider { provider_id } => write!(
                f,
                "adaptive music provider '{provider_id}' registered different definitions twice"
            ),
        }
    }
}

impl std::error::Error for AdaptiveMusicCatalogError {}

impl AdaptiveMusicCatalogRegistry {
    pub fn register(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<(), AdaptiveMusicCatalogError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(AdaptiveMusicCatalogError::EmptyProviderId);
        }
        if let Some(existing) = self.providers.get(&provider_id) {
            if existing == &catalog {
                return Ok(());
            }
            return Err(AdaptiveMusicCatalogError::DuplicateProvider { provider_id });
        }
        // Cue ids are provider-local. Two providers may deliberately use the
        // same neutral cue/state vocabulary while resolving different assets;
        // the active audio context selects one complete catalog.
        self.next_generation += 1;
        for (cue, description) in catalog.ungenerated() {
            warn!(
                "music provider {provider_id:?}: cue {cue:?} is unavailable until {description:?} is generated"
            );
        }
        self.generations
            .insert(provider_id.clone(), self.next_generation);
        self.providers.insert(provider_id, catalog);
        Ok(())
    }

    /// This registry with `provider_id`'s catalog replaced by `catalog`, or
    /// removed when `catalog` is `None`; the receiver is not touched.
    ///
    /// A content reload's replacement: [`Self::register`] refuses a provider's
    /// SECOND, different definition on purpose (two composition-time plugins
    /// disagreeing is a bug), so a reload cannot go through it. This is the
    /// explicit lifecycle road. It validates the candidate exactly as
    /// `register` does, leaves every other provider's catalog as it was, and
    /// returns a new value, so a caller can admit the candidate first and
    /// publish it later, or drop it with nothing changed. Removal is for a
    /// candidate that stops declaring cues: keeping the provider's old catalog
    /// would let one family stay at N while the pack moved to N+1.
    pub fn with_replaced(
        &self,
        provider_id: &str,
        catalog: Option<MusicCueCatalog>,
    ) -> Result<Self, AdaptiveMusicCatalogError> {
        if provider_id.trim().is_empty() {
            return Err(AdaptiveMusicCatalogError::EmptyProviderId);
        }
        let mut next = self.clone();
        match catalog {
            Some(catalog) => {
                if next.providers.get(provider_id) != Some(&catalog) {
                    next.next_generation += 1;
                    next.generations
                        .insert(provider_id.to_owned(), next.next_generation);
                }
                next.providers.insert(provider_id.to_string(), catalog);
            }
            None => {
                next.providers.remove(provider_id);
            }
        }
        Ok(next)
    }

    pub fn generation(&self, provider_id: &str) -> u64 {
        self.generations.get(provider_id).copied().unwrap_or(0)
    }

    pub fn catalog_for(&self, provider_id: &str) -> Option<&MusicCueCatalog> {
        self.providers.get(provider_id)
    }

    pub fn cue_ids_for(&self, provider_id: &str) -> impl Iterator<Item = &str> {
        self.catalog_for(provider_id)
            .into_iter()
            .flat_map(MusicCueCatalog::cue_ids)
    }

    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.providers.keys().map(String::as_str)
    }
}

pub trait AdaptiveMusicCatalogAppExt {
    fn try_register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<&mut Self, AdaptiveMusicCatalogError>;

    fn register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> &mut Self {
        self.try_register_adaptive_music_catalog(provider_id, catalog)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl AdaptiveMusicCatalogAppExt for App {
    fn try_register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<&mut Self, AdaptiveMusicCatalogError> {
        let registry = {
            let mut candidate = self
                .world()
                .get_resource::<AdaptiveMusicCatalogRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(provider_id, catalog)?;
            candidate
        };
        self.insert_resource(registry);
        Ok(self)
    }
}

#[cfg(test)]
mod provider_registry_tests {
    use super::*;
    use bevy::prelude::App;

    fn catalog(cue_id: &str, path: &str) -> MusicCueCatalog {
        MusicCueCatalog::prepare(
            crate::cue_spec::AuthoredMusicCues {
                cues: vec![MusicCueSpec {
                    id: cue_id.to_owned(),
                    media: MusicCueMedia::Inline {
                        asset_root: "audio/adaptive".to_owned(),
                        bpm: 120.0,
                        beats_per_bar: 4.0,
                        sections: vec![MusicSectionSpec {
                            id: "loop".to_owned(),
                            duration_beats: 4.0,
                            looped: true,
                            sources: vec![MusicLayerSourceSpec {
                                layer_id: "full".to_owned(),
                                path: path.to_owned(),
                            }],
                        }],
                    },
                    relative_volume: 1.0,
                    layers: vec![MusicLayerSpec {
                        id: "full".to_owned(),
                        slot: 0,
                    }],
                    states: vec![MusicStateSpec {
                        id: "main".to_owned(),
                        section_id: "loop".to_owned(),
                        next_state: None,
                        gains: vec![MusicLayerGainSpec {
                            layer_id: "full".to_owned(),
                            gain: 1.0,
                        }],
                    }],
                    outro_state: None,
                    post_clear_bridge_state: None,
                    runtime_balance_overrides: Vec::new(),
                }],
                ..default()
            },
            &crate::music_media::MusicMediaCatalog::default(),
        )
        .unwrap()
    }

    #[test]
    fn two_apps_keep_different_provider_catalogs() {
        let mut a = App::new();
        a.register_adaptive_music_catalog("a", catalog("a_cue", "a.ogg"));
        let mut b = App::new();
        b.register_adaptive_music_catalog("b", catalog("b_cue", "b.ogg"));

        let a_registry = a.world().resource::<AdaptiveMusicCatalogRegistry>();
        assert!(a_registry.catalog_for("a").is_some());
        assert!(a_registry.catalog_for("b").is_none());
        let b_registry = b.world().resource::<AdaptiveMusicCatalogRegistry>();
        assert!(b_registry.catalog_for("b").is_some());
        assert!(b_registry.catalog_for("a").is_none());
    }

    #[test]
    fn the_same_cue_id_is_provider_local() {
        let mut registry = AdaptiveMusicCatalogRegistry::default();
        registry.register("a", catalog("shared", "a.ogg")).unwrap();
        registry.register("b", catalog("shared", "b.ogg")).unwrap();

        let a_path = &registry
            .catalog_for("a")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("provider a owns shared")
            .sections[0]
            .sources[0]
            .path;
        let b_path = &registry
            .catalog_for("b")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("provider b owns shared")
            .sections[0]
            .sources[0]
            .path;
        assert_eq!(a_path, "audio/adaptive/a.ogg");
        assert_eq!(b_path, "audio/adaptive/b.ogg");
    }

    #[test]
    fn failed_app_registration_preserves_the_prior_catalog() {
        let mut app = App::new();
        app.register_adaptive_music_catalog("a", catalog("shared", "a.ogg"));
        let error = app
            .try_register_adaptive_music_catalog("a", catalog("shared", "changed.ogg"))
            .unwrap_err();
        assert!(matches!(
            error,
            AdaptiveMusicCatalogError::DuplicateProvider { .. }
        ));
        let registry = app.world().resource::<AdaptiveMusicCatalogRegistry>();
        let path = &registry
            .catalog_for("a")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("prior provider remains")
            .sections[0]
            .sources[0]
            .path;
        assert_eq!(path, "audio/adaptive/a.ogg");
    }

    fn first_path(registry: &AdaptiveMusicCatalogRegistry, provider: &str, cue: &str) -> String {
        registry
            .catalog_for(provider)
            .and_then(|catalog| catalog.cue(cue))
            .expect("the provider's cue")
            .sections[0]
            .sources[0]
            .path
            .clone()
    }

    /// A reload replaces ONE provider's catalog in a new value: the other
    /// provider's catalog survives, and the registry it was built from is not
    /// touched until the caller publishes the result.
    #[test]
    fn replacing_one_providers_catalog_leaves_the_others_and_the_original() {
        let mut registry = AdaptiveMusicCatalogRegistry::default();
        registry.register("a", catalog("a_cue", "a.ogg")).unwrap();
        registry.register("b", catalog("b_cue", "b.ogg")).unwrap();

        let next = registry
            .with_replaced("a", Some(catalog("a_cue", "a_v2.ogg")))
            .expect("a valid catalog replaces the provider's");

        assert_eq!(first_path(&next, "a", "a_cue"), "audio/adaptive/a_v2.ogg");
        assert_eq!(
            first_path(&next, "b", "b_cue"),
            "audio/adaptive/b.ogg",
            "another provider's catalog survived"
        );
        assert_eq!(
            first_path(&registry, "a", "a_cue"),
            "audio/adaptive/a.ogg",
            "⛔ building the candidate changed the live registry"
        );
    }

    /// A candidate that stops declaring cues removes the provider's catalog
    /// from the new value, and only that provider's.
    #[test]
    fn replacing_with_nothing_removes_only_that_provider() {
        let mut registry = AdaptiveMusicCatalogRegistry::default();
        registry.register("a", catalog("a_cue", "a.ogg")).unwrap();
        registry.register("b", catalog("b_cue", "b.ogg")).unwrap();
        let next = registry.with_replaced("a", None).expect("removal is valid");
        assert!(next.catalog_for("a").is_none());
        assert!(next.catalog_for("b").is_some());
        assert!(registry.catalog_for("a").is_some(), "the receiver changed");
    }

    /// ⛔ A cached source is a hit only for the path it was requested for. A
    /// reload that points a cue's section at another file must request it
    /// again; a cache keyed by cue, section and layer alone kept playing the
    /// replaced file.
    #[test]
    fn a_cached_cue_source_is_stale_when_its_path_changed() {
        let mut assets = LoadedMusicCueAssets::default();
        let key = MusicSourceKey::new("a", 1, "a_cue", "loop", "full");
        assets.sources.insert(
            key.clone(),
            ("audio/adaptive/a.ogg".to_owned(), Handle::default()),
        );
        assert!(assets.holds(&key, "audio/adaptive/a.ogg"));
        assert!(
            !assets.holds(&key, "audio/adaptive/a_v2.ogg"),
            "the entry for the old path answered for the new one"
        );
        assert!(!assets.holds(
            &MusicSourceKey::new("a", 1, "a_cue", "loop", "other"),
            "audio/adaptive/a.ogg"
        ));
    }

    /// ⛔ The same, through the loader the music directive actually calls: a
    /// cue whose file changed is requested again and the cache then holds the
    /// new path; an unchanged cue is not requested a second time.
    #[test]
    fn ensure_state_loaded_requests_a_changed_path_again_and_an_unchanged_one_once() {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
        ))
        .init_asset::<KiraAudioSource>();
        let server = app.world().resource::<AssetServer>().clone();
        let cue_of = |file: &str| {
            let cue = catalog("a_cue", file)
                .cue("a_cue")
                .expect("the cue the helper authored")
                .clone();
            cue
        };
        let key = |cue: &PreparedMusicCue| {
            let section = &cue.sections[0];
            MusicSourceKey::new("a", 1, &cue.id, &section.id, &section.sources[0].layer_id)
        };

        let mut assets = LoadedMusicCueAssets::default();
        let before = cue_of("a.ogg");
        assets.ensure_state_loaded("a", 1, &before, &before.states[0], &server);
        let first = assets.sources[&key(&before)].1.clone();
        assets.ensure_state_loaded("a", 1, &before, &before.states[0], &server);
        assert_eq!(
            assets.sources[&key(&before)].1,
            first,
            "an unchanged cue was requested twice"
        );

        let after = cue_of("a_v2.ogg");
        assets.ensure_state_loaded("a", 1, &after, &after.states[0], &server);
        let (path, handle) = &assets.sources[&key(&after)];
        assert_eq!(
            path, "audio/adaptive/a_v2.ogg",
            "the cache kept the replaced file's path"
        );
        assert_ne!(*handle, first, "the replaced file was never requested");

        for path in ["content://other/audio/a.ogg", "game://audio/a.ogg"] {
            let qualified = cue_of(path);
            assets.ensure_state_loaded("a", 1, &qualified, &qualified.states[0], &server);
            let (loaded, handle) = &assets.sources[&key(&qualified)];
            assert_eq!(loaded, path);
            assert_eq!(handle.path().unwrap().to_string(), path);
        }
    }
}
