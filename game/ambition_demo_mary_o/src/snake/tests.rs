//! Unit tests for the Solid Snake shell state machine. The choreography is a pure
//! function ([`super::step_snake_shell`]), so its whole lifecycle is checked here
//! without a running app — the one thing headless can't check is how it LOOKS.

use super::*;
use ambition::sprite_sheet::character::CharacterAnim;

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
        side_kick: Some(1.0),
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
        side_kick: Some(-1.0),
        ..Default::default()
    };
    let fx = step(SnakeShell::Boxed(BOXED_S), kick_left);
    assert!(matches!(fx.phase, SnakeShell::Sliding { dir, .. } if dir == -1.0));
    assert_eq!(fx.vel_x, Some(-SHELL_SLIDE_SPEED));
}

/// **Kicking a shell must not hurt the kicker.** The kick comes FROM the side, so
/// the shell starts moving while it still overlaps the player — the grace window is
/// what stands between "kick a shell" and "kick a shell and take a hit for it". It
/// burns down on the shell's own clock and expires.
#[test]
fn a_freshly_kicked_shell_cannot_hurt_the_kicker_until_its_grace_expires() {
    let kick = ShellInputs {
        side_kick: Some(1.0),
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

/// **Standing on a body must never be a kick.** THE bug: a player standing over a
/// resting shell was classified as a side touch, which kicked the shell out from
/// under their own feet and then registered that same overlap as a side HIT — over
/// and over, for as long as they stood there. From the top a shell is re-seated and
/// bounces you; it never launches.
#[test]
fn standing_on_a_resting_shell_bounces_it_never_kicks_it() {
    let fx = step(SnakeShell::Boxed(1.0), stomp());
    assert!(
        matches!(fx.phase, SnakeShell::Boxed(t) if t > 1.0),
        "a bounce re-seats the shell (its sit timer restarts) instead of launching it"
    );
    assert!(fx.just_squashed, "and bounces the player up off it");
    assert!(!fx.just_kicked, "a stomp is NEVER a kick");
    assert_eq!(fx.vel_x, Some(0.0), "it does not slide out from under you");
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
