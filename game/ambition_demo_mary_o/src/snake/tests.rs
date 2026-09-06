//! Unit tests for the Solid Snake shell state machine. The choreography is a pure
//! function ([`super::step_snake_shell`]), so its whole lifecycle is checked here
//! without a running app — the one thing headless can't check is how it LOOKS.

use super::*;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

const DT: f32 = 1.0 / 60.0;

fn step(phase: SnakeShell, inputs: ShellInputs) -> ShellEffects {
    step_snake_shell(phase, DT, inputs)
}

fn stomp() -> ShellInputs {
    ShellInputs {
        stomped: true,
        ..Default::default()
    }
}

/// A stomp on a walker begins the in-place withdraw: it bounces the player, plays
/// the `retreat` row, and the SAME (still-alive) body becomes an inert shell — no
/// death, no despawn, no separate shell entity.
#[test]
fn a_stomp_starts_an_in_place_withdraw() {
    let fx = step(SnakeShell::Walking, stomp());
    assert!(matches!(fx.phase, SnakeShell::Retreating(_)));
    assert_eq!(fx.anim, Some(CharacterAnim::Retreat));
    assert!(
        fx.just_squashed,
        "the stomp bounces the player and pops dust"
    );
    assert!(
        !fx.alive,
        "a withdrawing snake is an inert shell: frozen and untouchable"
    );
    assert_eq!(
        fx.vel_x,
        Some(0.0),
        "it stops walking the instant it is stomped"
    );
}

/// An un-stomped walker is just a walker: no pose pin, alive, its own physics.
#[test]
fn an_untouched_walker_stays_a_normal_walker() {
    let fx = step(SnakeShell::Walking, ShellInputs::default());
    assert_eq!(fx.phase, SnakeShell::Walking);
    assert_eq!(fx.anim, None, "the shared picker chooses walk/idle");
    assert!(fx.alive);
    assert_eq!(
        fx.vel_x, None,
        "the brain drives a walker; the shell doesn't"
    );
}

/// Left alone, the whole cycle runs retreat → boxed → peek → emerge → walk, and
/// only the final beat turns the snake back into a live, moving threat.
#[test]
fn the_shell_cycle_runs_to_completion_and_revives_only_at_the_end() {
    // Retreat settles into the boxed pose.
    let mut phase = SnakeShell::Retreating(0.0);
    let fx = step(phase, ShellInputs::default());
    assert!(matches!(fx.phase, SnakeShell::Boxed(_)));
    assert_eq!(fx.anim, Some(CharacterAnim::ShellIdle));
    assert!(!fx.alive);

    // Boxed times out into a peek.
    phase = SnakeShell::Boxed(0.0);
    let fx = step(phase, ShellInputs::default());
    assert!(matches!(fx.phase, SnakeShell::Peeking(_)));
    assert_eq!(fx.anim, Some(CharacterAnim::Peek));
    assert!(!fx.alive, "peeking is still an inert shell");

    // Peek climbs into emerge.
    phase = SnakeShell::Peeking(0.0);
    let fx = step(phase, ShellInputs::default());
    assert!(matches!(fx.phase, SnakeShell::Emerging(_)));
    assert_eq!(fx.anim, Some(CharacterAnim::Emerge));
    assert!(
        !fx.alive,
        "emerging is still an inert shell until the last beat"
    );

    // Emerge finishes: a live walker again, pose pin dropped.
    phase = SnakeShell::Emerging(0.0);
    let fx = step(phase, ShellInputs::default());
    assert_eq!(fx.phase, SnakeShell::Walking);
    assert_eq!(fx.anim, None, "back to shared picking");
    assert!(
        fx.alive,
        "only emerging fully turns it back into a live threat"
    );
    assert_eq!(fx.vel_x, None);
}

/// A boxed shell that is not disturbed stays boxed while its timer runs — it does
/// not peek early.
#[test]
fn a_boxed_shell_waits_out_its_timer_before_peeking() {
    let fx = step(SnakeShell::Boxed(BOXED_S), ShellInputs::default());
    assert!(
        matches!(fx.phase, SnakeShell::Boxed(t) if t < BOXED_S && t > 0.0),
        "still boxed, timer ticking"
    );
    assert_eq!(fx.anim, Some(CharacterAnim::ShellIdle));
}

/// Kicking a boxed shell launches it AWAY from the player (dir from the kick) and
/// sets its slide velocity on the spot — never a stationary "sliding" shell. The
/// kick also arms the grace window, because you kick from the SIDE and are still
/// standing inside the shell it just launched.
#[test]
fn kicking_a_boxed_shell_sends_it_sliding_away() {
    let kick_right = ShellInputs {
        kick_dir: Some(1.0),
        ..Default::default()
    };
    let fx = step(SnakeShell::Boxed(BOXED_S), kick_right);
    assert_eq!(
        fx.phase,
        SnakeShell::Sliding {
            dir: 1.0,
            grace: KICK_GRACE_S
        }
    );
    assert!(fx.just_kicked);
    assert_eq!(
        fx.vel_x,
        Some(SHELL_SLIDE_SPEED),
        "kicked rightward at full speed"
    );

    let kick_left = ShellInputs {
        kick_dir: Some(-1.0),
        ..Default::default()
    };
    let fx = step(SnakeShell::Boxed(BOXED_S), kick_left);
    assert!(matches!(fx.phase, SnakeShell::Sliding { dir, .. } if dir == -1.0));
    assert_eq!(fx.vel_x, Some(-SHELL_SLIDE_SPEED));
}

/// Kicking a shell must not hurt the kicker. The kick comes FROM the side, so
/// the shell starts moving while it still overlaps the player — the grace window is
/// what stands between "kick a shell" and "kick a shell and take a hit for it". It
/// burns down on the shell's own clock and expires.
#[test]
fn a_freshly_kicked_shell_cannot_hurt_the_kicker_until_its_grace_expires() {
    let kick = ShellInputs {
        kick_dir: Some(1.0),
        ..Default::default()
    };
    let mut phase = step(SnakeShell::Boxed(BOXED_S), kick).phase;
    let grace_of = |p| match p {
        SnakeShell::Sliding { grace, .. } => grace,
        other => panic!("expected a sliding shell, got {other:?}"),
    };
    assert!(
        grace_of(phase) > 0.0,
        "the kick tick itself is graced — this is the hit the kicker used to take"
    );

    // It expires on its own within the window, and never goes negative. (`+ 1` tick
    // for the float tail of subtracting `dt` off the window repeatedly.)
    let ticks = (KICK_GRACE_S / DT).ceil() as usize + 1;
    for _ in 0..ticks {
        phase = step(phase, ShellInputs::default()).phase;
    }
    assert_eq!(
        grace_of(phase),
        0.0,
        "past the window the shell is armed again — a ricochet back into you is a real threat"
    );
}

/// A sliding shell keeps its speed, and reverses when the world blocks it (so it
/// ricochets down a corridor instead of parking against a wall).
#[test]
fn a_sliding_shell_holds_speed_and_bounces_off_walls() {
    let armed = SnakeShell::Sliding {
        dir: 1.0,
        grace: 0.0,
    };
    let fx = step(armed, ShellInputs::default());
    assert_eq!(fx.phase, armed);
    assert_eq!(fx.vel_x, Some(SHELL_SLIDE_SPEED));

    let blocked = ShellInputs {
        blocked: true,
        ..Default::default()
    };
    let fx = step(armed, blocked);
    assert!(
        matches!(fx.phase, SnakeShell::Sliding { dir, .. } if dir == -1.0),
        "a wall flips its direction"
    );
    assert_eq!(fx.vel_x, Some(-SHELL_SLIDE_SPEED));
}

/// A stomp from ABOVE stops a running shell dead — it becomes a fresh boxed shell
/// you can re-kick — and bounces the stomper, exactly like stomping a walker. This
/// is the "stop the runaway" tech, and it is SAFE (no player damage).
#[test]
fn a_stomp_from_above_stops_a_sliding_shell_and_bounces() {
    let fx = step(
        SnakeShell::Sliding {
            dir: 1.0,
            grace: 0.0,
        },
        stomp(),
    );
    assert!(matches!(fx.phase, SnakeShell::Boxed(_)), "stomp stops it");
    assert_eq!(fx.vel_x, Some(0.0));
    assert!(
        fx.just_squashed,
        "stomping a moving shell bounces you off it, like stomping a walker"
    );
}

/// Landing on a resting shell kicks it, and bounces you.
///
/// That made a stomped shell stay put — so a shell under a ?-block trapped her in an endless
/// bounce, which the classic never does, and is `KICK_GRACE_S`, which did not exist then: a
/// freshly kicked shell cannot hurt the player at all, so the kick-then-hit loop cannot form.
#[test]
fn landing_on_a_resting_shell_kicks_it_out_from_under_you() {
    let fx = step(
        SnakeShell::Boxed(1.0),
        ShellInputs {
            kick_dir: Some(1.0),
            ..stomp()
        },
    );
    assert!(
        matches!(fx.phase, SnakeShell::Sliding { dir, grace }
            if dir > 0.0 && grace > 0.0),
        "a stomped shell slides away, with the grace that stops it hitting you back"
    );
    assert!(
        fx.just_squashed,
        "and it still bounces the player up off it"
    );
    assert!(fx.just_kicked, "and it is a kick, so it sounds like one");
    assert!(
        fx.vel_x.is_some_and(|v| v > 0.0),
        "it really moves, rather than sitting under her for another bounce"
    );
}

/// A shell that bounces off a wall is armed again, even mid-grace. The grace
/// only ever meant "it is still inside the person who kicked it"; one that has
/// turned around is coming back at them, and that is a hit.
#[test]
fn a_wall_bounce_arms_a_freshly_kicked_shell() {
    let fx = step(
        SnakeShell::Sliding {
            dir: 1.0,
            grace: 0.25,
        },
        ShellInputs {
            blocked: true,
            ..Default::default()
        },
    );
    assert!(
        matches!(fx.phase, SnakeShell::Sliding { dir, grace }
            if dir < 0.0 && grace == 0.0),
        "it reverses AND spends its grace, so the return trip can hurt her"
    );
}

/// A shell already RUNNING is stopped by a stomp rather than re-kicked — that is
/// the classic "catch the runaway" tech, and it is why the kick above is scoped
/// to a resting shell rather than to every stomp.
#[test]
fn stomping_a_running_shell_still_stops_it() {
    let fx = step(
        SnakeShell::Sliding {
            dir: 1.0,
            grace: 0.0,
        },
        ShellInputs {
            kick_dir: None,
            ..stomp()
        },
    );
    assert!(matches!(fx.phase, SnakeShell::Boxed(_)), "it re-seats");
    assert_eq!(fx.vel_x, Some(0.0), "and stops dead");
}

/// A stomp pre-empts the LATE phases too. Landing on a snake that is peeking or
/// climbing out shoves it straight back into its shell — it cannot finish emerging
/// under the player's feet and become a live contact threat on the same tick.
#[test]
fn a_stomp_shoves_a_peeking_or_emerging_snake_back_into_its_shell() {
    for phase in [SnakeShell::Peeking(0.0), SnakeShell::Emerging(0.0)] {
        let fx = step(phase, stomp());
        assert!(
            matches!(fx.phase, SnakeShell::Boxed(_)),
            "{phase:?} + stomp must re-seat the shell"
        );
        assert!(!fx.alive, "and it must NOT come back as a live threat");
        assert!(fx.just_squashed, "the stomper is bounced off it");
    }
}

// (The top/side geometry itself is proven in `crate::stomp`, the ONE authority
// both enemies read.)

/// THE WALKING SNAKE AND THE BOXED SNAKE RESCALE TOGETHER.
///
/// in-the-box form are consistent if we ever change the scale of the snake."*
///
/// it asserts PROPORTIONALITY rather than the sizes themselves, on purpose:
/// the numbers are the art's to choose and change, and a test that pinned them
/// would fail on an ordinary redraw. What may not change is that the two forms
/// answer to the same scale.
#[test]
fn both_snake_forms_follow_one_scale() {
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

    let at = |wpp: f32, anim: CharacterAnim| {
        ambition_platformer2d::character_sprites::posed_body_geometry(
            super::SNAKE_SHEET_TARGET,
            anim,
            wpp,
        )
        .map(|g| g.collision)
    };

    let base = super::snake_world_per_pixel();
    let (Some(walk), Some(boxed)) = (
        at(base, CharacterAnim::Idle),
        at(base, CharacterAnim::ShellIdle),
    ) else {
        // No baked art (`--no-assets`): there is no geometry to be consistent
        // about. Skipping is honest here BECAUSE the assertion below is about a
        // relationship between two measurements, not about their presence — and
        // a fixture with no sheet cannot produce either one.
        //
        // VERIFIED NOT TAKEN in this tree by making it panic and
        // watching the test still pass — the check below really runs, rather
        // than skipping the way three other checks in this session did.
        return;
    };
    let (Some(walk2), Some(boxed2)) = (
        at(base * 2.0, CharacterAnim::Idle),
        at(base * 2.0, CharacterAnim::ShellIdle),
    ) else {
        panic!("the sheet resolved at one scale and not at twice it");
    };

    // The premise: the two forms are actually different boxes, or "they agree"
    // is vacuous.
    assert!(
        (walk - boxed).length() > 1e-3,
        "the walking and boxed forms measure the same box ({walk:?}), so this \
         test would pass without them being related at all"
    );

    for (name, one, two) in [("walking", walk, walk2), ("boxed", boxed, boxed2)] {
        assert!(
            (two.x - one.x * 2.0).abs() < 1e-3 && (two.y - one.y * 2.0).abs() < 1e-3,
            "the {name} form does not follow the scale: {one:?} at 1x became \
             {two:?} at 2x. Both forms must read the SAME world-per-pixel, or a \
             rescale moves one and leaves the other."
        );
    }
}

/// ⛔⛔ THE SILENT FALLBACK IS A SECOND SIZE, AND IT IS NEARLY DOUBLE.
///
/// `snake_world_per_pixel` falls back to `NO_SHEET = 0.35` when the baked sheet
/// does not resolve, and the derivation that replaced it produced **0.182** —
/// this function's own table says so. ⇒ A snake sized by the fallback is about
/// TWICE the size of one sized by the art, and nothing at runtime says which
/// happened.
///
/// Jon, 2026-09-05: *"the size of the snake has seemed to vary depending on the
/// global game state"*. That is exactly the shape of a fallback that fires in
/// some compositions and not others, so this pins that it does not fire where
/// the demo actually runs.
#[test]
fn the_snake_is_sized_by_its_art_and_not_by_the_no_sheet_fallback() {
    let geometry = ambition_platformer2d::character_sprites::posed_body_geometry(
        super::SNAKE_SHEET_TARGET,
        ambition_platformer2d::sprite_sheet::character::CharacterAnim::Idle,
        1.0,
    );
    assert!(
        geometry.is_some(),
        "the `{}` sheet did not resolve, so `snake_world_per_pixel` returns the \
         0.35 fallback instead of the ~0.182 the art derives — a snake about \
         twice the right size, with nothing at runtime saying so",
        super::SNAKE_SHEET_TARGET
    );
    let scale = super::snake_world_per_pixel();
    assert!(
        (scale - 0.35).abs() > 1e-6,
        "the scale IS the fallback value; see this test's doc"
    );
    // ⚠ A range, not an equality: the number moves when the art is redrawn, and
    // pinning it exactly would make an art change look like a defect. What must
    // never happen is landing back near the fallback.
    assert!(
        (0.10..0.30).contains(&scale),
        "world_per_pixel {scale} is outside the range the authored art implies"
    );
}

/// ⛔⛔ EVERY POSE THE SNAKE PLAYS MUST RESOLVE, for the same reason Mary-O has
/// this test — and Jon's report is about the SNAKE specifically.
///
/// Jon, 2026-09-05: *"the size of the snake has seemed to vary depending on the
/// global game state, so maybe something else exposed it."*
///
/// `sync_sprite_posed_bodies` resolves geometry PER ANIMATION and, on `None`,
/// `continue`s — leaving the body whatever box it last had. ⇒ An animation whose
/// sheet publishes no body metrics does not fall back to a default; it FREEZES
/// the previous pose's box. The state that "varies" would then be WHICH
/// ANIMATION IS PLAYING, which from the outside looks exactly like a size that
/// depends on game state.
///
/// ⚠ The snake is the sharp case because its poses are DELIBERATELY different
/// sizes — `withdrawing_into_the_shell_shrinks_the_body` pins the boxed form as
/// far narrower than the sprawled one. A frozen box is therefore not a subtle
/// error here; it is one silhouette wearing another's collision.
#[test]
fn every_pose_the_snake_plays_resolves() {
    use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

    for anim in [
        CharacterAnim::Idle,
        CharacterAnim::Emerge,
        CharacterAnim::Peek,
        CharacterAnim::Retreat,
        CharacterAnim::ShellIdle,
    ] {
        let resolved = ambition_platformer2d::character_sprites::posed_body_geometry(
            super::SNAKE_SHEET_TARGET,
            anim,
            super::snake_world_per_pixel(),
        );
        assert!(
            resolved.is_some(),
            "{anim:?} publishes no body metrics, so `sync_sprite_posed_bodies` \
             skips the snake on every frame it plays that pose and the body keeps \
             the PREVIOUS pose's box — a size that changes with which animation \
             is playing."
        );
    }
}
