//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

fn snap_with_target_at_x(target_x: f32) -> BrainSnapshot {
    let mut s = BrainSnapshot::idle();
    s.actor_pos = ae::Vec2::new(0.0, 0.0);
    s.target_pos = ae::Vec2::new(target_x, 0.0);
    s.actor_on_ground = true;
    s.target_alive = true;
    s
}

#[test]
fn idles_when_target_out_of_range() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let snap = snap_with_target_at_x(2000.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert_eq!(
        frame.locomotion.x, 0.0,
        "actor outside aggro_radius should not move"
    );
    assert!(!frame.melee_pressed);
}

/// A **relentless** duelist never idles out: with a live foe well beyond
/// `aggro_radius` it still runs toward it (chases) instead of going inert. This
/// is the fix for "the fight just stops when they get far apart" — a committed
/// 1v1 fighter pursues across any distance and re-acquires after a gravity fling.
#[test]
fn relentless_duelist_chases_a_foe_past_aggro_radius() {
    let cfg = crisp_duelist(); // DUELIST base → relentless = true
    assert!(cfg.relentless, "duelist is relentless");
    let mut state = SmashState::default();
    let actions = melee_actions();
    let snap = snap_with_target_at_x(cfg.aggro_radius + 400.0);
    let mut f = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
    assert!(
        f.locomotion.x > 0.0,
        "a relentless fighter chases a far live foe; got {:?}",
        f.locomotion
    );
}

/// …but an **ambient** (non-relentless) striker still idles out beyond its
/// sensing radius, so a patrol enemy doesn't chase the player across the world.
#[test]
fn ambient_striker_still_idles_past_aggro_radius() {
    let cfg = SmashCfg::STRIKER_DEFAULT; // relentless = false
    assert!(!cfg.relentless);
    let mut state = SmashState::default();
    let actions = melee_actions();
    let snap = snap_with_target_at_x(cfg.aggro_radius + 400.0);
    let mut f = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
    assert_eq!(
        f.locomotion.x, 0.0,
        "a non-relentless enemy idles out beyond aggro; got {:?}",
        f.locomotion
    );
}

/// **Stale-fight re-aggression**: after a long enough drought of its own offense,
/// a duelist drops the neutral-game patience and re-commits. Here the post-poke
/// reset window is armed (which normally suppresses the swing), but the
/// stale-fight push swings anyway — breaking a passive standoff instead of both
/// fighters waiting forever. Committing then resets the drought clock.
#[test]
fn stale_fight_forces_an_offensive_push_after_a_lull() {
    let cfg = crisp_duelist();
    let mut state = SmashState {
        rng_seed: 1,
        neutral_reset_timer: 0.3, // would normally suppress this tick's offense
        time_since_offense: cfg.stale_fight_s + 0.1, // drought exceeded
        ..Default::default()
    };
    let actions = melee_actions();
    let snap = snap_with_target_at_x(40.0); // inside attack_range
    let mut f = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
    assert!(
        f.melee_pressed,
        "the stale-fight push re-commits a swing despite the armed reset window"
    );
    assert_eq!(
        state.time_since_offense, 0.0,
        "committing offense resets the offense-drought clock"
    );
}

/// Without the lull, the duelist keeps its patience: the same armed reset window
/// suppresses the swing (no premature stale-fight push).
#[test]
fn no_stale_push_while_offense_is_fresh() {
    let cfg = crisp_duelist();
    let mut state = SmashState {
        rng_seed: 1,
        neutral_reset_timer: 0.3,
        time_since_offense: 0.2, // fresh — well under the stale threshold
        ..Default::default()
    };
    let actions = melee_actions();
    let snap = snap_with_target_at_x(40.0);
    let mut f = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
    assert!(
        !f.melee_pressed,
        "with offense still fresh, the post-poke reset keeps suppressing the swing"
    );
}

/// The rotated-gravity flip fix: under sideways gravity a foe stacked on the
/// gravity axis has a side offset ≈ 0 that physics jitter nudges across zero. The
/// old 0.001 px deadzone re-derived the facing sign from that jitter EVERY frame
/// — the rapid side-to-side flip the user saw when flipping gravity. The
/// alignment deadzone HOLDS facing across the jitter, so the sign is stable.
#[test]
fn grounded_facing_does_not_rapid_flip_on_a_gravity_axis_stacked_foe() {
    // Footsies off so this isolates the per-frame JITTER flip (the "very fast"
    // one the deadzone fixes), not the deliberate ~1 Hz in/out weave.
    let cfg = SmashCfg {
        footsies_amplitude: 0.0,
        neutral_jump_cadence_s: 0.0,
        ..crisp_duelist()
    };
    let mut state = SmashState {
        rng_seed: 3,
        ..Default::default()
    };
    let actions = melee_actions();
    let down = ae::Vec2::new(1.0, 0.0); // gravity points screen-right
    let mut prev_facing = 1.0_f32;
    let mut flips = 0;
    for i in 0..120 {
        // Foe 80 px up-gravity (screen -x), with a small side (screen-y) jitter
        // INSIDE the alignment deadzone (±8 px < 22 px) flipping each frame.
        let jitter = if i % 2 == 0 { 8.0 } else { -8.0 };
        let mut snap = BrainSnapshot::idle();
        snap.control_down = down;
        snap.actor_pos = ae::Vec2::ZERO;
        snap.actor_facing = prev_facing;
        snap.target_pos = ae::Vec2::new(-80.0, jitter);
        snap.actor_on_ground = true;
        snap.target_alive = true;
        snap.sim_time = i as f32 / 60.0;
        let mut f = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
        if f.facing.signum() != prev_facing.signum() {
            flips += 1;
        }
        prev_facing = f.facing;
    }
    assert!(
        flips <= 2,
        "facing must not rapid-flip across gravity-axis jitter; flipped {flips}/120 frames \
         (the old code flipped nearly every frame)"
    );
}

#[test]
fn approaches_when_target_in_aggro_but_out_of_attack() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    // Target at 300 px — inside aggro (460), outside engage (70).
    let snap = snap_with_target_at_x(300.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(
        frame.locomotion.x > 0.0,
        "actor should approach a target to its right; got vel={:?}",
        frame.locomotion,
    );
}

#[test]
fn melee_smash_swings_when_target_is_point_blank() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let actions = ActionSet {
        melee: Some(crate::brain::MeleeActionSpec::Swipe(
            crate::brain::SwipeSpec::STRIKER_DEFAULT,
        )),
        ..ActionSet::peaceful()
    };
    // 20px is inside STRIKER_DEFAULT.too_close_distance, but a
    // melee-capable Smash actor should take the point-blank swing
    // instead of backing away forever. This pins the cove-pirate
    // regression where provoked NPCs approached, then held range
    // without ever swinging when the player was beside them.
    let snap = snap_with_target_at_x(20.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(frame.melee_pressed, "point-blank melee actor should swing");
}

/// Difficulty profile that always commits and never jitters, so the
/// ranged-cadence tests are deterministic regardless of rng seed.
fn crisp_striker_cfg() -> SmashCfg {
    SmashCfg {
        difficulty: DifficultyProfile {
            reaction_delay_s: 0.0,
            commit_probability: 1.0,
            accuracy: 1.0,
            ..DifficultyProfile::HARD
        },
        ..SmashCfg::STRIKER_DEFAULT
    }
}

fn ranged_actions() -> ActionSet {
    ActionSet {
        ranged: Some(crate::brain::RangedActionSpec::rock(300.0, 2)),
        ..ActionSet::peaceful()
    }
}

#[test]
fn ranged_capable_actor_fires_at_mid_range() {
    // A Smash actor with a ranged verb, at mid-range (inside aggro
    // 460, outside melee reach 56), fires ranged rather than silently
    // walking closer — the player/enemy "verb selection by range" flex.
    let cfg = crisp_striker_cfg();
    let mut state = SmashState::default();
    let actions = ranged_actions();
    let snap = snap_with_target_at_x(300.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(
        frame.fire.is_some(),
        "ranged actor should attempt fire at mid-range"
    );
    assert!(!frame.melee_pressed, "should not also melee at mid-range");
    // The brain no longer rate-limits: it attempts a shot on every in-band
    // tick. A second tick still emits `fire` (the BODY throttles, not the
    // brain — invariant I3). This is what a spam controller would also do.
    let mut frame2 = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame2);
    assert!(
        frame2.fire.is_some(),
        "brain keeps attempting fire every in-band tick; the body enforces the rate"
    );
}

/// A `WorldView` whose terrain is the given solids, with self at the origin —
/// the perception a body at (0,0) would have.
fn view_with_terrain(
    terrain: Vec<crate::perception::PerceivedSolid>,
) -> crate::perception::WorldView {
    use crate::perception::{SelfView, Viewport, WorldView};
    WorldView {
        self_view: SelfView {
            pos: ae::Vec2::ZERO,
            vel: ae::Vec2::ZERO,
            facing: 1.0,
            half_extent: ae::Vec2::new(10.0, 16.0),
            gravity_down: ae::Vec2::new(0.0, 1.0),
            on_ground: true,
            aerial: false,
            alive: true,
            faction: crate::actor::ActorFaction::Enemy,
            can_fire: true,
            can_blink: false,
            can_dash: false,
            can_shield: false,
            ..Default::default()
        },
        viewport: Viewport::around(ae::Vec2::ZERO, ae::Vec2::splat(800.0)),
        actors: vec![],
        projectiles: vec![],
        terrain,
        portals: vec![],
        sim_time: 0.0,
        ..Default::default()
    }
}

/// Line-of-fire gate (S5): the same mid-range body that fires with a clear
/// shot must NOT fire when a solid wall occludes the path to the target — it
/// falls back to closing instead of firing into a wall. Proven against the
/// REAL brain pipeline + REAL `WorldView::line_of_fire` over the carried solids.
#[test]
fn ranged_shot_suppressed_when_line_of_fire_blocked() {
    use crate::perception::{PerceivedSolid, SolidKind};
    let cfg = crisp_striker_cfg();
    let actions = ranged_actions();
    let snap = snap_with_target_at_x(300.0); // body (0,0) → target (300,0)

    // Clear view: the body fires (matches the None-perception behavior).
    let clear = view_with_terrain(vec![]);
    let mut state = SmashState::default();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, Some(&clear), &mut frame);
    assert!(
        frame.fire.is_some(),
        "with a clear line of fire the body still shoots"
    );

    // Wall at x=150 squarely between the body and the target → no shot, and
    // the body keeps closing (a movement intent) toward a clear line.
    let blocked = view_with_terrain(vec![PerceivedSolid {
        aabb: ae::Aabb::new(ae::Vec2::new(150.0, 0.0), ae::Vec2::new(8.0, 60.0)),
        kind: SolidKind::Solid,
    }]);
    let mut state = SmashState::default();
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(
        &cfg,
        &mut state,
        &actions,
        &snap,
        Some(&blocked),
        &mut frame,
    );
    assert!(
        frame.fire.is_none(),
        "a wall between body and target suppresses the ranged shot (no firing into walls)"
    );
    assert!(
        frame.locomotion.length() > 0.0,
        "with the shot blocked the body falls back to closing for a clear line"
    );
}

#[test]
fn melee_takes_precedence_over_ranged_in_reach() {
    // With BOTH verbs, a point-blank target gets the melee swing,
    // not a ranged shot — ranged only substitutes for *closing*
    // actions outside melee range.
    let cfg = crisp_striker_cfg();
    let mut state = SmashState::default();
    let actions = ActionSet {
        melee: Some(crate::brain::MeleeActionSpec::Swipe(
            crate::brain::SwipeSpec::STRIKER_DEFAULT,
        )),
        ..ranged_actions()
    };
    let snap = snap_with_target_at_x(20.0); // inside attack_range
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(frame.melee_pressed, "in-reach actor swings");
    assert!(frame.fire.is_none(), "does not fire ranged in melee reach");
}

#[test]
fn brain_does_not_self_rate_limit_fire_body_owns_the_rate() {
    // Invariant I3: the brain no longer gates its own fire rate — it attempts
    // a ranged shot on EVERY in-band tick. The body (`try_fire_ranged`) is
    // the floor that turns those attempts into the weapon's rate. So back-to-
    // back ticks both emit `fire`; nothing in the brain throttles them. (The
    // body-side throttle is proven over real systems in the fighter harness.)
    let cfg = crisp_striker_cfg();
    let mut state = SmashState::default();
    let actions = ranged_actions();
    let mut snap = snap_with_target_at_x(300.0);
    snap.dt = 0.2;

    for tick in 0..8 {
        let mut frame = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
        assert!(
            frame.fire.is_some(),
            "tick {tick}: brain keeps attempting fire — the body, not the brain, enforces cadence"
        );
    }
}

fn dash_striker_cfg() -> SmashCfg {
    SmashCfg {
        dash_to_close: true,
        ..crisp_striker_cfg()
    }
}

#[test]
fn dash_capable_actor_bursts_to_close_a_large_gap() {
    // A dash-capable Smash actor closing a large gap (beyond
    // DASH_CLOSE_FRACTION * aggro ≈ 0.55 * 460 ≈ 253) bursts a Dash
    // (260 px/s) instead of plodding at walk speed (170).
    let cfg = dash_striker_cfg();
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful(); // no ranged → dash, not a poke
    let snap = snap_with_target_at_x(300.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(
        frame.locomotion.x > 0.8,
        "dash burst should exceed walk speed; got {}",
        frame.locomotion.x
    );
    assert!(
        state.dash_cooldown_remaining > 0.0,
        "dash cadence armed on commit"
    );
}

#[test]
fn dash_is_only_for_large_gaps() {
    // Inside the dash fraction (120 < 253) the actor walks, not dashes.
    let cfg = dash_striker_cfg();
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let snap = snap_with_target_at_x(120.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(
        frame.locomotion.x > 0.0 && frame.locomotion.x < 0.8,
        "a small gap walks, not dashes; got {}",
        frame.locomotion.x
    );
    assert_eq!(
        state.dash_cooldown_remaining, 0.0,
        "no dash armed for a small gap"
    );
}

#[test]
fn non_dash_actor_walks_the_same_large_gap() {
    // The SAME large gap, but dash_to_close OFF → a plain walk: the
    // capability is gated on the cfg flag, not on by default.
    let cfg = crisp_striker_cfg(); // dash_to_close = false
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let snap = snap_with_target_at_x(300.0);
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(
        frame.locomotion.x > 0.0 && frame.locomotion.x < 0.8,
        "no dash capability → a walk; got {}",
        frame.locomotion.x
    );
}

/// Crisp difficulty (always commit, no jitter) with an explicit reaction
/// delay, so latency tests aren't confounded by the commit roll.
fn crisp_cfg_with_delay(delay_s: f32) -> SmashCfg {
    SmashCfg {
        difficulty: DifficultyProfile {
            reaction_delay_s: delay_s,
            commit_probability: 1.0,
            accuracy: 1.0,
            ..DifficultyProfile::HARD
        },
        ..SmashCfg::STRIKER_DEFAULT
    }
}

fn run_tick(
    cfg: &SmashCfg,
    state: &mut SmashState,
    actions: &ActionSet,
    target_x: f32,
    t: f32,
) -> ae::Vec2 {
    let mut snap = snap_with_target_at_x(target_x);
    snap.sim_time = t;
    snap.dt = 1.0 / 60.0;
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(cfg, state, actions, &snap, None, &mut frame);
    frame.locomotion
}

#[test]
fn reaction_latency_delays_response_to_a_sudden_move() {
    // The never-cheats guarantee: after the opponent suddenly teleports
    // from far-right to far-left, the brain keeps pursuing the STALE
    // (right) position for ~reaction_delay_s before it perceives the new
    // one and flips. This is the headless proof the AI can't frame-
    // perfectly counter.
    let dt = 1.0 / 60.0;
    let delay = 0.15;
    let cfg = crisp_cfg_with_delay(delay);
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let mut t = 0.0;
    // Settle approaching the right-hand target so the buffer fills.
    for _ in 0..30 {
        let loco = run_tick(&cfg, &mut state, &actions, 300.0, t);
        assert!(loco.x > 0.0, "should approach the right-hand target");
        t += dt;
    }
    // Opponent teleports to the LEFT. The very next tick still pursues
    // right (perceiving the lagged position).
    let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
    assert!(
        loco.x > 0.0,
        "right after the teleport the brain still chases the stale position; got {loco:?}",
    );
    t += dt;
    // Within the reaction window the brain must NOT have flipped yet.
    let mut flipped_at: Option<f32> = None;
    let teleport_t = t - dt;
    for _ in 0..40 {
        let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
        if loco.x < 0.0 {
            flipped_at = Some(t - teleport_t);
            break;
        }
        t += dt;
    }
    let elapsed = flipped_at.expect("brain eventually pursues the new position");
    assert!(
        elapsed >= delay - dt,
        "brain flipped after {elapsed:.3}s — faster than its {delay:.3}s reaction delay (cheating)",
    );
    assert!(
        elapsed <= delay + 6.0 * dt,
        "brain flipped after {elapsed:.3}s — far later than its {delay:.3}s reaction delay",
    );
}

#[test]
fn zero_reaction_delay_responds_immediately() {
    // Control: with reaction_delay_s == 0 the brain has no perception lag
    // and flips the very next tick after the opponent moves.
    let dt = 1.0 / 60.0;
    let cfg = crisp_cfg_with_delay(0.0);
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let mut t = 0.0;
    for _ in 0..30 {
        run_tick(&cfg, &mut state, &actions, 300.0, t);
        t += dt;
    }
    let loco = run_tick(&cfg, &mut state, &actions, -300.0, t);
    assert!(
        loco.x < 0.0,
        "with zero reaction delay the brain pursues the new position immediately; got {loco:?}",
    );
}

/// Crisp duelist: a real neutral-game cfg with reaction lag and commit jitter
/// stripped, and full defense reactivity, so the post-poke + defense tests are
/// deterministic regardless of seed.
fn crisp_duelist() -> SmashCfg {
    SmashCfg {
        difficulty: DifficultyProfile {
            reaction_delay_s: 0.0,
            commit_probability: 1.0,
            accuracy: 1.0,
            ..DifficultyProfile::HARD
        },
        defense_reactivity: 1.0,
        ..SmashCfg::DUELIST_DEFAULT
    }
}

fn melee_actions() -> ActionSet {
    ActionSet {
        melee: Some(crate::brain::MeleeActionSpec::Swipe(
            crate::brain::SwipeSpec::STRIKER_DEFAULT,
        )),
        ..ActionSet::peaceful()
    }
}

/// The keystone neutral-game behavior: after a poke completes, a duelist does
/// NOT re-swing point-blank — it arms the reset and backs out toward its outer
/// spacing pocket. This is the fix for "they smack into each other and never
/// move away."
#[test]
fn duelist_resets_to_neutral_after_a_poke() {
    let cfg = crisp_duelist();
    let mut state = SmashState {
        rng_seed: 7,
        ..Default::default()
    };
    let actions = melee_actions();

    // Tick 1: mid-swing (active window) at point-blank → the swing latches.
    let mut swinging = snap_with_target_at_x(20.0);
    swinging.attack_active_remaining = 0.05;
    let mut f1 = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &swinging, None, &mut f1);
    assert!(state.was_attacking, "the swing should latch was_attacking");
    assert!(!f1.melee_pressed, "no new swing is committed mid-swing");

    // Suppress the neutral hop so we isolate the ground back-out (a hop would
    // also be a valid disengage, but we want to pin the spacing direction).
    state.neutral_jump_cooldown = 1.0;

    // Tick 2: swing done (timers clear) but target still point-blank. The
    // falling edge arms the reset, and the duelist backs AWAY instead of
    // re-swinging.
    let done = snap_with_target_at_x(20.0);
    let mut f2 = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &done, None, &mut f2);
    assert!(
        state.neutral_reset_timer > 0.0,
        "the reset window arms on the swing's falling edge"
    );
    assert!(
        !f2.melee_pressed,
        "a duelist does not immediately re-swing — it resets to neutral first"
    );
    assert!(
        f2.locomotion.x < 0.0,
        "the duelist backs out of point-blank (target on the right → move left); got {:?}",
        f2.locomotion
    );
}

/// A grunt (no poke_reset) is unaffected: it stays in range. Pins that the
/// reset is strictly opt-in and doesn't change every melee enemy's feel.
#[test]
fn grunt_has_no_neutral_reset() {
    let cfg = SmashCfg::STRIKER_DEFAULT; // poke_reset_s = 0
    let mut state = SmashState::default();
    let actions = melee_actions();
    let mut swinging = snap_with_target_at_x(20.0);
    swinging.attack_active_remaining = 0.05;
    let mut f1 = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &swinging, None, &mut f1);
    let done = snap_with_target_at_x(20.0);
    let mut f2 = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &done, None, &mut f2);
    assert_eq!(
        state.neutral_reset_timer, 0.0,
        "a grunt has no neutral reset window"
    );
}

/// Run `ticks` with the target closing from the right at `closing_px_s`, and
/// report whether the fighter ever blinked / shielded. Drives the real defense
/// path (obs_history → perceived_threat → blink/shield split).
fn defense_over_approach(cfg: &SmashCfg, closing_px_s: f32, start_x: f32) -> (bool, bool) {
    let mut state = SmashState {
        rng_seed: 5,
        ..Default::default()
    };
    let actions = melee_actions();
    let dt = 1.0 / 60.0;
    let mut t = 0.0;
    let mut x = start_x;
    let (mut blinked, mut shielded) = (false, false);
    for _ in 0..40 {
        let mut snap = snap_with_target_at_x(x);
        snap.sim_time = t;
        snap.dt = dt;
        let mut f = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(cfg, &mut state, &actions, &snap, None, &mut f);
        blinked |= f.blink_pressed;
        shielded |= f.shield_held;
        x -= closing_px_s * dt;
        t += dt;
    }
    (blinked, shielded)
}

/// Layered defense: a fast committed lunge is met with a BLINK; an ordinary
/// walk-in is met with a BLOCK. This is the readable defensive game the duel
/// was missing.
#[test]
fn defense_blinks_a_lunge_and_blocks_a_walk_in() {
    let cfg = crisp_duelist(); // blink ≥ 230, shield ≥ 70
                               // A 360 px/s dash-in clears the blink threshold.
    let (blinked, _) = defense_over_approach(&cfg, 360.0, 130.0);
    assert!(blinked, "a fast lunge should be blinked");
    // A 120 px/s walk-in is below the blink threshold but above shield → block.
    let (lunge_blinked, shielded) = defense_over_approach(&cfg, 120.0, 130.0);
    assert!(shielded, "an ordinary walk-in should be blocked");
    assert!(
        !lunge_blinked,
        "a mere walk-in should NOT trigger the blink (that's reserved for lunges)"
    );
}

/// `defense_reactivity = 0` disables reactive defense entirely (grunt parity),
/// even under a clear lunge — the imperfect-defense knob bottoms out cleanly.
#[test]
fn defense_reactivity_zero_never_defends() {
    let mut cfg = crisp_duelist();
    cfg.defense_reactivity = 0.0;
    let (blinked, shielded) = defense_over_approach(&cfg, 360.0, 130.0);
    assert!(
        !blinked && !shielded,
        "reactivity 0 must never blink or shield"
    );
}

/// Damage-triggered regroup: after taking a bunch of hits a duelist breaks off —
/// arms the regroup window and DASHES away (exercising the body dash) instead of
/// trading at point-blank.
#[test]
fn duelist_regroups_and_dashes_after_taking_damage() {
    let cfg = crisp_duelist(); // dash_to_close + regroup enabled
    let mut state = SmashState {
        rng_seed: 11,
        ..Default::default()
    };
    let actions = melee_actions();
    let dt = 1.0 / 60.0;
    let mut t = 0.0;
    let mut hp = 1.0_f32;
    let mut dashed = false;
    let mut regrouped = false;
    // Target point-blank to the right; bleed health a bit each tick (a beating).
    for i in 0..120 {
        let mut snap = snap_with_target_at_x(30.0);
        snap.sim_time = t;
        snap.dt = dt;
        snap.health_fraction = hp;
        let mut f = crate::actor::control::ActorControlFrame::neutral();
        tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
        if state.regroup_timer > 0.0 {
            regrouped = true;
        }
        if f.dash_pressed {
            dashed = true;
        }
        // Take ~2% of max HP every few ticks for the first ~0.5s (a flurry).
        if i < 30 && i % 5 == 0 {
            hp -= 0.02;
        }
        t += dt;
    }
    assert!(regrouped, "a beaten duelist should enter a regroup");
    assert!(
        dashed,
        "the regroup should DASH away to cover ground (exercises the body dash)"
    );
}

#[test]
fn dead_actor_emits_neutral_frame() {
    let cfg = SmashCfg::STRIKER_DEFAULT;
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let mut snap = snap_with_target_at_x(100.0);
    snap.alive = false;
    let mut frame = crate::actor::control::ActorControlFrame::neutral();
    // Pre-poison: if `tick_smash` early-returns without writing,
    // the assertion below would catch a leak from the caller's
    // pre-existing frame state.
    frame.melee_pressed = true;
    frame.locomotion = ae::Vec2::new(999.0, 999.0);
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut frame);
    assert!(!frame.melee_pressed, "dead actor must not emit melee");
    assert_eq!(frame.locomotion, ae::Vec2::ZERO);
}

// --- S2: frame-agnostic motor / perception (invariant I10) ---

/// Build an idle snapshot with rotated gravity. `down` is the world gravity
/// direction; `target` the world target position.
fn snap_rotated(down: ae::Vec2, target: ae::Vec2) -> BrainSnapshot {
    let mut s = BrainSnapshot::idle();
    s.actor_pos = ae::Vec2::ZERO;
    s.control_down = down;
    s.target_pos = target;
    s.target_alive = true;
    s.actor_on_ground = false;
    s.actor_aerial = true;
    s
}

/// Under gravity rotated 90° (down = screen `+x`), the reactive evade points
/// AGAINST gravity (screen `-x`), not screen `-y`. The old code hard-coded
/// `-y`, which would dodge sideways into a wall under this orientation. The
/// dodge must climb the open vertical space whatever the gravity frame.
#[test]
fn evade_dodges_against_gravity_under_rotated_gravity() {
    let cfg = crisp_striker_cfg(); // reaction_delay_s = 0
    let down = ae::Vec2::new(1.0, 0.0);
    // Target closing fast on the actor from the side, along screen -y.
    let near = ae::Vec2::new(0.0, 40.0);
    let far = ae::Vec2::new(0.0, 60.0);
    let mut snap = snap_rotated(down, near);
    let now = 1.0;
    snap.sim_time = now;
    let obs = observe(&snap);
    let mut state = SmashState::default();
    state.obs_history.push(now - THREAT_WINDOW_S, far);
    state.obs_history.push(now, near);

    let (away, _closing) =
        perceived_threat(&obs, &cfg, &state, now).expect("a fast lunge is a threat");
    assert!(
        away.dot(obs.up_axis()) > 0.5,
        "evade must climb against gravity (up = -down); got {away:?}"
    );
    assert!(
        away.dot(down) < 0.0,
        "evade must never dive into gravity; got {away:?}"
    );
}

/// Grounded APPROACH is gravity-relative: under gravity rotated 90° (down =
/// screen `+x`), the run axis is screen-vertical, so a target offset along
/// screen `+y` must drive `locomotion.x` (the body's local-side run scalar)
/// toward it. The old code keyed the run on screen `to_target_x` (here 0), so it
/// would NOT pursue — this pins the relativity fix for the player's gravity flip.
#[test]
fn grounded_approach_runs_toward_target_under_rotated_gravity() {
    let cfg = crisp_striker_cfg(); // reaction_delay 0, grounded striker
    let mut state = SmashState::default();
    let actions = ActionSet::peaceful();
    let mut snap = BrainSnapshot::idle();
    snap.control_down = ae::Vec2::new(1.0, 0.0); // down = +x → side = (0,-1)
    snap.actor_pos = ae::Vec2::ZERO;
    snap.target_pos = ae::Vec2::new(0.0, 300.0); // purely screen-vertical (no screen-x)
    snap.actor_on_ground = true;
    snap.target_alive = true;
    let mut f = crate::actor::control::ActorControlFrame::neutral();
    tick_smash(&cfg, &mut state, &actions, &snap, None, &mut f);
    // to_target_side = (0,300)·(0,-1) = -300 ⇒ run toward it is negative.
    assert!(
        f.locomotion.x < 0.0,
        "should run toward the target along the LOCAL side axis under rotated \
         gravity; got {:?} (the old screen-x code would stall here)",
        f.locomotion
    );
}

/// `to_target_up` is frame-correct: a target offset against gravity reads as
/// "above" regardless of screen orientation. Under down = screen `+x`, a
/// target at screen `-x` is above.
#[test]
fn target_above_is_gravity_relative() {
    // down = +x ⇒ up = -x. Target at screen -x (200 left) is "above".
    let snap = snap_rotated(ae::Vec2::new(1.0, 0.0), ae::Vec2::new(-200.0, 0.0));
    let obs = observe(&snap);
    assert!(
        obs.to_target_up() > 100.0,
        "target opposite gravity must read as above; got {}",
        obs.to_target_up()
    );
    // And a target *along* gravity (screen +x) reads as below.
    let snap_below = snap_rotated(ae::Vec2::new(1.0, 0.0), ae::Vec2::new(200.0, 0.0));
    assert!(
        observe(&snap_below).to_target_up() < -100.0,
        "target along gravity must read as below"
    );
}

/// The aerial dive/perch steers into the gravity-relative "up" space: a flyer
/// engaging a target perches against gravity, not toward screen `-y`. Under
/// down = screen `+x`, the steered velocity carries the flyer to the up side.
#[test]
fn aerial_perch_climbs_against_gravity() {
    let cfg = SmashCfg::DUELIST_DEFAULT; // a real neutral game / flyer cfg
    let down = ae::Vec2::new(1.0, 0.0);
    // Flyer sitting ON the target's gravity-line so the only steer is up/down.
    let target = ae::Vec2::new(0.0, 0.0);
    let mut snap = snap_rotated(down, target);
    snap.actor_pos = ae::Vec2::new(0.0, 0.0);
    let obs = observe(&snap);
    let state = SmashState::default();
    // Engage mode rides the dive→perch arc; perch sits above-and-beside.
    let vel = aerial_steer(&obs, BroadMode::Engage, &cfg, &state);
    // The desired point biases against gravity, so the steer has a positive
    // up-component (it is not allowed to be a screen-`-y`-only push).
    assert!(
        vel.dot(obs.up_axis()) >= 0.0,
        "aerial steer must not drive into gravity; got {vel:?} under down={down:?}"
    );
}

// --- S3b: hybrid flight prefers grounded, flies to traverse ---

fn hybrid_obs(distance_x: f32, currently_aerial: bool) -> ObservationFrame {
    let mut snap = snap_with_target_at_x(distance_x);
    snap.actor_aerial = currently_aerial;
    snap.actor_on_ground = !currently_aerial;
    observe(&snap)
}

/// The hybrid PREFERS grounded: with a target close in, it does not take to
/// the air; with a target a long traversal away, it does. (Brain *policy* —
/// flight is free for now, so this preference is the only thing keeping it
/// grounded.)
#[test]
fn hybrid_flight_prefers_grounded_flies_to_traverse() {
    let mut cfg = SmashCfg::DUELIST_DEFAULT;
    cfg.can_fly = true;
    cfg.aggro_radius = 500.0; // take-off > 300, land > 210
    let mut state = SmashState::default();

    // Close target, on the ground → stay grounded.
    assert!(
        !decide_flight(&hybrid_obs(120.0, false), &cfg, &mut state),
        "a grounded hybrid should NOT fly to a target it can just walk to"
    );
    // Distant target, on the ground → take off to cover the gap.
    assert!(
        decide_flight(&hybrid_obs(420.0, false), &cfg, &mut state),
        "a grounded hybrid SHOULD fly to close a long traversal gap"
    );
    // A target beyond sensing range is not chased into the air.
    assert!(
        !decide_flight(&hybrid_obs(900.0, false), &cfg, &mut state),
        "no live target in range → no reason to leave the ground"
    );
}

/// Hysteresis: once airborne it keeps flying through the mid-band (so the
/// toggle doesn't chatter at the boundary), but lands once it has closed in.
#[test]
fn hybrid_flight_has_landing_hysteresis() {
    let mut cfg = SmashCfg::DUELIST_DEFAULT;
    cfg.can_fly = true;
    cfg.aggro_radius = 500.0; // take-off 300, land 210
    let mut state = SmashState::default();

    // Mid-band (between land=210 and take-off=300): keep flying if already up…
    assert!(
        decide_flight(&hybrid_obs(250.0, true), &cfg, &mut state),
        "an airborne hybrid keeps flying through the mid-band (hysteresis)"
    );
    // …but a grounded one would NOT have taken off at the same distance.
    assert!(
        !decide_flight(&hybrid_obs(250.0, false), &cfg, &mut state),
        "a grounded hybrid does not take off in the mid-band"
    );
    // Closed all the way in → land and brawl.
    assert!(
        !decide_flight(&hybrid_obs(150.0, true), &cfg, &mut state),
        "once closed inside the landing band, the hybrid comes down to fight"
    );
}
