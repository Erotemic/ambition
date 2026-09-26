//! Tests of the moves the game ships for `npc_oiler`.
//!
//! The table is content: `assets/data/movesets/oiler.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.





use ambition_entity_catalog::{
    ImpulseMode, MoveSpec, MovesetContract, WindowTag,
};


/// When the column arrives — after a windup you can see and hear (the ground
/// swells first: `oil_geyser_emerge`).
const GEYSER_AT_S: f32 = 0.22;

/// When the move lets go. `the_geyser_is_a_save_and_not_a_flight` checks the
/// arithmetic.
const GEYSER_ENDS_S: f32 = 1.20;

/// The rise the geyser commands, engine units per second against gravity.
///
/// Authored as a speed and applied with [`ImpulseMode::Set`], so a falling
/// Oiler gets the same climb as a standing one. An additive impulse would be
/// weakest when he needs it most.
const GEYSER_SPEED: f32 = 980.0;

/// The tolerance band: the least time any Oiler move keeps a hitbox in the
/// world, summed over its active windows.
///
/// This defines the character. Retune within the band, never below it: a
/// mechanic whose windows closed as fast as a goblin's would be a slower
/// goblin.
const TOLERANCE_S: f32 = 0.10;

/// And what the forward smash grows at instead. The gap between the two is the
/// whole reason Oiler has to land a specific move to take a stock.
const TORQUE_GROWTH: f32 = 3.30;

/// The one bolt torqued to spec. No move but the forward smash may grow
/// harder than this with the victim's damage.
const WITHIN_TOLERANCE_GROWTH: f32 = 2.10;

/// How long this move keeps a hitbox in the world, summed over its active
/// windows. The measurement [`TOLERANCE_S`] is about.
fn total_active_s(m: &MoveSpec) -> f32 {
    m.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active))
        .map(|w| w.end_s - w.start_s)
        .sum()
}

#[cfg(test)]
mod tests {
    /// The geyser leaves a pool, and it must not be a second way home.
    ///
    /// The pool's launch must be weaker than his own climb. Otherwise the recovery
    /// would improve with repeated use, which the geyser's design forbids.
    #[test]
    fn the_geysers_pool_throws_less_hard_than_the_geyser_itself() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_oiler");
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "oil_geyser")
            .expect("his up-B is in the table");
        let placed = up
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_spring::PLACE_SPRING =>
                {
                    Some((
                        e.at_s,
                        effect
                            .params
                            .hydrate::<ambition_entity_catalog::smash_spring::PlaceSpringParams>(),
                    ))
                }
                _ => None,
            })
            .expect("the geyser leaves nothing behind, so the column is one the caster alone met");
        let (at_s, params) = placed;
        let params = params.expect("the pool's params hydrate");

        assert!(
            params.launch.1 < 0.0,
            "the pool throws DOWNWARD ({:?}) — up is negative y",
            params.launch
        );
        assert!(
            -params.launch.1 < super::GEYSER_SPEED,
            "the pool throws at {} against the geyser's own {} — a plate stronger \
             than the move that made it is a recovery that improves by being used \
             twice",
            -params.launch.1,
            super::GEYSER_SPEED
        );
        // It lands at the crest, where `oil_geyser_impact` draws, not under him at
        // the press.
        assert!(
            at_s > 0.5,
            "the pool is placed at {at_s}s, before the column has finished climbing"
        );
        assert_eq!(params.uses, 1, "a multi-use geyser pool is a platform");
    }

    use super::*;
    use ambition_entity_catalog::{AttackDir, MoveEventKind};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn growth(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .filter_map(|v| v.knockback_growth)
            .fold(0.0f32, f32::max)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The tolerance band, as an assertion: no Oiler move closes its window inside
    /// [`TOLERANCE_S`].
    ///
    /// The goblin is the control. If its windows were this wide too, the band
    /// would describe the helper, not the character.
    #[test]
    fn every_move_holds_its_hitbox_for_the_tolerance_band() {
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        // Only moves that hold a box, as in the builder's own assertion. Pummels and
        // throws have no Active window (see `capture_beat`).
        let mut reaching = 0;
        for m in oiler
            .moves
            .iter()
            .filter(|m| m.windows.iter().any(|w| matches!(w.tag, WindowTag::Active)))
        {
            reaching += 1;
            let held = total_active_s(m);
            assert!(
                held + 1e-4 >= TOLERANCE_S,
                "`{}` keeps a box in the world for {held}s, inside the band this \
                 fighter works to ({TOLERANCE_S}s)",
                m.id
            );
        }
        // Zero floor: a filter that removed every move would pass trivially.
        assert!(
            reaching >= 16,
            "only {reaching} Oiler moves hold a box at all — the band is being \
             asserted over a population that shrank"
        );

        let goblin = crate::authored_movesets::shipped("goblin");
        let tighter = goblin
            .moves
            .iter()
            .filter(|m| total_active_s(m) + 1e-4 < TOLERANCE_S)
            .count();
        assert!(
            tighter >= 8,
            "only {tighter} goblin moves close inside the band, so the band is a \
             property of `strike` rather than a property of Oiler"
        );
    }

    /// Exactly one move is a kill move. The forward smash grows at
    /// [`TORQUE_GROWTH`] and nothing else may pass [`WITHIN_TOLERANCE_GROWTH`].
    ///
    /// The goblin is the control: it has four moves above the same line.
    #[test]
    fn only_one_move_grows_past_the_tolerance_band() {
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let torqued: Vec<&str> = oiler
            .moves
            .iter()
            .filter(|m| growth(m) > WITHIN_TOLERANCE_GROWTH)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            torqued,
            vec!["smash_forward"],
            "exactly one move may close a stock; these grow past \
             {WITHIN_TOLERANCE_GROWTH}"
        );
        assert!(growth(&find(&oiler, "smash_forward")) >= TORQUE_GROWTH);

        let goblin = crate::authored_movesets::shipped("goblin");
        let goblin_torqued = goblin
            .moves
            .iter()
            .filter(|m| growth(m) > WITHIN_TOLERANCE_GROWTH)
            .count();
        assert!(
            goblin_torqued >= 3,
            "the goblin is supposed to have an ordinary spread of kill options \
             ({goblin_torqued} above the line); if it does not, this test is \
             asserting a property of the threshold rather than of Oiler"
        );
    }

    /// The geyser is a save, not a flight.
    ///
    /// He cannot re-press while the move plays (no `Cancelable` window), so the
    /// only question is whether one full cycle gains height. It cannot: the move
    /// outlasts its arc. So no cooldown, per-airtime counter or rollback state is
    /// needed.
    #[test]
    fn the_geyser_is_a_save_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let to_apex = GEYSER_SPEED / g;
        let tail = GEYSER_ENDS_S - GEYSER_AT_S;
        assert!(
            tail > 2.0 * to_apex,
            "the column climbs for {to_apex:.3}s and is handed back {tail:.3}s \
             after the burst; anything at or under {:.3}s returns Oiler higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // Landing out of it costs, so it is a bad panic button on the stage.
        let up_b = find(&crate::authored_movesets::shipped("npc_oiler"), "oil_geyser");
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
        assert_eq!(up_b.duration_s, GEYSER_ENDS_S);
        assert!(
            up_b.motion_scale_at(GEYSER_ENDS_S - 0.01) < 0.5,
            "the ride down is supposed to be helpless"
        );
    }

    /// The rise is commanded, not added, and it is the only one.
    ///
    /// Under `ImpulseMode::Add` a falling Oiler would climb only what was left
    /// over. `Set` makes the climb a property of the move. `lift_speed` is
    /// derived from `Set` impulses only, so this also shows the brain and the
    /// recovery probe can see the move.
    #[test]
    fn the_geyser_commands_its_rise_and_is_the_only_way_home() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let up_b = find(&set, "oil_geyser");
        let burst = up_b
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, mode } => Some((e.at_s, *local, *mode)),
                _ => None,
            })
            .expect("the recovery special displaces its owner");
        assert_eq!(burst.2, ImpulseMode::Set);
        assert!(burst.1 .1 < 0.0, "the burst must point AGAINST gravity");
        assert_eq!(burst.0, GEYSER_AT_S);

        let frames = up_b.frame_data();
        assert_eq!(frames.lift_speed, GEYSER_SPEED);
        assert_eq!(frames.lift_at_s, GEYSER_AT_S);

        // Control: nothing else advertises a lift.
        let others: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| m.id != "oil_geyser" && m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            others.is_empty(),
            "these moves also claim to be ways home: {others:?}"
        );
    }

    /// The geyser plays its three rows, in order: the ground swells, the column
    /// runs for the whole climb, the crest breaks last.
    #[test]
    fn the_geyser_stages_its_three_rows_in_order() {
        let up_b = find(&crate::authored_movesets::shipped("npc_oiler"), "oil_geyser");
        let at = |row: &str| -> Vec<f32> {
            up_b.events
                .iter()
                .filter_map(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } if effect == row => Some(e.at_s),
                    _ => None,
                })
                .collect()
        };
        let emerge = at("oil_geyser_emerge");
        let stream = at("oil_geyser_stream");
        let impact = at("oil_geyser_impact");
        assert_eq!(emerge.len(), 1, "one swell, before anything else");
        assert!(
            stream.len() >= 3,
            "the column must be re-struck across the climb or it reads as a \
             single puff: {stream:?}"
        );
        assert_eq!(impact.len(), 1);
        assert!(
            emerge[0] < GEYSER_AT_S,
            "the swell is the TELL: it has to arrive before the burst does"
        );
        assert!(stream.iter().all(|t| *t >= GEYSER_AT_S));
        assert!(impact[0] > *stream.last().unwrap(), "the crest breaks last");
        assert!(
            impact[0] < up_b.duration_s,
            "and inside the move, or nothing plays it"
        );
    }

    /// Four specials, four mechanisms: one lands three times and never displaces
    /// him, one commands a steerable slide, one commands a rise he cannot steer,
    /// one adds to his motion at the press. No two share a mechanism.
    #[test]
    fn the_four_specials_are_four_mechanisms() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };

        // Neutral: no displacement; it lands three times instead.
        let convergence = find(&set, "convergence");
        assert!(commanded("convergence").is_none());
        assert!(convergence.start_impulse.is_none());
        let terms: Vec<(f32, f32)> = convergence
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| (w.start_s, w.end_s))
            .collect();
        assert_eq!(terms.len(), 3, "the neutral special's idea IS the series");
        // The gaps make it rehit: contiguous windows share one hit set.
        let gaps: Vec<f32> = terms.windows(2).map(|p| p[1].0 - p[0].1).collect();
        assert!(
            gaps.iter().all(|g| *g > 0.0),
            "a series with no gap between its terms is ONE hit: {gaps:?}"
        );
        assert!(
            gaps[1] < gaps[0],
            "the terms are supposed to CONVERGE ({gaps:?})"
        );
        let damages: Vec<i32> = convergence
            .windows
            .iter()
            .filter(|w| matches!(w.tag, WindowTag::Active))
            .filter_map(|w| w.volumes.first().map(|v| v.damage))
            .collect();
        assert!(
            damages.windows(2).all(|p| p[1] > p[0]),
            "and each term must be worth more than the last: {damages:?}"
        );

        // Side: a commanded slide whose tail leaves his steering alone.
        let side = commanded("slick_dash").expect("the side special travels");
        assert!(side.0 > 0.0 && side.1 == 0.0, "flat, and forward");
        let slick = find(&set, "slick_dash");
        // The tail must exist; otherwise `strike`'s own 1.0 recovery window answers
        // below.
        assert!(
            slick.duration_s > 0.60,
            "the slide is supposed to outlast its own swing ({}s)",
            slick.duration_s
        );
        assert_eq!(
            slick.motion_scale_at(slick.duration_s - 0.01),
            1.0,
            "oil takes your brakes, not your steering — a locked tail makes this \
             the same move as everybody else's charge"
        );

        // Up: a rise only, with a tail that does lock, measured the same way.
        let up = commanded("oil_geyser").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        let geyser = find(&set, "oil_geyser");
        assert!(geyser.motion_scale_at(geyser.duration_s - 0.01) < 0.5);

        // Down: displaced at the press, additively (the only `start_impulse`).
        assert!(commanded("pressure_vent").is_none());
        let vent = find(&set, "pressure_vent")
            .start_impulse
            .expect("the vent shoves at the press");
        assert!(vent.1 > 0.0, "and it shoves DOWNWARD");
        let pressers: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| m.start_impulse.is_some())
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(
            pressers,
            vec!["tilt_forward", "pressure_vent"],
            "only the stride into the forward tilt and the vent shove at the \
             press; a second SPECIAL doing it makes two of these one move"
        );
    }

    // Burst sounds are covered by `a_paired_burst_is_heard_exactly_once`
    // (`src/moveset_sound.rs`), which runs these tables through the real
    // dispatcher and counts what reaches the SFX channel.

    /// Oiler's art is his own, and it all exists. An effect no shipped sheet
    /// carries never plays; another fighter's bursts give him no look.
    /// `is_authored_effect` reads the baked manifests, so this asks what the
    /// renderer asks.
    #[test]
    fn the_kit_looks_like_oiler_and_the_art_all_ships() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let mut effects = std::collections::BTreeSet::new();
        // Collect problems across every move, then assert once, so one run reports
        // every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
            for ev in &m.events {
                if let MoveEventKind::Vfx { effect, .. } = &ev.kind {
                    effects.insert(effect.clone());
                }
            }
        }
        // Before the palette checks below: a renamed effect fails those too, with a
        // less helpful message.
        assert!(problems.is_empty(), "{problems:?}");
        assert!(
            effects.len() >= 12,
            "a jab, a smash, a launcher, four specials and a recovery cannot all \
             look the same: {effects:?}"
        );
        // Every effect comes from his own sheet.
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert_eq!(
                authored.sheet, "oiler_vfx",
                "`{effect}` is drawn from `{}` — Oiler has his own sheet",
                authored.sheet
            );
        }

        // A heavy landing is heard apart from a poke landing.
        let heavy_hit = |id: &str| -> Option<String> {
            find(&set, id)
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .find_map(|v| v.hit_sfx.clone())
        };
        assert_ne!(heavy_hit("smash_forward"), heavy_hit("tilt_forward"));
        assert!(heavy_hit("smash_forward").is_some());
        assert!(heavy_hit("jab").is_none(), "a knuckle-rap does not clang");
    }

    /// Every press a body can make reaches a move, in both postures. The CPU kit
    /// builder and a human's stick use the same function.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        let set = crate::authored_movesets::shipped("npc_oiler");
        let reachable = |grounded: bool| -> std::collections::BTreeSet<String> {
            let mut ids = std::collections::BTreeSet::new();
            for base in ["attack", "smash", "special"] {
                for dir in [
                    AttackDir::Neutral,
                    AttackDir::Forward,
                    AttackDir::Back,
                    AttackDir::Up,
                    AttackDir::Down,
                ] {
                    if let Some(m) = set.move_for_directional_verb(base, dir, grounded) {
                        ids.insert(m.id.clone());
                    }
                }
            }
            ids
        };
        let on_ground = reachable(true);
        let airborne = reachable(false);
        assert!(
            on_ground.len() >= 8,
            "a grounded Oiler reaches {on_ground:?}"
        );
        assert!(
            airborne.len() >= 8,
            "an airborne Oiler reaches {airborne:?}"
        );
        // The recovery is reachable from both postures.
        assert!(on_ground.contains("oil_geyser"));
        assert!(airborne.contains("oil_geyser"));
        // The forward press does not fall through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// Every move names a clip the sheet draws. Without the fight rows, every
    /// swing would fall down the structural chain to `idle`.
    ///
    /// The oracle is the baked sheet record, so this fails if the sheet is
    /// republished without them.
    #[test]
    fn every_move_names_a_row_the_published_sheet_carries() {
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key("oiler")
                .expect("Oiler's sheet is baked into the binary");
        let rows: std::collections::BTreeSet<&str> = record
            .rows
            .iter()
            .map(|row| row.animation.as_str())
            .collect();
        assert!(
            rows.contains("idle") && rows.contains("walk"),
            "this is not Oiler's sheet: {rows:?}"
        );
        for m in &crate::authored_movesets::shipped("npc_oiler").moves {
            let chain: Vec<&str> = std::iter::once(m.clip.clip.as_str())
                .chain(m.clip.fallbacks.iter().map(String::as_str))
                .collect();
            let drawn = chain.iter().find(|row| rows.contains(*row));
            assert!(
                drawn.is_some_and(|row| *row != "idle"),
                "`{}` draws {chain:?}, and the published sheet answers none of it \
                 before `idle` — so the move draws the standing pose. Rows: {rows:?}",
                m.id,
            );
        }
    }
}
