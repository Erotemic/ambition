//! Actor-runtime facade for authored placement records.
//!
//! `ambition_platformer2d_world` owns the pure generic lowering registry. The actor runtime
//! specializes that registry with the App-local [`CharacterCatalog`] context,
//! so placement interpreters can resolve authored character ids without adding
//! an upward dependency to the world IR or consulting process-global state.

use ambition_characters::actor::character_catalog::CharacterCatalog;

/// Immutable App-local authored context supplied to room placement lowering.
#[derive(Clone, Debug)]
pub struct ActorPlacementContext {
    pub characters: CharacterCatalog,
    /// Sheets this app's providers authored (U1). The same authored-content
    /// class as the catalog beside it: what a body looks like decides how big
    /// its collision box is, so lowering needs it exactly where it needs the
    /// catalog.
    pub sheets: ambition_sprite_sheet::character::sheets::AuthoredSheets,
    /// The prepared characters this host can build, so lowering can ask what a character's
    /// own DEFAULT autonomous profile is.
    ///
    /// cloned like its three neighbours, and for the same reason: lowering runs
    /// against a snapshot of authored content taken when staging was requested,
    /// not against live resources that may change mid-commit.
    pub prepared: ambition_characters::prepared::PreparedCharacterRegistry,
    /// The shared controller policies this host published, so a PLACEMENT
    /// may name one (`EnemySpawnSpec::brain_profile`).
    ///
    /// a separate authority from the catalog beside it, deliberately: a
    /// character catalog answers who exists, and this answers what may drive a
    /// body. An EMPTY registry means "this composition publishes no shared
    /// policies", which is the correct reading for a fixture and for a host that
    /// authors none — and it is why a placement that NAMES one against an empty
    /// registry is a construction error rather than a shrug.
    pub brain_profiles: ambition_characters::actor::character_catalog::BrainProfileRegistry,
    /// What a DEVELOPER has forced every authored actor's brain to, captured
    /// with the rest of the snapshot.
    ///
    /// ⭐⭐ THE VALUE, NOT THE CRATE THAT OWNS THE KNOB. Lowering used to call
    /// `ambition_dev_tools::brain_override::forced_profile()` from inside
    /// `resolve_npc_brain` — the simulation reaching up into a developer crate,
    /// mid-brain-construction, to decide what the world contains. It rides here
    /// now for the same reason its four neighbours do: lowering runs against a
    /// SNAPSHOT taken when staging was requested, and a knob that could change
    /// under a commit would be a different room half way through.
    ///
    /// ⛔ DEFAULT IS "THE AUTHOR DECIDES", which is what an unset environment
    /// variable has always meant — so a composition with no developer tools
    /// lowers exactly as before.
    pub forced_brains: ambition_characters::brain::AuthoredBrainOverride,
}

impl ActorPlacementContext {
    /// Supply the prepared cast, so lowering can read a character's own default autonomous
    /// profile.
    ///
    /// a builder rather than a fourth constructor argument, and that is a
    /// judgement not a shortcut. Two of the four construction sites have the
    /// registry to hand and two do not (a summon system and a pair of
    /// construction fixtures), and an EMPTY registry is already a meaningful,
    /// correct value here — it means "no character states a default", which is
    /// what the catalog-only path has always assumed. Forcing the argument would
    /// make those sites write `&Default::default()`, which says less.
    #[must_use]
    pub fn with_prepared(
        mut self,
        prepared: &ambition_characters::prepared::PreparedCharacterRegistry,
    ) -> Self {
        self.prepared = prepared.clone();
        self
    }

    /// Supply the developer brain override, so a forced run reaches the cast.
    ///
    /// A builder for the reason its two neighbours are: `Default` — "the author
    /// decides" — is already the correct value for every composition that
    /// installs no developer tools, and forcing the argument would make those
    /// sites write `&Default::default()`, which says less.
    #[must_use]
    pub fn with_forced_brains(
        mut self,
        forced: &ambition_characters::brain::AuthoredBrainOverride,
    ) -> Self {
        self.forced_brains = forced.clone();
        self
    }

    /// Supply the published controller policies, so a placement may name
    /// one. A builder for the same reason `with_prepared` is: an empty registry
    /// is a meaningful value, and the sites that have none should not have to
    /// write one.
    #[must_use]
    pub fn with_brain_profiles(
        mut self,
        profiles: &ambition_characters::actor::character_catalog::BrainProfileRegistry,
    ) -> Self {
        self.brain_profiles = profiles.clone();
        self
    }

    pub fn new(
        characters: &CharacterCatalog,
        sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    ) -> Self {
        Self {
            characters: characters.clone(),
            sheets: sheets.clone(),
            prepared: Default::default(),
            brain_profiles: Default::default(),
            forced_brains: Default::default(),
        }
    }
}

pub type LoweringCtx<'w, 's, 'a> =
    ambition_platformer2d_world::placements::LoweringCtx<'w, 's, 'a, ActorPlacementContext>;
pub type LoweringFn = ambition_platformer2d_world::placements::LoweringFn<ActorPlacementContext>;
pub type PlacementLoweringRegistry =
    ambition_platformer2d_world::placements::PlacementLoweringRegistry<ActorPlacementContext>;
