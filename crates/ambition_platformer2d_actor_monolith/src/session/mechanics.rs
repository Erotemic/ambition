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
/// ⛔ **WHAT IS NOT IN HERE YET, said plainly rather than left to be assumed:**
/// `CharacterCatalog`, `BrainProfileRegistry`, `AuthoredBrainOverride` and
/// `AuthoredPopulationCap` are also construction inputs and are still read from
/// the App on every road. The first two are not in `PreparedContentIdentity`
/// either; the last two are `Q126`. ⇒ **A generation owns the three registries
/// its identity binds, and no more than that** — extending the ownership without
/// extending the identity would make this type claim a freeze the fingerprint
/// cannot corroborate.
#[derive(bevy::prelude::Resource, Clone, Debug, Default)]
pub struct SessionMechanics {
    /// `None` where the composition published no cast — a real state, and not a
    /// missing value.
    pub characters: Option<ambition_characters::prepared::PreparedCharacterRegistry>,
    pub sheets: ambition_sprite_sheet::character::sheets::AuthoredSheets,
    pub bosses: ambition_boss_encounter::BossCatalog,
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
}
