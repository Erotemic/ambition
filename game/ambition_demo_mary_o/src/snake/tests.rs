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
/// sets its slide velocity on the spot — never a stationary "sliding" shell.
#[test]
fn kicking_a_boxed_shell_sends_it_sliding_away() {
    let kick_right = ShellInputs {
        side_kick: Some(1.0),
        ..Default::default()
    };
    let fx = step(SnakeShell::Boxed(BOXED_S), kick_right);
    assert_eq!(fx.phase, SnakeShell::Sliding(1.0));
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
    assert_eq!(fx.phase, SnakeShell::Sliding(-1.0));
    assert_eq!(fx.vel_x, Some(-SHELL_SLIDE_SPEED));
}

/// A sliding shell keeps its speed, and reverses when the world blocks it (so it
/// ricochets down a corridor instead of parking against a wall).
#[test]
fn a_sliding_shell_holds_speed_and_bounces_off_walls() {
    let fx = step(SnakeShell::Sliding(1.0), ShellInputs::default());
    assert_eq!(fx.phase, SnakeShell::Sliding(1.0));
    assert_eq!(fx.vel_x, Some(SHELL_SLIDE_SPEED));

    let blocked = ShellInputs {
        blocked: true,
        ..Default::default()
    };
    let fx = step(SnakeShell::Sliding(1.0), blocked);
    assert_eq!(
        fx.phase,
        SnakeShell::Sliding(-1.0),
        "a wall flips its direction"
    );
    assert_eq!(fx.vel_x, Some(-SHELL_SLIDE_SPEED));
}

/// Stomping or bumping a sliding shell stops it dead — it becomes a fresh boxed
/// shell you can re-kick, which is what makes a shell a tool and not a runaway.
#[test]
fn a_sliding_shell_stops_dead_when_stomped_or_bumped() {
    let fx = step(SnakeShell::Sliding(1.0), stomp());
    assert!(matches!(fx.phase, SnakeShell::Boxed(_)));
    assert_eq!(fx.vel_x, Some(0.0));

    let bump = ShellInputs {
        side_kick: Some(-1.0),
        ..Default::default()
    };
    let fx = step(SnakeShell::Sliding(1.0), bump);
    assert!(
        matches!(fx.phase, SnakeShell::Boxed(_)),
        "a side bump also stops it"
    );
    assert_eq!(fx.vel_x, Some(0.0));
}
