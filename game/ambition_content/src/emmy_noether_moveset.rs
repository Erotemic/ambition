//! Tests of the moves the game ships for `npc_emmy_noether`.
//!
//! The table is content: `assets/data/movesets/emmy_noether.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.





use ambition_entity_catalog::{
    MoveSpec, MovesetContract,
    WindowTag,
};


/// The launcher's growth. The blueprint calls `symmetry_break` *"the moment
/// the invariant stops holding"*, and it is the one move that grows like it.
const BREAK_GROWTH: f32 = 3.15;

/// How far a move may sit off the invariant. Damage is an integer and time is
/// authored in hundredths, so exact products are not always possible; this is
/// the rounding, not a licence.
const INVARIANT_BAND: f32 = 0.12;

/// Not a feel number. Under the engine baseline the lift climbs
/// `LIFT_SPEED^2 / 2g` in `LIFT_SPEED / g`; a tail shorter than twice that
/// returns her higher on every press, which is flight.
/// `the_lift_is_a_save_and_not_a_flight` checks the arithmetic.
const LIFT_ENDS_S: f32 = 1.16;

/// The rise her ethereal lift commands, in px/s.
///
/// A speed applied with [`ImpulseMode::Set`], so a falling Emmy climbs as far
/// as a standing one. An additive impulse is weakest exactly when it is all
/// that stands between her and the blast zone.
const LIFT_SPEED: f32 = 940.0;

/// The conserved quantity: `damage x active_seconds`, in damage-seconds.
///
/// This is the character, not a fitted tuning constant. Retune Emmy by moving
/// a move along the curve (trade damage for window time, or the reverse),
/// never off it.
const NOETHER_IMPULSE: f32 = 0.90;

/// What every other move of hers grows at, at most.
const ORDINARY_GROWTH: f32 = 1.95;

fn conserved_impulse(spec: &MoveSpec) -> f32 {
    let damage = spec
        .windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .map(|v| v.damage as f32)
        .fold(0.0_f32, f32::max);
    damage * total_active_s(spec)
}

/// The active seconds a move keeps a box in the world, summed over its windows.
fn total_active_s(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
        .map(|w| (w.end_s - w.start_s).max(0.0))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::MoveEventKind;

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

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .fold(0, i32::max)
    }

    fn knockback(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.knockback)
            .fold(0.0f32, f32::max)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The symmetry, with every other fighter as the negative control.
    ///
    /// Her forward and back aerials must be the same move except for direction.
    /// Oiler's and the goblin's pairs differ, which makes this a claim about Emmy,
    /// not about aerials in general.
    #[test]
    fn her_forward_and_back_aerials_are_the_same_move() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let f = find(&set, "air_forward");
        let b = find(&set, "air_back");
        assert_eq!(damage(&f), damage(&b), "same damage");
        assert_eq!(knockback(&f), knockback(&b), "same knockback");
        assert_eq!(growth(&f), growth(&b), "same growth");
        assert_eq!(f.duration_s, b.duration_s, "same clock");
        assert_eq!(
            total_active_s(&f),
            total_active_s(&b),
            "same time in the world"
        );

        // The negative control: nobody else on the grid is symmetric.
        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let of = find(&oiler, "air_forward");
        let ob = find(&oiler, "air_back");
        assert!(
            damage(&of) != damage(&ob) || knockback(&of) != knockback(&ob),
            "Oiler's aerial pair differs — if it stopped differing this test \
             would stop being about Emmy"
        );
    }

    /// The conserved quantity.
    ///
    /// Every striking move sits on `damage x active_seconds =` [`NOETHER_IMPULSE`]
    /// within [`INVARIANT_BAND`]. The band is not vacuous: Oiler's table, authored
    /// from the opposite idea, must miss it.
    #[test]
    fn every_strike_she_throws_conserves_the_same_quantity() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let mut striking = 0;
        for m in &set.moves {
            let impulse = conserved_impulse(m);
            if impulse == 0.0 {
                continue;
            }
            striking += 1;
            assert!(
                (impulse - NOETHER_IMPULSE).abs() <= INVARIANT_BAND,
                "`{}` is at {impulse:.3} damage-seconds, off the curve at \
                 {NOETHER_IMPULSE} +/- {INVARIANT_BAND}",
                m.id
            );
        }
        assert!(striking >= 14, "only {striking} moves strike at all");

        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let off_curve = oiler
            .moves
            .iter()
            .map(conserved_impulse)
            .filter(|i| *i > 0.0 && (i - NOETHER_IMPULSE).abs() > INVARIANT_BAND)
            .count();
        assert!(
            off_curve >= 6,
            "only {off_curve} of Oiler's moves miss Emmy's curve, so the curve is \
             a description of fighters in general rather than of her"
        );
    }

    /// The launcher is the one move whose growth leaves the ordinary band: the
    /// blueprint's *"the moment the invariant stops holding"*.
    #[test]
    fn exactly_one_move_grows_like_a_kill_move() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let loud: Vec<&str> = set
            .moves
            .iter()
            .filter(|m| growth(m) > ORDINARY_GROWTH)
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(loud, ["smash_forward"], "one break, and it is the break");
        assert_eq!(growth(&find(&set, "smash_forward")), BREAK_GROWTH);
    }

    /// Her recovery does not attack, and nothing else on the grid is like it.
    ///
    /// The blueprint asked for this. The negative control is Oiler's geyser,
    /// which carries a box.
    #[test]
    fn her_recovery_carries_no_hitbox_and_that_is_unusual() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows.iter().all(|w| w.volumes.is_empty()),
            "the ethereal lift is a traversal, not a second offensive option"
        );
        assert_eq!(conserved_impulse(&lift), 0.0);

        let oiler = crate::authored_movesets::shipped("npc_oiler");
        let geyser = find(&oiler, "oil_geyser");
        assert!(
            geyser.windows.iter().any(|w| !w.volumes.is_empty()),
            "Oiler's recovery hits — if it stopped, hers would no longer be the \
             one that gives that up"
        );
    }

    /// The lift is a save, not flight, held by arithmetic, not by a cooldown,
    /// like Oiler's geyser.
    #[test]
    fn the_lift_is_a_save_and_not_a_flight() {
        // Engine baseline gravity, the same number the geyser's guard uses.
        const G: f32 = 2200.0;
        let climb_s = LIFT_SPEED / G;
        assert!(
            LIFT_ENDS_S >= 2.0 * climb_s,
            "the lift ends at {LIFT_ENDS_S}s but its own arc takes {:.2}s up and \
             the same down, so repeated presses would gain height",
            climb_s
        );
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let lift = find(&set, "ethereal_lift");
        assert!(
            lift.windows
                .iter()
                .all(|w| !matches!(w.tag, WindowTag::Cancelable { .. })),
            "a cancelable window would let her re-press before the arc is spent"
        );
    }

    /// The side special buys distance BACKWARD without turning her round.
    #[test]
    fn the_symmetry_shift_retreats_without_conceding_the_facing() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let shift = find(&set, "symmetry_shift");
        let displacement = shift
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, .. } => Some(*local),
                _ => None,
            })
            .expect("the shift commands a displacement");
        assert!(
            displacement.0 < 0.0,
            "body-local +x is her facing, so a retreat that keeps the facing has \
             to be negative; got {displacement:?}"
        );
    }

    // Burst sound is guarded by `a_paired_burst_is_heard_exactly_once`
    // (`src/moveset_sound.rs`). It drives these tables through the real
    // dispatcher and fan-out and counts what reaches the SFX channel, so it
    // catches both silence and double-play.

    /// The art is hers, and it all ships.
    ///
    /// The oracle is the art: `is_authored_effect` reads the rows out of the
    /// baked manifests, so this asks what the renderer will ask.
    #[test]
    fn the_kit_looks_like_emmy_and_the_art_all_ships() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
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
        // Before the palette checks below: a renamed effect makes those fail too,
        // with a message about breadth, not the rename.
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(
            effects.len(),
            12,
            "all twelve of her rendered rows are bound, and nothing else is: \
             {effects:?}"
        );
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert!(
                authored.sheet.contains("noether"),
                "`{effect}` is drawn off `{}`, which is not Emmy's sheet",
                authored.sheet
            );
        }
    }

    /// Every clip she names is a row her rig actually publishes.
    ///
    /// The fallback chain makes a missing clip silent: the move still runs,
    /// drawn as `idle`. Right at runtime, wrong for authoring, so the table is
    /// checked against the sheet here.
    #[test]
    fn every_clip_names_a_row_her_sheet_carries() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let record =
            ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key("noether")
                .expect("Emmy's sheet is baked into the registry");
        let rows: std::collections::BTreeSet<&str> =
            record.rows.iter().map(|r| r.animation.as_str()).collect();
        for m in &set.moves {
            assert!(
                rows.contains(m.clip.clip.as_str()),
                "`{}` draws `{}`, which her sheet does not publish — it would \
                 fall down the chain to `idle`",
                m.id,
                m.clip.clip
            );
        }
    }

    /// The theorem is the move: the answer to being struck is that the energy is
    /// kept. A test that only found a counter would pass against one that
    /// answered with a grab.
    #[test]
    fn her_field_answers_a_blow_by_conserving_it() {
        let set = crate::authored_movesets::shipped("npc_emmy_noether");
        let field = set
            .moves
            .iter()
            .find(|m| m.id == "invariant_field")
            .expect("her grounded down special");

        let params: ambition_entity_catalog::smash_counter::CounterParams = field
            .windows
            .iter()
            .filter_map(|w| w.sustain_effect.as_ref())
            .find(|e| e.key == ambition_entity_catalog::smash_counter::COUNTER)
            .expect("the field holds a counter stance")
            .params
            .hydrate()
            .expect("counter params hydrate");

        assert_eq!(
            params.response,
            ambition_entity_catalog::smash_vitality::VITALITY,
            "she must answer by keeping the energy, not by grabbing or leaving"
        );
        let gain: ambition_entity_catalog::smash_vitality::VitalityParams = params
            .response_params
            .hydrate()
            .expect("vitality params hydrate");
        assert!(
            gain.change > 0,
            "a conservation law that COSTS her health is the opposite of the move: {}",
            gain.change
        );
        // Small: a parry is already a full punish window.
        assert!(
            gain.change <= 5,
            "the heal is worth turtling for: {}",
            gain.change
        );

        // She absorbs, not reflects: returning the shot is conservation of
        // momentum, which is George's riposte.
        assert!(
            params.absorbs_projectiles,
            "she returns shots, which is the other fighter's law"
        );

        // The stance is a real read: her old active window was 0.15s (nine
        // frames). The 0.05s stances elsewhere are for fighters who answer fast.
        let stance = field
            .windows
            .iter()
            .find(|w| w.sustain_effect.is_some())
            .expect("a stance window");
        assert!(
            stance.end_s - stance.start_s >= 0.12,
            "the stance is {}s, which is a guess rather than a read",
            stance.end_s - stance.start_s
        );
    }
}
