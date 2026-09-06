//! Authored encounter data types (serde RON). `EncounterSpec` is the whole
//! encounter: ordered `EncounterWaveSpec`s of `EncounterMobSpec`s, the trigger
//! AABB, camera zoom, intro timing, optional `LockWallSpec`, music track, and
//! reward. The lib's `loading.rs` builds these from LDtk + the content wave
//! book; the `state.rs` machine consumes them. Pure data — no behavior here.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use ambition_platformer2d_core as ae;

/// App-owned authored encounter wave timelines keyed by trigger id.
///
/// The LDtk adapter uses these before falling back to marker-derived enemy spawns.
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
    // Each App owns its encounter wave book independently.
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
// Refuse unknown authored fields so misspelled mechanics do not disappear during decode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMobSpec {
    /// `CharacterBrain::Custom(kind)` payload — what this mob DOES, i.e.
    /// which controller its body is driven by.
    ///
    /// This selects the controller only. The body comes from
    /// [`Self::character`].
    pub kind: String,
    /// WHICH CHARACTER THIS MOB IS — a catalog character id, and the thing
    /// its body is built from.
    ///
    /// `spawn_encounter_mob` resolves this id against the prepared cast and
    /// builds the body from that character. A named character that cannot be
    /// built is an error rather than an archetype fallback.
    ///
    /// `None` is valid because encounters without wave-book entries are assembled
    /// from the level's authored `EnemySpawn` markers.
    #[serde(default)]
    pub character: Option<String>,
    /// Spawn position in active-area-local coordinates (the mob's
    /// center, not top-left).
    pub spawn: [f32; 2],
    /// Mob hitbox size; defaults to a sensible per-archetype value.
    pub size: [f32; 2],
    /// `0.0` means "with the wave".
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

    /// Separate Apps own independent encounter wave books.
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
