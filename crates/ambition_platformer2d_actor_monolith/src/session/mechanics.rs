//! The mechanical values ONE gameplay generation is made of.
//!
//! ⛔⛤ **THIS TYPE EXISTS BECAUSE A FREEZE WITH THE WRONG LIFETIME IS BARELY A
//! FREEZE.** A 2026-09-13 review measured the shape the previous closure left:
//!
//! ```text
//! session activation     -> frozen generation values
//! later room transition  -> current App values
//! later reset            -> current App values
//! ```
//!
//! ⇒ *"a prepared generation MEANS its frozen mechanical values"* was true of the
//! FIRST construction call and of nothing else. Every later road that rebuilds a
//! room — a door, a death, a reset — went back to querying whatever the App
//! happened to hold, so a generation's world could be reconstructed out of a
//! generation it was never prepared against.
//!
//! ⭐⭐ **ONE OWNER, PROJECTIONS TO THE ROADS — NOT A FROZEN COPY PER ROAD.** A
//! "transition registry" beside a "reset registry" beside a "checkpoint
//! registry" is the duplicate-authority defect wearing a lifecycle hat, and they
//! would need synchronising, which is the thing this repository removes rather
//! than automates.
//!
//! ⚠ **THE BINDING AND THE VALUES ARE DIFFERENT FACTS AND STAY DIFFERENT
//! TYPES.** `ActiveContentBinding` names WHICH generation the session runs under
//! and is compared at the commit boundary; this holds WHAT that generation
//! contains. Folding the registries into that tiny identifier was considered and
//! rejected in review: a comparison value and a content bag have no business
//! being one resource.

/// The mechanical registries a session is CONSTRUCTED from.
///
/// Frozen when the generation's identity was taken, and — once activation
/// promotes it — the authority every later construction road reads instead of
/// the App.
///
/// ⚠ **CLONED, AND THE COST IS THE POINT.** These are the values the prepared
/// fingerprint is over; holding handles instead would reintroduce the
/// read-at-activation this exists to remove. The shipped cast is 58 characters,
/// so the cost is bounded and paid once per generation.
///
/// ⛔ **WHAT IS NOT IN HERE, said plainly rather than left to be assumed:**
/// `CharacterCatalog` and `BrainProfileRegistry` are also construction inputs and
/// are still read from the App on every road — bound TRANSITIVELY (see below), so
/// a freeze of them would be a freeze the fingerprint corroborates only at one
/// remove.
///
/// ⛔⛤ **`AuthoredBrainOverride` AND `AuthoredPopulationCap` WERE ALSO ON THAT
/// LIST, AND THE 2026-09-13 REVIEW NAMED WHY THAT WAS NOT ENOUGH.** `Q126` put
/// them in the identity hash; construction went on reading the live App
/// resources — at activation (`lifecycle.rs`), at reset (`session/reset`), and at
/// room transition. ⇒ A value edited after preparation constructs **B under
/// identity A**, which is a TOCTOU between the fingerprint and its own subject.
/// *"It appears somewhere in the hash"* is not the invariant: **execution must
/// consume the same generation-bound input the identity describes.** They are
/// owned here now, and every road projects them.
///
/// ⛔⛤ **AND MY FIRST VERSION OF THIS PARAGRAPH SAID THE FIRST TWO ARE NOT IN
/// `PreparedContentIdentity` EITHER. THAT WAS WRONG — MEASURED 2026-09-13.** Both
/// are built by ONE assembly from `CharacterCatalogRegistry`'s fragments (see
/// `character_catalog::registry`, whose own comment is *"same values, same keys,
/// so the two cannot disagree"*), the production road never inserts either as a
/// standalone authority (*"Not `insert_resource(CharacterCatalog)`: that would be
/// a second authority on what the cast is"*), and the identity binds
/// `CharacterCatalogRegistry::canonical_fragments()` — the SOURCE RON. ⇒ They are
/// bound TRANSITIVELY, and poisoning that one binding reddens the catalog arm and
/// the controller-policy arm together, which is what a shared authority looks
/// like. `AuthoredBrainOverride` and `AuthoredPopulationCap` were the real gap
/// and are closed (`Q126`).
///
/// ⇒ **A generation owns exactly what its identity binds DIRECTLY, and no more
/// than that** — extending the ownership without extending the identity would
/// make this type claim a freeze the fingerprint cannot corroborate, and
/// extending the identity without extending the ownership leaves the gap above.
/// The two move together or neither moves.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct SessionMechanics {
    /// `None` where the composition published no cast — a real state, and not a
    /// missing value.
    pub characters: Option<ambition_characters::prepared::PreparedCharacterRegistry>,
    pub sheets: ambition_sprite_sheet::character::sheets::AuthoredSheets,
    pub bosses: ambition_boss_encounter::BossCatalog,
    /// The forced preset/profile a developer build asked for.
    ///
    /// ⚠ NOT `Option`: absent and default are the same mechanical fact here, and
    /// `AuthoredBrainOverride::default()` is exactly *"force nothing"*. An
    /// `Option` would let a road distinguish two states the construction code
    /// cannot.
    pub forced_brains: ambition_characters::brain::AuthoredBrainOverride,
    /// The developer population ceiling this generation was prepared under.
    pub population_cap: ambition_characters::actor::AuthoredPopulationCap,
    /// The perception viewport override this generation was prepared under.
    ///
    /// ⛔⛤ **THE 2026-09-13 REVIEW FOUND THIS ONE OUTSIDE EVERY IDENTITY, AND
    /// ITS OWN DOC IS THE ARGUMENT FOR BINDING IT:** *"IT CHANGES THE SIMULATION
    /// AND NO NUMBER TAKEN UNDER IT DESCRIBES THE SHIPPED GAME, exactly as
    /// `AuthoredPopulationCap` says of itself."* MEASURED at HEAD: its only
    /// production writer is `dev_tools::perception_extent::from_env`, read once
    /// at build — so it is MECHANICAL and IMMUTABLE, `Q119`'s first row, and gets
    /// the first row's treatment rather than a policy of its own.
    ///
    /// ⚠ The rollback answer (*"the resulting `Perception` component is rollback
    /// state"*) covers bodies that already exist. It says nothing about an actor
    /// constructed later in the generation, and nothing about two peers agreeing
    /// they run the same mechanics.
    pub perception_extent: ambition_characters::perception::PerceptionExtentOverride,
}

impl SessionMechanics {
    /// The prepared cast for construction, or `None` when this composition
    /// published none.
    pub fn prepared_cast(
        &self,
    ) -> Option<&ambition_characters::prepared::PreparedCharacterRegistry> {
        self.characters.as_ref()
    }
}

/// Read the generation's mechanics, falling back to the App's registries.
///
/// ⛔⛤ **THE FALLBACK IS FOR COMPOSITIONS WITH NO ACTIVATED GENERATION, NOT FOR
/// CONVENIENCE.** Fixtures, demos and the headless harnesses build rooms without
/// ever going through provider activation, and refusing them a cast would turn a
/// lifecycle improvement into a compatibility break across ~90 test binaries.
///
/// ⚠ **SO THE GUARANTEE IS CONDITIONAL AND THE CONDITION IS NAMED:** wherever a
/// generation HAS been activated, its values win over anything the App is
/// holding now. That is the property the review asked for and the property the
/// acceptance arm poisons. A composition with no activated generation has no
/// generation to be stale against — the same honest gap
/// `ActiveContentBinding` documents for the same reason.
pub struct GenerationMechanics<'a> {
    active: Option<&'a SessionMechanics>,
    app_characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    app_sheets: &'a ambition_sprite_sheet::character::sheets::AuthoredSheets,
    app_bosses: &'a ambition_boss_encounter::BossCatalog,
    app_forced_brains: Option<&'a ambition_characters::brain::AuthoredBrainOverride>,
    app_population_cap: Option<&'a ambition_characters::actor::AuthoredPopulationCap>,
}

impl<'a> GenerationMechanics<'a> {
    pub fn new(
        active: Option<&'a SessionMechanics>,
        app_characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
        app_sheets: &'a ambition_sprite_sheet::character::sheets::AuthoredSheets,
        app_bosses: &'a ambition_boss_encounter::BossCatalog,
    ) -> Self {
        Self {
            active,
            app_characters,
            app_sheets,
            app_bosses,
            app_forced_brains: None,
            app_population_cap: None,
        }
    }

    /// The App's developer knobs, for a composition with NO activated generation.
    ///
    /// ⚠ Separate from [`Self::new`] because most callers have no such
    /// resources: a builder method keeps the fallback OPTIONAL at the call site
    /// rather than making every fixture name two values it does not have.
    pub fn with_app_developer_knobs(
        mut self,
        forced_brains: Option<&'a ambition_characters::brain::AuthoredBrainOverride>,
        population_cap: Option<&'a ambition_characters::actor::AuthoredPopulationCap>,
    ) -> Self {
        self.app_forced_brains = forced_brains;
        self.app_population_cap = population_cap;
        self
    }

    /// A generation's values with NO App fallback.
    ///
    /// ⭐ **FOR THE ROAD THAT CANNOT BE WITHOUT A GENERATION: ACTIVATION.** The
    /// provider holds the exact frozen record it is building from and has no
    /// handles to fall back to — Q121 removed them — so offering it a fallback
    /// parameter would be offering it a value it must never use.
    pub fn of(generation: &'a SessionMechanics) -> Self {
        Self {
            active: Some(generation),
            app_characters: None,
            app_sheets: &generation.sheets,
            app_bosses: &generation.bosses,
            app_forced_brains: None,
            app_population_cap: None,
        }
    }

    pub fn characters(
        &self,
    ) -> Option<&'a ambition_characters::prepared::PreparedCharacterRegistry> {
        match self.active {
            Some(generation) => generation.characters.as_ref(),
            None => self.app_characters,
        }
    }

    pub fn sheets(&self) -> &'a ambition_sprite_sheet::character::sheets::AuthoredSheets {
        match self.active {
            Some(generation) => &generation.sheets,
            None => self.app_sheets,
        }
    }

    pub fn bosses(&self) -> &'a ambition_boss_encounter::BossCatalog {
        match self.active {
            Some(generation) => &generation.bosses,
            None => self.app_bosses,
        }
    }

    /// The forced brain preset/profile this construction must use.
    ///
    /// ⛔ **AN ACTIVATED GENERATION'S VALUE WINS OVER WHATEVER THE App HOLDS
    /// NOW**, which is the whole point: the identity was taken over this value,
    /// so construction reading a newer one would build B under identity A.
    pub fn forced_brains(&self) -> Option<&'a ambition_characters::brain::AuthoredBrainOverride> {
        match self.active {
            Some(generation) => Some(&generation.forced_brains),
            None => self.app_forced_brains,
        }
    }

    /// The population ceiling this construction must spend.
    pub fn population_cap(&self) -> Option<&'a ambition_characters::actor::AuthoredPopulationCap> {
        match self.active {
            Some(generation) => Some(&generation.population_cap),
            None => self.app_population_cap,
        }
    }
}

/// The perception viewport override a body being built must be given.
///
/// ⭐ **A FREE FUNCTION RATHER THAN A `GenerationMechanics` METHOD, because its
/// caller is a SYSTEM and not a construction road.** `ensure_perception` attaches
/// senses to bodies as they appear — it holds no plan, no room and no
/// `GenerationMechanics` — so what it needs is the same ranking expressed where
/// it can be asked: the activated generation's value, or the App's when a
/// composition has none.
pub fn perception_extent_for(
    generation: Option<&SessionMechanics>,
    app: Option<&ambition_characters::perception::PerceptionExtentOverride>,
) -> ambition_characters::perception::PerceptionExtentOverride {
    match generation {
        Some(generation) => generation.perception_extent,
        None => app.copied().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two casts that differ in a value the BODY BUILDER reads.
    ///
    /// ⭐ `max_health`, not a generation counter: a resolver that returns the
    /// right STAMP with the wrong CONTENTS would pass a counter comparison.
    fn cast(max_health: i32) -> ambition_characters::prepared::PreparedCharacterRegistry {
        let mut app = bevy::app::App::new();
        let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
            "alpha", "Alpha", "fixture",
        );
        definition.vitals.max_health = Some(max_health);
        ambition_characters::prepared::stage_authored_character(
            &mut app,
            definition,
            &Default::default(),
        )
        .expect("stages");
        ambition_characters::prepared::close_preparation_barrier(app.world_mut());
        app.world()
            .get_resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
            .cloned()
            .expect("the barrier publishes a cast")
    }

    fn health(
        registry: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    ) -> Option<i32> {
        registry
            .and_then(|cast| cast.get("alpha"))
            .and_then(|definition| definition.vitals.max_health)
    }

    /// ⛔⛤ **A GENERATION'S ROOMS ARE REBUILT FROM THAT GENERATION, WHATEVER THE
    /// App IS HOLDING NOW.**
    ///
    /// ⛤ THIS ARM EXISTS BECAUSE ITS ABSENCE WAS MEASURED. A review found the
    /// previous freeze governed the FIRST construction call and nothing else:
    /// `PreparedPlatformerSessions::take` dropped the frozen state at
    /// activation, so a door, a death and a reset all went back to querying
    /// whatever registries the App held by then. A session prepared under
    /// generation N could walk through a door and get N+1's fighters, because a
    /// reload had published one in between without this session ever activating
    /// it.
    #[test]
    fn an_activated_generation_outranks_whatever_the_app_publishes_later() {
        let generation = SessionMechanics {
            characters: Some(cast(9)),
            ..Default::default()
        };
        // What a later reload published into the App, with no admitted
        // transition for THIS session.
        let app_now = cast(3);

        // ⭐ THE PREMISE FIRST: the two casts must actually differ, or every
        // assertion below is satisfied by a resolver that returns anything.
        assert_eq!(health(generation.characters.as_ref()), Some(9));
        assert_eq!(health(Some(&app_now)), Some(3));

        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let bosses = ambition_boss_encounter::BossCatalog::default();

        // ⛔ THE ASSERTION THE ARM IS FOR.
        assert_eq!(
            health(
                GenerationMechanics::new(Some(&generation), Some(&app_now), &sheets, &bosses)
                    .characters()
            ),
            Some(9),
            "a room rebuilt inside an activated generation was built from a cast \
             the App published later and this session never activated",
        );

        // ⚠ THE FALLBACK, AND IT IS A NAMED CONDITION RATHER THAN A DEFAULT:
        // a composition that never went through provider activation has no
        // generation to be stale against, and must keep building rooms.
        assert_eq!(
            health(GenerationMechanics::new(None, Some(&app_now), &sheets, &bosses).characters()),
            Some(3),
            "a composition with no activated generation lost its cast",
        );

        // ⛔ AND AN ACTIVATED GENERATION THAT PUBLISHED NO CAST MEANS NO CAST —
        // not "fall through to the App's". Publishing none is a legal state, and
        // silently substituting a stranger's registry for it is the defect this
        // whole type removes, one layer down.
        assert_eq!(
            health(
                GenerationMechanics::new(
                    Some(&SessionMechanics::default()),
                    Some(&app_now),
                    &sheets,
                    &bosses,
                )
                .characters()
            ),
            None,
            "a castless generation was handed the App's cast",
        );
    }

    /// ⛔⛤ **A GENERATION'S DEVELOPER KNOBS OUTRANK WHATEVER THE App HOLDS NOW —
    /// AND UNTIL 2026-09-13 THEY DID NOT EXIST HERE AT ALL.**
    ///
    /// `Q126` put `AuthoredBrainOverride` and `AuthoredPopulationCap` into
    /// `PreparedContentIdentity`. Construction went on reading the LIVE App
    /// resources on all three roads — activation, reset, room transition — so a
    /// value edited between preparation and construction built **B under
    /// identity A**. A fingerprint over a value somebody else is free to change
    /// before it is spent is a fingerprint of nothing.
    ///
    /// ⭐ THE TWO SIDES ARE DELIBERATELY BOTH PRESENT AND DIFFERENT. An arm where
    /// the App holds nothing would pass against a projection that reads the App
    /// first and falls back to the generation — the exact inversion of the
    /// contract.
    #[test]
    fn an_activated_generations_developer_knobs_outrank_the_apps() {
        use ambition_characters::actor::AuthoredPopulationCap;
        use ambition_characters::brain::AuthoredBrainOverride;

        let generation = SessionMechanics {
            population_cap: AuthoredPopulationCap::capped_at(1),
            forced_brains: AuthoredBrainOverride {
                preset: Some("generation_brain".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let app_cap = AuthoredPopulationCap::capped_at(99);
        let app_brains = AuthoredBrainOverride {
            preset: Some("app_brain".into()),
            ..Default::default()
        };
        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let bosses = ambition_boss_encounter::BossCatalog::default();

        let mechanics = GenerationMechanics::new(Some(&generation), None, &sheets, &bosses)
            .with_app_developer_knobs(Some(&app_brains), Some(&app_cap));

        assert_eq!(
            mechanics.population_cap(),
            Some(&AuthoredPopulationCap::capped_at(1)),
            "construction would spend the App's population cap, so a room rebuilt \
             inside this generation admits a roster its identity never described"
        );
        assert_eq!(
            mechanics
                .forced_brains()
                .and_then(AuthoredBrainOverride::preset),
            Some("generation_brain"),
            "construction would force the App's brain preset, so the actors built \
             under this identity are not the actors it was fingerprinted over"
        );

        // ⭐ THE CONTROL: with NO activated generation the App's values are what
        // there is, so a projection that simply ignored the App would pass the
        // arm above and break every fixture.
        let no_generation =
            GenerationMechanics::new(None, None, &sheets, &bosses)
                .with_app_developer_knobs(Some(&app_brains), Some(&app_cap));
        assert_eq!(
            no_generation.population_cap(),
            Some(&AuthoredPopulationCap::capped_at(99)),
            "a composition with no activated generation lost the App's cap, so the \
             assertion above is about ignoring the App rather than about ranking"
        );
    }
}
