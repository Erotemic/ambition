//! Authored encounter data types (serde RON). `EncounterSpec` is the whole
//! encounter: ordered `EncounterWaveSpec`s of `EncounterMobSpec`s, the trigger
//! AABB, camera zoom, intro timing, optional `LockWallSpec`, music track, and
//! reward. The lib's `loading.rs` builds these from LDtk + the content wave
//! book; the `state.rs` machine consumes them. Pure data — no behavior here.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use ambition_platformer2d_core as ae;

/// **Authored encounter wave timelines, keyed by trigger id — an App RESOURCE.**
///
/// The LDtk adapter asks for an authored multi-wave sequence before falling back
/// to marker-derived enemy spawns. Keeping the vocabulary here rather than in an
/// actor-crate facade is unchanged; what changed is WHO OWNS THE VALUE.
///
/// ⛔ **it was a process-global `OnceLock` and "first install wins".** A second,
/// DIFFERENT wave book was swallowed with an error log, so the first provider in
/// a process defined encounters for every App built after it — two games, or a
/// game and a tool, silently sharing one game's content. The seam looked
/// provider-local and was not.
///
/// ⭐ **and unlike the item catalog, this one was CHEAP to fix.** That row says
/// the two are "the same shape" and they are not: `Item::display_name` and its
/// siblings are methods on a plain enum returning `&'static str` — a borrow only
/// possible BECAUSE the storage is global — consumed at 59 sites. This book has
/// ONE reader, it already returns an owned `Vec`, and that reader's production
/// caller is a Bevy system with a `World` in hand. Measuring the read surface is
/// what separated them; assuming the shape would have left this one global too.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
pub struct EncounterWaveBook(pub HashMap<String, Vec<EncounterWaveSpec>>);

impl EncounterWaveBook {
    /// The authored timeline for a trigger id, if this book has one.
    pub fn waves(&self, id: &str) -> Option<Vec<EncounterWaveSpec>> {
        self.0.get(id).cloned()
    }
}

/// Install authored encounter wave timelines. Content crates call this during
/// plugin build, before the LDtk adapter populates the live encounter registry.
pub fn install_encounter_waves(
    app: &mut bevy::prelude::App,
    book: HashMap<String, Vec<EncounterWaveSpec>>,
) {
    // ⭐ **the App owns it now**, so two Apps in one process carry two books and
    // neither has to win. The loud "a SECOND, DIFFERENT wave book was installed
    // and was IGNORED" error this replaced was the best a process-global could
    // do: report the collision it could not avoid.
    app.insert_resource(EncounterWaveBook(book));
}

/// Look up an authored multi-wave timeline for a trigger id.
///
/// `None` means the adapter should fall back to one wave assembled from the
/// level's own spawn markers.
pub fn authored_encounter_waves(
    book: Option<&EncounterWaveBook>,
    id: &str,
) -> Option<Vec<EncounterWaveSpec>> {
    book.and_then(|book| book.waves(id))
}

/// One mob to spawn during a wave.
// ⛔ **`deny_unknown_fields` is the CONTRACT, not a nicety.**
// `ContentSchemaHandler::check`'s own doc: *"a handler MUST report an authored
// field it does not consume … rolling your own field walk and forgetting is how
// a typo becomes a mechanic that silently never fires."* This type had no such
// guard, and an audit measured the consequence by authoring
// `favourite_snack: "worms"` into a real file: the pack compiled CLEAN, and the
// field reached neither the runtime nor the fingerprint. A misspelled tuning
// value is exactly that shape, and it looks identical to authoring nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMobSpec {
    /// `CharacterBrain::Custom(kind)` payload — picks the archetype
    /// (`small_skitter`, `medium_striker`, `large_brute`, ...).
    pub kind: String,
    /// **WHAT IT LOOKS LIKE** — a catalog character id.
    ///
    /// ⚠ **read this against `EnemySpawnSpec` next door**
    /// (`ambition_platformer2d_world::rooms`), because the two are the same
    /// question on two spawn paths and the near-identical names would otherwise
    /// invite a guess. There, `brain` is *what it DOES* and `character_id` is
    /// *what it LOOKS LIKE*, reached only through `art_identity()`. **This field
    /// means exactly that and no more**: the sheet, the sprite-derived collision
    /// box, hurt feedback, the catalog bark pool, and the display label.
    ///
    /// ⛔ **it does NOT pick up the catalog's `default_brain` or
    /// `default_action_set`.** `kind` above answers *what it DOES*, through the
    /// archetype roster, and it keeps answering it.
    ///
    /// ⭐ **THE OPEN QUESTION THIS PARAGRAPH USED TO NAME IS ANSWERED** (Jon,
    /// 2026-08-10, D73): an enemy IS a character. A character is a reusable
    /// authored template, and presentation is a projection of it. The paragraph
    /// above therefore describes a TRANSITIONAL state, not a design:
    /// `EnemySpawnSpec` next door has already split the two questions —
    /// `gameplay_character_id()` answers *which character*, and
    /// `presentation_identity()` answers *which sheet*, with a display-name
    /// fallback kept for pixels alone. **This path has not been migrated yet.**
    /// When it is, this field becomes the character the mob instantiates and
    /// `kind` stops deciding the body.
    ///
    /// See `docs/planning/character-template-architecture-2026-08-10.md`.
    ///
    /// ⭐ **three fields, three questions** — and two of them were one field
    /// until 2026-08-09. The spawner passed the wave director's minted
    /// `encounter:<id>:w<n>:<n>` as the actor's name as well as its id, so the
    /// art lookup asked the catalog for a character called
    /// `encounter:goblin_encounter:w0:1`, no sheet resolved, and every mob in
    /// the goblin lab drew the unclaimed-body placeholder.
    ///
    /// ⚠ `Option`, and that is not laziness — the same call `EnemySpawnSpec`
    /// made. An encounter with no entry in the wave book is assembled from its
    /// level's LDtk `EnemySpawn` markers, so `None` must keep resolving exactly
    /// as it did.
    #[serde(default)]
    pub character: Option<String>,
    /// Spawn position in active-area-local coordinates (the mob's
    /// center, not top-left).
    pub spawn: [f32; 2],
    /// Mob hitbox size; defaults to a sensible per-archetype value.
    pub size: [f32; 2],
    /// Seconds after the wave starts before this mob spawns. `0.0`
    /// means "with the wave".
    pub delay: f32,
}

impl EncounterMobSpec {
    pub fn new(kind: impl Into<String>, spawn: [f32; 2]) -> Self {
        Self {
            kind: kind.into(),
            character: None,
            spawn,
            size: [22.0, 38.0],
            delay: 0.0,
        }
    }

    /// Name the catalog character this mob wears.
    pub fn with_character(mut self, character: impl Into<String>) -> Self {
        self.character = Some(character.into());
        self
    }

    pub fn with_size(mut self, size: [f32; 2]) -> Self {
        self.size = size;
        self
    }

    pub fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay;
        self
    }
}

/// One wave of mobs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterWaveSpec {
    pub label: String,
    pub mobs: Vec<EncounterMobSpec>,
}

/// Marker for an encounter-spawned solid wall (the "lock wall" that
/// appears in the doorway while the encounter is Active and is
/// removed when the encounter ends).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LockWallSpec {
    pub min: [f32; 2],
    pub size: [f32; 2],
}

impl LockWallSpec {
    pub fn aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(
            ae::Vec2::new(self.min[0], self.min[1]),
            ae::Vec2::new(self.size[0], self.size[1]),
        )
    }
}

/// Whole encounter authored data: ordered list of waves plus the
/// activation AABB, intro/music settings, optional lock wall, and the
/// camera-zoom factor to apply while the encounter is active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterSpec {
    pub id: String,
    pub waves: Vec<EncounterWaveSpec>,
    /// AABB in active-area-local coordinates that triggers the
    /// encounter when the player enters.
    pub trigger_min: [f32; 2],
    pub trigger_size: [f32; 2],
    /// Camera zoom multiplier while the encounter is active. `1.0`
    /// disables the zoom-out.
    pub camera_zoom: f32,
    /// Optional dynamic wall that spawns when the encounter goes
    /// Active and is removed when it leaves Active.
    pub lock_wall: Option<LockWallSpec>,
    /// Seconds the encounter spends in `Starting` (intro) before the
    /// first wave kicks off. The camera + lock + music change happen
    /// at the start of `Starting`; enemies don't spawn until `Active`.
    pub intro_seconds: f32,
    /// Music track id to play while the encounter is Active. Empty
    /// disables the music swap.
    pub music_track: String,
    /// Reward dropped in the encounter's chest when it clears. Authored
    /// per-encounter instead of the old hardcoded `Health { amount: 2 }`
    /// at the chest spawn site, so a fight can grant currency, an
    /// ability, a story flag, or a bigger heal. Defaults to the legacy
    /// small heal for back-compat / specs that don't set it.
    #[serde(default = "default_encounter_reward")]
    pub reward: ambition_interaction::PickupKind,
}

/// Legacy default encounter reward (small heal) used when a spec omits
/// `reward`.
pub fn default_encounter_reward() -> ambition_interaction::PickupKind {
    ambition_interaction::PickupKind::Health { amount: 2 }
}

impl EncounterSpec {
    pub fn trigger_aabb(&self) -> ae::Aabb {
        ae::aabb_from_min_size(
            ae::Vec2::new(self.trigger_min[0], self.trigger_min[1]),
            ae::Vec2::new(self.trigger_size[0], self.trigger_size[1]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_interaction::PickupKind;

    const BASE: &str = r#"(
        id: "t", waves: [], trigger_min: (0.0, 0.0), trigger_size: (10.0, 10.0),
        camera_zoom: 1.2, lock_wall: None, intro_seconds: 1.0, music_track: """#;

    #[test]
    fn reward_defaults_to_small_heal_when_omitted() {
        let ron_text = format!("{BASE})");
        let spec: EncounterSpec = ron::from_str(&ron_text).expect("parse without reward");
        assert_eq!(spec.reward, PickupKind::Health { amount: 2 });
    }

    #[test]
    fn reward_round_trips_an_authored_kind() {
        let ron_text = format!("{BASE}, reward: Currency(amount: 50))");
        let spec: EncounterSpec = ron::from_str(&ron_text).expect("parse with reward");
        assert_eq!(spec.reward, PickupKind::Currency { amount: 50 });
    }

    #[test]
    fn lock_wall_aabb_is_min_plus_size() {
        let lw = LockWallSpec {
            min: [10.0, 20.0],
            size: [30.0, 40.0],
        };
        let bb = lw.aabb();
        assert_eq!(bb.min, ae::Vec2::new(10.0, 20.0));
        assert_eq!(bb.max, ae::Vec2::new(40.0, 60.0));
    }
}

#[cfg(test)]
mod wave_book_tests {
    use super::*;

    fn book(trigger: &str) -> EncounterWaveBook {
        let mut map = HashMap::new();
        map.insert(
            trigger.to_string(),
            vec![EncounterWaveSpec {
                label: "only".into(),
                mobs: Vec::new(),
            }],
        );
        EncounterWaveBook(map)
    }

    /// **Two Apps in one process carry two different wave books.**
    ///
    /// ⛔ **this test could not have been written before.** The book was a
    /// process-global `OnceLock` whose contract was "first install wins": a
    /// second, DIFFERENT book was rejected with an error log, so the first
    /// provider built in a process defined encounters for every App after it.
    /// Two games, or a game and a tool, in one binary silently shared one game's
    /// content — and a test binary builds Apps constantly.
    ///
    /// ⭐ **and unlike the item catalog, this was CHEAP.** The carried row calls
    /// them "the same shape" and they are not: `Item::display_name` and its
    /// siblings are methods on a plain enum returning `&'static str` — a borrow
    /// only possible BECAUSE the storage is global — consumed at 59 sites. This
    /// book had ONE reader, already returned an owned `Vec`, and its production
    /// caller is a Bevy system. Measuring the read surface is what separated
    /// them; assuming the shape would have left this global too.
    #[test]
    fn two_apps_in_one_process_carry_different_wave_books() {
        let mut first = bevy::prelude::App::new();
        install_encounter_waves(&mut first, book("goblins").0);
        let mut second = bevy::prelude::App::new();
        install_encounter_waves(&mut second, book("wolves").0);

        let of = |app: &bevy::prelude::App, id: &str| {
            authored_encounter_waves(app.world().get_resource::<EncounterWaveBook>(), id).is_some()
        };
        assert!(of(&first, "goblins"), "the first App has its own book");
        assert!(of(&second, "wolves"), "and the second has its own");
        assert!(
            !of(&first, "wolves") && !of(&second, "goblins"),
            "neither App can see the other's encounters — which is exactly what \
             the process-global made impossible, and it reported the collision \
             with an error log because that was the best it could do"
        );
    }

    /// A composition with no authored encounters is an empty answer, not a
    /// panic — the fallback to a level's own spawn markers depends on it.
    #[test]
    fn no_book_at_all_falls_back_rather_than_failing() {
        assert!(authored_encounter_waves(None, "goblins").is_none());
    }
}
