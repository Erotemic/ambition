//! **Two characters authored to fight.** (queue L7)
//!
//! Couch versus works: two controllers, two bodies, each driving only its own.
//! It is two people walking into each other, because neither `mary_o` nor
//! `sanic` authors a move list. Sanic's catalog row says why in as many words —
//! *"a peaceful speedster: the momentum ride + ball dash ARE the kit; no combat
//! moveset"* — and Mary-O must play like SMB1, where a jab is a flourish
//! nobody asked for. Bolting attacks onto either would be authoring against
//! their design to make a different mode work.
//!
//! So the arena gets fighters of its own, and they are the first production
//! callers of [`CharacterDefinition::with_moveset`] — the seam C3 landed and
//! recorded as "waiting on its first real fighter".
//!
//! ## Why two definitions and one archetype
//!
//! A fighting game is characters who share a grammar and differ in numbers: the
//! jab is fast and weak, the smash is slow and heavy, and WHICH fast and which
//! heavy is the character. [`duelist_moveset`] is the grammar and
//! [`DuelistNumbers`] is the character, so adding a third fighter is a struct
//! literal rather than a fifth copy of four `MoveSpec`s — and the two that exist
//! cannot silently drift into different move sets.
//!
//! ## Art is SHARED, not authored here
//!
//! Both fighters name an existing sheet. Characters are shared by id and a
//! definition names its art rather than owning it, which is exactly what lets a
//! versus-only fighter exist without anybody drawing anything. Swings fall back
//! to `idle` on a sheet with no attack row — visibly plain, honestly plain, and
//! not a placeholder pretending to be art.

use ambition::actors::character_runtime::CharacterDefinition;
use ambition::combat::moveset::{simple_melee, SimpleMeleeParams};
use ambition::entity_catalog::{MoveGates, MovesetContract};

/// What makes one duelist different from another.
///
/// Deliberately four numbers and a reach, not a full `MoveSpec` budget: the
/// point of an archetype is that the shape is shared, and a knob that only one
/// fighter ever sets belongs on that fighter's move, not here.
#[derive(Clone, Copy, Debug)]
pub struct DuelistNumbers {
    /// The fast poke. Low damage, low commitment — what you press when unsure.
    pub jab_damage: i32,
    /// The committed hit. Slow enough to be punished, heavy enough to be worth
    /// the risk; this is the number that decides how a fighter FEELS.
    pub smash_damage: i32,
    /// How far every swing reaches, in pixels. A short-reach fighter has to
    /// close distance, which is the whole of its game plan.
    pub reach_px: f32,
    /// Seconds of windup on the smash. The tell.
    pub smash_windup_s: f32,
}

/// The grammar every duelist speaks: a jab, an up-tilt, a down-tilt, and a
/// forward smash.
///
/// Four moves is the smallest set that makes a FIGHT rather than a button:
/// something safe, something that covers above, something that covers below,
/// and something that ends the round. Anything less and both players press the
/// same button forever.
pub fn duelist_moveset(numbers: DuelistNumbers) -> MovesetContract {
    let jab = SimpleMeleeParams {
        windup_s: 0.06,
        active_s: 0.05,
        recover_s: 0.10,
        damage: numbers.jab_damage,
        reach_px: numbers.reach_px,
        knockback: 90.0,
        ..Default::default()
    };
    let smash = SimpleMeleeParams {
        windup_s: numbers.smash_windup_s,
        active_s: 0.07,
        recover_s: 0.28,
        damage: numbers.smash_damage,
        reach_px: numbers.reach_px * 1.35,
        knockback: 420.0,
        swing_vfx: Some("shockwave".to_string()),
        ..Default::default()
    };
    let up_tilt = SimpleMeleeParams {
        windup_s: 0.08,
        active_s: 0.06,
        recover_s: 0.16,
        damage: numbers.jab_damage + 1,
        reach_px: numbers.reach_px * 0.8,
        knockback: 260.0,
        ..Default::default()
    };
    let down_tilt = SimpleMeleeParams {
        windup_s: 0.07,
        active_s: 0.05,
        recover_s: 0.18,
        damage: numbers.jab_damage,
        reach_px: numbers.reach_px * 0.9,
        knockback: 140.0,
        ..Default::default()
    };

    let mut moves = Vec::new();
    let mut verbs = std::collections::BTreeMap::new();
    for (verb, id, params, grounded_only) in [
        ("attack", "jab", jab, false),
        ("attack_forward", "smash_forward", smash, true),
        ("attack_up", "tilt_up", up_tilt, false),
        ("attack_down", "tilt_down", down_tilt, true),
    ] {
        let mut spec = simple_melee(&params);
        spec.id = id.to_string();
        // A grounded-only move is SKIPPED for an airborne body rather than
        // refused, so the directional chain falls through to the plain `attack`.
        // That is why the smash and the down-tilt can be grounded without an
        // airborne fighter losing the ability to press the button.
        spec.gates = MoveGates {
            grounded: grounded_only.then_some(true),
        };
        verbs.insert(verb.to_string(), id.to_string());
        moves.push(spec);
    }
    MovesetContract { verbs, moves }
}

/// The arena's two fighters, in roster order.
///
/// One from each side of the crossover the versus stage was built to show: the
/// stage's whole reason for living in the app rather than a provider is that it
/// is the only composition where more than one cast exists.
pub fn duelists() -> [CharacterDefinition; 2] {
    [
        // A reach fighter. Longer swings, a slower smash — wins by keeping the
        // other fighter at arm's length and punishing the approach.
        CharacterDefinition::new("arena_duelist_long", "Long Guard", VERSUS_PROVIDER)
            .with_sheet("robot")
            // A round has to last long enough to be a round. At these damage
            // numbers 60 HP is roughly a dozen committed hits or thirty pokes,
            // which is the length that makes the jab/smash trade a DECISION
            // rather than a coin flip — one exchange deciding the match is not a
            // fighting game, it is a duel of who pressed first.
            .with_health(60, 1.0)
            .with_moveset(duelist_moveset(DuelistNumbers {
                jab_damage: 2,
                smash_damage: 9,
                reach_px: 46.0,
                smash_windup_s: 0.26,
            })),
        // A rushdown fighter. Shorter reach, a faster smash — has to get inside
        // to do anything, and is rewarded for being there.
        CharacterDefinition::new("arena_duelist_close", "Close Guard", VERSUS_PROVIDER)
            .with_sheet("super_mary_o")
            // Slightly frailer than the long guard, to pay for getting to swing
            // faster. Same round length, different price.
            .with_health(52, 0.9)
            .with_moveset(duelist_moveset(DuelistNumbers {
                jab_damage: 3,
                smash_damage: 7,
                reach_px: 32.0,
                smash_windup_s: 0.17,
            })),
    ]
}

/// `Vitals` is a plain public field on `CharacterDefinition`, so this is the
/// one place the builder chain breaks. Kept as a private extension rather than
/// inline so the two fighters read as two rows of numbers.
trait WithHealth {
    fn with_health(self, max_health: i32, mass: f32) -> Self;
}

impl WithHealth for CharacterDefinition {
    fn with_health(mut self, max_health: i32, mass: f32) -> Self {
        self.vitals = ambition::actors::character_runtime::Vitals { max_health, mass };
        self
    }
}

/// The catalog rows the two fighters need.
///
/// A `CharacterDefinition` is not a catalog row, and both are required for
/// different reasons: preparation refuses an experience whose starting
/// character has no ROW (the row is the character's identity — art, movement,
/// default brain), and the hand-authored moveset lives on the DEFINITION, which
/// the persona derive prefers over anything the row implies. So the row says
/// who they are and the definition says what they do, which is the split the
/// two seams were built with.
///
/// `tier: Basement` keeps them out of the Hall of Characters. They are arena
/// fighters; a gallery of playable characters is a different claim and one they
/// have not earned — they have no art of their own.
pub const VERSUS_CATALOG_RON: &str = r#"(
    brain_presets: { "stand_still": StandStill },
    action_set_presets: {
        // Empty ON PURPOSE. Every attack these fighters have is authored on
        // their `CharacterDefinition` moveset, which the persona derive prefers
        // over the catalog-derived one — so an `ActionSet` melee here would be
        // a second opinion that never wins and would read as the real kit.
        "authored_elsewhere": (
            move_style: Walk,
            melee: None,
            ranged: None,
            special: None,
        ),
    },
    characters: {
        "arena_duelist_long": (
            sprite_tuning: Some((collision_scale: 2.1, frame_sample_inset: 1)),
            display_name: "Long Guard",
            spritesheet: "sprites/robot_spritesheet.png",
            manifest: "sprites/robot_spritesheet.ron",
            tier: Basement,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "authored_elsewhere",
            tags: ["player", "versus"],
        ),
        "arena_duelist_close": (
            display_name: "Close Guard",
            spritesheet: "sprites/super_mary_o_spritesheet.png",
            manifest: "sprites/super_mary_o_spritesheet.ron",
            tier: Basement,
            body_kind: Standard,
            composition: None,
            default_brain: "stand_still",
            default_action_set: "authored_elsewhere",
            tags: ["player", "versus"],
        ),
    },
)"#;

/// The provider these fighters are attributed to.
///
/// The versus experience, not the host and not either demo: a cue or a swing
/// these fighters make is the ARENA's, which is what keeps presentation source
/// attribution honest for a crossover stage.
pub const VERSUS_PROVIDER: &str = super::versus::VERSUS_EXPERIENCE;

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::entity_catalog::AttackDir;

    #[test]
    fn a_duelist_answers_every_direction_and_the_smash_is_the_heavy_one() {
        let moveset = duelist_moveset(DuelistNumbers {
            jab_damage: 2,
            smash_damage: 9,
            reach_px: 40.0,
            smash_windup_s: 0.25,
        });
        for dir in [
            AttackDir::Neutral,
            AttackDir::Forward,
            AttackDir::Up,
            AttackDir::Down,
            AttackDir::Back,
        ] {
            assert!(
                moveset
                    .move_for_directional_verb("attack", dir, true)
                    .is_some(),
                "a grounded duelist pressed attack while holding {dir:?} and got \
                 nothing — a direction with no answer is a dead button"
            );
            assert!(
                moveset
                    .move_for_directional_verb("attack", dir, false)
                    .is_some(),
                "an AIRBORNE duelist got nothing for {dir:?}: the grounded-only \
                 moves must fall through to the plain jab, not refuse"
            );
        }

        // The airborne fighter falls through to the jab rather than getting the
        // grounded smash. This is the property the gates exist for, and asserting
        // only "something came back" above would pass while they did nothing.
        assert_eq!(
            moveset
                .move_for_directional_verb("attack", AttackDir::Forward, false)
                .map(|mv| mv.id.as_str()),
            Some("jab")
        );
        assert_eq!(
            moveset
                .move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|mv| mv.id.as_str()),
            Some("smash_forward")
        );
    }

    #[test]
    fn the_two_fighters_differ_in_numbers_and_not_in_grammar() {
        let [long, close] = duelists();
        let (a, b) = (
            long.moveset.as_ref().expect("the long guard has a moveset"),
            close
                .moveset
                .as_ref()
                .expect("the close guard has a moveset"),
        );
        assert_eq!(
            a.verbs, b.verbs,
            "the two fighters must answer the same buttons: a fighting game where \
             one character has a button the other does not is two games"
        );
        let heavy = |m: &MovesetContract| {
            m.move_by_id("smash_forward")
                .and_then(|mv| mv.windows.iter().find_map(|w| w.volumes.first()))
                .map(|v| v.damage)
                .expect("the smash lands a volume")
        };
        assert_ne!(
            heavy(a),
            heavy(b),
            "the two fighters are numerically identical, so the roster is one \
             character wearing two names"
        );
    }
}
