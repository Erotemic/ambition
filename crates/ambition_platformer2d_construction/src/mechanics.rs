//! Mechanical inputs bound to one admitted content generation.
//!
//! Session activation and every later room transition, reset, or checkpoint
//! rebuild read the same frozen values. The content binding names the
//! generation; `SessionMechanics` holds the values that identity describes.
//! Do not keep separate mechanical registries for different rebuild paths.

/// Frozen construction values for one content generation.
///
/// The generation identity must cover these inputs, including authored
/// brain and population overrides. Character and brain registries are
/// bound through their source catalog fragments.
///
/// A live rebuild must not fall back to mutable App registries. This
/// resource and the active cast are removed at session retirement. A
/// reader may see neither after retirement in the same schedule run, so
/// it must accept an absent generation and skip its work.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct SessionMechanics {
    pub sprite_content: ambition_sprite_sheet::SpriteContent,
    // The prepared character cast is stored in `ActiveSessionCast`,
    // with the same lifetime as this resource. Read it through `SessionCast`.
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
    /// Perception bounds are mechanical inputs to generation identity.
    /// Rollback state on existing bodies does not bind bodies built later.
    pub perception_extent: ambition_characters::perception::PerceptionExtentOverride,
}

/// Generation-bound registries for one construction operation.
/// Use [`GenerationMechanics::of`] for an explicit prepared generation or
/// [`GenerationMechanics::for_live_session`] for a live rebuild that must refuse when
/// either the mechanics or the active cast is absent. There is no
/// fallback to process-global App registries.
pub struct GenerationMechanics<'a> {
    sprite_content: &'a ambition_sprite_sheet::SpriteContent,
    characters: Option<&'a ambition_characters::prepared::PreparedCharacterRegistry>,
    sheets: &'a ambition_sprite_sheet::character::sheets::AuthoredSheets,
    bosses: &'a ambition_boss_encounter::BossCatalog,
    forced_brains: Option<&'a ambition_characters::brain::AuthoredBrainOverride>,
    population_cap: Option<&'a ambition_characters::actor::AuthoredPopulationCap>,
}

impl<'a> GenerationMechanics<'a> {
    /// Use the frozen values of an explicitly selected generation.
    pub fn of(
        generation: &'a SessionMechanics,
        cast: &'a ambition_characters::prepared::ActiveSessionCast,
    ) -> Self {
        Self {
            sprite_content: &generation.sprite_content,
            characters: cast.cast(),
            sheets: &generation.sheets,
            bosses: &generation.bosses,
            forced_brains: Some(&generation.forced_brains),
            population_cap: Some(&generation.population_cap),
        }
    }

    /// Read both live generation inputs or refuse the rebuild.
    /// Never reconstruct a live room from current App registries.
    pub fn for_live_session(
        active: Option<&'a SessionMechanics>,
        cast: Option<&'a ambition_characters::prepared::ActiveSessionCast>,
    ) -> Option<Self> {
        // Inserted and removed together; one without the other is a session
        // that lost half its generation, and it is refused like a lost whole.
        Some(Self::of(active?, cast?))
    }

    pub fn sprite_content(&self) -> &'a ambition_sprite_sheet::SpriteContent { self.sprite_content }

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

/// The perception viewport override a sighted body sees with.
///
/// ⭐ **A FREE FUNCTION RATHER THAN A `GenerationMechanics` METHOD, because its
/// caller is a SYSTEM and not a construction road.** The brain tick reads the
/// extent when a body decides (`SenseExtent`). It holds no plan, no room and no
/// `GenerationMechanics`, so what it needs is the same ranking expressed where
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
        let generation = SessionMechanics::default();
        let frozen = ambition_characters::prepared::ActiveSessionCast(Some(cast(9)));
        // What a later reload published into the App, with no admitted
        // transition for THIS session.
        let app_now = cast(3);

        // ⭐ THE PREMISE FIRST: the two casts must actually differ, or every
        // assertion below is satisfied by a resolver that returns anything.
        assert_eq!(health(frozen.cast()), Some(9));
        assert_eq!(health(Some(&app_now)), Some(3));

        let sheets = ambition_sprite_sheet::character::sheets::AuthoredSheets::default();
        let bosses = ambition_boss_encounter::BossCatalog::default();

        let _ = (&sheets, &bosses);

        // ⛔ THE ASSERTION THE ARM IS FOR — and since the fallback was deleted
        // it is structural rather than a ranking: a live rebuild is given the
        // generation and has nowhere else to read.
        assert_eq!(
            health(GenerationMechanics::of(&generation, &frozen).characters()),
            Some(9),
            "a room rebuilt inside an activated generation was built from a cast \
             the App published later and this session never activated",
        );

        // ⛔ AND AN ACTIVATED GENERATION THAT PUBLISHED NO CAST MEANS NO CAST —
        // not "fall through to the App's". Publishing none is a legal state, and
        // silently substituting a stranger's registry for it is the defect this
        // whole type removes, one layer down.
        assert_eq!(
            health(
                GenerationMechanics::of(
                    &SessionMechanics::default(),
                    &ambition_characters::prepared::ActiveSessionCast(None),
                )
                .characters()
            ),
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
                GenerationMechanics::of(
                    &SessionMechanics::default(),
                    &ambition_characters::prepared::ActiveSessionCast(Some(app_now.clone())),
                )
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
        // ⚠ THERE IS NO "App" SIDE TO SET ANY MORE, and that is the point of
        // the arms below reading as tautologies now: the only constructor that
        // took loose registries is deleted, so a generation's knobs cannot
        // lose to anything. The arms stay because they name WHICH value a
        // construction spends, and the control below gives them their teeth.
        let no_cast = ambition_characters::prepared::ActiveSessionCast(None);
        let mechanics = GenerationMechanics::of(&generation, &no_cast);

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
        let other_generation = GenerationMechanics::of(&capped_generation, &no_cast);
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
            GenerationMechanics::for_live_session(None, None).is_none(),
            "a live rebuild with no activated generation was handed something to \
             build out of anyway"
        );

        // ⭐ AND THE ORDINARY CASE: a session WITH its generation proceeds,
        // reading that generation. Without this arm the assertion above is
        // satisfied by a constructor that refuses everything.
        let generation = SessionMechanics::default();
        let frozen = ambition_characters::prepared::ActiveSessionCast(None);
        assert!(
            GenerationMechanics::for_live_session(Some(&generation), None).is_none(),
            "a session that kept its mechanics and lost its cast was handed a rebuild"
        );
        let mechanics = GenerationMechanics::for_live_session(Some(&generation), Some(&frozen))
            .expect("a session holding its generation may rebuild");
        assert!(
            mechanics.characters().is_none(),
            "the projection invented a cast the generation does not carry"
        );
    }

    /// ⛔⛤ **THE PERCEPTION READER OWES THE SAME CONTRACT AS THE ROOM ROADS, AND
    /// MY FIRST VERSION OF IT DID NOT (review, 2026-09-13).**
    ///
    /// The brain tick reads the extent when a body decides, so it consumes the
    /// generation at CONSUMPTION time rather than at plan time. With
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
