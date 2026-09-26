//! Tests of the moves the game ships for `npc_carl_stargan`.
//!
//! The table is content: `assets/data/movesets/carl_stargan.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.





use ambition_entity_catalog::{MoveSpec, MovesetContract, WindowTag};


/// The far end of his reach, in world units from his centre — the leading
/// edge of `billions_and_billions`.
const FURTHEST_REACH: f32 = 96.0;

/// The near end — the jab, which barely leaves his sleeve.
const NEAREST_REACH: f32 = 22.0;

const STARSTUFF_ENDS_S: f32 = 1.14;

/// The rise `starstuff` commands. Authored as a speed and applied with
/// [`ImpulseMode::Set`], like every recovery here.
const STARSTUFF_SPEED: f32 = 900.0;

/// Does this move reach forward (a volume centred ahead of him, not above or
/// below)?
///
/// This makes [`reach_of`] comparable: an up-smash has a small x-extent
/// because it points up, not because it is short.
fn points_forward(spec: &MoveSpec) -> bool {
    spec.windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .any(|v| match v.shape {
            ambition_entity_catalog::VolumeShape::Rect { offset, .. } => offset.0.abs() >= 12.0,
            _ => false,
        })
}

/// How far a move's leading edge reaches from the owner's centre.
fn reach_of(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .flat_map(|w| w.volumes.iter())
        .filter_map(|v| match v.shape {
            ambition_entity_catalog::VolumeShape::Rect {
                offset,
                half_extents,
            } => Some(offset.0.abs() + half_extents.0),
            _ => None,
        })
        .fold(0.0_f32, f32::max)
}

/// The move's startup: the time before its first box exists.
fn startup_of(spec: &MoveSpec) -> f32 {
    spec.windows
        .iter()
        .filter(|w| matches!(w.tag, WindowTag::Active) && !w.volumes.is_empty())
        .map(|w| w.start_s)
        .fold(f32::MAX, f32::min)
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

    /// The charge holds and does not store. `stores` is the assertion: a stored
    /// charge would make him a different character (the brawler's haymaker).
    #[test]
    fn the_cosmic_calendar_is_held_on_the_page_it_is_thrown_on() {
        let calendar = find(&crate::authored_movesets::shipped("npc_carl_stargan"), "cosmic_calendar");
        let charge = calendar
            .smash_charge
            .as_ref()
            .expect("fourteen billion years is a hold");
        assert!(!charge.stores, "a stored calendar is somebody else's move");
        assert!(
            charge.roots,
            "a wind-up you can walk around with is not a commitment"
        );
        assert!(
            charge.hold_at_s > 0.0 && charge.hold_at_s < calendar.duration_s,
            "the freeze must sit inside the move"
        );
        assert!(
            calendar.smash_charge_mult > 1.0,
            "holding it must buy something"
        );
    }

    /// The pixel is the kill, and its rank is the mechanic.
    ///
    /// `pale_blue_dot` is a tipper: one Active window with two volumes, the far
    /// one first. The strike seam takes the first authored volume that reaches.
    ///
    /// This asserts the order, not the presence. With the tip appended, the near
    /// sourspot would win wherever both reach and spacing would be punished.
    #[test]
    fn the_pale_blue_dot_kills_at_the_pixel_and_pokes_up_close() {
        let dot = find(&crate::authored_movesets::shipped("npc_carl_stargan"), "pale_blue_dot");
        let window = dot
            .windows
            .iter()
            .find(|w| w.tag == WindowTag::Active && !w.volumes.is_empty())
            .expect("the dot has an active window");
        assert_eq!(
            window.volumes.len(),
            2,
            "the dot is a sweetspot and a sourspot: {:?}",
            window.volumes.len()
        );
        let tip = &window.volumes[0];
        let base = &window.volumes[1];
        assert!(
            tip.shape.leading_edge_x() > base.shape.leading_edge_x(),
            "rank 0 must be the FAR volume, or spacing is punished rather than \
             rewarded: tip reaches {}, base reaches {}",
            tip.shape.leading_edge_x(),
            base.shape.leading_edge_x(),
        );
        assert!(
            tip.damage > base.damage,
            "the pixel must hit harder than the ground he is standing on: \
             tip {} vs base {}",
            tip.damage,
            base.damage,
        );
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// Reach is bought with time: sort his forward line by reach, and startup
    /// never goes down.
    ///
    /// Forward moves only: an up-smash's x-reach is small because it points up.
    #[test]
    fn reach_is_monotonic_in_startup() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        let mut grounded: Vec<(f32, f32, String)> = set
            .moves
            .iter()
            .filter(|m| m.gates.grounded == Some(true) && points_forward(m))
            .map(|m| (reach_of(m), startup_of(m), m.id.clone()))
            .collect();
        assert!(
            grounded.len() >= 5,
            "the forward line is {} moves",
            grounded.len()
        );
        grounded.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for pair in grounded.windows(2) {
            let (near, far) = (&pair[0], &pair[1]);
            assert!(
                far.1 + 1e-4 >= near.1,
                "`{}` reaches {:.0} in {:.2}s but `{}` reaches only {:.0} and takes {:.2}s — \
                 a longer reach must never be quicker",
                far.2,
                far.0,
                far.1,
                near.2,
                near.0,
                near.1
            );
        }

        // No control fighter: Oiler's forward line is monotonic too, so this is a
        // shared discipline. Carl's own trait is the spread, tested below.
    }

    /// The spread is the widest on the grid — the very small and the very
    /// large, in hitboxes.
    #[test]
    fn his_reach_spans_further_than_anybody_elses() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        let reaches: Vec<f32> = set
            .moves
            .iter()
            .filter(|m| points_forward(m))
            .map(reach_of)
            .collect();
        let far = reaches.iter().cloned().fold(0.0_f32, f32::max);
        let near = reaches.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            (far - FURTHEST_REACH).abs() < 1.0,
            "his furthest is {far:.0}, and the module doc says {FURTHEST_REACH}"
        );
        assert!((near - NEAREST_REACH).abs() < 1.0);
        let mine = far / near;

        for (who, other) in [
            ("oiler", crate::authored_movesets::shipped("npc_oiler")),
            (
                "emmy_noether",
                crate::authored_movesets::shipped("npc_emmy_noether"),
            ),
        ] {
            let theirs: Vec<f32> = other
                .moves
                .iter()
                .filter(|m| points_forward(m))
                .map(reach_of)
                .filter(|r| *r > 0.0)
                .collect();
            let ratio = theirs.iter().cloned().fold(0.0_f32, f32::max)
                / theirs.iter().cloned().fold(f32::MAX, f32::min);
            assert!(
                mine > ratio * 1.5,
                "Carl's spread is {mine:.1}x and {who}'s is {ratio:.1}x — not a wide enough \
                 gap to be his defining property"
            );
        }
    }

    /// `starstuff` is a save, not flight — arithmetic, not a cooldown.
    #[test]
    fn the_recovery_outlasts_its_own_arc() {
        const G: f32 = 2200.0;
        assert!(STARSTUFF_ENDS_S >= 2.0 * (STARSTUFF_SPEED / G));
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        assert!(find(&set, "starstuff")
            .windows
            .iter()
            .all(|w| !matches!(w.tag, WindowTag::Cancelable { .. })));
    }

    /// None of his bursts sit on his navel: bursts are placed on their box, not
    /// on his chest.
    ///
    /// Sound is covered by `a_paired_burst_is_heard_exactly_once` in
    /// `src/moveset_sound.rs`.
    #[test]
    fn none_of_his_bursts_sit_on_his_navel() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        let mut placed = 0;
        for m in &set.moves {
            let mut bursts = 0;
            for ev in &m.events {
                if let MoveEventKind::Vfx { at, .. } = &ev.kind {
                    bursts += 1;
                    if *at != (0.0, 0.0) {
                        placed += 1;
                    }
                }
            }
            assert!(bursts > 0, "`{}` throws no effect at all", m.id);
        }
        // Non-vacuity: most bursts must carry an offset.
        assert!(placed >= 12, "only {placed} bursts are placed on their box");
    }

    /// His art is his own, and it all ships.
    #[test]
    fn the_kit_looks_like_carl_and_the_art_all_ships() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
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
        assert!(effects.len() >= 10, "a thin palette: {effects:?}");
        for effect in &effects {
            let authored = ambition_platformer2d::sprite_sheet::fx::authored_effect(effect)
                .unwrap_or_else(|| panic!("`{effect}` ships"));
            assert!(
                authored.sheet.contains("carl_stargan"),
                "`{effect}` is drawn off `{}`, which is not his sheet",
                authored.sheet
            );
        }
    }

    /// Every clip he names is a row his sheet carries.
    #[test]
    fn every_clip_names_a_row_his_sheet_carries() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        let record = ambition_platformer2d::sprite_sheet::character::sheets::record_for_sheet_key(
            "carl_stargan",
        )
        .expect("his sheet is baked into the registry");
        let rows: std::collections::BTreeSet<&str> =
            record.rows.iter().map(|r| r.animation.as_str()).collect();
        for m in &set.moves {
            assert!(
                rows.contains(m.clip.clip.as_str()),
                "`{}` draws `{}`, which his sheet does not publish",
                m.id,
                m.clip.clip
            );
        }
    }

    /// The pass homes, matching its `orbit_lock` art, and it still ends before
    /// the move does.
    #[test]
    fn his_slingshot_bends_toward_what_it_passes_and_lets_go_first() {
        let set = crate::authored_movesets::shipped("npc_carl_stargan");
        let pass = find(&set, "planetary_orbit");

        let homing: ambition_entity_catalog::smash_homing::HomingDashParams = pass
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_homing::HOMING_DASH =>
                {
                    effect.params.hydrate().ok()
                }
                _ => None,
            })
            .expect("his slingshot homes");

        // It lets go before the move ends, so a whiff is still punishable.
        let homing_ends = 0.18 + homing.duration_s;
        assert!(
            homing_ends < pass.duration_s,
            "the homing runs to {homing_ends}s on a {}s move, so a whiff carries \
             him through his own recovery",
            pass.duration_s
        );

        // A read, not a missile: past the half-plane the cone reaches behind him.
        assert!(
            homing.cone_degrees <= 90.0 && homing.cone_degrees > 0.0,
            "the cone is {}°",
            homing.cone_degrees
        );

        // The swing is unchanged; only the impulse was replaced.
        assert!(
            pass.windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .any(|v| v.damage == 10),
            "the pass lost its authored hitbox"
        );
    }
}
