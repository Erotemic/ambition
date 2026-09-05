//! Versus-specific fighters with a shared duelist move grammar.
//!
//! Their definitions vary combat numbers while sharing the same move structure.
//! They reference existing sprite sheets; missing attack rows use the normal
//! animation fallback path.

use ambition_platformer2d::character::CharacterDefinition;
use ambition_platformer2d::characters::brain::ActionSet;
use ambition_platformer2d::combat::hurtbox_resolution::POSE_HITSTUN;
use ambition_platformer2d::combat::moveset::{simple_melee, SimpleMeleeParams};
use ambition_platformer2d::entity_catalog::{
    HurtboxDoc, HurtboxKeyframe, HurtboxTimeline, HurtboxVolume, VolumeShape,
};
use ambition_platformer2d::entity_catalog::{MoveGates, MovesetContract, RecoveryUse};

/// Per-character parameters for the shared duelist moveset.
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

/// Build the shared jab, directional-tilt, and forward-smash grammar.
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
            // These duelists' moves resolve from EITHER stance (only two are
            // grounded-gated at all), so this table cannot make the posture
            // statement `SmashRepertoire::GROUNDED` makes. Left at the engine
            // default, which is "the body keeps steering".
            roots_steering: false,
            recovery_route: None,
            // Free: nothing in the duel arena fills a meter, so a price here
            // would buy a fixed number of uses and no way to earn another.
            meter_cost: 0.0,
            // Not a recovery: the duel arena authors no up-B slot, so nothing
            // here spends the once-per-airtime budget.
            recovery: RecoveryUse::None,
            // A posture says nothing about being HELD. Whether a move refuses to
            // start from a saddle is that move's own statement -- `call_the_shark`
            // makes it -- and a stance default answering for every move would be
            // this file deciding a question it cannot see.
            forbidden_while_held: false,
        };
        verbs.insert(verb.to_string(), id.to_string());
        moves.push(spec);
    }
    MovesetContract { verbs, moves }
}

/// What a fighter is HITTABLE through, and when it changes.
///
/// The `HurtboxDoc` seam has existed since A7 and no character authored one, so
/// every body in the game was damageable through a box derived from its sprite —
/// which is a reasonable default and says nothing about what a fighter is doing.
/// The versus duelists are where that stops being acceptable: a fighting game
/// whose smash costs nothing to whiff is a game where you always smash.
///
/// Three rules, and each is a decision rather than a number:
///
/// * Standing is a torso — narrower than the sprite quad, because a swing
///   that clips the empty air beside a fighter should miss. The old
///   sprite-derived box made every character as wide as its widest frame.
/// * Hitstun is BIGGER. A fighter already being hit is easier to keep
///   hitting, which is what makes a combo a combo instead of a coincidence.
/// * A committed smash EXTENDS it for the length of the move. This is the
///   whole point of a per-move timeline: the fighter leans in, and the reach
///   that makes the smash dangerous is also what makes whiffing it punishable.
///   Jab and the tilts do not extend — they are the safe options, and they are
///   safe because they leave the silhouette alone.
///
/// Sized against the 30×48 collision box the arena's fighters carry, so these
/// are body-relative numbers rather than sprite-relative ones: the doc is a
/// statement about the FIGHTER, and it stays true if somebody redraws the art.
fn duelist_hurtboxes(numbers: DuelistNumbers) -> HurtboxDoc {
    let torso = |half_w: f32, half_h: f32, offset_x: f32| HurtboxTimeline {
        keyframes: vec![HurtboxKeyframe {
            at_s: 0.0,
            volumes: vec![HurtboxVolume {
                shape: VolumeShape::Rect {
                    offset: (offset_x, 0.0),
                    half_extents: (half_w, half_h),
                },
            }],
        }],
    };
    // The smash leans a fighter forward by a fraction of its own reach, so the
    // long guard commits further than the close guard — the same knob that makes
    // its smash dangerous makes its whiff worse, which is the trade the two
    // archetypes exist to express.
    let lean = numbers.reach_px * 0.18;
    HurtboxDoc {
        default: Some(torso(11.0, 22.0, 0.0)),
        poses: std::collections::BTreeMap::from([(
            POSE_HITSTUN.to_string(),
            torso(13.0, 24.0, 0.0),
        )]),
        moves: std::collections::BTreeMap::from([(
            "smash_forward".to_string(),
            torso(11.0 + lean * 0.5, 22.0, lean * 0.5),
        )]),
    }
}

/// The arena's two fighters, in roster order.
///
/// One from each side of the crossover the versus stage was built to show: the
/// stage's whole reason for living in the app rather than a provider is that it
/// is the only composition where more than one cast exists.
/// The two archetypes' numbers, named once. The moveset and the hurtbox doc are
/// both derived from them, so a fighter cannot swing with one character's reach
/// and lean with another's.
pub const LONG_GUARD: DuelistNumbers = DuelistNumbers {
    jab_damage: 2,
    smash_damage: 9,
    reach_px: 46.0,
    smash_windup_s: 0.26,
};
pub const CLOSE_GUARD: DuelistNumbers = DuelistNumbers {
    jab_damage: 3,
    smash_damage: 7,
    reach_px: 32.0,
    smash_windup_s: 0.17,
};

/// WHAT A DUELIST'S BODY CAN DO, authored on the fighter rather than inherited
/// from the stage's ceiling.
///
/// this is the kit both duelists ALREADY had; naming it changes nothing
/// today, and that is the point. `versus.rs` declares
/// `MatchAbilities::at_most(…)` — a ceiling and no floor, meaning *"a character
/// keeps what it authored, minus what this duel forbids"*. Neither duelist
/// authored anything, so `MatchAbilities::apply(None)` took its `unwrap_or`
/// branch and handed them the whole CEILING. The stage's stated rule and its
/// behaviour disagreed, and the disagreement was invisible because the answer
/// happened to be the one everybody wanted.
///
/// That `None → permitted` arm turns PERMISSION into a GRANT — most damagingly in the use the
/// type's own docs propose, where `permitted ⊃ granted` says *"the one character who authored a
/// wall jump keeps it"* and an unauthored character takes the wall jump too. It cannot simply
/// be deleted while it is the only thing dressing these two, so they get dressed first.
///
/// `reset` and `interact` ride in from `basic()`, and they were already
/// arriving by the same route. Preserved deliberately rather than tidied: this
/// change is behaviour-neutral or it is not worth making.
pub const VERSUS_FIGHTER_KIT: ambition_platformer2d::engine_core::AbilitySet =
    ambition_platformer2d::engine_core::AbilitySet {
        attack: true,
        fast_fall: true,
        ..ambition_platformer2d::engine_core::AbilitySet::basic()
    };

pub fn duelists() -> [CharacterDefinition; 2] {
    [
        // A reach fighter. Longer swings, a slower smash — wins by keeping the
        // other fighter at arm's length and punishing the approach.
        CharacterDefinition::new("arena_duelist_long", "Long Guard", VERSUS_PROVIDER)
            .with_sheet("robot")
            .with_abilities(VERSUS_FIGHTER_KIT)
            // A round has to last long enough to be a round. At these damage
            // numbers 60 HP is roughly a dozen committed hits or thirty pokes,
            // which is the length that makes the jab/smash trade a DECISION
            // rather than a coin flip — one exchange deciding the match is not a
            // fighting game, it is a duel of who pressed first.
            .with_health(60, 1.0)
            // EMPTY, and authored here rather than left to the catalog row.
            //
            // Everything these fighters do is on the moveset below. The action set says what a body
            // REACHES FOR — what the brain believes it can press — and theirs is nothing beyond it.
            .with_action_set(ActionSet::default())
            .with_moveset(duelist_moveset(LONG_GUARD))
            .with_hurtboxes(duelist_hurtboxes(LONG_GUARD))
            // A VOICE. These two are registered-only, so nothing else can
            // give them one, and a fighter on a Hall pedestal saying nothing
            // reads as unfinished.
            .with_voice([
                "Reach is patience with a longer arm.",
                "I will meet you exactly where you were.",
                "Come closer. That is the whole plan, is it not?",
            ]),
        // A rushdown fighter. Shorter reach, a faster smash — has to get inside
        // to do anything, and is rewarded for being there.
        CharacterDefinition::new("arena_duelist_close", "Close Guard", VERSUS_PROVIDER)
            .with_sheet("mary_o_v2")
            .with_abilities(VERSUS_FIGHTER_KIT)
            // Slightly frailer than the long guard, to pay for getting to swing
            // faster. Same round length, different price.
            .with_health(52, 0.9)
            .with_action_set(ActionSet::default())
            .with_moveset(duelist_moveset(CLOSE_GUARD))
            .with_hurtboxes(duelist_hurtboxes(CLOSE_GUARD))
            // A VOICE. These two are registered-only, so nothing else can
            // give them one, and a fighter on a Hall pedestal saying nothing
            // reads as unfinished.
            .with_voice([
                "Everything good happens inside arm's length.",
                "Your reach is a commitment. I am counting on it.",
                "Guard up. Guard close. Guard now.",
            ]),
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
        // `Some`, because these two fighters DID author their pools — 60 and 52
        // is the trade this stage is built on. `None` on `Vitals` means "the
        // author said nothing", which is the case a body must be able to
        // distinguish from an authored one-hit pool.
        self.vitals = ambition_platformer2d::characters::actor::definition::Vitals {
            max_health: Some(max_health),
            mass: Some(mass),
            // these two author no knockback WEIGHT, deliberately: the versus
            // stage declares no growth (its rounds end on health, not a blast
            // zone), so a weight here would divide a term that is always zero
            // and read as a tuning knob that does nothing.
            knockback_weight: None,
            // and no canonical HEIGHT, for a different reason: these two are
            // arena duellists drawn from the shared duelist art, so how tall
            // they stand is the sheet's answer rather than a character fact
            // anyone has authored. `None` says exactly that.
            canonical_height: None,
        };
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
    autonomous_profiles: {
        // it was `VERSUS_ROSTER_RON`, one `ArchetypeSpec` row registered as a
        // `CharacterRosterFragment` and existing for exactly one lookup: a CPU
        // seat naming `versus_duelist`, resolved through an ENEMY ARCHETYPE
        // TABLE, so the controller half of `character + controller + team`
        // arrived by way of a body definition. This is what a controller policy
        // IS, and publishing it is what lets `seat_brain_profile` stop having a
        // second authority to fall through to.
        //
        // the numbers are the row's controller half verbatim — template,
        // both radii and both efforts. Its BODY half went nowhere because it was
        // already dead: `max_health`, `run_speed`, `melee`, `move_style` and
        // `respawn` stopped being read the day a seat was built from its
        // CHARACTER (P1.11), and the note at `versus.rs`'s `fighter_abilities`
        // records exactly that — the authored `melee` "reached the body
        // regardless of what the match said the body could do", and taking it
        // away is what exposed the missing `attack` verb.
        //
        // That is an open finding, not a preference — see `the_cpu_opponent_is_not_a_statue`
        // and the queue row.
        "versus_duelist": (
            template: Smash,
            aggro_radius: 460.0,
            attack_range: 150.0,
            patrol_effort: 0.6176,
            chase_effort: 1.0,
        ),
    },
    action_set_presets: {
        // A required FIELD on every row, and no longer this character's kit.
        //
        // Both fighters author an explicit empty `ActionSet` on their
        // `CharacterDefinition`, which outranks anything here (C3). This exists
        // because the row schema demands a `default_action_set` name, and it is
        // empty so that the two authorities cannot disagree while the catalog
        // is still an input to preparation.
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
            spritesheet: "sprites/mary_o_v2_spritesheet.png",
            manifest: "sprites/mary_o_v2_spritesheet.ron",
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
    use ambition_platformer2d::entity_catalog::AttackDir;

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
    /// DRESSING THE DUELISTS CHANGED NOTHING, and that is what makes it safe to retire the
    /// bridge afterwards.
    ///
    /// the arm being retired is `MatchAbilities::apply`'s
    /// `authored.unwrap_or(self.permitted)` — an unauthored character receives
    /// the whole CEILING, so PERMISSION becomes a GRANT. It reads harmless until
    /// the type is used the way its own docs propose (`permitted ⊃ granted`, so
    /// one character keeps a wall jump nobody else has), at which point every
    /// unauthored character silently keeps it too.
    ///
    /// This asserts the step that fixes that: what they author and what the bridge was handing
    /// them are the same set, so the semantics can move without any fighter changing.
    #[test]
    fn what_the_duelists_author_is_exactly_what_the_bridge_was_handing_them() {
        use ambition_platformer2d::engine_core::{AbilitySet, MatchAbilities};

        let rules = MatchAbilities::at_most(VERSUS_FIGHTER_KIT);
        let authored = rules.apply(Some(VERSUS_FIGHTER_KIT));
        assert_eq!(
            authored, VERSUS_FIGHTER_KIT,
            "a duelist no longer gets the kit it authored, so the ceiling is \
             narrowing something it claims merely to permit"
        );
        assert_eq!(
            rules.apply(None),
            AbilitySet::NONE,
            "an unauthored fighter took the ceiling again — the migration bridge \
             D151 retired is back, and permission is a grant once more"
        );

        // non-vacuity: a kit of nothing would satisfy the equality above for
        // the wrong reason. These two fighters throw punches.
        assert!(
            authored.attack && authored.move_horizontal && authored.jump,
            "the versus kit cannot fight, so the equality above compares two \
             empty sets rather than a real fighter's verbs"
        );

        // and the fighters really do carry it now — the whole point is that
        // the answer stops depending on the ceiling.
        for duelist in duelists() {
            assert_eq!(
                duelist.abilities,
                Some(VERSUS_FIGHTER_KIT),
                "a duelist still authors no kit, so retiring the bridge would \
                 leave it unable to act"
            );
        }
    }
}
