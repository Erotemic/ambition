//! Tests of the moves the game ships for `npc_pirate_admiral`.
//!
//! The table is content: `assets/data/movesets/pirate_admiral.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.




use ambition_entity_catalog::{
    ImpulseMode, MovesetContract,
};


/// The weapon the side-B draws. His own row, not the shared `gun_sword`
/// (an adventure pickup and a raider's sidearm), so the side-special can be
/// balanced separately.
const ADMIRAL_GUN_SWORD: &str = "admiral_gun_sword";

/// How long the admiral may stay aboard. A first-pass design value (Jon);
/// expect it to be tuned down.
const SHARK_RIDE_SECONDS: f32 = 5.0;

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_entity_catalog::{MoveSpec, VolumeShape, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    fn startup(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .expect("a strike has an active window")
            .start_s
    }

    fn reach(m: &MoveSpec) -> f32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| match v.shape {
                VolumeShape::Rect {
                    offset,
                    half_extents,
                } => offset.0.abs() + half_extents.0,
                _ => 0.0,
            })
            .fold(0.0f32, f32::max)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    // Verb binding is checked by construction: `SmashRepertoire` owns the verb
    // strings and is a struct with no `Default`, so a missing slot is a compile
    // error. Coverage in every posture is checked by
    // `ambition_entity_catalog::smash_repertoire` and by
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// The commanded (`Set`) velocity a move states, if it states one.
    fn commanded(set: &MovesetContract, id: &str) -> Option<(f32, f32)> {
        use ambition_entity_catalog::MoveEventKind;
        find(set, id).events.iter().find_map(|e| match &e.kind {
            MoveEventKind::Impulse {
                local,
                mode: ImpulseMode::Set,
            } => Some(*local),
            _ => None,
        })
    }

    // -----------------------------------------------------------------------
    // No `RecoveryLens` fixture for this fighter: the shark up-B commands no
    // velocity, so the lens's commanded-velocity routes cannot see it (D207).
    // The planner reads it through its carry instead (D250, tested below). The
    // engine-level guard stays in `RecoveryLens`
    // (`a_tiny_lifting_move_does_not_suppress_a_viable_recovery`).

    /// Four specials, four mechanisms: a commanded retreat, a drawn weapon with
    /// an additive step, a summoned ride, a full stop. No two share a mechanism.
    ///
    /// They also exercise different corners of the catalog's route derivation (a
    /// negative side, an `Add` that states nothing, a `Set` of zero).
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");

        // Neutral: a recoil. Commanded, and it points backward.
        let shot = commanded(&set, "grapeshot").expect("the pistol shoves its owner");
        assert!(shot.0 < 0.0 && shot.1 < 0.0);

        // Side: he draws a weapon and fires it. The step is additive, so it
        // commands nothing and advertises no route.
        assert!(
            commanded(&set, "run_out_the_guns").is_none(),
            "the step must ADD to the admiral's momentum, not replace it"
        );
        assert!(
            find(&set, "run_out_the_guns")
                .events
                .iter()
                .any(|e| matches!(
                    &e.kind,
                    MoveEventKind::Impulse {
                        mode: ImpulseMode::Add,
                        ..
                    }
                )),
            "…and it must still displace him"
        );
        assert_eq!(find(&set, "run_out_the_guns").frame_data().lift_speed, 0.0);
        // Both halves together. A draw with no shot is a taunt; a shot with no draw
        // fires his pistol.
        assert_eq!(
            find(&set, "run_out_the_guns").equips.as_deref(),
            Some(ADMIRAL_GUN_SWORD),
            "the side-B must draw the gun-sword; without it the shot is the pistol's"
        );
        assert!(
            find(&set, "run_out_the_guns")
                .events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Ranged)),
            "the side-B must fire; a draw with no shot is a taunt"
        );
        // The draw outlives the shot. The brandish ends with the move, so a fire
        // event at or after the end would leave from a bare hand.
        let guns = find(&set, "run_out_the_guns");
        let fires_at = guns
            .events
            .iter()
            .find(|e| matches!(&e.kind, MoveEventKind::Ranged))
            .map(|e| e.at_s)
            .expect("the side-B fires");
        assert!(
            fires_at < guns.duration_s,
            "the shot fires at {fires_at}s of a {}s move, so the gun-sword is \
             already back in its sheath when the trigger is pulled",
            guns.duration_s
        );

        // Up: a vehicle. It displaces nobody; the technique on its timeline makes
        // it a recovery. The CPU recovery search reads commanded velocity, so it
        // cannot see this move (D207). Asserted so that adding an impulse later is
        // noticed.
        assert!(
            commanded(&set, "call_the_shark").is_none(),
            "the shark up-B commands a velocity, which means it is no longer the \
             vehicle recovery this fighter is built around"
        );
        assert!(
            find(&set, "call_the_shark")
                .events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_ride::SUMMON_RIDE)),
            "the up-B summons nothing, so the admiral has no recovery at all"
        );
        assert_eq!(
            find(&set, "call_the_shark").gates.recovery,
            ambition_entity_catalog::RecoveryUse::SpendWithoutFreefall,
            "the whole price is one value: one use per airtime, and no freefall. \
             It was two booleans that had to agree, and a move stating one \
             without the other was a different mechanic wearing this one's name"
        );

        // Down: a full stop, which is a commanded velocity of nothing.
        assert_eq!(commanded(&set, "heave_to"), Some((0.0, 0.0)));
        assert_eq!(
            find(&set, "heave_to").frame_data().lift_speed,
            0.0,
            "stopping dead in mid-air is not a way home from anywhere"
        );
    }

    /// Every important move is heard and seen. VFX ids are checked against the
    /// shipped FX spritesheet rows, which the renderer resolves against.
    #[test]
    fn the_specials_and_the_juggle_carry_their_own_feedback() {
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");
        for id in [
            "grapeshot",
            "run_out_the_guns",
            "call_the_shark",
            "heave_to",
            "air_up",
        ] {
            let m = find(&set, id);
            assert!(
                m.events
                    .iter()
                    .any(|e| matches!(&e.kind, MoveEventKind::Sfx { .. })),
                "`{id}` makes no sound"
            );
            assert!(
                m.events.iter().any(|e| match &e.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        assert!(
                            ambition_platformer2d::sprite_sheet::fx::is_authored_effect(effect),
                            "`{id}` names vfx `{effect}`, which the engine's \
                             vocabulary does not contain — this is a refused load"
                        );
                        true
                    }
                    _ => false,
                }),
                "`{id}` shows nothing"
            );
            // A move with no volumes cannot land. `call_the_shark` has no hitbox by
            // design, so it needs no contact cue.
            let lands = m.windows.iter().any(|w| !w.volumes.is_empty());
            assert!(
                !lands
                    || m.windows
                        .iter()
                        .flat_map(|w| w.volumes.iter())
                        .any(|v| v.hit_sfx.is_some()),
                "`{id}` lands silently"
            );
        }

        // Control: ordinary swings are not dressed up.
        let jab = find(&set, "jab");
        assert!(
            !jab.events
                .iter()
                .any(|e| matches!(&e.kind, MoveEventKind::Vfx { .. })),
            "a jab that bursts makes the specials look like nothing"
        );
    }

    /// Three tables, three fighters, one ORDERING — and it is checked against
    /// the other two rather than against literals.
    ///
    /// The module docs make comparative claims (shorter, slower, harder), so this
    /// test compares the three tables, not literals. Retuning any of them must
    /// keep the ordering or say why.
    #[test]
    fn the_admiral_is_longer_slower_and_heavier_than_the_other_two() {
        let admiral = crate::authored_movesets::shipped("npc_pirate_admiral");
        let goblin = crate::authored_movesets::shipped("goblin");
        let robot = crate::authored_movesets::shipped("player_robot_v3");

        let jabs = |set: &MovesetContract| {
            let jab = find(set, "jab");
            (reach(&jab), startup(&jab))
        };
        let (a_reach, a_startup) = jabs(&admiral);
        let (r_reach, r_startup) = jabs(&robot);
        let (g_reach, g_startup) = jabs(&goblin);

        assert!(
            a_reach > r_reach && r_reach > g_reach,
            "reach orders admiral > robot > goblin (got {a_reach}, {r_reach}, {g_reach})"
        );
        assert!(
            a_startup > r_startup && r_startup > g_startup,
            "and startup orders the same way — the longer blade is the slower one \
             (got {a_startup}, {r_startup}, {g_startup})"
        );

        let smash = |set: &MovesetContract| damage(&find(set, "smash_forward"));
        assert!(
            smash(&admiral) > smash(&robot) && smash(&robot) > smash(&goblin),
            "and the kill move pays for the commitment: {} > {} > {}",
            smash(&admiral),
            smash(&robot),
            smash(&goblin)
        );
    }

    /// The up-B is a way home the planner can see (D250).
    ///
    /// `call_the_shark` commands no impulse, so `lift_speed` stays `0.0`. Do not
    /// add a fake lift: the search would then certify a rise that does not
    /// exist.
    #[test]
    fn the_sharks_summon_advertises_seconds_of_authority_and_no_lift() {
        use ambition_entity_catalog::RecoveryRoute;
        let set = crate::authored_movesets::shipped("npc_pirate_admiral");
        let frames = find(&set, "call_the_shark").frame_data();
        assert_eq!(
            frames.lift_speed, 0.0,
            "the summon must still command no rise; a fabricated one would have \
             the recovery search certify height the move never throws"
        );
        let RecoveryRoute::SustainedAuthority { seconds, reach } = frames.recovery_route else {
            panic!(
                "the summon offers {:?}, so a recovery planner reads it as no way \
                 home — which is exactly D250",
                frames.recovery_route
            );
        };
        assert_eq!(seconds, SHARK_RIDE_SECONDS, "the ride's own length");
        assert!(
            reach > 0.0 && reach < SHARK_RIDE_SECONDS * 260.0,
            "the claimed reach is {reach}, which is either nothing or the whole \
             straight-line ride — a recovery turns back toward the stage, and a \
             search that over-claims kills the fighter it meant to save"
        );

        // The 650px is travel, not threat. The move has no hitbox, the shark is
        // `Neutral` with no contact damage, and `reach` is half the ride's distance:
        // where he can go. So `frame_data` must not fold it into hazard. The brain
        // reads it on the motion road (`brain::fighter::options::travel_of` →
        // `RecoveryRoute::carry`); the attack menu gets nothing.
        assert_eq!(
            frames.hazard, None,
            "the mobility special is advertised as {:?} of OFFENSIVE reach, so \
             the CPU admiral will summon a shark at an opponent it cannot \
             touch and stand in the resulting move while they walk up",
            frames.hazard
        );
        assert_eq!(
            frames.threat_live_at_s, None,
            "a move that threatens nobody named a time at which it does"
        );
        assert!(
            frames.coverage.is_none() && frames.push_coverage.is_none(),
            "the up-b grew a volume: coverage={:?} push={:?}",
            frames.coverage,
            frames.push_coverage
        );
        // The carry is still readable, so it is not off every menu.
        assert_eq!(
            frames.recovery_route.carry(),
            reach,
            "the travel a motion planner reads is not the ride's own reach"
        );
    }
}
