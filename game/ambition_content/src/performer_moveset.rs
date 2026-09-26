//! Tests of the moves the game ships for `performer`.
//!
//! The table is content: `assets/data/movesets/performer.ron`. These tests read it
//! through [`crate::authored_movesets::shipped`], the table the pack loads.





/// The trap. Jon:
///
/// > *"When down-b is resolved to the trapdoor move, a trapdoor opens and the
/// > character descends underground. In this subterranean state they can move
/// > for up to the timelimit of the move (3 seconds) where they move is shown
/// > by a unopened trap door sprite on the ground. When the move ends or the
/// > character ends the move by pressing a non-move action, the final stage of
/// > the move happens, where the trapdoor opens and then they leap out with an
/// > explosion and hurtbox in the area."*
///
/// Five stages, one constant each:
///
/// 1. `DOOR_OPENS_S`: the boards give. She is still in the world.
/// 2. `SINK_AT_S`: she descends. `BodyMode::Submerged`: not drawn, not
///    hittable, no gravity, no geometry, and the door closes over her.
/// 3. `HOLD_UNDER_AT_S`..`MAX_UNDER_S`: the subterranean state. She steers,
///    surface-locked to the boards she went through, shown as an unopened
///    trapdoor sliding along the floor. The clock freezes on this beat.
/// 4. `EXIT_DOOR_OPENS_S`: a press ends the freeze (or the time limit does)
///    and the exit trapdoor opens. She is still under it.
/// 5. `SURFACE_AT_S`: she leaps out: the surfacing launch, the firework, and
///    a hitbox over the door. The launch is part of the placement
///    (`TrapdoorParams::leap_speed`), not a separate event.
///
/// The subterranean beat is a duration, not a hold: she gets the full three
/// seconds without holding B, and an action press ends it early. See
/// [`ChargeSustain::UntilPressedAgain`].
///
/// `blink_out` runs at 52ms a frame. The boards give on frame 2 and she is
/// through them by frame 3, when she leaves the world.
const DOOR_OPENS_S: f32 = 0.10;

/// How long the lift takes.
///
/// Long enough to see: at 60Hz this is 33 ticks, and the largest single tick
/// moves her about 13px. She is lifted, not teleported.
const LIFT_S: f32 = 0.55;

/// The move's time limit under the stage. Jon: *"they can move for up to the
/// timelimit of the move (3 seconds)."*
///
/// At 1.2× her 204 run speed this is about 735 world px, several stage widths.
/// The design biases toward strong moves; this is the first knob to turn if it
/// is too good.
const MAX_UNDER_S: f32 = 3.0;

const SINK_AT_S: f32 = 0.16;

/// The puff of smoke that hides the trick, like a stage trap door.
///
/// It fires on both forms and on the first frame, before anyone knows if the
/// boards will give. It is the misdirection: an effect that appeared only on
/// success would tell the opponent which form they are watching.
///
/// `smoke_puff` (a `generic_exotic_fx` row whose frames grow), not
/// `smoke_burst`, which is a row of `generic_explosions` despite its name.
const SMOKE_VFX: &str = "smoke_puff";

const SURFACE_AT_S: f32 = 0.30;

const TRAP_ENDS_S: f32 = 0.54;

/// The flyline catches her later than the trap drops her: the wire goes taut
/// before it pulls, which is the beat `fly`'s first two frames draw.
const WIRE_AT_S: f32 = 0.12;

/// When the winch stops and the rope lets go.
///
/// The move must outlast the lift. The kernel owns the wire's clock
/// (`WireState::lift_remaining_s`), so a timeline that ended first would leave
/// a maneuver with no animation. Guarded by
/// `the_lift_fits_inside_the_move_that_authors_it`.
const WIRE_RELEASES_S: f32 = WIRE_AT_S + LIFT_S;

#[cfg(test)]
mod tests {
    #[test]
    fn normal_contact_windows_match_the_authored_light_and_pose_clock() {
        use ambition_entity_catalog::WindowTag;
        let library = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/data/motion/humanoid/performer_stage_v1",
        );
        let read = |folder: &str, clip: &str, kind: &str| -> serde_json::Value {
            let path = library.join(folder).join(format!("{clip}.{kind}.json"));
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
        };
        for mv in crate::authored_movesets::shipped("performer").moves {
            let clip = mv.clip.clip.as_str();
            if !matches!(
                clip,
                "attack_side"
                    | "attack_up"
                    | "attack_down"
                    | "smash_forward"
                    | "smash_up"
                    | "smash_down"
                    | "air_neutral"
                    | "air_forward"
                    | "air_back"
                    | "air_up"
                    | "air_down"
            ) {
                continue;
            }
            let animation = read("clips", clip, "clip");
            let effect = read("specs", clip, "spec");
            let frame_s = animation["sampling"]["frame_duration_ms"].as_f64().unwrap() / 1000.0;
            let active = effect["hitbox"]["active"].as_array().unwrap();
            let first = active.first().unwrap().as_u64().unwrap();
            let end = active.last().unwrap().as_u64().unwrap() + 1;
            let contacts: Vec<_> = mv
                .windows
                .iter()
                .filter(|w| w.tag == WindowTag::Active)
                .collect();
            assert_eq!(
                contacts.len(),
                active.len(),
                "{clip}: one shape per active pose"
            );
            for (window, frame) in contacts.iter().zip(active) {
                let frame = frame.as_u64().unwrap();
                assert!((window.start_s as f64 - frame as f64 * frame_s).abs() < 1e-6);
                assert!((window.end_s as f64 - (frame + 1) as f64 * frame_s).abs() < 1e-6);
            }
            for (actual, expected) in [
                (contacts.first().unwrap().start_s, first as f64 * frame_s),
                (contacts.last().unwrap().end_s, end as f64 * frame_s),
                (mv.duration_s, animation["duration_s"].as_f64().unwrap()),
            ] {
                assert!(
                    (actual as f64 - expected).abs() < 1e-6,
                    "{clip}: runtime {actual}s disagrees with authored {expected}s"
                );
            }
        }
    }

    /// A confirmed tilt can cancel and a smash commits. Both halves are asserted,
    /// so giving the smashes the same window fails.
    ///
    /// This is a claim about the authored data only.
    ///
    /// A chain probe must put the target in reach and chain into a different
    /// move. The window is `OnHit`, so a short tilt refuses correctly. A tilt
    /// cancelled into the same tilt reads as one run unless the take is read by
    /// `move_starts`, not by move id.
    #[test]
    fn a_tilt_confirms_into_a_follow_up_and_a_smash_owes_its_recovery() {
        use ambition_entity_catalog::{CancelCondition, WindowTag};
        let set = crate::authored_movesets::shipped("performer");
        let cancels = |clip: &str| -> Vec<(Vec<String>, CancelCondition)> {
            set.moves
                .iter()
                .filter(|m| m.clip.clip == clip)
                .flat_map(|m| m.windows.iter())
                .filter_map(|w| match &w.tag {
                    WindowTag::Cancelable { into, condition } => Some((into.clone(), *condition)),
                    _ => None,
                })
                .collect()
        };

        for tilt in ["attack_side", "attack_up", "attack_down"] {
            let found = cancels(tilt);
            assert_eq!(found.len(), 1, "{tilt}: one cancel window");
            let (into, condition) = &found[0];
            assert_eq!(
                *condition,
                CancelCondition::OnHit,
                "{tilt}: an `Always` cancel swings the follow-up into a raised \
                 shield and takes the read out of the exchange"
            );
            assert!(
                into.iter().any(|n| n == "jump"),
                "{tilt}: without a jump escape a confirm cannot become an aerial"
            );
            assert!(
                into.iter().any(|n| n == "any_attack"),
                "{tilt}: no attack follow-up"
            );
        }

        for smash in ["smash_forward", "smash_up", "smash_down"] {
            assert!(
                cancels(smash).is_empty(),
                "{smash} can be cancelled. A smash is the commitment the rest of \
                 the kit is balanced against; one that escapes its own recovery \
                 is a tilt that hits harder"
            );
        }
    }

    use super::*;

    /// She steers under the stage. The submerged beat must have no window over it,
    /// because rooting folds with `min` (see `trapdoor`).
    ///
    /// The subterranean beat uses the timeline hold. Jon: *"or the character ends
    /// the move by pressing a non-move action."* Each check guards a failure:
    /// the gesture must be `Special`, because `charged_by_gesture` enters charge
    /// mode only when the starting press and the move's `charge_gesture` agree.
    /// The hold point must be after the submerge event, or she freezes above
    /// ground. And the hold must not root her.
    #[test]
    fn the_subterranean_beat_is_a_duration_an_action_press_can_cut_short() {
        let set = crate::authored_movesets::shipped("performer");
        // Grounded form only. The airborne form authors no hold; see
        // `the_puff_in_the_air_...` below.
        for id in ["performer_trapdoor"] {
            let trap = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("{id} is in the table"));
            assert_eq!(
                trap.charge_gesture,
                ambition_entity_catalog::ChargeGesture::Special,
                "{id} is reached by a Special press, so that is the press that holds it",
            );
            let policy = trap
                .charge_policy()
                .unwrap_or_else(|| panic!("{id} authors a hold"));
            assert!(
                policy.hold_at_s > SINK_AT_S,
                "{id} freezes at {}s, which is not after the submerge beat at {SINK_AT_S}s —                  she would be held at the mouth of the hole, above ground and hittable",
                policy.hold_at_s,
            );
            assert!(
                policy.hold_at_s < SURFACE_AT_S,
                "{id} freezes at {}s, which is not before she surfaces at {SURFACE_AT_S}s",
                policy.hold_at_s,
            );
            assert_eq!(
                policy.sustain,
                ambition_entity_catalog::ChargeSustain::UntilPressedAgain,
                "{id} freezes only while a button is DOWN, so a player steering \
                 with the stick spends three ticks under the boards instead of \
                 three seconds — the exact shape Jon reported",
            );
            assert!(
                !policy.roots,
                "{id} would root her for the whole held beat, and travelling is what \
                 the beat is FOR",
            );
            assert!(
                (policy.max_hold_s - MAX_UNDER_S).abs() < 1e-6,
                "{id} may be held for {}s, wanted the authored ceiling {MAX_UNDER_S}s",
                policy.max_hold_s,
            );
        }
    }

    /// The emergence hits whoever is on or above the door.
    ///
    /// The arms straddle the door so a column is told apart from a puddle: a body
    /// on the boards and one a body-height above are inside; a body a stage-width
    /// away is not.
    #[test]
    fn coming_up_through_the_boards_is_a_strike_over_the_door() {
        let set = crate::authored_movesets::shipped("performer");
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in the table");
        let firework = trap
            .windows
            .iter()
            .find(|w| matches!(w.tag, ambition_entity_catalog::WindowTag::Active))
            .expect("the trap ends in a strike");
        assert!(
            firework.start_s >= SURFACE_AT_S && firework.start_s < TRAP_ENDS_S,
            "the firework starts at {}s, which is not when she surfaces ({SURFACE_AT_S}s)",
            firework.start_s,
        );
        let volume = firework.volumes.first().expect("the strike has a volume");
        assert!(
            volume.damage > 0,
            "a firework that hits for nothing is a puff"
        );
        let ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents,
        } = volume.shape
        else {
            panic!("the firework is a rect");
        };
        assert_eq!(
            offset.0, 0.0,
            "the door is UNDER her, so the column may not lean the way she faces",
        );
        for (point, inside, what) in [
            ((0.0, 24.0), true, "standing on the boards"),
            ((0.0, -48.0), true, "a body-height above the door"),
            ((0.0, -120.0), false, "well above the door"),
            ((120.0, 0.0), false, "a stage-width away"),
        ] {
            let hit = (point.0 - offset.0).abs() <= half_extents.0
                && (point.1 - offset.1).abs() <= half_extents.1;
            assert_eq!(
                hit,
                inside,
                "a body {what} is {}inside the firework",
                if hit { "" } else { "not " }
            );
        }
    }

    /// The arms straddle both edges, so "the gap is open" is told apart from
    /// "every window is 1.0". Dropping through and climbing out stay committed.
    #[test]
    fn the_trap_roots_her_at_both_ends_and_lets_her_steer_between_them() {
        let set = crate::authored_movesets::shipped("performer");
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in the table");
        for (t, want, what) in [
            (SINK_AT_S * 0.5, 0.0, "dropping through the boards"),
            (SINK_AT_S + 0.01, 1.0, "just under"),
            (SURFACE_AT_S - 0.01, 1.0, "about to come up"),
            (SURFACE_AT_S + 0.01, 0.0, "climbing back out"),
        ] {
            assert!(
                (trap.motion_scale_at(t) - want).abs() < 1e-4,
                "at {t}s ({what}) the trap allows {} of her steering, wanted {want}",
                trap.motion_scale_at(t),
            );
        }
    }

    /// Containment is the design, so it is the guard. The sleep must reach less
    /// far than the strike it rides on; otherwise it is a pure buff.
    #[test]
    fn her_monologue_sleeps_only_whoever_stood_closer_than_it_hits() {
        let set = crate::authored_movesets::shipped("performer");
        let speech = set
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special");

        let sleep = speech
            .events
            .iter()
            .find_map(|event| match &event.kind {
                ambition_entity_catalog::MoveEventKind::Effect(effect)
                    if effect.key == ambition_entity_catalog::smash_sleep::SLEEP =>
                {
                    Some((
                        event.at_s,
                        effect
                            .params
                            .hydrate::<ambition_entity_catalog::smash_sleep::SleepParams>()
                            .expect("sleep params hydrate"),
                    ))
                }
                _ => None,
            })
            .expect("the monologue sings");
        let (at_s, sleep) = sleep;

        // The strike, untouched: still a hit that hurts, still fixed knockback.
        let volume = speech
            .windows
            .iter()
            .flat_map(|window| window.volumes.iter())
            .find(|volume| volume.damage > 0)
            .expect("she still addresses the room");
        assert_eq!(volume.damage, 6, "the speech itself is unchanged");

        // Corner by corner, from each box's own centre: the sleep is centred on
        // her, the strike is offset in front.
        let (sx, sy) = sleep.half_extents;
        let ambition_entity_catalog::VolumeShape::Rect {
            offset,
            half_extents,
        } = volume.shape
        else {
            panic!("her speech is a rect");
        };
        let (hx, hy) = half_extents;
        let (ox, oy) = offset;
        assert!(
            -sx >= ox - hx && sx <= ox + hx && -sy >= oy - hy && sy <= oy + hy,
            "the sleep box (±{sx}, ±{sy} about her) leaves the strike box \
             ({hx}×{hy} at {ox},{oy}), so she would sleep somebody she does not hit"
        );

        // It fires late, so targets had time to step away.
        let active = speech
            .windows
            .iter()
            .find(|window| window.volumes.iter().any(|volume| volume.damage > 0))
            .expect("an active window");
        assert!(
            at_s >= active.start_s,
            "the sleep fires at {at_s}s, before her swing opens at {}s",
            active.start_s,
        );
    }

    /// The speech roots her too. [`the_monologue`] builds on `strike`, which
    /// authors `motion_scale: 1.0` on every window, so without the root she could
    /// walk away while her targets are pinned.
    #[test]
    fn the_monologue_holds_the_speaker_as_well_as_the_room() {
        let set = crate::authored_movesets::shipped("performer");
        let speech = set
            .moves
            .iter()
            .find(|m| m.id == "performer_monologue")
            .expect("her neutral special");
        for t in [0.05, 0.3, 0.5] {
            assert_eq!(
                speech.motion_scale_at(t),
                0.0,
                "at {t}s she can still steer out of her own speech"
            );
        }
    }

    /// The flyline is not a teleport. Jon: *"It is not a teleport and should not
    /// get the teleport sound."*
    ///
    /// Assert on the technique, not on the cue. `apply_authored_teleports` emits
    /// `PLAYER_BLINK` at every transit, so any move authored through
    /// `author_teleport` makes the sound whatever its timeline says. The move
    /// must never reach that executor. The exemption in
    /// `director_teleport_blink.rs` relies on this.
    #[test]
    fn the_wire_is_a_flyline_and_never_reaches_the_teleport_executor() {
        use ambition_entity_catalog::smash_flyline::FLYLINE;
        use ambition_entity_catalog::smash_teleport::TELEPORT;
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        assert!(
            wire.events.iter().any(|e| matches!(
                &e.kind,
                MoveEventKind::Effect(effect) if effect.key == FLYLINE
            )),
            "the up-B authors no flyline at all, so it lifts nobody"
        );
        for event in &wire.events {
            match &event.kind {
                MoveEventKind::Effect(effect) => assert_ne!(
                    effect.key, TELEPORT,
                    "the wire is a flyline, not a teleport with a longer clip"
                ),
                MoveEventKind::Sfx { cue } => assert!(
                    !cue.contains("blink"),
                    "the wire plays `{cue}`; it is rope and pulley"
                ),
                MoveEventKind::Vfx { effect, .. } => assert!(
                    !effect.contains("glint"),
                    "the wire draws `{effect}`, which is the Director's blink"
                ),
                _ => {}
            }
        }
    }

    /// The move must outlast the lift it authors. The wire's clock belongs to the
    /// kernel (`WireState::lift_remaining_s`), and `author_flyline` cannot see the
    /// rest of the timeline. A 0.55s lift on a 0.46s move would leave her flown
    /// with no animation and her Recovery already over.
    #[test]
    fn the_lift_fits_inside_the_move_that_authors_it() {
        use ambition_entity_catalog::smash_flyline::{FlylineParams, FLYLINE};
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        let (at_s, params) = wire
            .events
            .iter()
            .find_map(|ev| match &ev.kind {
                MoveEventKind::Effect(effect) if effect.key == FLYLINE => Some((
                    ev.at_s,
                    effect
                        .params
                        .hydrate::<FlylineParams>()
                        .expect("flyline params hydrate"),
                )),
                _ => None,
            })
            .expect("the up-B authors a flyline");
        assert!(
            at_s + params.lift_s <= wire.duration_s,
            "the wire catches at {at_s}s and reels for {}s, past the move's own \
             {}s",
            params.lift_s,
            wire.duration_s
        );
        // The rope must outlast the rise, or the winch reels past its pulley and
        // stops at the kernel's minimum length.
        assert!(
            params.rope_length > params.rise,
            "a {}px rope cannot deliver a {}px lift",
            params.rope_length,
            params.rise
        );
        // The carry must be slower than the average climb, or the winch would have
        // to accelerate into the release. `apply_authored_flylines` clamps in that
        // case.
        assert!(
            params.release_rise < 2.0 * params.rise / params.lift_s,
            "release_rise {} is faster than the climb it is supposed to end",
            params.release_rise
        );
    }

    /// She steers on the wire. `hitless_special` authors `motion_scale: 0.0` on
    /// Startup and Recovery, the wire reads the damped stick, and
    /// `motion_scale_at` folds with `min`, so a Recovery window from the catch
    /// would zero the swing. Other tests would not notice.
    ///
    /// The arm straddles the catch: the beat before it must still be rooted, or
    /// she could walk out of the special.
    #[test]
    fn she_steers_the_swing_and_is_rooted_either_side_of_it() {
        let set = crate::authored_movesets::shipped("performer");
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        for t in [
            WIRE_AT_S + 0.01,
            WIRE_AT_S + LIFT_S * 0.5,
            WIRE_RELEASES_S - 0.01,
        ] {
            assert!(
                wire.motion_scale_at(t) > 0.0,
                "at {t}s her swing is multiplied by {}",
                wire.motion_scale_at(t)
            );
        }
        assert_eq!(
            wire.motion_scale_at(WIRE_AT_S - 0.01),
            0.0,
            "she can walk out of her own startup"
        );
        assert_eq!(
            wire.motion_scale_at(WIRE_RELEASES_S + 0.01),
            0.0,
            "the landing lag is not lag"
        );
    }

    /// In the air the down special is only a puff of smoke.
    ///
    /// The assertions check what is absent. A trapdoor beat here would be refused
    /// by the engine and leave the rest of the timeline running (including the
    /// three-second freeze), so she would hang in mid-air.
    #[test]
    fn the_airborne_form_is_a_puff_of_smoke_and_no_trapdoor_at_all() {
        use ambition_entity_catalog::smash_trapdoor::TRAPDOOR;
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        let air = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor_air")
            .expect("her airborne down special");

        assert!(
            air.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Vfx { effect, .. } if effect == SMOKE_VFX
            )),
            "the airborne press makes no smoke, which leaves it a dead button"
        );
        assert!(
            !air.events.iter().any(|ev| matches!(
                &ev.kind,
                MoveEventKind::Effect(effect) if effect.key == TRAPDOOR
            )),
            "the airborne form authors a trapdoor beat; there is no floor to cut \
             a hatch in, and the engine refusing it leaves the rest of this \
             timeline running over a body standing in the air"
        );
        assert!(
            air.smash_charge.is_none(),
            "the airborne form authors a hold; a freeze whose beat never happens \
             is three seconds of a fighter stuck in mid-air"
        );
        assert!(
            air.duration_s <= 0.5,
            "the failing trick lasts {}s — it buys nothing, so it may not cost \
             the opponent a read that long",
            air.duration_s
        );
    }

    /// The smoke is on both forms, on the first frame. It is the misdirection:
    /// smoke only on the grounded form would tell the opponent which form it is.
    ///
    /// It comes before the boards (`DOOR_OPENS_S`), so it hides the hole instead
    /// of revealing it.
    #[test]
    fn the_smoke_goes_off_on_the_first_frame_of_both_forms() {
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        for id in ["performer_trapdoor", "performer_trapdoor_air"] {
            let mv = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her table"));
            let smoke_at = mv
                .events
                .iter()
                .find_map(|ev| match &ev.kind {
                    MoveEventKind::Vfx { effect, .. } if effect == SMOKE_VFX => Some(ev.at_s),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("`{id}` makes no smoke"));
            assert_eq!(
                smoke_at, 0.0,
                "`{id}` puffs at {smoke_at}s; the misdirection is the FIRST thing \
                 that happens or it is not misdirection"
            );
            assert!(
                smoke_at < DOOR_OPENS_S,
                "`{id}` puffs at {smoke_at}s, at or after the boards give at \
                 {DOOR_OPENS_S}s — that is smoke revealing a hole, not hiding one"
            );
        }
    }

    /// Every effect she names is a shipped row. `is_authored_effect` reads the
    /// rows from the baked manifests, so this asks what the renderer asks.
    ///
    /// This only checks that the art exists, not that it is the right art. A real
    /// explosion row used as smoke would pass. See
    /// [`the_wire_names_nothing_belonging_to_the_trapdoor`] for one such case.
    #[test]
    fn every_effect_she_names_is_a_row_that_ships() {
        let set = crate::authored_movesets::shipped("performer");
        // Collect problems across every move, then assert once, so one run reports
        // every move that references a renamed effect.
        let mut problems: Vec<String> = Vec::new();
        for m in &set.moves {
            problems.extend(m.presentation_problems(
                ambition_platformer2d::sprite_sheet::fx::is_authored_effect,
            ));
        }
        // Before the palette checks below: a renamed effect fails those too, with a
        // less helpful message.
        assert!(problems.is_empty(), "{problems:?}");
    }

    /// Her up-B names no part of her down-B. Jon: *"her up-b uses the trap
    /// door, and I don't think it should."*
    ///
    /// `FlylineParams::vfx` was a required `String`, so the wire got the other
    /// special's furniture. It is now optional.
    #[test]
    fn the_wire_names_nothing_belonging_to_the_trapdoor() {
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        let wire = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up special");
        for event in &wire.events {
            if let MoveEventKind::Vfx { effect, .. } = &event.kind {
                assert!(
                    !effect.contains("trapdoor") && !effect.contains("door"),
                    "the wire draws `{effect}`, which belongs to the hatch"
                );
            }
        }
    }

    /// The down slot is a pair. The archetype's down special is a
    /// `DownSpecial::ByPosture`, so `special_air_down` comes before
    /// `special_down` in the verb chain. Replacing only the grounded form sends
    /// an airborne press to the archetype's move.
    #[test]
    fn both_postures_of_her_down_special_are_the_trap() {
        let set = crate::authored_movesets::shipped("performer");
        for verb in ["special_down", "special_air_down"] {
            let bound = set.verbs.get(verb).map(String::as_str);
            assert!(
                matches!(bound, Some(id) if id.starts_with("performer_trapdoor")),
                "{verb} must be the trap, saw {bound:?}"
            );
            let id = bound.unwrap();
            assert!(
                set.moves.iter().any(|m| m.id == id),
                "{verb} names `{id}`, which is not in the table"
            );
        }
    }

    /// One writer of exit velocity.
    ///
    /// An `Impulse` event at `SURFACE_AT_S` is applied inline in
    /// `advance_move_playback`, and the `smash.trapdoor` surfacing beat is a
    /// message handled later whose `TransitVelocity::Zero` overwrites it. So the
    /// timeline must not carry both. `TrapdoorParams::leap_speed` is the
    /// authority.
    #[test]
    fn the_leap_has_one_authority_and_it_is_the_surfacing_beat() {
        use ambition_entity_catalog::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_entity_catalog::MoveEventKind;
        let set = crate::authored_movesets::shipped("performer");
        let trap = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("the trap is in her table");

        assert!(
            !trap
                .events
                .iter()
                .any(|e| matches!(e.kind, MoveEventKind::Impulse { .. })),
            "the trap authors an Impulse again. It lands on the same frame as \
             the surfacing beat, which writes velocity from a LATER system, so \
             the impulse is silently deleted — put the launch on \
             `TrapdoorParams::leap_speed` instead"
        );

        let surfacing: Vec<TrapdoorParams> = trap
            .events
            .iter()
            .filter_map(|e| match &e.kind {
                MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => {
                    effect.params.clone().hydrate().ok()
                }
                _ => None,
            })
            .filter(|p: &TrapdoorParams| !p.submerge)
            .collect();
        assert_eq!(
            surfacing.len(),
            1,
            "she comes back up exactly once; {} beats claim to",
            surfacing.len()
        );
        assert!(
            surfacing[0].leap_speed > 0.0,
            "she surfaces at {} px/s, so she steps up out of the hole instead of \
             leaping — the move's whole payoff is the space the leap buys",
            surfacing[0].leap_speed
        );
    }

    /// The archetype's down special is removed, not left unreachable, where every
    /// census over `moves` would count it.
    #[test]
    fn the_archetypes_down_special_does_not_linger() {
        let set = crate::authored_movesets::shipped("performer");
        for stale in ["performer_low_arc", "performer_falling_edge"] {
            assert!(
                !set.moves.iter().any(|m| m.id == stale),
                "`{stale}` is the archetype's down special and must not survive \
                 the replacement"
            );
        }
    }

    /// She goes under and comes back: two beats of `smash.trapdoor`. Without the
    /// second she stays in `BodyMode::Submerged` for the rest of the match.
    /// `author_trapdoor` cannot see the rest of the timeline, so the guard is
    /// here.
    #[test]
    fn the_trap_puts_her_under_the_stage_and_brings_her_back() {
        use ambition_entity_catalog::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        // Grounded form only: the airborne one goes nowhere.
        for id in ["performer_trapdoor"] {
            let mv = set
                .moves
                .iter()
                .find(|m| m.id == id)
                .unwrap_or_else(|| panic!("`{id}` is in her table"));
            let beats: Vec<(f32, TrapdoorParams)> = mv
                .events
                .iter()
                .filter_map(|ev| match &ev.kind {
                    MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => Some((
                        ev.at_s,
                        effect.params.hydrate().expect("trapdoor params hydrate"),
                    )),
                    _ => None,
                })
                .collect();
            assert_eq!(beats.len(), 2, "`{id}` is a round trip, not a one-way door");
            assert!(beats[0].1.submerge, "the first beat drops her through");
            assert!(!beats[1].1.submerge, "the second beat brings her back");
            assert!(
                beats[0].0 < beats[1].0,
                "`{id}` surfaces at {}s before it submerges at {}s",
                beats[1].0,
                beats[0].0
            );
            assert!(
                beats[1].0 < mv.duration_s,
                "`{id}` surfaces after the move ends, so it never surfaces"
            );
            assert!(
                beats[1].1.surface_reach > 0.0,
                "`{id}` comes up through a FLOOR; a zero reach disables the \
                 search and drops her wherever she happened to be"
            );
        }
    }

    /// The trap is not a blink. Jon: *"It looks like a blink… It's not a
    /// blink."* So no `smash.teleport` and no Director's `four_point_glint`.
    #[test]
    fn the_trap_carries_no_teleport_and_no_blink_dressing() {
        use ambition_entity_catalog::smash_teleport::TELEPORT;
        use ambition_entity_catalog::MoveEventKind;

        let set = crate::authored_movesets::shipped("performer");
        for id in ["performer_trapdoor", "performer_trapdoor_air"] {
            let mv = set.moves.iter().find(|m| m.id == id).expect("her trap");
            for event in &mv.events {
                match &event.kind {
                    MoveEventKind::Effect(effect) => assert_ne!(
                        effect.key, TELEPORT,
                        "`{id}` is a trapdoor, not a teleport with a longer clip"
                    ),
                    MoveEventKind::Sfx { cue } => assert!(
                        !cue.contains("blink"),
                        "`{id}` plays `{cue}`; the trap is carpentry"
                    ),
                    MoveEventKind::Vfx { effect, .. } => assert!(
                        !effect.contains("glint"),
                        "`{id}` draws `{effect}`, which is the Director's blink"
                    ),
                    _ => {}
                }
            }
        }
    }

    /// She cannot be hit under the stage, stated on the timeline as well as by
    /// the mode. The mode protects her; the window is what the cancel and scoring
    /// layers read.
    #[test]
    fn the_timeline_says_she_is_untouchable_for_the_whole_trip() {
        use ambition_entity_catalog::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
        use ambition_entity_catalog::{MoveEventKind, WindowTag};

        let set = crate::authored_movesets::shipped("performer");
        let mv = set
            .moves
            .iter()
            .find(|m| m.id == "performer_trapdoor")
            .expect("her trap");
        let beat = |submerge: bool| -> f32 {
            mv.events
                .iter()
                .find_map(|ev| match &ev.kind {
                    MoveEventKind::Effect(effect) if effect.key == TRAPDOOR => {
                        let params: TrapdoorParams =
                            effect.params.hydrate().expect("trapdoor params");
                        (params.submerge == submerge).then_some(ev.at_s)
                    }
                    _ => None,
                })
                .expect("both beats")
        };
        let (under, up) = (beat(true), beat(false));
        assert!(
            mv.windows.iter().any(|w| matches!(w.tag, WindowTag::Invuln)
                && w.start_s <= under + 1e-4
                && w.end_s >= up - 1e-4),
            "no Invuln window covers {under}s..{up}s, the span she is not in the world"
        );
    }

    /// The recovery still costs an airtime. `UpSpecial::Standard` stamps
    /// `gates.recovery` on the move it lowers; a replacement inserted after that
    /// must carry the cost, or she gets unlimited flight.
    #[test]
    fn the_flyline_spends_the_airtimes_recovery() {
        let set = crate::authored_movesets::shipped("performer");
        let up = set
            .moves
            .iter()
            .find(|m| m.id == "performer_curtain_call")
            .expect("her up-B is in the table");
        assert_ne!(
            up.gates.recovery,
            ambition_entity_catalog::RecoveryUse::None,
            "an up-B that costs nothing is flight"
        );
    }
}
