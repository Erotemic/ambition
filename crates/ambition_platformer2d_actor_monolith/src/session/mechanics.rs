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

/// The mechanical registries ONE construction reads — and only one set of them.
///
/// ⛔⛤ **THIS USED TO HOLD TWO SOURCES AND RANK THEM, WHICH IS THE LAST OPEN
/// DUPLICATE-AUTHORITY FAMILY IN THE CENSUS (`DUP-GENERATION-MECHANICS`).** It
/// carried `active: Option<&SessionMechanics>` beside five `app_*` registries
/// and every accessor chose between them, so which authority a construction
/// spent was decided inside the type, by whichever constructor the caller
/// happened to pick. `Q144`'s option 1 named its own weakness in as many
/// words: *"nothing but review enforces that choice at a new call site."*
///
/// ⭐⭐ **THE CHOICE IS NOW THE CALLER'S AND IT IS MADE ONCE, AT A NAMED
/// CONSTRUCTOR.** Three roads, three constructors, no ranking left inside:
///
/// ```text
/// of                an activated generation's frozen values
/// for_live_session  the same, or a REFUSAL
/// ```
///
/// ⛔⛤ **THERE WAS A THIRD, `for_the_generation_being_built`, AND ITS ONE
/// PRODUCTION CALLER WAS THE DEFECT IT WAS WRITTEN FOR.** It handed
/// construction five App registries under the argument that *"a hot reload
/// legitimately has no active generation to read: it is assembling the one that
/// replaces the live one"*. That is true of a MECHANICAL replacement and false
/// of the only caller there was: `reload_ldtk_world_from_disk` builds its
/// candidate with `prepare_world_replacement_candidate`, which copies every
/// non-`world.` fingerprint section out of the ACTIVE content — so the candidate
/// published *"same mechanics, new world"* over a room built from whatever the
/// App happened to hold. Reviewed 2026-09-20.
///
/// ⇒ A road that really does replace the mechanics builds the new
/// `SessionMechanics` and reads it through [`Self::of`], so the values
/// construction spends and the values the identity is taken over are one
/// object. There is no constructor left that accepts loose registries, and
/// therefore no road on which a live rebuild can reach the App at all. The 2026-09-19 composition ruling asked
/// for exactly that: *"explicit direct/headless/test compositions may hold
/// scoped fixture/direct-entry authority where needed, but no anonymous
/// App-global fallback state returns."*
///
/// ⚠ **AND THE COST OF DELETING THE FALLBACK WAS MEASURED, NOT ESTIMATED.**
/// The row had been costed against *"~90 test binaries"*. Poisoning the
/// `!shell_routed && active.is_none()` branch and running the whole workspace
/// (2026-09-20) reddened THIRTEEN tests in two files: five in
/// `session/reset/tests.rs`, which share one `min_app()`, and seven in
/// `demo_shell_smoke.rs`, plus the arm that asserted the branch existed. The
/// 740-test `app_it` suite — the whole `Platformer2dSimHarness` population the
/// estimate was about — passed untouched, because that harness composes
/// `MinimalShellPlugins` and therefore activates a generation like the shipped
/// game does.
pub struct GenerationMechanics<'a> {
    characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    sheets: &'a ambition_sprite_sheet::character::sheets::AuthoredSheets,
    bosses: &'a ambition_boss_encounter::BossCatalog,
    forced_brains: Option<&'a ambition_characters::brain::AuthoredBrainOverride>,
    population_cap: Option<&'a ambition_characters::actor::AuthoredPopulationCap>,
}

impl<'a> GenerationMechanics<'a> {
    /// An activated generation's frozen values, and nothing else.
    ///
    /// ⭐ **THE ROAD THAT CANNOT BE WITHOUT A GENERATION: ACTIVATION, AND NOW
    /// EVERY LIVE REBUILD TOO.** The provider holds the exact frozen record it
    /// is building from and has no handles to fall back to — `Q121` removed
    /// them — so offering it a fallback parameter would be offering it a value
    /// it must never use. Once the fallback left the live roads, the same is
    /// true of them.
    pub fn of(generation: &'a SessionMechanics) -> Self {
        Self {
            characters: generation.characters.as_ref(),
            sheets: &generation.sheets,
            bosses: &generation.bosses,
            forced_brains: Some(&generation.forced_brains),
            population_cap: Some(&generation.population_cap),
        }
    }

    /// The generation a LIVE room rebuild must read — or `None`, which is a
    /// refusal.
    ///
    /// ⛔⛤ **A 2026-09-13 REVIEW NAMED THE COST OF GETTING THIS WRONG.**
    /// Rebuilding a live world out of whatever the App is holding now is how a
    /// session prepared under generation N walks through a door into N+1. The
    /// first repair distinguished *"this composition intentionally has no
    /// generation"* from *"this shell session should have one and it is
    /// missing"* with a `shell_routed` flag read off `SessionGatedSimulation`,
    /// and refused only the second.
    ///
    /// ⭐ **THE FLAG IS GONE BECAUSE THE CASE IT PROTECTED IS GONE.** It
    /// existed to keep the App-registry road open for direct-entry
    /// compositions; the composition ruling closed that road, so a live rebuild
    /// with no generation is a refusal whoever asks. A composition that means
    /// to rebuild rooms declares its construction inputs — a `SessionMechanics`
    /// it installs itself is the scoped fixture authority the ruling permits —
    /// and one that declares nothing is told no rather than handed the App.
    pub fn for_live_session(active: Option<&'a SessionMechanics>) -> Option<Self> {
        active.map(Self::of)
    }

    pub fn characters(
        &self,
    ) -> Option<&'a ambition_characters::prepared::PreparedCharacterRegistry> {
        self.characters
    }

    pub fn sheets(&self) -> &'a ambition_sprite_sheet::character::sheets::AuthoredSheets {
        self.sheets
    }

    pub fn bosses(&self) -> &'a ambition_boss_encounter::BossCatalog {
        self.bosses
    }

    /// The forced brain preset/profile this construction must use.
    pub fn forced_brains(&self) -> Option<&'a ambition_characters::brain::AuthoredBrainOverride> {
        self.forced_brains
    }

    /// The population ceiling this construction must spend.
    pub fn population_cap(&self) -> Option<&'a ambition_characters::actor::AuthoredPopulationCap> {
        self.population_cap
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
    shell_routed: bool,
    generation: Option<&SessionMechanics>,
    app: Option<&ambition_characters::perception::PerceptionExtentOverride>,
) -> Option<ambition_characters::perception::PerceptionExtentOverride> {
    match generation {
        Some(generation) => Some(generation.perception_extent),
        // ⛔⛤ **THE SAME LIVE-GENERATION CONTRACT AS
        // [`GenerationMechanics::for_live_session`], AND MY FIRST VERSION OF THIS
        // FUNCTION DID NOT HAVE IT.** A 2026-09-13 review found the hole: it fell
        // back to the App whenever the generation was absent, without asking
        // whether this composition OWES one. ⇒ In a shell session that lost its
        // mechanics, an actor constructed after that point would be given senses
        // derived from whatever the App holds now — a room part
        // generation-derivative and part current App, which is precisely the
        // fail-open shape the rest of this module exists to remove.
        //
        // ⚠ `None` is a REFUSAL, and its caller must treat it as one. A
        // direct-entry fixture is the composition entitled to state no
        // generation, and it still gets the App's value.
        None if shell_routed => None,
        None => Some(app.copied().unwrap_or_default()),
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

        let _ = (&sheets, &bosses);

        // ⛔ THE ASSERTION THE ARM IS FOR — and since the fallback was deleted
        // it is structural rather than a ranking: a live rebuild is given the
        // generation and has nowhere else to read.
        assert_eq!(
            health(GenerationMechanics::of(&generation).characters()),
            Some(9),
            "a room rebuilt inside an activated generation was built from a cast \
             the App published later and this session never activated",
        );

        // ⛔ AND AN ACTIVATED GENERATION THAT PUBLISHED NO CAST MEANS NO CAST —
        // not "fall through to the App's". Publishing none is a legal state, and
        // silently substituting a stranger's registry for it is the defect this
        // whole type removes, one layer down.
        assert_eq!(
            health(GenerationMechanics::of(&SessionMechanics::default()).characters()),
            None,
            "a castless generation was handed the App's cast",
        );

        // ⭐ THE CONTROL: a generation that DID publish this cast hands it over.
        // Without it the arms above would also pass if the type had stopped
        // carrying a cast at all.
        //
        // ⚠ It reaches construction by being INSIDE a `SessionMechanics`, which
        // is the whole remaining road. A caller that means to build the next
        // generation's mechanics assembles that value; it cannot hand five
        // loose registries to a room any more.
        assert_eq!(
            health(
                GenerationMechanics::of(&SessionMechanics {
                    characters: Some(app_now.clone()),
                    ..SessionMechanics::default()
                })
                .characters()
            ),
            Some(3),
            "a generation that published a cast did not hand it to construction",
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

        let _ = (&sheets, &bosses);
        let mechanics = GenerationMechanics::of(&generation);

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

        // ⭐ THE CONTROL: a generation prepared under a DIFFERENT cap hands that
        // one over, so the arms above are about READING THE GENERATION rather
        // than about the type having stopped carrying knobs at all.
        let capped_generation = SessionMechanics {
            population_cap: AuthoredPopulationCap::capped_at(99),
            ..SessionMechanics::default()
        };
        let other_generation = GenerationMechanics::of(&capped_generation);
        assert_eq!(
            other_generation.population_cap(),
            Some(&AuthoredPopulationCap::capped_at(99)),
            "the generation's own cap did not reach construction"
        );
    }

    /// ⛔⛤ **A LIVE ROOM REBUILD WITH NO GENERATION IS A REFUSAL, WHOEVER
    /// ASKS — AND IT USED TO DEPEND ON WHO WAS ASKING.**
    ///
    /// A 2026-09-13 review named the class: *"this composition intentionally
    /// has no generation"* and *"this shell session should have generation
    /// mechanics and they are missing"* were one `Option`, and the first
    /// repair told them apart with a `shell_routed` flag so the direct-entry
    /// half could keep reading the App.
    ///
    /// ⭐ **THE FLAG IS GONE BECAUSE THE RULING DELETED THE CASE IT
    /// PROTECTED.** *"No anonymous App-global fallback state returns"*
    /// (2026-09-19), so a composition that means to rebuild rooms installs a
    /// `SessionMechanics` of its own — the scoped fixture authority the same
    /// ruling permits — and one that installs nothing is told no.
    ///
    /// ⚠ **THE COST OF THAT WAS MEASURED BEFORE IT WAS PAID:** poisoning the
    /// branch and running the whole workspace reddened thirteen tests in two
    /// files, not the *"~90 test binaries"* the row had been costed at.
    #[test]
    fn a_live_room_rebuild_refuses_when_there_is_no_generation_to_rebuild_from() {
        assert!(
            GenerationMechanics::for_live_session(None).is_none(),
            "a live rebuild with no activated generation was handed something to \
             build out of anyway"
        );

        // ⭐ AND THE ORDINARY CASE: a session WITH its generation proceeds,
        // reading that generation. Without this arm the assertion above is
        // satisfied by a constructor that refuses everything.
        let generation = SessionMechanics::default();
        let mechanics = GenerationMechanics::for_live_session(Some(&generation))
            .expect("a session holding its generation may rebuild");
        assert!(
            mechanics.characters().is_none(),
            "the projection invented a cast the generation does not carry"
        );
    }

    /// ⛔⛤ **THE PERCEPTION READER OWES THE SAME CONTRACT AS THE ROOM ROADS, AND
    /// MY FIRST VERSION OF IT DID NOT (review, 2026-09-13).**
    ///
    /// `ensure_perception` attaches senses to bodies as they appear, so it
    /// consumes the generation at CONSUMPTION time rather than at plan time. With
    /// a plain App fallback, a shell session that lost its mechanics would give
    /// every later actor a sight range derived from whatever the App holds now —
    /// a room part generation-N derivative and part current App.
    ///
    /// ⭐ **THE THREE STATES ARE ASSERTED SEPARATELY** because a fallback that
    /// merely "usually wins" is the shape this module exists to remove.
    #[test]
    fn the_perception_extent_follows_the_generation_and_refuses_a_shell_without_one() {
        use ambition_characters::perception::PerceptionExtentOverride;

        let app_value = PerceptionExtentOverride(Some(ambition_platformer2d_core::Vec2::new(
            480.0, 480.0,
        )));
        let generation = SessionMechanics {
            perception_extent: PerceptionExtentOverride(Some(
                ambition_platformer2d_core::Vec2::new(120.0, 120.0),
            )),
            ..Default::default()
        };

        assert_eq!(
            perception_extent_for(true, Some(&generation), Some(&app_value)),
            Some(generation.perception_extent),
            "an activated generation's sight range lost to the App's, so actors \
             built later see a distance the identity never described"
        );
        assert_eq!(
            perception_extent_for(false, None, Some(&app_value)),
            Some(app_value),
            "a direct-entry composition lost the App's override, which is the one \
             composition entitled to supply it"
        );
        assert_eq!(
            perception_extent_for(true, None, Some(&app_value)),
            None,
            "a SHELL session with no generation was handed the App's sight range, \
             so a room comes out part generation-derivative and part App"
        );
    }
}
