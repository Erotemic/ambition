//! Public facade for Ambition-derived platformer games.
//!
//! This crate is the E9 umbrella surface: a downstream game should depend on
//! `ambition_platformer2d` plus its own content crate instead of copying the app shell's wall
//! of lower `ambition_*` dependencies. The supported API is organized by game
//! concepts (`actor`, `character`, `participant`, `session`, `sim`, `world`,
//! `view`) rather than by the crates that currently implement them. Temporary
//! crate mirrors remain for first-party migration, but they are not the SDK
//! boundary and are removed as real consumers stop naming them.

pub mod app;
pub mod demo_shell;
/// Asset install for a game that DRAWS — rides the `ambition_render` capability
/// edge, because its Startup ordering anchor is `ambition_render`'s.
#[cfg(feature = "ambition_render")]
pub mod game_assets;
pub mod prelude;
pub mod autopilot;
pub mod scripted_input;
pub mod session;

/// The platformer experience-provider protocol (authoring identity + the one
/// shared preparation/activation lifecycle). Lives in
/// `ambition_platformer2d_provider`; re-exported here so provider crates keep
/// the `crate::provider::…` path.
pub use ambition_platformer2d_provider as provider;

/// The causal inspector — "why did this actor change on this tick".
///
/// Behind the `causal` feature, default-OFF, for the reason every instrument in
/// this engine is: a game that never opens one must not link one. Turning it on
/// gives a consumer the whole vocabulary — facts, domains, subjects, the log,
/// `explain` — plus the host plugin, WITHOUT importing an internal-shaped
/// module. That last clause is a stated goal of the program this belongs to: an
/// agent should be able to inspect why a body moved without reading engine
/// source or naming an engine-private path.
///
/// ```ignore
/// use crate::causal::{CausalRecording, RecordingPolicy, SubjectKey, domains};
///
/// app.add_plugins(crate::causal::CausalPlugin);
/// app.world_mut()
/// .resource_mut::<CausalRecording>()
/// .set_policy(RecordingPolicy::only([domains::MOVEMENT]));
/// // … drive the sim …
/// println!("{}", log.explain(tick, &SubjectKey::Seat(1)).render());
/// ```
#[cfg(feature = "causal")]
pub mod causal {
    /// The vocabulary: facts, domains, subjects, the log, `explain`.
    pub use ambition_causal::*;
    /// The host half: the plugin, the frame stamp, the publishers' ordering set.
    ///
    /// a game installs `CausalPlugin` and gets recording it can TURN ON. It
    /// never turns itself on: an instrument that is on by default is one
    /// somebody switches off, and then it is not there when it is needed.
    pub use ambition_platformer2d_runtime::causal::{
        assert_no_offthread_loss, record_domains, CausalPlugin, RecordingSet,
    };
}

/// The content compiler — author, validate, prepare, without a Rust rebuild.
///
/// Behind the `content_pack` feature, default-OFF, because a game that ships its
/// content embedded and never validates at runtime should not link a compiler.
/// Turning it on gives a consumer the whole pipeline — draft, schema registry,
/// diagnostics, prepared pack, fingerprint — plus [`engine_schemas`], WITHOUT
/// naming an engine-private path.
///
/// ```ignore
/// let mut registry = crate::content::engine_schemas();
/// registry.register(my_capability::my_schema())?;      // a capability's own
/// let pack = crate::content::compile_dir(path, &registry, &assets)?;
/// ```
#[cfg(feature = "content_pack")]
pub mod content {
    pub use ambition_content_pack::*;

    /// The authored audio manifests a game registers its own tracks and cues in.
    pub use ambition_audio::spec::{MusicRegistry, SfxRegistry};
    /// Which sprite an authored entity draws as.
    pub use ambition_sprite_sheet::game_assets::EntitySprite;

    /// The schemas the ENGINE itself owns, ready for a consumer to add to.
    ///
    /// ⭐ THE LIST LIVES IN `ambition_engine_schemas` — one place, reachable by
    /// BOTH the facade and `ambition_content_cli`, which cannot link this crate.
    /// It used to be spelled here and again in the CLI, with a test holding the
    /// two equal; a rule about a list that should not have been two lists.
    ///
    /// this is the piece a consumer cannot assemble for itself without
    /// knowing which crates own which schemas — which is exactly the internal
    /// topology the SDK is supposed to hide. A capability's own schema is added
    /// on top; nobody has to know that the character catalog lives in
    /// `ambition_characters`.
    /// The content CLI, with a game's own schemas beside the engine's. A game
    /// whose pack names its own schemas has a check binary that calls this
    /// with `PACK.game_schemas()`. Without it, the CLI refuses the pack:
    /// no installed capability owns the game's schema.
    #[cfg(feature = "content_check")]
    pub use ambition_content_cli::main_with as check_pack_main;

    pub fn engine_schemas() -> ambition_content_pack::SchemaRegistry {
        ambition_engine_schemas::engine_schemas()
    }

    /// A game's own pack, with its manifest and sources compiled into the
    /// binary.
    ///
    /// A game states its content as data and reads it back through this: one
    /// `static`, then `PACK.moveset(id)` where a character is registered. The
    /// compile, the cache and the refusal wording are here once, not in each
    /// game.
    ///
    /// Assets are not checked: art may be absent on a fresh clone, and the pack
    /// must still explain what it carries. The content CLI's strict mode is
    /// where art is a gate.
    ///
    /// ```ignore
    /// pub static PACK: EmbeddedPack = EmbeddedPack::new(
    ///     include_str!("../assets/pack.ron"),
    ///     &[("data/movesets/hero.ron", include_str!("../assets/data/movesets/hero.ron"))],
    /// );
    /// definition.with_moveset(PACK.moveset("hero"))
    /// ```
    pub struct EmbeddedPack {
        texts: PackTexts,
        /// The schemas the game itself owns, which the pack compiles against
        /// with the engine's. See [`Self::with_schemas`].
        schemas: fn() -> Vec<SchemaRegistration>,
        compiled: std::sync::OnceLock<PreparedContentPack>,
        /// The generation a live reload published ([`live_reload`]), when
        /// one did. It replaces `compiled` for every later read.
        ///
        /// Each published generation is leaked on purpose: a reader holds a
        /// `&'static` borrow of a lowered row (`smash_rules()`), so an old
        /// generation must stay valid. Only a build that reads its pack off
        /// disk publishes, and only once for each saved edit.
        live: std::sync::RwLock<Option<&'static PreparedContentPack>>,
    }

    /// No schemas of the game's own.
    fn no_schemas() -> Vec<SchemaRegistration> {
        Vec::new()
    }

    /// Where an [`EmbeddedPack`] gets its texts.
    enum PackTexts {
        /// Compiled into the binary: the manifest, and each source `pack.ron`
        /// declares as `(declared path, text)`.
        Embedded {
            manifest_ron: &'static str,
            sources: &'static [(&'static str, &'static str)],
        },
        /// The pack at `root` ([`crate::content_pack!`]): its `pack.ron` and
        /// each source that manifest declares, read off disk when the pack
        /// compiles, unless the build embedded them (`embedded`, the manifest
        /// and each `(declared path, text)`).
        Rooted {
            root: &'static str,
            embedded: Option<(&'static str, &'static [(&'static str, &'static str)])>,
        },
    }

    /// One authored file written with [`crate::content_text!`]: the file in
    /// the source tree, and its bytes when the build embeds them.
    #[derive(Clone, Copy, Debug)]
    pub struct PackText {
        /// The file, as an absolute path in the source tree.
        pub file: &'static str,
        /// `Some` when the crate that states the pack embeds its content (its
        /// `static_content` feature). `None` in a development build: the file is
        /// read when the pack is first compiled, so an edit costs a restart and
        /// no Rust build.
        pub embedded: Option<&'static str>,
    }

    impl PackText {
        /// The file's text: the embedded bytes when the build has them, else
        /// the file as it is on disk now. Write a [`PackText`] for one file
        /// with [`crate::content_text!`].
        ///
        /// ⚠ Read it where it is used, not once into a `static`: in a
        /// development build each read sees the file as last saved.
        pub fn text(&self) -> Result<std::borrow::Cow<'static, str>, String> {
            match self.embedded {
                Some(text) => Ok(std::borrow::Cow::Borrowed(text)),
                None => std::fs::read_to_string(self.file)
                    .map(std::borrow::Cow::Owned)
                    .map_err(|error| {
                        format!(
                            "{} is not readable: {error}; build with the crate's `static_content` \
                             feature to embed it",
                            self.file
                        )
                    }),
            }
        }

        /// Put the file's text on Bevy's `embedded://` source at
        /// `embedded://<crate_name>/<asset_path>`. Use [`crate::content_asset!`],
        /// which gives the path that `embedded_asset!` gives.
        ///
        /// The text is read now, so a development build serves the file as it
        /// was saved when the app started: a shader edit costs a restart and no
        /// Rust build. `embedded_asset!` compiles the bytes in.
        ///
        /// # Panics
        ///
        /// If the app has no `AssetPlugin`, or a development build cannot read
        /// the file: the game would draw without it and not say why.
        pub fn serve_embedded(&self, app: &bevy::prelude::App, crate_name: &str, asset_path: &str) {
            let text = self.text().unwrap_or_else(|error| panic!("{error}"));
            app.world()
                .resource::<bevy::asset::io::embedded::EmbeddedAssetRegistry>()
                .insert_asset(
                    std::path::PathBuf::new(),
                    &std::path::Path::new(crate_name).join(asset_path),
                    text.into_owned().into_bytes(),
                );
        }
    }

    /// The text of `file`, the pack file declared as `declared`, as it is on
    /// disk now; or a diagnostic that names the declared path.
    fn read_declared(file: &std::path::Path, declared: &str) -> Result<String, Diagnostic> {
        std::fs::read_to_string(file).map_err(|error| {
            Diagnostic::error(
                DiagnosticCode::MalformedSource,
                CompileStage::Parse,
                format!("declared source is not readable: {error}"),
            )
            .in_source(declared)
            .fix(format!(
                "create {}, or build with the crate's `static_content` feature to embed it",
                file.display()
            ))
        })
    }

    impl EmbeddedPack {
        /// A pack from its embedded manifest and sources. Nothing compiles
        /// until the pack is first read.
        pub const fn new(
            manifest_ron: &'static str,
            sources: &'static [(&'static str, &'static str)],
        ) -> Self {
            Self {
                texts: PackTexts::Embedded { manifest_ron, sources },
                schemas: no_schemas,
                compiled: std::sync::OnceLock::new(),
                live: std::sync::RwLock::new(None),
            }
        }

        /// The pack at `root`, a directory that holds `pack.ron`: the
        /// manifest is the list of sources. Off disk unless the build embedded
        /// the texts (`embedded`). Write it with [`crate::content_pack!`].
        pub const fn rooted(
            root: &'static str,
            embedded: Option<(&'static str, &'static [(&'static str, &'static str)])>,
        ) -> Self {
            Self {
                texts: PackTexts::Rooted { root, embedded },
                schemas: no_schemas,
                compiled: std::sync::OnceLock::new(),
                live: std::sync::RwLock::new(None),
            }
        }

        /// The pack also compiles against `schemas`: the schemas of rules
        /// that are the game's own policy, which no engine capability can
        /// read (a game's rounds, its tuning). Each route that compiles the
        /// pack adds them to the schemas it has, so the game's file is
        /// checked, lowered and refused as an engine schema's is. A game
        /// schema with an engine schema's id is refused
        /// (`AmbiguousSchemaOwnership`). Write it with
        /// [`crate::content_pack!`]'s `schemas:`.
        pub const fn with_schemas(mut self, schemas: fn() -> Vec<SchemaRegistration>) -> Self {
            self.schemas = schemas;
            self
        }

        /// The schemas the game itself owns: the list a check binary gives
        /// the content CLI ([`check_pack_main`]), so the CLI and the game
        /// read one list.
        pub fn game_schemas(&self) -> fn() -> Vec<SchemaRegistration> {
            self.schemas
        }

        /// Compile the pack now, without the cache: the road a test takes to
        /// read a refusal.
        pub fn compile(&self) -> Result<PreparedContentPack, CompileFailure> {
            let draft = self.compile_draft()?;
            self.compile_against(draft, &engine_schemas())
        }

        /// Compile the pack against the schemas `profile` installs: a pack
        /// that needs a capability the profile omits is refused
        /// (`MissingCapability`). A host that composes the engine group for a
        /// profile admits its own pack with this.
        pub fn compile_for(
            &self,
            profile: &ambition_platformer2d_runtime::profile::EngineProfile,
        ) -> Result<PreparedContentPack, CompileFailure> {
            self.compile_with(&ambition_engine_schemas::engine_schemas_without(&profile.omitted_content_capabilities()))
        }

        /// Compile the pack against `schemas`: the road a composition takes,
        /// with the schemas of what it installed (`PlatformerApp::profile`).
        pub fn compile_with(&self, schemas: &SchemaRegistry) -> Result<PreparedContentPack, CompileFailure> {
            let draft = self.compile_draft()?;
            self.compile_against(draft, schemas)
        }

        fn compile_against(
            &self,
            draft: ContentPackDraft,
            schemas: &SchemaRegistry,
        ) -> Result<PreparedContentPack, CompileFailure> {
            let own = (self.schemas)();
            if own.is_empty() {
                return compile(&draft, schemas, &AssetsUnchecked);
            }
            let mut schemas = schemas.clone();
            let refused: Vec<_> = own
                .into_iter()
                .filter_map(|schema| schemas.register(schema).err())
                .collect();
            if !refused.is_empty() {
                return Err(CompileFailure::new(CompileStage::SchemaResolution, refused));
            }
            compile(&draft, &schemas, &AssetsUnchecked)
        }

        /// The pack's draft, from its texts.
        fn compile_draft(&self) -> Result<ContentPackDraft, CompileFailure> {
            Ok(match &self.texts {
                PackTexts::Embedded { manifest_ron, sources } => ContentPackDraft::from_manifest_ron(
                    manifest_ron,
                    sources
                        .iter()
                        .map(|(path, text)| ((*path).to_string(), (*text).to_string())),
                )?,
                PackTexts::Rooted { embedded: Some((manifest_ron, sources)), .. } => {
                    ContentPackDraft::from_manifest_ron(
                        manifest_ron,
                        sources
                            .iter()
                            .map(|(path, text)| ((*path).to_string(), (*text).to_string())),
                    )?
                }
                PackTexts::Rooted { root, embedded: None } => {
                    let root = std::path::Path::new(root);
                    let file = |declared: &str| read_declared(&root.join(declared), declared);
                    let manifest = file("pack.ron")
                        .map_err(|unreadable| CompileFailure::new(CompileStage::Parse, vec![unreadable]))?;
                    let mut read = Vec::new();
                    let mut unreadable = Vec::new();
                    for declared in ambition_content_pack::embed::declared_paths(&manifest) {
                        match file(&declared) {
                            Ok(text) => read.push((declared, text)),
                            Err(diagnostic) => unreadable.push(diagnostic),
                        }
                    }
                    // Every unreadable file in one refusal, not the first one.
                    if !unreadable.is_empty() {
                        return Err(CompileFailure::new(CompileStage::Parse, unreadable));
                    }
                    ContentPackDraft::from_manifest_ron(&manifest, read)?
                }
            })
        }

        /// The compiled pack: compiled once, or the generation a live reload
        /// published last ([`live_reload`]).
        ///
        /// # Panics
        ///
        /// When the pack does not compile. Content a game ships and cannot
        /// load is a build fault, and the refusal says which file and why.
        pub fn prepared(&self) -> &PreparedContentPack {
            if let Some(live) = *self.live.read().unwrap_or_else(std::sync::PoisonError::into_inner) {
                return live;
            }
            self.compiled.get_or_init(|| {
                self.compile()
                    .unwrap_or_else(|failure| panic!("an embedded content pack does not compile:\n{failure}"))
            })
        }

        /// The directory the pack is read from when the build reads it off
        /// disk, or `None` when the build embedded its texts. Only such a pack
        /// can change while the game runs.
        pub fn root_on_disk(&self) -> Option<&'static str> {
            match self.texts {
                PackTexts::Rooted { root, embedded: None } => Some(root),
                _ => None,
            }
        }

        /// Make `generation` the pack every later [`Self::prepared`] reads.
        /// Only [`live_reload`] calls it, between two sessions.
        fn publish(&self, generation: PreparedContentPack) {
            let leaked: &'static PreparedContentPack = Box::leak(Box::new(generation));
            *self.live.write().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(leaked);
        }

        /// The move table this pack authors for `character`.
        ///
        /// # Panics
        ///
        /// When the pack authors none: a fighter with no moves is a
        /// composition fault, not a default. The message names the characters
        /// the pack does author.
        pub fn moveset(&self, character: &str) -> crate::entity_catalog::MovesetContract {
            let table =
                ambition_characters::moveset_content_schema::lowered_movesets(self.prepared());
            table
                .and_then(|table| table.get(character))
                .cloned()
                .unwrap_or_else(|| {
                    let authored: Vec<&str> = table
                        .map(|table| table.keys().map(String::as_str).collect())
                        .unwrap_or_default();
                    panic!(
                        "the embedded pack `{}` authors no move table for `{character}`; it \
                         authors {authored:?}. Declare the file in the pack's manifest and \
                         embed it with the other sources",
                        self.prepared().id.0
                    )
                })
        }

        /// The music and SFX this pack states, as `provider`'s audio catalog.
        ///
        /// A pack that states no `music_registry` or no `sfx_registry` gives
        /// the fragment none of that kind. Register it with
        /// `register_audio_catalog_fragment`, after any composition choice
        /// such as `with_resident_sfx_bank`.
        ///
        /// # Panics
        ///
        /// When the registries the pack states do not validate as a catalog.
        pub fn audio(&self, provider: &str) -> crate::audio::catalog::AudioCatalogFragment {
            let pack = self.prepared();
            crate::audio::catalog::AudioCatalogFragment::new(
                provider,
                crate::audio::content_schema::lowered_music_registry(pack).cloned(),
                crate::audio::content_schema::lowered_sfx_registry(pack).cloned(),
            )
            .unwrap_or_else(|error| panic!("the embedded pack `{}`: {error}", pack.id.0))
        }

        /// The walk-into pickup art this pack states (`world_item_art`), as
        /// entries to register with `register_world_item_art`. A pack that
        /// states none gives none.
        pub fn world_item_art(&self) -> Vec<crate::platformer::world_item_art::WorldItemArtEntry> {
            world_item_art_entries(self.prepared())
        }

        /// The art-to-world scale at which [`Self::cast`] builds `character`'s
        /// body, for anything else that must agree with it (a level that sizes
        /// a gap to a body, a test that measures one).
        ///
        /// # Panics
        ///
        /// When the pack states no cast, or its row for `character` states no
        /// `posed_body`: a caller that asks has assumed one.
        pub fn posed_body_world_per_pixel(&self, character: &str) -> f32 {
            let pack = self.prepared();
            let catalog = ambition_characters::actor::character_catalog::lowered_catalog(pack)
                .unwrap_or_else(|| panic!("the embedded pack `{}` states no cast", pack.id.0));
            ambition_character_sprites::posed_body_world_per_pixel(catalog, character)
                .unwrap_or_else(|| {
                    panic!(
                        "the embedded pack `{}` states no posed_body for `{character}`",
                        pack.id.0
                    )
                })
        }

        /// The cast this pack states: every row of its `character_catalog`
        /// source, for `provider`.
        ///
        /// Each character is built from what the pack says about it and from
        /// nothing else: its catalog row (name, sheet, ability grants, feel,
        /// health) and every character facet the pack authors for it (its
        /// move table, its platform-fighter weight), each folded by the
        /// capability that owns it (`pack_facets`). A game registers
        /// its cast with one call and writes no per-character Rust.
        ///
        /// # Panics
        ///
        /// When the pack has no `character_catalog` source, or `default_character`
        /// names no row in it.
        /// The name `character` is shown by, as this pack's cast states it
        /// (`display_name`). A game names a character's HUD label, sheet or
        /// placement by it; reading it here keeps the cast its one author.
        ///
        /// # Panics
        ///
        /// When the pack's cast has no row `character`. The message names the
        /// rows it has.
        pub fn display_name(&self, character: &str) -> String {
            use ambition_characters::actor::character_catalog::lowered_catalog;
            let pack = self.prepared();
            let catalog = lowered_catalog(pack)
                .unwrap_or_else(|| panic!("the embedded pack `{}` states no cast", pack.id.0));
            catalog
                .characters
                .get(character)
                .map(|row| row.display_name.clone())
                .unwrap_or_else(|| {
                    panic!(
                        "the embedded pack `{}` casts no `{character}`; it casts {:?}",
                        pack.id.0,
                        catalog.characters.keys().collect::<Vec<_>>()
                    )
                })
        }

        pub fn cast(
            &'static self,
            provider: &str,
            default_character: Option<&str>,
        ) -> PackCast {
            use ambition_characters::actor::character_catalog::lowered_catalog;
            let pack = self.prepared();
            assert!(
                lowered_catalog(pack).is_some(),
                "the embedded pack `{}` states no cast: declare a `character_catalog` \
                 source in its manifest",
                pack.id.0
            );
            let fragment = cast_fragment(pack, provider, default_character, None)
                .unwrap_or_else(|error| panic!("the embedded pack `{}`: {error}", pack.id.0));
            PackCast {
                pack: self,
                provider: provider.to_string(),
                default_character: default_character.map(str::to_string),
                actor_default_abilities: None,
                fragment,
                carried: Vec::new(),
            }
        }
    }

    /// The catalog fragment `pack`'s cast registers as `provider`: the one
    /// road for a registration and a reload.
    fn cast_fragment(
        pack: &PreparedContentPack,
        provider: &str,
        default_character: Option<&str>,
        actor_default_abilities: Option<ambition_platformer2d_core::AbilitySet>,
    ) -> Result<ambition_characters::actor::character_catalog::CharacterCatalogFragment, String> {
        use ambition_characters::actor::character_catalog::{lowered_catalog, CharacterCatalogFragment};
        let catalog = lowered_catalog(pack).ok_or_else(|| "the pack states no cast".to_string())?;
        let fragment = CharacterCatalogFragment::from_prepared(
            format!("{}:character_catalog", pack.id.0),
            provider,
            default_character,
            catalog.clone(),
        )
        .map_err(|error| error.to_string())?;
        Ok(match actor_default_abilities {
            Some(abilities) => fragment.with_actor_default_abilities(abilities),
            None => fragment,
        })
    }

    pub mod live_reload;
    pub mod sprite_reload;

    /// The `world_item_art` rows of `pack`, as the entries the render layer
    /// resolves: the one lowering for a registration and a reload.
    pub fn world_item_art_entries(pack: &PreparedContentPack) -> Vec<crate::platformer::world_item_art::WorldItemArtEntry> {
        ambition_items::content_schema::world_item_art::lowered_world_item_art(pack)
            .map(|rows| {
                rows.iter()
                    .map(|(id, art)| {
                        crate::platformer::world_item_art::WorldItemArtEntry::new(
                            id.clone(),
                            art.path.clone(),
                            bevy::math::Vec2::new(art.size.0, art.size.1),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A pack's cast, ready to register. See [`EmbeddedPack::cast`].
    pub struct PackCast {
        pack: &'static EmbeddedPack,
        provider: String,
        /// What the fragment was built from, so a reload builds it again.
        default_character: Option<String>,
        actor_default_abilities: Option<ambition_platformer2d_core::AbilitySet>,
        fragment: ambition_characters::actor::character_catalog::CharacterCatalogFragment,
        /// What each named character's bodies carry. See [`Self::carrying`].
        carried: Vec<(String, ambition_platformer2d_core::CarriedComponent)>,
    }

    impl PackCast {
        /// What a character of this provider that states no verbs can do as an
        /// actor. A provider fact, not a character fact.
        pub fn with_actor_default_abilities(
            mut self,
            abilities: ambition_platformer2d_core::AbilitySet,
        ) -> Self {
            self.fragment = self.fragment.with_actor_default_abilities(abilities);
            self.actor_default_abilities = Some(abilities);
            self
        }

        /// Every body of `character` carries `C` from the batch that builds
        /// it, at `C::default()`: the game's own state of that creature, which
        /// a catalog row cannot name.
        ///
        /// # Panics
        ///
        /// In [`Self::register`], when the pack states no row `character`.
        pub fn carrying<C: bevy::prelude::Component + Default>(mut self, character: &str) -> Self {
            self.carried.push((
                character.to_string(),
                ambition_platformer2d_core::CarriedComponent::of::<C>(),
            ));
            self
        }

        /// Register the catalog and every character in it.
        pub fn register(self, app: &mut bevy::prelude::App) {
            use ambition_characters::actor::character_catalog::CharacterCatalogAppExt;
            use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinitionAppExt;

            let pack = self.pack.prepared();
            let catalog = self.fragment.catalog();
            // A carried component for a character the pack does not state
            // would be carried by nobody, and the game would lose its state
            // without a word.
            for (character, carried) in &self.carried {
                assert!(
                    catalog.characters.contains_key(character),
                    "the embedded pack `{}` states no character `{character}` to carry `{}`",
                    pack.id.0,
                    carried.type_name(),
                );
            }
            let rigs_admitted = ambition_characters::actor::BodyRigAdmission::of(app.world()).admit;
            let definitions = cast_definitions(pack, catalog, &self.provider, &self.carried, rigs_admitted);
            app.register_character_catalog_fragment(self.fragment);
            for definition in definitions {
                app.register_character(definition);
            }
            // A build that reads the pack off disk plays an edit to the
            // cast, a move table or a fighter facet from the next session.
            let reload = live_reload::EmbeddedPackReloadPlugin::cast(live_reload::CastReload {
                pack: self.pack,
                provider: self.provider,
                default_character: self.default_character,
                actor_default_abilities: self.actor_default_abilities,
                carried: self.carried,
            });
            // The definitions read their body scale, hurtboxes and rig from
            // the sheets, so a sheet edit states the cast again too. Each
            // restater stages the whole cast, so one is enough.
            if let Some(restater) = reload.restated.first() {
                let (restate, embedded) = (restater.restate.clone(), self.pack);
                sprite_reload::restate_with_the_sheets(app, format!("the cast of `{}`", pack.id.0), move |world| {
                    restate(world, embedded.prepared())
                });
            }
            app.add_plugins(reload);
        }
    }

    /// Each character `catalog` states, defined from what `pack` says about
    /// it and from nothing else: its catalog row (name, sheet, ability
    /// grants, feel, health) and every character facet the pack authors for
    /// it (its move table, its platform-fighter weight), each folded by the
    /// capability that owns it. The one definition road for a registration
    /// and a reload.
    fn cast_definitions(
        pack: &PreparedContentPack,
        catalog: &ambition_characters::actor::character_catalog::CharacterCatalogData,
        provider: &str,
        carried: &[(String, ambition_platformer2d_core::CarriedComponent)],
        rigs_admitted: bool,
    ) -> Vec<ambition_characters::actor::definition::CharacterDefinition> {
        use ambition_characters::actor::definition::CharacterDefinition;

        catalog
            .characters
            .iter()
            .map(|(id, row)| {
                // The sheet, the grants, the feel, the health, the gait, the
                // contact damage and the policy come from the catalog row at
                // preparation, so the definition names only what the row
                // cannot say.
                let mut definition = CharacterDefinition::new(id.clone(), row.display_name.clone(), provider.to_string());
                for (_, carried) in carried.iter().filter(|(character, _)| character == id) {
                    definition = definition.carrying(*carried);
                }
                // The scale is asked of the baked sheet once, here, and the
                // body is built with it (`BodySource::SpriteAuthored`).
                if let Some(world_per_pixel) = ambition_character_sprites::posed_body_world_per_pixel(catalog, id) {
                    definition = definition.with_sprite_authored_body(world_per_pixel);
                }
                // Built on that body, so it too is asked of the sheet here.
                if let Some(doc) = ambition_character_sprites::posed_body_inset_hurtboxes(catalog, id) {
                    definition = definition.with_hurtboxes(doc);
                }
                // Only where the composition admits rigs: see
                // `BodyRigAdmission` for why the answer is read here.
                if let Some(rig) = rigs_admitted.then(|| ambition_character_sprites::published_body_rig(catalog, id)).flatten() {
                    definition = definition.with_body_rig(rig);
                }
                // The move table and every other facet the pack authors for
                // this character, each folded by the capability that owns it.
                ambition_characters::pack_facets::fold_character_facets(pack, definition)
            })
            .collect()
    }
}

pub use ambition_asset_manager as asset_manager;
pub use ambition_audio as audio;
/// The platformer's audio runtime and music direction: what a game with
/// audio installs (`Platformer2dAudioPlugin`).
#[cfg(feature = "audio")]
pub use ambition_platformer2d_audio as platformer_audio;
pub use ambition_boss_encounter as boss_encounter;
/// Derivations from a character sheet — the animation-row pickers, the
/// sheet-authored body geometry, the manifest attack hitbox.
pub use ambition_character_sprites as character_sprites;
pub use ambition_characters as characters;
pub use ambition_combat as combat;
/// The gameplay TRACE: the buffer a headless run records into, its dump road
/// and the reasons a dump is taken. Named from its owning crate.
pub use ambition_gameplay_trace as gameplay_trace;
/// The gameplay trace RECORDER: the systems that sample each frame into that
/// buffer, the F8 hotkey and the headless driver's `record_simulation_frame`.
pub use ambition_platformer2d_trace as trace_recorder;
// The conversation authority — and, under `ui`, the Yarn host glue that used to
// sit in the monolith as `actors:dialog`.
/// The body seed every construction path spawns from; a game that builds an
/// NPC by hand names it from here.
/// The WIELDED ability kit: ranged, thrown grenades, the traversal verbs a held
/// item fires, and their shared cooldown (D33, 2026-09-03). Not under [`actors`]
/// for the same reason as its siblings. ⛔ `actors::abilities` still exists and
/// is a DIFFERENT module — possession, teleport, trapdoor and flyline, which are
/// control authority sharing a directory name. Both paths resolve, so read that
/// module's header before assuming which one you want.
pub use ambition_abilities as abilities;
pub use ambition_body_seed as body_seed;
pub use ambition_conversation as conversation;
pub use ambition_cutscene as cutscene;
/// THE MOUNT PAIR — two linked bodies where one carries the other. Carved out
/// of `actors` (D33); a game that wires mounts names this domain, exactly as it
/// names `boss_encounter`.
/// Victim-side damage resolution — shield blocks, knockback, hitstun, the
/// safe-respawn and death-respawn roads. Carved out of the actor monolith
/// 2026-08-26, exposed as its own domain beside `mount` and `boss_encounter`.
pub use ambition_damage as damage;
pub use ambition_dev_tools as dev_tools;
pub use ambition_dialog as dialog;
pub use ambition_encounter as encounter;
/// The room-feature side of encounters: LDtk loading, ECS spawning, banners,
/// save/quest projection. Carved out of the actor kernel 2026-09-03; games
/// reach it here rather than through `actors`, which IS the monolith.
pub use ambition_encounter_features as encounter_features;
pub use ambition_entity_catalog as entity_catalog;
pub use ambition_resource_spec as resource_spec;
pub use ambition_game_shell as game_shell;
/// The PRESSED collectible: `GroundItem`, `ItemCustody`, the held specs and
/// the pickup / use / throw / physics / residency chain it owns (D33,
/// 2026-09-03). Not under [`actors`] any more, for the same reason as
/// [`world_items`]: `actors` IS the actor monolith, and the kernel keeps only
/// checkpoint policy over these components.
pub use ambition_held_items as held_items;
pub use ambition_input as input;
pub use ambition_interaction as interaction;
#[cfg(feature = "ambition_inventory_ui")]
pub use ambition_inventory_ui as inventory_ui;
pub use ambition_items as items;
pub use ambition_load as load;
pub use ambition_load_presentation as load_presentation;
/// The versus match, prepared: roster, rules, plan and receipt. The kernel
/// activates it; a game stages and reads it from here.
pub use ambition_match as versus_match;
#[cfg(feature = "ambition_menu")]
pub use ambition_menu as menu;
pub use ambition_mount as mount;
pub use ambition_shrine as shrine;
pub use ambition_persistence as persistence;
pub use ambition_platformer2d_actor_monolith as actors;
/// Room publication: prepare a hidden candidate room, check its receipt, and
/// publish or refuse it as one operation.
pub use ambition_platformer2d_publication as publication;
pub use ambition_platformer2d_core as engine_core;
pub use ambition_platformer2d_host as host;
#[cfg(feature = "ambition_platformer2d_ldtk")]
pub use ambition_platformer2d_ldtk as ldtk_map;
pub use ambition_platformer2d_runtime as runtime;

/// Procedural extensions: a game's own algorithms with their own rollback
/// state, run by the engine's extension host (fast-iteration I4) and, with
/// the `wasm_modules` feature, loaded from `.wasm` files so a module edit
/// does not rebuild the engine (I6/I7).
///
/// [`sdk`] is what a module is written against. The rest is what a game or a
/// tool composes with: declaring modules, naming module files to load, and
/// reading what the host admitted.
pub mod extension {
    pub use ambition_extension_host::{
        AdmittedExtensions, BodyRecords, DeclaredModule, SessionRecords, EntryRunner, ExtensionAppExt, ExtensionFaults,
        ExtensionHostPlugin, ExtensionSet, ModuleBackend, ModuleCode,
    };
    pub use ambition_extension_host::{inspect, reload};
    pub use ambition_extension_sdk as sdk;
    #[cfg(feature = "wasm_modules")]
    pub use ambition_extension_wasm::{build_module_crate, LoadError, WasmModules};
    pub use ambition_platformer2d_runtime::extension_composition::{
        ExtensionGeneration, ExtensionModuleFiles, EXTENSION_MODULES_SECTION, EXTENSION_MODULES_VAR,
    };
}
pub use ambition_platformer2d_shared_tangle as platformer;
#[cfg(feature = "ambition_portal2d")]
pub use ambition_portal2d as portal;
#[cfg(feature = "ambition_portal2d_presentation")]
pub use ambition_portal2d_presentation as portal_presentation;
pub use ambition_projectiles as projectiles;
#[cfg(feature = "relativity")]
pub use ambition_relativity as relativity;
#[cfg(feature = "relativity")]
pub use ambition_relativity2d as relativity2d;
#[cfg(feature = "ambition_render")]
pub use ambition_render as render;
#[cfg(feature = "ambition_settings_menu")]
pub use ambition_settings_menu as settings_menu;
pub use ambition_sfx as sfx;
pub use ambition_sim_view as sim_view;
pub use ambition_sprite_sheet as sprite_sheet;
pub use ambition_time as time;
#[cfg(feature = "ambition_touch_input")]
pub use ambition_touch_input as touch_input;
pub use ambition_ui_nav as ui_nav;
pub use ambition_vfx as vfx;
/// The physical life of a touched collectible: `WorldItem`, its motion, and the
/// touch-collect pass.
///
/// ⭐ IT IS NOT UNDER [`actors`] ANY MORE, and the path change is the point.
/// `actors` IS the actor monolith, so a game importing
/// `actors::items::WorldItem` was naming the kernel for something the kernel no
/// longer owns (D33, 2026-09-02). `actors::items::pickup` still holds the
/// PRESSED pickup — a held weapon taken with `Attack` — which is a different
/// domain that stayed.
pub use ambition_world_items as world_items;
/// Participants: local seats and the device/channel topology that feeds them.
///
/// This is a curated control-authority surface. A consumer should not need to
/// know that device discovery currently lives in `ambition_input`.
pub mod participant {
    pub use ambition_input::{
        InstalledActions, LocalChannelPlan, LocalDeviceOrder, LocalInputSource, LocalSeatTopology,
        ParticipantId, SemanticActionId, GAMEPLAY_CONTEXT,
    };
}

/// Items as simulation state, when the item capability is installed.
pub mod item {
    pub use ambition_held_items::{
        GroundItem, ItemCustody, ItemStruckBody, ItemWorldPos, SettledItem,
    };
    pub use ambition_items::{shop, Inventory, Item, ItemGrantRequested, OwnedItems};
}

/// User-facing gameplay settings, when persistence/settings support is installed.
pub mod settings {
    pub use ambition_persistence::settings::UserSettings;
}

/// Bodies: what a game queries, moves and transits.
///
/// A curated domain module, second of the set ADR 0031's decision 1 lists. It
/// exists because the two sentinel consumers had to reach into
/// `crate::engine_core` — an IMPLEMENTATION crate the facade mirrors — for
/// `transit_body`, `TransitVelocity` and `BodyClusterQueryData`, and into
/// `crate::platformer` for the marker and kinematics types you need to ask
/// "where is the player".
///
/// Closed list, like [`world`]. Adding a type to a mirrored crate does not
/// silently become public API.
pub mod actor {
    /// WHICH gameplay session owns the world right now. A resource that names an
    /// earlier scope is a leftover, and reading one is how a retired match's
    /// verdict gets applied to the match that replaced it.
    pub use ambition_platformer2d_shared_tangle::lifecycle::ActiveSessionScope;
    /// The lifecycle marker every feature-spawned sim entity carries.
    pub use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
    /// Who the body is.
    pub use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
    /// What a body holds: its declared resources, read by name.
    pub use ambition_platformer2d_core::resources::{ActorResources, ResourceLevel};
    /// The main game's Mana declaration — the name a reader asks a bank for.
    pub use ambition_entity_catalog::mana;
    /// The query filter for "the body this player already has", which a game
    /// needs to re-dress it — see `WornCharacter` and the seating contract.
    pub use ambition_platformer2d_shared_tangle::markers::PrimaryPlayerOnly;

    /// Which seat of the match a body is.
    ///
    /// The answer to "who is player two?", and the only correct one — the
    /// engine's own docs say every other way to identify a fighter is a guess:
    /// a brain slot misses the CPU seat, the worn character id collides in a
    /// mirror match, and entity order is not an order.
    ///
    /// `RollbackSession::participants()` reports the count a composition DECLARED; the run
    /// declared 1, 2 and 4 through the public builder and got one body every time, and no
    /// consumer could see the difference. Seats come from DEVICES — a composition with two
    /// gamepads seats two — so a consumer needs this to check that the match it asked for is
    /// the match it got.
    ///
    /// the input half is CLOSED :
    /// `crate::sim::drive_slot_frame` drives any seat, beside
    /// `drive_control_frame` naming the primary. The seam had existed in
    /// `ambition_platformer2d_runtime` since and was simply never re-exported — so
    /// the finding described the FACADE, not the engine, which is the more
    /// embarrassing of the two and the harder one to notice.
    pub use ambition_match::MatchSeat;

    /// Declaring a MATCH: who is in it, who drives them, and what it costs to
    /// lose.
    ///
    /// That is the leak `minimal-game-names-only-the-public-sdk` exists to catch, and the only
    /// reason it went uncaught is that no consumer had ever declared a match: the shipped versus
    /// stage lives in `ambition_app`, which is allowed to name anything.
    ///
    /// A second consumer is the only instrument that finds this class, which is
    /// the entire argument for keeping one.
    pub use ambition_match::{
        ControlAuthority, ControllerBinding, MatchParticipant, MatchParticipantRoster,
        MatchPreparationProblems, PreparedMatch, RosterProblem, RosterSeating,
    };

    /// What a human seat is playing ON, and the map from those sources to
    /// the dense channels a rollback session opens.
    ///
    /// Re-exported beside the roster because a composition that names
    /// `ControllerBinding::Human` cannot say who is holding what without it.
    pub use ambition_input::{LocalChannelPlan, LocalInputSource};

    /// The stocks economy: the count on a body, the fact that it is out, and
    /// the two messages a ruleset acts on.
    ///
    /// The engine owns the COUNT — spend one, decide whether it was the last,
    /// clear the meter of a fighter coming back. It refuses to place the body or
    /// to say what a match ending means, because those need a stage and a
    /// scoreboard. These are the seam between the two halves.
    pub use ambition_combat::components::FighterStocks;
    pub use ambition_combat::stocks::{
        BodyKnockedOut, FighterEliminated, FighterRespawnDue, FighterRespawnsDue,
        FighterStockSpent, MatchVerdict, PendingRespawn, RespawnGrace, RespawnGraceTicked,
        StocksMatchDecided,
    };

    /// How a body came to exist — ADR 0030's construction provenance.
    pub use ambition_platformer2d_actor_monolith::construction::ActorConstructionRegistry;

    pub use ambition_characters::actor::{BodyCombat, BodyHealth, Health};
    /// WHICH MOVE THE BODY IS PLAYING, and where in it. Read by any driver that
    /// has to know whether the press it made became the move it asked for.
    pub use ambition_combat::moveset::MovePlayback;
    /// THE MOVES THE BODY WEARS, with the verbs of the game it is in. A seat
    /// on a platform-fighter stage wears its fighter's verbs; the prepared
    /// character has those of its home game. A driver that asks which move a
    /// press should become reads the body.
    pub use ambition_combat::moveset::ActorMoveset;
    /// WHAT THE BODY IS RIDING, when it is riding something. An observer that
    /// cannot see this reports a mounted body's own locomotion as if it were
    /// driving.
    pub use ambition_mount::RidingOn;
    /// Canonical fallback body size for a body not yet materialized.
    pub use ambition_platformer2d_core::default_player_body_size as default_body_size;
    /// Where the body is, and how it moves.
    pub use ambition_platformer2d_core::movement::{transit_body, TransitVelocity};
    pub use ambition_platformer2d_core::BodyClusterQueryData;
    /// Diagnostic counters beside a body (time since restart, restart count).
    pub use ambition_platformer2d_core::BodyLifeStats;
    /// ⭐ WHETHER THE BODY IS STANDING ON SOMETHING — the companion to
    /// [`BodyFlightState`], and a question every observer of a body asks.
    ///
    /// ⛔ ADDED 2026-08-29 BECAUSE A CUSTOMER HAD TO SPELL A CRATE PATH FOR IT.
    /// `ambition_sim_harness` moved its move-exercise machinery in and the
    /// `sim-harness-names-only-the-public-sdk` contract went red on
    /// `engine_core`, `combat` and `mount` — and that contract's own reason says
    /// what to do about it: *"if it needs crate-shaped facade paths, those are
    /// SDK gaps."* These three rows are those gaps, closed rather than waived.
    pub use ambition_platformer2d_core::BodyGroundState;
    pub use ambition_platformer2d_core::{BodyFlightState, BodyMode, BodyMotionFacts};
    pub use ambition_platformer2d_core::BodyKinematics;
    /// The body-local safe-position state used by reset/hazard observers.
    ///
    /// The implementation type still carries its historical player-centric name;
    /// the SDK does not.
    pub use ambition_platformer2d_shared_tangle::safe_position::PlayerSafetyState as BodySafetyState;
    /// WHOSE SHOT THIS IS. The other half of the same question `RidingOn`
    /// answers for a mount: a projectile is somebody's, and an observer that
    /// cannot follow the link credits a stage hazard to a fighter.
    ///
    /// ⛔ GATED, because `ambition_projectiles` is OPTIONAL. An ungated `pub
    /// use` compiles inside the workspace, where the default feature set is on,
    /// and fails for a consumer who took a narrower one — the exact shape the
    /// external-consumer fixture exists to catch.
    pub use ambition_projectiles::ProjectileOwner;
    /// ⭐⭐ WHAT THE GAME INTENDS TO DRAW FOR THIS BODY, and it needs no renderer.
    ///
    /// The semantic pose row, the requested clip, and the presentation facts
    /// derived from them — a pure function of sim state, rebuilt every tick and
    /// declared rollback-DERIVED, so a `NoWindow` composition publishes it
    /// exactly as a windowed one does.
    ///
    /// ⛔ IT IS PUBLISHED ONLY FOR A BODY THAT ASKS. See
    /// `lifecycle::PosedBody` — the marker every granted character body
    /// receives. Before 2026-08-29 the gate was `PlayerVisual`, which only the
    /// exploration player's avatar ever gets, so no match fighter had one and a
    /// headless tool had to reconstruct the frame cursor from sprite sheets.
    pub use ambition_sim_view::{BodyPoseView, ClipRequest};
    /// The 56 semantic body states a pose row can be.
    pub use ambition_sprite_sheet::character::CharacterAnim;

    /// Boss spawn policy belongs to actor construction at the SDK boundary even
    /// though its implementation is split between the catalog and encounter crates.
    pub use ambition_boss_encounter::BossOverrides;
    pub use ambition_entity_catalog::placements::BossBrain;

    /// What a game spawns and configures.
    pub use ambition_combat::actor_tuning::ActorConfig;
    /// The policy a spawned body's autonomous driver plays it by.
    pub use ambition_combat::actor_tuning::ActorPolicy;
    /// Who a spawned body is: its stable id and display name.
    pub use ambition_combat::components::ActorIdentity;
    pub use ambition_characters::actor::ActorFaction;
    pub use ambition_platformer2d_actor_spawn::{SpawnActorKind, SpawnActorRequest};
    /// ⛔ NAMED FROM `_core`, NOT THROUGH THE ACTOR CRATE. `MotionModel` is the
    /// movement kernel's own type; the monolith re-exported it twice
    /// (`features` → `features::ecs::actors::motion` → `_core`), so 125 call
    /// sites read as coupling to the actor crate for a type it does not own —
    /// and `damage_apply` spelled the SAME type both ways in one file. A curated
    /// SDK alias is fine; a domain crate laundering a foundation type is not.
    pub use ambition_platformer2d_core::movement::MotionModel;

    /// What a room stages when it opens.
    pub use ambition_platformer2d_actor_monolith::features::RoomContentStagingRegistry;

    /// What a body can reach for.
    ///
    /// A capability MASK, not a promise of combat: an actor with an empty
    /// ability set is an ordinary actor, and a game should be able to check
    /// that its cast is not secretly armed.
    pub use ambition_platformer2d_core::abilities::AbilitySet;

    /// The body's INTRINSIC kit as authored, and the EFFECTIVE set the movement
    /// kernel reads. `effective = base ∩ session_mask`, so a game checking what
    /// its cast can actually do wants the base.
    pub use ambition_platformer2d_core::body_clusters::{AbilityBase, BodyAbilities};
}

/// Characters: the cast, its art, and what it can do.
///
/// The fourth curated domain module, and the one that absorbs the largest remaining spread.
///
/// Closed list.
pub mod character {
    /// The cast, as authored content.
    pub use ambition_characters::actor::character_catalog::{
        parse_catalog, CharacterCatalog, CharacterCatalogAppExt, CharacterCatalogFragment,
    };
    /// What providers have authored, for a game that wants to inspect its own
    /// content before a session exists.
    pub use ambition_platformer2d_provider::PlatformerAuthoredCatalogRegistry;

    /// Declaring a character in Rust, for the cases ADR 0032 keeps in Rust:
    /// tests, procedural generation, unrepresentable schemas, and a cast whose
    /// behavior is supplied by host code as a deliberate authoring choice.
    pub use ambition_characters::actor::definition::{CharacterDefinition, Lineage, Vitals};
    pub use ambition_characters::actor::{CharacterLocomotion, ContactDamage, WornCharacter};
    /// Controller policy and locomotion vocabulary used by Rust-authored characters.
    ///
    /// These are CHARACTER-AUTHORING values, not runtime implementation types: a
    /// third-party game must be able to state how its own autonomous character
    /// moves and decides without importing the engine's internal crates.
    pub use ambition_characters::brain::{BrainProfile, CharacterBrainTemplate, MoveStyleSpec};
    pub use ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinitionAppExt;

    /// The pieces a match needs to author a fighter's kit — see
    /// `MatchParticipant::action_set`.
    pub use ambition_characters::brain::action_set::{
        MeleeActionSpec, RangedActionSpec, SwipeSpec,
    };
    /// The registered prop a `MoveSpec::equips` names, resolved the way the
    /// brandish system resolves it. Published because a MOVE's shot is the
    /// drawn weapon's shot, so anything reasoning about what a move can do has
    /// to be able to follow that edge.
    pub use ambition_characters::brain::held_item_by_id;
    /// What a character can do, and how it decides.
    pub use ambition_characters::brain::ActionSet;
    pub use ambition_entity_catalog::placements::CharacterBrain;

    /// WHICH character a placement is, and it is authoring vocabulary rather than a mirror
    /// of our crate list.
    pub use ambition_entity_catalog::CharacterId;

    pub use ambition_characters::actor::character_catalog::CharacterPortraitRef;
    /// What a provider REGISTERED, so a screen can ask what a character
    /// declared rather than only what a catalog derived.
    pub use ambition_characters::prepared::PreparedCharacterRegistry;
    pub use ambition_platformer2d_actor_monolith::character_runtime::registered_portrait_target;
    /// What a character looks like, and whether its art has arrived.
    pub use ambition_platformer2d_actor_monolith::character_runtime::CharacterLoadStates;
    /// The sheet tier every character is realized at: the user's setting, and
    /// nothing else — no room, view or distance may lower it (Jon, 2026-09-02).
    pub use ambition_platformer2d_actor_monolith::character_sprites::character_sprite_tier;
    pub use ambition_platformer2d_actor_monolith::character_sprites::portrait_for_declared_character;
    pub use ambition_platformer2d_actor_monolith::character_sprites::sheet_for_prepared_character;
    pub use ambition_sprite_sheet::character::sheets::AuthoredSheets;
    pub use ambition_sprite_sheet::character::CharacterSheetState;
    pub use ambition_sprite_sheet::AuthoredSheetAppExt;
    pub use ambition_sprite_sheet::{PortraitSheetRegistry, PortraitSheetRegistryPlugin};
}

/// The simulation schedule a game joins its own systems to.
pub mod sim {
    /// The dev-only physics overrides a sandbox host may install.
    pub use ambition_platformer2d_shared_tangle::physics::PhysicsSandboxSettings;
    pub use ambition_platformer2d_shared_tangle::schedule::{
        BossSteerSlot, FeatureWorldOverlayContributions, GameMode,
        Platformer2dSimulationPhase, PresentationSetupSet, SimSchedule, SimScheduleExt,
        SimulationSetupSet, WorldItemSet,
    };

    /// How device/screen/body axes are interpreted by scripted or participant input.
    pub use ambition_platformer2d_core::InputFrameMode;
    /// The deterministic per-tick input artifact used by replay and harnesses.
    pub use ambition_platformer2d_core::InputStream;
    /// WHICH TICK IT IS. The absolute fixed-step counter every recorded artifact
    /// names itself by — a screenshot, a take, a trace row.
    ///
    /// ⛔ ADDED 2026-08-29 for the same reason as the `actor` rows above: a
    /// supported customer had to spell `runtime::SimTick`, and a crate-shaped
    /// path in the SDK's own customers is an SDK gap rather than a waiver.
    pub use ambition_platformer2d_runtime::SimTick;
    /// Simulation time. Not wall time — a game reads the clock the sim advances.
    pub use ambition_time::WorldTime;

    /// One frame of input, and the one seam that delivers it.
    pub use ambition_input::AttackStrengthHint;
    pub use ambition_platformer2d_core::ControlFrame;
    #[cfg(feature = "rollback")]
    pub use ambition_platformer2d_rollback_ggrs::drive_control_frame;
    #[cfg(not(feature = "rollback"))]
    pub use ambition_platformer2d_runtime::input_drive::drive_control_frame;

    /// Who owns the clock — the companion to driving input, and on this surface
    /// for the same reason.
    ///
    /// ⭐⭐ A DRIVER THAT WRITES INPUT ALSO DECIDES WHEN TIME PASSES. These lived
    /// on `app` and `ambition_sim_harness` reached across to them, which the
    /// `sim-harness-names-only-the-public-sdk` contract caught — and its own note
    /// says why that is the right answer rather than a nuisance: *"If it needs
    /// crate-shaped facade paths, those are SDK gaps."* Stepping a simulation is
    /// not app-assembly trivia; it is the other half of the seam beside
    /// `drive_control_frame`.
    pub use crate::app::{enable_manual_stepping, manual_step_period};

    /// Drive input to ANY seat, and the half blind run 7's finding (g)
    /// recorded as missing.
    ///
    /// That is a worse shape than a missing feature: the capability was built, tested, and
    /// unreachable, and the finding it produced described the facade rather than the code.
    ///
    /// A driver that needs two independent streams — couch versus, a character
    /// select where four people press their own buttons, a two-peer replay —
    /// writes every seat through this. [`drive_control_frame`] is the same road
    /// with the primary slot filled in, kept because naming the primary seat is
    /// the common case.
    ///
    /// Under a latching host it folds into that seat's latch, so a sub-tick press
    /// survives exactly as the primary seat's does; without one it writes the
    /// pending seat input directly, which is what a headless or replay driver
    /// wants.
    pub use ambition_characters::control::PlayerSlot;

    /// EACH SEAT'S CONTROL-FRAME INTERPRETATION POLICY, resolved at input capture.
    ///
    /// Exported beside [`PlayerSlot`] for the reason its own doc gives: this is
    /// what deterministic simulation reads instead of `UserSettings`, so a
    /// headless or replay driver that stands in for input capture has to be able
    /// to publish into it. `ambition_sim_harness` is exactly that driver.
    pub use ambition_characters::control::SeatControlFrameModes;
    /// The pair of frame-mode policies a seat's row holds — exported with the
    /// table, because a caller that can publish a row needs the value type.
    pub use ambition_platformer2d_core::ControlFrameModes;

    /// WHO DRIVES THIS BODY. A body wearing one is steered by that seat's
    /// `SlotControls` frame; a body without one is left to its own `Brain`.
    ///
    /// Exported beside [`PlayerSlot`] because a composition that seats a second
    /// local participant has to say which body that seat holds, and the seat
    /// number alone cannot say it. Possession is the only thing that MOVES one
    /// at runtime; a spawn site authoring one is stating a seat, not competing
    /// with that reconcile.
    pub use ambition_characters::control::DrivingParticipant;

    /// EVERY SEAT'S RAW FRAME, before any shaping stage has run — the table
    /// a stage that refines input writes into.
    ///
    /// ⛔ **the global `ControlFrame` is NOT that table any more.** It was, and
    /// because it was one resource it could only ever be one seat's: a gesture,
    /// a portal warp or a scripted substitution applied to the primary and to
    /// nobody else, which is why player two could not fast-fall (D175). A stage
    /// that shapes input names the seat it shapes now.
    pub use ambition_characters::control::SeatRawFrames;

    /// WHAT EACH SEAT ACTUALLY RECEIVED — the committed frame every body
    /// reads through its `DrivingParticipant(slot)`.
    ///
    /// the read side of [`SeatRawFrames`]: that table is what the local device
    /// proposed and this is what the simulation agreed to, which under a
    /// rollback host are different frames on a resimulated tick.
    pub use ambition_characters::control::SlotControls;

    /// THE FRAME→TICK LATCH, one row per seat. A tap that opens and closes
    /// between two ticks folds in here and drains on the tick, so a fixed-tick
    /// or rollback host never swallows it.
    pub use ambition_characters::control::SlotControlLatches;
    #[cfg(feature = "rollback")]
    pub use ambition_platformer2d_rollback_ggrs::drive_slot_frame;
    #[cfg(not(feature = "rollback"))]
    pub use ambition_platformer2d_runtime::input_drive::drive_slot_frame;
}

/// What is drawn, as a game observes it.
///
/// Deliberately thin. A consumer reads the presented world; it does not own the
/// render path.
/// OFFSCREEN CAPTURE — photograph what the renderer drew, with no window.
///
/// ⭐ A SUPPORTED SURFACE, NOT AN INTERNAL ONE. Developer tools (the moveset
/// renderer, room capture, visual regression) are real customers, and they were
/// reaching `render::capture::…` — a crate-shaped path. `VisibleRenderMode::
/// OffscreenGpu` creates no window and disables winit, so this needs an adapter
/// that can render to a texture and neither a physical GPU nor an X server.
///
/// ⛔ BEHIND `capture`, DEFAULT-OFF, for the reason every instrument here is:
/// a game that never takes a screenshot must not link the machinery.
#[cfg(feature = "capture")]
pub mod capture {
    pub use ambition_render::capture::{
        adopt_cameras_into_capture_target, finish_after_capture, request_capture,
        setup_capture_target, CaptureAdopted, CaptureProgress, CaptureSettings, CaptureTarget,
    };
}

pub mod view {
    pub use ambition_platformer2d_shared_tangle::lifecycle::RoomVisual;

    /// The decoded art the presentation draws from.
    pub use ambition_sprite_sheet::game_assets::GameAssets;

    /// Which sprite an authored entity draws as.
    ///
    /// ⭐ HERE AS WELL AS IN `content`, because the two answer different
    /// questions and only one of them is gated. `content::EntitySprite` is the
    /// CONTENT-PACK surface and lives behind `feature = "content_pack"`; a
    /// consumer that merely wants to read what an entity draws as should not
    /// have to enable a content COMPILER to do it. Found when
    /// `cargo check -p ambition_demo_mary_o_app --features capture --all-targets`
    /// failed on `unresolved import ambition_platformer2d::content` — the
    /// `capture` feature does not imply `content_pack`, and the test only wanted
    /// this one type.
    pub use ambition_sprite_sheet::game_assets::EntitySprite;

    /// Where the art comes from: every asset path/source policy the
    /// presentation reads.
    pub use ambition_asset_manager::platformer_assets::{ids, Platformer2dAssetCatalog};

    /// The marker on a generated background layer.
    ///
    /// Exported so a consumer can ask whether its backdrop is DRAWN and whether
    /// it MOVES — two questions `fixtures/external_consumer` now asks, and could
    /// not ask at all while the component sat behind a private module. Its first
    /// facade path was a raw renderer-crate mirror; `outlander-names-only-the-public-sdk`
    /// caught that immediately: a third
    /// party reaching through the facade into an implementation module is the
    /// leak ADR 0031 exists to close, and "the test needed it" is how those get
    /// made.
    #[cfg(feature = "ambition_render")]
    pub use ambition_render::rendering::ParallaxLayerVisual;
}

/// Rollback, as a supported session mode.
///
/// The six properties ADR 0031 required before this could be a promise, and a
/// test for each — see the module docs.
#[cfg(feature = "rollback")]
pub mod rollback;

/// The authored world: rooms, geometry, placements, collision.
///
/// A CURATED MODULE, not a crate mirror — and the difference is the whole
/// point of ADR 0031. `pub use ambition_platformer2d_world as world` made the compatibility
/// surface change whenever the crate did: a new submodule became public API by
/// existing. This list is CLOSED, so adding one to `ambition_platformer2d_world` is an
/// internal change until somebody adds it here on purpose.
///
/// `ron_room` is deliberately absent. It is an authoring backend, nothing
/// outside the engine reaches for it, and the mirror was publishing it anyway.
///
/// TODO(compat-remove): replace the remaining implementation-shaped crate mirrors
/// with intentional domain API surfaces, then remove the mirrors.
pub mod world {
    /// A room's damaging volumes, as simulation state.
    pub use ambition_combat::hazard_runtime::HazardFeature;
    /// The derived solids/hazards a room contributes on top of its authored base.
    /// Named here for the same reason `ResolvedMotionFrame` is: it is a world
    /// fact, and reaching it through the actor crate is how a census mistakes it
    /// for that crate's coupling.
    pub use ambition_platformer2d_shared_tangle::feature_overlay::{
        FeatureEcsWorldOverlay, GatePass, RoomOverlays,
    };
    /// The per-tick motion environment a body is stepped in. Named here rather
    /// than through the actor crate: it is a world-physics fact, and routing it
    /// through a domain crate's re-export is how a census mistakes it for that
    /// domain's coupling.
    pub use ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame;
    /// The authored/base gravity and its resolved live field.
    pub use ambition_platformer2d_shared_tangle::gravity::{
        gravity_dir_or_default, AmbientGravityRequest, BaseGravity, GravityField, GravityZone,
        TemporaryZone,
    };
    /// Everything needed to author a room, in one import.
    pub use ambition_platformer2d_world::prelude;

    pub use ambition_platformer2d_world::{
        collision, debug_label, navigation, placements, platforms, rooms, world_manifest,
    };
}
// Re-exported so a game can name bevy TYPES through `crate::bevy::…`. NOTE:
// this does NOT let a crate `#[derive(Component)]`/`#[derive(Resource)]` through
// the umbrella alone — bevy's derive macros resolve `::bevy_ecs` via the
// CONSUMER's own Cargo.toml (`BevyManifest`), which a re-export does not satisfy.
// A content crate that defines its own components/resources must ALSO list `bevy`
// in its manifest (one line, version pinned by the workspace). See
// docs/planning/demos/README.md.
pub use bevy;

/// What an OBSERVER reads: authoritative combat geometry, stable identity, and
/// the geometric vocabulary both are expressed in.
///
/// ⭐⭐ THE SAME REASON `causal` IS HERE. An agent should be able to inspect what
/// combat did — where a body can be struck, where a strike is, what has
/// connected — without reading engine source or naming an engine-private path.
/// A tool that had to say `sim_view::` or `engine_core::` to ask those questions
/// was naming implementation topology to reach a supported capability, which is
/// the definition of an SDK gap.
///
/// ⛔ READ-ONLY BY CONSTRUCTION. `CombatGeometryView` is rebuilt from simulation
/// truth every tick; an observer compares and keys on what it finds here and
/// must not reach back through an entity to the authoritative component, which
/// is the coupling this surface exists to remove.
pub mod observation {
    /// The vocabulary an observation is expressed in. A consumer that can read a
    /// volume must be able to name its shape and its bounds.
    pub use ambition_platformer2d_core::{Aabb, AabbExt, CombatVolume, Vec2};
    /// The engine's own stable identity for a simulated entity.
    ///
    /// ⛔ A RAW ENTITY ID IS NOT AN IDENTITY: an entity index depends on every
    /// spawn and despawn the app made first, so two runs of one binary label the
    /// same body differently and a byte-diff of two recordings reports physics
    /// that did not change.
    pub use ambition_platformer2d_shared_tangle::sim_id::SimId;
    /// The runtime's combat geometry, as the production debug overlay draws it:
    /// effective hurtboxes with the rule that produced them, exact live strike
    /// volumes in world space, and each strike's hit-once memory.
    pub use ambition_sim_view::{
        CombatBodyGeometryView, CombatGeometryView, CombatStrikeGeometryView, HurtboxSource,
    };
}

/// Engine assembly helpers most games need first.
pub mod engine {
    pub use ambition_platformer2d_runtime::{
        add_headless_foundation, init_engine_states, Platformer2dSimulationFoundationPlugin,
        PlatformerEnginePlugins, SimCoreResourcesPlugin, SimulationHost, SimulationHostAppExt,
        SIM_TICK_HZ,
    };
}

/// Windowed host plugin groups and host-facing seams.
pub mod windowed_host {
    /// Windowed / fullscreen, as persisted host state.
    ///
    /// ⛔ GATED, because `ambition_persistence` is an OPTIONAL dependency that
    /// only `all_capabilities` turns on. An ungated `pub use` of it compiles
    /// fine inside the workspace — where the default feature set is on — and
    /// fails for a consumer who took `visible + basic_shell_presentation +
    /// input`, which is the exact shape the external-consumer fixture exists to
    /// catch. The two other `ambition_persistence` re-exports in this file
    /// carry the same gate.
    pub use ambition_persistence::host::windowing::DisplayModeState;
    #[cfg(feature = "input")]
    pub use ambition_platformer2d_host::HostInputBindingsPlugin;
    /// The camera follow/shake cluster. Gated with the renderer it wires: the
    /// host installs it only under its own `render` feature, so a composition
    /// that selected no presentation has nothing here to name.
    #[cfg(feature = "ambition_render")]
    pub use ambition_platformer2d_host::HostCameraPlugin;
    pub use ambition_platformer2d_host::PlatformerHostPlugins;
}

/// The generic platformer PRESENTATION face: a camera, the room's static visuals,
/// and the sprite/animation chain. A demo adds this beside the engine and host
/// groups; a game layers its own HUD/menus/dev stack on top (oracle-violation OV1).
#[cfg(feature = "ambition_render")]
pub mod presentation {
    pub use ambition_render::dialog_ui::{
        DefaultDialogUiPlugin, DialogOverlayRoot, DialogPresentationSet,
    };
    pub use ambition_render::platformer_presentation::{
        PlatformerPresentationPlugin, PlatformerPresentationSetupSet,
    };

    /// How gameplay is framed on the physical display: the four presentation
    /// policy axes, the pure layout resolver, and the tested author presets a
    /// provider declares with
    /// [`with_presentation_profiles`](ambition_platformer2d_provider::PlatformerExperienceAuthoring::with_presentation_profiles).
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation;
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation::profiles;

    /// The DECLARED-HUD seam: a game declares its readout slots with
    /// [`with_hud`](ambition_platformer2d_provider::PlatformerExperienceAuthoring::with_hud)
    /// and publishes their live values into [`HudReadouts`] each frame. The
    /// engine holds no readout vocabulary — every label is a string the game
    /// writes.
    pub use ambition_platformer2d_shared_tangle::gameplay_presentation::{
        ActiveDefensePresentationPolicy, ActiveHudDeclaration, DefenseCueCauses,
        DefensePresentationCues, DefensePresentationPolicy, HudDeclaration, HudFigure, HudReadout,
        HudReadouts, HudSlotId, HudSlotSpec, HudStanding, SurroundRegion,
    };
    pub use ambition_render::hud::declared::{
        declared_hud_owns, DeclaredHudPlugin, DeclaredHudRoot, DeclaredHudSlot,
    };
}

/// The SDK claim about the inspector, tested.
///
/// The program this belongs to says an agent should be able to *"inspect why the
/// resulting actor moved, attacked, took damage, or changed lifecycle state …
/// without importing internal-shaped engine modules or reading engine source"*.
///
/// So this module uses ONLY `crate::causal::…` — no `ambition_causal`, no
/// `ambition_platformer2d_runtime`, no `ambition_platformer2d_actor_monolith`. If a consumer would have to reach
/// past the facade to do this, that shows up here as a compile error rather
/// than as a paragraph somebody has to believe.
#[cfg(all(test, feature = "causal"))]
mod causal_sdk_tests {
    use crate::causal::{
        domains, record_domains, CausalFact, CausalPlugin, CausalRecording, FactDetail,
        RecordingPolicy, RecordingSet, SubjectKey,
    };
    use bevy::prelude::*;

    /// A game's own publisher, written the way a consumer would write one.
    fn a_game_publishes_something(mut log: ResMut<CausalRecording>) {
        log.record(
            CausalFact::new(
                domains::MOVEMENT,
                0,
                FactDetail::new("my_game_fact", "the game said something"),
            )
            .about(SubjectKey::Seat(0))
            .field("value", 7_i64),
        );
    }

    #[test]
    fn a_consumer_inspects_a_tick_through_the_facade_alone() {
        let mut app = App::new();
        app.add_plugins(CausalPlugin);
        app.insert_resource(crate::time::SimTick(11));
        record_domains(&mut app, RecordingPolicy::All);
        app.add_systems(
            Update,
            a_game_publishes_something.in_set(RecordingSet::Publish),
        );
        app.update();

        let why = app
            .world()
            .resource::<CausalRecording>()
            .explain(11, &SubjectKey::Seat(0));
        assert_eq!(
            why.first("my_game_fact").and_then(|f| f.get("value")),
            Some(&crate::causal::FactValue::Int(7)),
            "a game's own fact comes back out of the facade's own explainer"
        );
        assert!(
            why.render().contains("seat:0"),
            "and renders for a human: {}",
            why.render()
        );
    }

    /// the tick is the HOST's, even for a consumer's own fact. A game that
    /// had to stamp its own would be a second clock nothing could join against.
    #[test]
    fn a_consumers_fact_carries_the_hosts_tick_without_being_told_it() {
        let mut app = App::new();
        app.add_plugins(CausalPlugin);
        app.insert_resource(crate::time::SimTick(0));
        record_domains(&mut app, RecordingPolicy::All);
        app.add_systems(
            Update,
            a_game_publishes_something.in_set(RecordingSet::Publish),
        );

        app.world_mut().resource_mut::<crate::time::SimTick>().0 = 40;
        app.update();
        app.world_mut().resource_mut::<crate::time::SimTick>().0 = 41;
        app.update();

        let log = app.world().resource::<CausalRecording>();
        assert_eq!(log.explain(40, &SubjectKey::Seat(0)).facts().len(), 1);
        assert_eq!(log.explain(41, &SubjectKey::Seat(0)).facts().len(), 1);
        assert!(
            log.explain(42, &SubjectKey::Seat(0)).is_empty(),
            "and nothing lands on a tick that never ran"
        );
    }
}

/// The SDK claim about authoring, tested.
///
/// The program says an agent should *"add or modify a character and validate it
/// without rebuilding Rust"* and do so *"without importing internal-shaped
/// engine modules"*. This module uses ONLY `crate::content::…` — no
/// `ambition_content_pack`, no `ambition_characters`. A consumer forced past the
/// facade shows up here as a compile error.
#[cfg(all(test, feature = "content_pack"))]
mod content_sdk_tests {
    use crate::content::{
        compile, engine_schemas, AssetsUnchecked, CompileStage, ContentPackDraft,
        ContentPackManifest, DiagnosticCode, ModuleNamespace, PackId, PackVersion, SchemaId,
        SchemaVersion, SourceDeclaration,
    };

    const CATALOG: &str = r#"(
        brain_presets: { "stand_still": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "newcomer": (
                display_name: "Newcomer",
                manifest: "newcomer.ron",
                tier: MainHall,
                body_kind: Standard,
                composition: None,
                default_brain: "stand_still",
                default_action_set: "peaceful",
                tags: [],
            ),
        },
    )"#;

    fn pack(name: &str, catalog: &str) -> ContentPackDraft {
        let root = std::env::temp_dir().join(format!("ambition_content_sdk/{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp pack");
        std::fs::write(root.join("cast.ron"), catalog).expect("write");
        ContentPackDraft::read_manifest(
            root,
            ContentPackManifest {
                id: PackId("my_game".into()),
                version: PackVersion("1.0.0".into()),
                namespace: ModuleNamespace("my_game".into()),
                requires: Vec::new(),
                sources: vec![SourceDeclaration {
                    path: "cast.ron".into(),
                    schema: SchemaId::new("character_catalog"),
                    version: SchemaVersion(1),
                }],
            },
        )
        .expect("draft reads")
    }

    /// A consumer composes the engine's schemas and validates its own cast.
    #[test]
    fn a_consumer_validates_its_own_content_through_the_facade_alone() {
        let compiled = compile(&pack("valid", CATALOG), &engine_schemas(), &AssetsUnchecked)
            .expect("a well-formed cast compiles");
        assert_eq!(compiled.namespace.0, "my_game");
        assert!(
            compiled
                .get(&SchemaId::new("character"), "newcomer")
                .is_some(),
            "the consumer's own character is a prepared identity in its OWN namespace"
        );
    }

    /// and the refusals are the facade's too — a consumer does not have to
    /// reach into the compiler crate to learn WHY its content was rejected.
    #[test]
    fn a_typo_is_refused_through_the_facade_with_a_code_a_tool_can_branch_on() {
        let failure = compile(
            &pack(
                "typo",
                &CATALOG.replace(
                    r#"default_brain: "stand_still""#,
                    r#"default_brain: "stand_stil""#,
                ),
            ),
            &engine_schemas(),
            &AssetsUnchecked,
        )
        .expect_err("a preset typo must refuse");
        assert_eq!(failure.stage, CompileStage::ReferenceResolution);
        assert!(failure.has(DiagnosticCode::UnknownPreset));
        assert!(
            failure.render().contains("did you mean `stand_still`?"),
            "and it answers the typo: {}",
            failure.render()
        );
    }

    /// A pack read off disk (`content_pack!` without `static_content`): it
    /// plays each file as it is now, the manifest is the list of sources (a
    /// source line added to `pack.ron` is compiled with no Rust list), and a
    /// file it cannot read is a refusal that names each one, not only the
    /// first.
    #[test]
    fn a_rooted_pack_reads_its_manifests_sources_and_names_every_one_it_cannot() {
        use crate::content::EmbeddedPack;
        let dir = std::env::temp_dir().join(format!("ambition_rooted_pack_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("moves")).unwrap();
        let manifest = |sources: &[&str]| {
            let lines: String = sources
                .iter()
                .map(|path| format!(r#"(path: "{path}", schema: "moveset", version: 1),"#))
                .collect();
            format!(
                r#"(
                    id: "rooted_probe",
                    version: "0.1.0",
                    namespace: "probe",
                    requires: [],
                    sources: [{lines}],
                )"#
            )
        };
        let mover = |id: &str, duration: f32| {
            format!(
                r#"(
                    schema_version: 1,
                    entities: [(
                        id: "{id}",
                        contracts: (moveset: Some((
                            verbs: {{ "attack": "{id}_jab" }},
                            moves: [(
                                id: "{id}_jab",
                                clip: (clip: "jab"),
                                duration_s: {duration},
                                windows: [(start_s: 0.0, end_s: {duration}, tag: Recovery, volumes: [])],
                            )],
                        ))),
                    )],
                )"#
            )
        };
        let root: &'static str = Box::leak(dir.display().to_string().into_boxed_str());
        let jab = |pack: &EmbeddedPack, id: &str| {
            pack.compile()
                .ok()
                .and_then(|_| pack.moveset(id).move_by_id(&format!("{id}_jab")).map(|jab| jab.duration_s))
        };

        // The file as it is when the pack compiles, not when the crate did.
        std::fs::write(dir.join("pack.ron"), manifest(&["moves/hero.ron"])).unwrap();
        for duration in [0.25, 0.5] {
            std::fs::write(dir.join("moves/hero.ron"), mover("hero", duration)).unwrap();
            assert_eq!(jab(&EmbeddedPack::rooted(root, None), "hero"), Some(duration));
        }

        // A source line added to the manifest is a source: no Rust names it.
        std::fs::write(dir.join("moves/rival.ron"), mover("rival", 0.75)).unwrap();
        std::fs::write(dir.join("pack.ron"), manifest(&["moves/hero.ron", "moves/rival.ron"])).unwrap();
        assert_eq!(jab(&EmbeddedPack::rooted(root, None), "rival"), Some(0.75));

        std::fs::write(
            dir.join("pack.ron"),
            manifest(&["moves/hero.ron", "moves/absent.ron", "moves/gone.ron"]),
        )
        .unwrap();
        let failure = EmbeddedPack::rooted(root, None)
            .compile()
            .expect_err("a source that cannot be read must refuse the pack");
        let named: Vec<_> = failure
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == DiagnosticCode::MalformedSource)
            .collect();
        assert_eq!(named.len(), 2, "each unreadable file is named: {}", failure.render());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A game's pack, embedded: the table a character is registered with is
    /// the one its file carries, and a character the pack does not author is
    /// a refusal that names what the pack does author.
    #[test]
    fn an_embedded_pack_answers_with_the_table_its_file_carries() {
        use crate::content::EmbeddedPack;
        static PACK: EmbeddedPack = EmbeddedPack::new(
            r#"(
                id: "embedded_probe",
                version: "0.1.0",
                namespace: "probe",
                requires: [],
                sources: [(path: "moves/hero.ron", schema: "moveset", version: 1)],
            )"#,
            &[(
                "moves/hero.ron",
                r#"(
                    schema_version: 1,
                    entities: [(
                        id: "hero",
                        contracts: (moveset: Some((
                            verbs: { "attack": "hero_jab" },
                            moves: [(
                                id: "hero_jab",
                                clip: (clip: "jab"),
                                duration_s: 0.25,
                                windows: [(start_s: 0.0, end_s: 0.25, tag: Recovery, volumes: [])],
                            )],
                        ))),
                    )],
                )"#,
            )],
        );
        let table = PACK.moveset("hero");
        assert_eq!(table.verbs.get("attack").map(String::as_str), Some("hero_jab"));
        assert_eq!(table.move_by_id("hero_jab").map(|jab| jab.duration_s), Some(0.25));

        let refusal = std::panic::catch_unwind(|| PACK.moveset("stranger"))
            .expect_err("a character the pack does not author must be refused");
        let text = refusal
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(
            text.contains("stranger") && text.contains("\"hero\""),
            "the refusal does not name the character and what the pack authors: {text}"
        );

        // A pack with no catalog states no cast, and asking it for one is a
        // refusal that says what to declare, not an empty roster.
        let refusal = std::panic::catch_unwind(|| {
            let _ = PACK.cast("probe", None::<&str>);
        })
        .expect_err("a pack with no `character_catalog` source must refuse to state a cast");
        let text = refusal
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(
            text.contains("embedded_probe") && text.contains("character_catalog"),
            "the refusal does not name the pack and the source it lacks: {text}"
        );
    }

    /// A game's own schema: a pack source whose rules no engine capability
    /// reads is checked and lowered when the pack names the game's schema
    /// (`content_pack! { schemas: [..] }`), on each route that compiles it.
    #[test]
    fn a_pack_compiles_against_the_schemas_its_game_owns() {
        use crate::content::{
            CapabilityId, ContentSchemaHandler, EmbeddedPack,
            FacetOutcome, FacetSource, RuntimeDisposition, SchemaRegistration,
        };

        struct Rounds;
        impl ContentSchemaHandler for Rounds {
            fn check(&self, facet: &FacetSource<'_>, out: &mut FacetOutcome) {
                match facet.text.trim().parse::<u8>() {
                    Ok(rounds) => {
                        out.define(facet.content_id_in("probe_rounds", "rounds"), rounds.to_string());
                        out.lower(rounds);
                    }
                    Err(error) => out.report(facet.diagnostic(DiagnosticCode::MalformedSource, error.to_string())),
                }
            }
        }
        fn registration(id: &str) -> SchemaRegistration {
            SchemaRegistration {
                id: SchemaId::new(id),
                version: SchemaVersion(1),
                capability: CapabilityId::new("probe_game"),
                disposition: RuntimeDisposition::Runtime,
                doc: "a probe game's rounds",
                handler: std::sync::Arc::new(Rounds),
            }
        }
        fn rounds() -> SchemaRegistration {
            registration("probe_rounds")
        }
        fn claims_an_engine_schema() -> SchemaRegistration {
            registration("room_rules")
        }
        const MANIFEST: &str = r#"(
            id: "game_schema_probe",
            version: "0.1.0",
            namespace: "probe",
            requires: [],
            sources: [(path: "rounds.txt", schema: "probe_rounds", version: 1)],
        )"#;
        const SOURCES: &[(&str, &str)] = &[("rounds.txt", "3")];

        static OWNS: EmbeddedPack = EmbeddedPack::new(MANIFEST, SOURCES).with_schemas(|| vec![rounds()]);
        let lowered = |pack: &crate::content::PreparedContentPack| {
            pack.lowered::<u8>(&SchemaId::new("probe_rounds")).copied()
        };
        assert_eq!(lowered(OWNS.prepared()), Some(3), "the game's schema lowers its file");
        // A check binary gives the content CLI this list, so the CLI checks
        // the pack against the schemas the game compiles it with.
        let ids = |schemas: fn() -> Vec<SchemaRegistration>| schemas().into_iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(ids(OWNS.game_schemas()), vec![SchemaId::new("probe_rounds")]);
        assert!(ids(EmbeddedPack::new(MANIFEST, SOURCES).game_schemas()).is_empty());
        // The composition's road: a profile's schemas, plus the game's own.
        let admitted = OWNS
            .compile_with(&engine_schemas())
            .unwrap_or_else(|failure| panic!("{}", failure.render()));
        assert_eq!(lowered(&admitted), Some(3), "the composition's compile drops the game's schema");

        // Control: the same pack without the game's schema is refused, and
        // the refusal names the schema.
        let failure = EmbeddedPack::new(MANIFEST, SOURCES)
            .compile()
            .expect_err("no capability owns `probe_rounds` without the game's schema");
        assert!(failure.has(DiagnosticCode::UnknownSchema), "{}", failure.render());

        // A game cannot take an engine schema's id: two owners for one file
        // kind is refused, not settled by registration order.
        let failure = EmbeddedPack::new(MANIFEST, SOURCES)
            .with_schemas(|| vec![rounds(), claims_an_engine_schema()])
            .compile()
            .expect_err("a game schema with an engine schema's id must be refused");
        assert!(failure.has(DiagnosticCode::AmbiguousSchemaOwnership), "{}", failure.render());
    }

}

/// A game's [`content::EmbeddedPack`], read off disk in a development build and
/// embedded in a build that turns on the CALLING crate's `static_content`
/// feature.
///
/// An `include_str!` is a compilation input: with every text embedded, a
/// tuning edit to a move table compiles the game crate and links the binary
/// again. Off disk, the edit costs a restart. Turn `static_content` on where
/// there is no source tree to read (web, Android, a bundled build); the game
/// crate declares the feature, and the app that hosts it forwards its own.
///
/// `root` is relative to the calling crate's directory and holds `pack.ron`.
/// The manifest is the one list of the pack's sources: each is the path
/// `pack.ron` declares, relative to `root` (`..` included). A source line added
/// to `pack.ron` is played at the next start with no Rust build.
///
/// To embed, the calling crate has a `build.rs` that calls
/// `ambition_content_pack::embed::embed_pack` with the same `root`; it writes
/// the `include_str!` of each declared source under `static_content`.
///
/// ```ignore
/// pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! { root: "assets" };
/// ```
///
/// A rule that is the game's own policy (its rounds, its tuning) has a schema
/// the game owns. Name it with `schemas:`, and the pack compiles against it
/// with the engine's schemas, so the game's file is checked at admission and
/// read as data. `content::rules_schema` makes such a schema from a struct and
/// its range check:
///
/// ```ignore
/// pub static PACK: EmbeddedPack = ambition_platformer2d::content_pack! {
///     root: "assets",
///     schemas: [crate::rules::my_rules_schema],
/// };
/// ```
#[cfg(feature = "content_pack")]
#[macro_export]
macro_rules! content_pack {
    (root: $root:literal, schemas: [$($schema:path),* $(,)?] $(,)?) => {
        $crate::content_pack! { root: $root }.with_schemas({
            fn schemas() -> Vec<$crate::content::SchemaRegistration> {
                vec![$($schema()),*]
            }
            schemas
        })
    };
    (root: $root:literal $(,)?) => {
        $crate::content::EmbeddedPack::rooted(concat!(env!("CARGO_MANIFEST_DIR"), "/", $root), {
            #[cfg(feature = "static_content")]
            const TEXTS: Option<(&str, &[(&str, &str)])> =
                Some(include!(concat!(env!("OUT_DIR"), "/content_pack/", $root, "/embedded.rs")));
            #[cfg(not(feature = "static_content"))]
            const TEXTS: Option<(&str, &[(&str, &str)])> = None;
            TEXTS
        })
    };
}

/// One authored file of a game that is not a pack source, such as a world
/// file: a [`content::PackText`], read off disk in a development build and
/// embedded under the CALLING crate's `static_content` feature, as each
/// source of [`content_pack!`] is. Read it with [`content::PackText::text`].
///
/// `$file` is relative to the calling crate's directory.
///
/// ```ignore
/// pub static WORLD: PackText = ambition_platformer2d::content_text!("assets/worlds/hero.ldtk");
/// let project = LdtkProject::from_json_str(&WORLD.text()?)?;
/// ```
#[cfg(feature = "content_pack")]
#[macro_export]
macro_rules! content_text {
    ($file:literal $(,)?) => {
        $crate::__content_pack_text!($file)
    };
}

/// One authored text file of a game, such as a shader, served on Bevy's
/// `embedded://` source at the path `embedded_asset!` gives it:
/// `embedded://<crate>/<path after src/>`. A material names it as before.
///
/// It is a [`content::PackText`]: read off disk in a development build and
/// embedded under the calling crate's `static_content` feature. So an edit to
/// a game's shader costs a restart, not a build of the game's crate.
///
/// `$file` is relative to the calling crate's directory, and is under `src/`.
///
/// ```ignore
/// ambition_platformer2d::content_asset!(app, "src/shaders/glow.wgsl");
/// // "embedded://my_game/shaders/glow.wgsl"
/// ```
#[cfg(feature = "content_pack")]
#[macro_export]
macro_rules! content_asset {
    ($app:expr, $file:literal $(,)?) => {{
        const ASSET_PATH: &str = match $file.as_bytes() {
            [b's', b'r', b'c', b'/', ..] => $file.split_at(4).1,
            _ => panic!(concat!("content_asset!: ", $file, " is not under src/")),
        };
        $crate::__content_pack_text!($file).serve_embedded($app, env!("CARGO_CRATE_NAME"), ASSET_PATH)
    }};
}

/// One [`content::PackText`] for [`content_text!`]. The `cfg` is read in the
/// crate that calls `content_text!`, so each game embeds by its own feature.
#[cfg(feature = "content_pack")]
#[doc(hidden)]
#[macro_export]
macro_rules! __content_pack_text {
    ($file:expr) => {
        $crate::content::PackText {
            file: concat!(env!("CARGO_MANIFEST_DIR"), "/", $file),
            embedded: {
                #[cfg(feature = "static_content")]
                const TEXT: Option<&str> = Some(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $file)));
                #[cfg(not(feature = "static_content"))]
                const TEXT: Option<&str> = None;
                TEXT
            },
        }
    };
}
