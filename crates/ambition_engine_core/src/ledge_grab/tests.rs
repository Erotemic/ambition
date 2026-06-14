use super::*;
use crate::world::Block;

fn world_with(blocks: Vec<Block>) -> World {
    World::new("ledge", Vec2::new(800.0, 600.0), Vec2::ZERO, blocks)
}

#[test]
fn finds_ledge_when_clinging_to_a_wall_with_open_space_above() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    // Player center to the left of the wall (player's right edge
    // touches the block's left face). wall_normal_x = -1 (wall on
    // the player's right pushes them left).
    let player_pos = Vec2::new(86.0, 110.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(contact.is_some(), "expected ledge contact");
    let contact = contact.unwrap();
    assert!(contact.wall_normal_x < 0.0);
    // Anchor hugs the wall edge (block.left = 100) just outboard
    // of the player (player half is 14 → anchor.x ≈ 87).
    assert!(
        (contact.anchor.x - 87.0).abs() < 4.0,
        "anchor.x = {}, expected ~87",
        contact.anchor.x
    );
    // Climb target is on top of the block, slightly inboard from
    // the edge.
    assert!(contact.climb_target.x > 100.0);
    assert!(contact.climb_target.y < contact.anchor.y);
}

#[test]
fn rejects_when_above_is_blocked() {
    let world = world_with(vec![
        Block::solid("ledge", Vec2::new(100.0, 100.0), Vec2::new(200.0, 200.0)),
        Block::solid("low_ceiling", Vec2::new(60.0, 50.0), Vec2::new(100.0, 50.0)),
    ]);
    let player_pos = Vec2::new(86.0, 110.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(
        contact.is_none(),
        "should not return a ledge whose top has another block above"
    );
}

#[test]
fn rejects_when_hang_space_has_wall_in_front_of_ledge() {
    let world = world_with(vec![
        Block::solid("ledge", Vec2::new(100.0, 100.0), Vec2::new(200.0, 200.0)),
        // A blocking wall occupies the player's hang lane outside the ledge.
        // The climb target on top of the ledge is clear, but snapping to the
        // hang anchor would overlap this wall and let the getup path clip
        // through it.
        Block::solid("front_wall", Vec2::new(70.0, 82.0), Vec2::new(20.0, 96.0)),
    ]);
    let player_pos = Vec2::new(86.0, 110.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(
        contact.is_none(),
        "ledge should be rejected when the hang space in front is blocked"
    );
}

#[test]
fn rejects_when_no_wall_present() {
    let world = world_with(vec![]);
    let player_pos = Vec2::new(50.0, 50.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(contact.is_none());
}

#[test]
fn rejects_zero_wall_normal() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), 0.0, &world);
    assert!(contact.is_none());
}

/// Regression: a ledge whose top sits near the world's ceiling
/// must be rejected — climbing onto it would put the player
/// out of bounds. This was the May 2026 mob_lab teleport-loop
/// bug: a ceiling tile near y=0 produced a climb_target above
/// the world, the climb-up snapped the player OOB, and the
/// engine's collision-correction yanked them back, looping.
#[test]
fn rejects_ledge_when_player_would_land_above_world_top() {
    // Ceiling block: top edge at y=1 (world ranges y=0..600).
    // Player half-height is 23, so a body sitting on this ledge
    // would have its top at y = 1 - 46 - 1 = -46 (above world).
    let world = world_with(vec![Block::solid(
        "ceiling",
        Vec2::new(100.0, 1.0),
        Vec2::new(200.0, 80.0),
    )]);
    // Player wall-clinging just below the ceiling block, with
    // their head right under the block's top.
    let player_pos = Vec2::new(86.0, 24.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(
        contact.is_none(),
        "ceiling-adjacent ledge must be rejected (climb_target would be OOB)"
    );
}

#[test]
fn finds_ledge_on_left_facing_wall() {
    // Block from x=0 to x=100. Player to the right of the block
    // with `wall_normal_x = +1` (wall on player's left, normal
    // pushes player right).
    let world = world_with(vec![Block::solid(
        "left_wall",
        Vec2::new(0.0, 100.0),
        Vec2::new(100.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    let player_pos = Vec2::new(114.0, 110.0); // hugging right edge of block
    let contact = probe_ledge_grab(player_pos, player_size, 1.0, &world);
    assert!(contact.is_some(), "should find ledge on the right face");
    let contact = contact.unwrap();
    assert!(contact.wall_normal_x > 0.0);
    // Climb target is to the left of the anchor (toward the
    // block's interior on top).
    assert!(contact.climb_target.x < contact.anchor.x);
}

#[test]
fn finds_ledge_when_player_is_slightly_low() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    // Head is at y=127, so the lip at y=100 is 27px above the
    // head. The previous 12px upward reach rejected this common
    // near-miss; the forgiving reach should still catch it.
    let player_pos = Vec2::new(86.0, 150.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(
        contact.is_some(),
        "ledge slightly above the old chin band should be reachable"
    );
}

#[test]
fn finds_ledge_when_player_is_slightly_off_wall() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    // Player's right edge is x=92, 8px short of the wall face at
    // x=100. That used to miss because the face tolerance was 4px.
    let player_pos = Vec2::new(78.0, 110.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    assert!(
        contact.is_some(),
        "a small horizontal near-miss should still grab the ledge"
    );
}

#[test]
fn forgiving_vertical_grab_is_not_precise_for_boost() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    let player_pos = Vec2::new(86.0, 150.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world)
        .expect("forgiving ledge probe should still catch the player");
    assert!(
        !is_precise_ledge_grab(player_pos, player_size, contact),
        "outer vertical forgiveness should latch but not earn boost precision"
    );
}

#[test]
fn forgiving_horizontal_grab_is_not_precise_for_boost() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    let player_pos = Vec2::new(78.0, 110.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world)
        .expect("forgiving ledge probe should still catch the player");
    assert!(
        !is_precise_ledge_grab(player_pos, player_size, contact),
        "outer horizontal forgiveness should latch but not earn boost precision"
    );
}

#[test]
fn precise_grab_keeps_momentum_boost_reward() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    scratch.kinematics.vel = Vec2::new(240.0, -120.0);
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched, "tight-window grab should latch");
    let state = scratch.ledge.grab.expect("grab state should be active");
    assert!(
        state.grab_quality.is_precise(),
        "old tight window should be precise"
    );
    let tuning = crate::movement::MovementTuning::default();
    assert!(
        ledge_boost_for_state(state, &tuning).length_squared() > 0.0,
        "precise grab should keep the momentum-carry reward"
    );
    assert!(
        ledge_boost_weight_for_state(state, &tuning) > 0.0,
        "precise grab should keep the fast-getup reward"
    );
}

#[test]
fn forgiving_grab_latches_but_suppresses_momentum_boost_reward() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 150.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    scratch.kinematics.vel = Vec2::new(240.0, -120.0);
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched, "outer-window grab should still latch");
    let state = scratch.ledge.grab.expect("grab state should be active");
    assert!(
        !state.grab_quality.is_precise(),
        "outer-window grab should be forgiving, not precise"
    );
    let tuning = crate::movement::MovementTuning::default();
    assert_eq!(
        ledge_boost_for_state(state, &tuning),
        Vec2::ZERO,
        "forgiving grab should not receive the momentum boost"
    );
    assert_eq!(
        ledge_boost_weight_for_state(state, &tuning),
        0.0,
        "forgiving grab should not receive the fast-getup reward"
    );
}

#[test]
fn forgiving_grab_still_allows_regular_ledge_jump_without_bonus_velocity() {
    let contact = rightward_ledge_contact();
    let mut scratch = make_hanging_player_with_momentum(contact, Vec2::new(240.0, -120.0));
    scratch.ledge.grab = Some(LedgeGrabState {
        grab_quality: LedgeGrabQuality::Forgiving,
        ..scratch.ledge.grab.expect("hanging state")
    });
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        jump_pressed: true,
        ..InputState::default()
    };

    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);

    assert!(consumed);
    assert!(scratch.ledge.grab.is_none());
    let expected_regular_x = into_platform_axis(contact) * tuning.jump_speed * 0.35;
    assert!(
        (scratch.kinematics.vel.x - expected_regular_x).abs() < 0.01,
        "forgiving ledge jump should keep only the regular inboard hop velocity; got vx={} expected {}",
        scratch.kinematics.vel.x,
        expected_regular_x,
    );
    assert!(
        (scratch.kinematics.vel.y + tuning.jump_speed).abs() < 0.01,
        "forgiving ledge jump should not add upward momentum boost; got vy={} expected {}",
        scratch.kinematics.vel.y,
        -tuning.jump_speed,
    );
}

#[test]
fn precise_grab_quality_is_reported_as_an_explicit_state() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let player_size = Vec2::new(28.0, 46.0);
    let precise_pos = Vec2::new(86.0, 110.0);
    let forgiving_pos = Vec2::new(86.0, 150.0);
    let precise_contact = probe_ledge_grab(precise_pos, player_size, -1.0, &world)
        .expect("precise ledge probe should catch");
    let forgiving_contact = probe_ledge_grab(forgiving_pos, player_size, -1.0, &world)
        .expect("forgiving ledge probe should catch");

    assert_eq!(
        classify_ledge_grab(precise_pos, player_size, precise_contact),
        LedgeGrabQuality::Precise,
    );
    assert_eq!(
        classify_ledge_grab(forgiving_pos, player_size, forgiving_contact),
        LedgeGrabQuality::Forgiving,
    );
}

#[test]
fn finds_ledge_on_blink_wall() {
    let world = world_with(vec![Block::blink_wall(
        "blink_ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
        crate::world::BlinkWallTier::Soft,
    )]);
    let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
    assert!(
        contact.is_some(),
        "blink walls are standable ledge surfaces"
    );
}

#[test]
fn finds_ledge_on_one_way_platform_edge() {
    let world = world_with(vec![Block::one_way(
        "thin_ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 16.0),
    )]);
    let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
    assert!(contact.is_some(), "one-way platforms can be pulled up onto");
}

#[test]
fn rejects_when_lock_door_blocks_pull_up_space() {
    let world = world_with(vec![
        Block::one_way("ledge", Vec2::new(100.0, 100.0), Vec2::new(200.0, 16.0)),
        Block::solid("lock_door", Vec2::new(104.0, 40.0), Vec2::new(48.0, 80.0)),
    ]);
    let contact = probe_ledge_grab(Vec2::new(86.0, 110.0), Vec2::new(28.0, 46.0), -1.0, &world);
    assert!(
        contact.is_none(),
        "a solid lock door in the climb target must block the grab"
    );
}

/// Regression: two adjacent solid blocks forming a continuous
/// vertical wall must still surface a grabbable ledge at the
/// topmost block's top edge. Tests that the probe's "find the
/// closest block to the chin band" logic doesn't snag on the
/// lower block and miss the actual ledge above it.
#[test]
fn finds_ledge_at_top_of_stacked_solid_wall() {
    // Two stacked 200×100 solids form a continuous wall from
    // x=[100, 300], y=[100, 300]. The actual ledge is the top
    // of the upper block at y=100.
    let world = world_with(vec![
        Block::solid(
            "wall_lower",
            Vec2::new(100.0, 200.0),
            Vec2::new(200.0, 100.0),
        ),
        Block::solid(
            "wall_upper",
            Vec2::new(100.0, 100.0),
            Vec2::new(200.0, 100.0),
        ),
    ]);
    // Player clinging on the wall's left face (wall_normal_x = -1
    // pushes player left), with head near the upper block's top.
    let player_pos = Vec2::new(86.0, 110.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    let contact = contact.expect("stacked-wall ledge must surface a contact");
    assert!(contact.wall_normal_x < 0.0);
    // Anchor should hug the wall edge (block.left = 100) just
    // outboard of the player.
    assert!(
        (contact.anchor.x - 87.0).abs() < 4.0,
        "anchor.x = {}, expected ~87",
        contact.anchor.x
    );
    // Climb target is on top of the UPPER block (y < 100), not
    // wedged between the two stacked blocks at y≈200.
    assert!(
        contact.climb_target.y < 100.0,
        "climb_target.y = {}, expected < 100 (top of upper block)",
        contact.climb_target.y
    );
    assert!(contact.climb_target.x > 100.0);
}

/// Regression: an L-shaped corner geometry (lower block extending
/// further right than the upper block, leaving a shelf at the
/// upper block's right edge). The ledge should be the upper
/// block's top corner, NOT the inner corner where the two blocks
/// meet.
#[test]
fn finds_ledge_at_l_corner_when_clinging_to_upper_block() {
    // Upper block: x=[100, 200], y=[100, 200].
    // Lower block: x=[100, 300], y=[200, 300] (extends further right).
    // The composite shape is an L; clinging the upper block's
    // left face should find its top corner at y=100, not the
    // lower block's top at y=200.
    let world = world_with(vec![
        Block::solid("upper", Vec2::new(100.0, 100.0), Vec2::new(100.0, 100.0)),
        Block::solid("lower", Vec2::new(100.0, 200.0), Vec2::new(200.0, 100.0)),
    ]);
    let player_pos = Vec2::new(86.0, 110.0);
    let player_size = Vec2::new(28.0, 46.0);
    let contact = probe_ledge_grab(player_pos, player_size, -1.0, &world);
    let contact = contact.expect("L-corner ledge must surface a contact");
    assert!(
        contact.climb_target.y < 100.0,
        "climb_target.y = {}, expected < 100 (top of UPPER block, not lower)",
        contact.climb_target.y
    );
}

use crate::player_clusters::PlayerClusterScratch;

fn make_hanging_player(contact: LedgeContact) -> PlayerClusterScratch {
    make_hanging_player_with_momentum(contact, Vec2::ZERO)
}

fn make_hanging_player_with_momentum(
    contact: LedgeContact,
    momentum: Vec2,
) -> PlayerClusterScratch {
    let mut scratch = scratch_at(Vec2::ZERO);
    scratch.abilities.abilities.ledge_grab = true;
    scratch.abilities.abilities.shield = true;
    scratch.ledge.grab = Some(LedgeGrabState {
        contact,
        elapsed: LEDGE_MIN_CLIMB_DELAY + 0.01,
        climbing: false,
        getup_kind: LedgeGetupKind::Climb,
        climb_elapsed: 0.0,
        momentum_at_grab: momentum,
        grab_quality: LedgeGrabQuality::Precise,
    });
    scratch.wall.wall_clinging = true;
    scratch.wall.on_wall = true;
    scratch
}

fn scratch_at(pos: Vec2) -> PlayerClusterScratch {
    PlayerClusterScratch::new_with_abilities(pos, crate::AbilitySet::sandbox_all())
}

#[test]
fn ledge_jump_away_launches_player_outward() {
    // Wall on the right of the player (wall_normal_x = -1, pushes left).
    // away_from_platform = left = negative x.
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        jump_pressed: true,
        axis_x: -1.0, // pressing away from the platform (away = wall_normal direction = -1)
        ..InputState::default()
    };
    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(consumed, "tick should consume the frame");
    assert!(scratch.ledge.grab.is_none(), "ledge should be released");
    // Player should move left (away from the right-side wall).
    assert!(
        scratch.kinematics.vel.x < -100.0,
        "should have leftward velocity, got {}",
        scratch.kinematics.vel.x
    );
    assert!(
        scratch.kinematics.vel.y < -100.0,
        "should have upward velocity, got {}",
        scratch.kinematics.vel.y
    );
    assert!(!scratch.wall.on_wall, "should not be on wall");
}

#[test]
fn jump_toward_platform_now_hops_up_not_climbs() {
    // Smash-style split: pressing Jump from a ledge is the
    // "ledge jump" option (vertical hop with control). It used
    // to trigger a climb instead. The climb is now reserved
    // for Up / Into / Interact.
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        jump_pressed: true,
        axis_x: 1.0, // pressing into the platform (into = -wall_normal = +1)
        ..InputState::default()
    };
    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(consumed);
    assert!(
        scratch.ledge.grab.is_none(),
        "ledge should be released by the hop"
    );
    assert!(
        scratch.kinematics.vel.y < -100.0,
        "ledge jump should fling upward, got vy={}",
        scratch.kinematics.vel.y
    );
    // Inboard drift: for a -1 wall_normal, into_x = +1, so vx > 0.
    assert!(
        scratch.kinematics.vel.x > 0.0,
        "ledge jump should drift inboard, got vx={}",
        scratch.kinematics.vel.x
    );
    assert!(!scratch.wall.on_wall);
}

/// Pure jump from the ledge with NO horizontal input also hops
/// UP. This was the case the old code mapped to "climb" because
/// jump was a confirm cue; now the player gets a vertical hop
/// they can air-control.
#[test]
fn jump_with_no_horizontal_input_hops_up() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        jump_pressed: true,
        ..InputState::default()
    };
    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(consumed);
    assert!(scratch.ledge.grab.is_none());
    assert!(
        scratch.kinematics.vel.y < -100.0,
        "pure jump should still go up, got vy={}",
        scratch.kinematics.vel.y
    );
}

/// `Up` (without jump) is still the slow climb path. The split
/// must NOT have broken the regular pull-up.
#[test]
fn up_alone_still_starts_a_climb() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        axis_y: -1.0, // up
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    let state = scratch
        .ledge
        .grab
        .expect("climb should leave transitioning state");
    assert!(state.climbing, "Up should start a climb");
    assert_eq!(state.getup_kind, LedgeGetupKind::Climb);
}

/// The climb path is a quadratic Bezier whose control point sits
/// at `(anchor.x, climb_target.y)` — so the player goes UP the
/// wall first and ACROSS onto the platform second. At t=0.5 the
/// player's position should be much closer to the bend (above
/// the anchor on the wall) than to the midpoint of a straight
/// line between anchor and climb_target. This is the curved-feel
/// Jon asked for; the straight-diagonal was the old behavior.
#[test]
fn climb_path_curves_up_before_going_over() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mid_curved = climb_position(contact, 0.5);
    let mid_straight = (contact.anchor + contact.climb_target) * 0.5;
    // The curved midpoint should be closer to `(anchor.x, target.y)`
    // — the control point — than the straight-line midpoint is.
    let control = Vec2::new(contact.anchor.x, contact.climb_target.y);
    let curved_to_ctrl = (mid_curved - control).length();
    let straight_to_ctrl = (mid_straight - control).length();
    assert!(
        curved_to_ctrl < straight_to_ctrl,
        "curved midpoint should bias toward the bend; got {:.2} vs straight {:.2}",
        curved_to_ctrl,
        straight_to_ctrl,
    );
}

/// Falling fast past a ledge should auto-snap to it, even with
/// no stick input — the Smash recovery snap. Without this you
/// have to hold a stick INTO the wall to grab; in practice
/// players want a near-miss snap.
#[test]
fn falling_player_auto_snaps_to_nearby_ledge() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.kinematics.vel = Vec2::new(0.0, 150.0); // falling fast, no horizontal input
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched, "fast-falling near a ledge should auto-snap");
    assert!(scratch.ledge.grab.is_some());
}

/// A loitering player (slow descent, no stick input) should NOT
/// auto-snap — only an active recovery does. Keeps the snap
/// from feeling like sticky-wall.
#[test]
fn drifting_player_does_not_auto_snap() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.kinematics.vel = Vec2::new(0.0, 20.0); // gentle drift, well below FALL_SNAP_MIN_VY
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(!latched, "slow drift must not auto-snap");
}

#[test]
fn light_horizontal_intent_can_request_a_ledge_grab() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    let mut events = crate::movement::FrameEvents::default();
    let latched = try_start_ledge_grab_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: 0.30,
            ..InputState::default()
        },
        &mut events,
    );
    assert!(
        latched,
        "sub-0.4 horizontal intent should still request a ledge probe"
    );
}

/// Holding shield while hanging on a ledge triggers a Smash-Bros
/// style roll: the getup_kind switches to Roll, the player starts
/// climbing (interpolating along the roll trajectory), and
/// `dodge_roll_timer` is set so the player is invulnerable for
/// the duration of the roll.
#[test]
fn shield_held_starts_a_ledge_roll() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        shield_held: true,
        ..InputState::default()
    };
    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(consumed);
    let state = scratch
        .ledge
        .grab
        .expect("roll should leave a transitioning state");
    assert!(state.climbing, "roll must enter the climbing state");
    assert_eq!(state.getup_kind, LedgeGetupKind::Roll);
    assert!(
        scratch.dodge.roll_timer > 0.0,
        "ledge roll must arm dodge_roll_timer for invuln",
    );
}

/// Shield wins over climb when both inputs are present (e.g. Up
/// + Shield). Matches Smash where shield-from-ledge is the
/// universal roll cue regardless of stick direction.
#[test]
fn shield_overrides_climb_when_both_inputs_are_held() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        shield_held: true,
        axis_y: -1.0, // up — would otherwise climb
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    let state = scratch.ledge.grab.expect("active transition");
    assert_eq!(state.getup_kind, LedgeGetupKind::Roll);
}

/// At the end of the roll the player lands FURTHER inboard than a
/// climb would, by ``LEDGE_ROLL_OVERSHOOT`` along the into-platform
/// axis. That overshoot is what makes the roll feel like a real
/// commitment past the ledge edge instead of a snappier climb.
#[test]
fn ledge_roll_lands_further_inboard_than_climb() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    // Start the roll.
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState {
            shield_held: true,
            ..InputState::default()
        },
        0.001,
        tuning,
        &mut events,
    );
    // Run the full roll duration in one big tick to land.
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState::default(),
        LEDGE_ROLL_TIME + 0.05,
        tuning,
        &mut events,
    );
    // After the roll finishes, the player should be at the roll
    // landing position which is past climb_target along the
    // into-platform axis. For a -1 wall normal that's +x.
    assert!(scratch.ledge.grab.is_none(), "roll should have finished");
    assert!(
        scratch.ground.on_ground,
        "roll lands the player on the platform"
    );
    let expected = roll_target(contact);
    assert!(
        (scratch.kinematics.pos.x - expected.x).abs() < 0.5,
        "expected roll landing x ≈ {}, got {}",
        expected.x,
        scratch.kinematics.pos.x,
    );
    assert!(
        (expected.x - contact.climb_target.x).abs() >= LEDGE_ROLL_OVERSHOOT - 0.01,
        "roll target must overshoot the climb target by ~{}px",
        LEDGE_ROLL_OVERSHOOT,
    );
}

/// The roll path should arc up onto the platform before sweeping
/// across it. A midpoint on the trajectory must sit above the
/// straight-line diagonal between start and end, otherwise the
/// roll still reads as a diagonal slide.
#[test]
fn ledge_roll_uses_a_curved_arc_not_a_diagonal() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mid = roll_position(contact, 0.5);
    let target = roll_target(contact);
    let diagonal_mid = contact.anchor + (target - contact.anchor) * 0.5;

    assert!(
        mid.y < diagonal_mid.y - 4.0,
        "roll midpoint should ride higher onto the platform than a diagonal lerp: mid_y={} diagonal_mid_y={}",
        mid.y,
        diagonal_mid.y,
    );
    assert!(
        mid.x > contact.anchor.x + 2.0,
        "roll midpoint should already be committing inboard rather than staying glued to the ledge wall"
    );
}

/// Smash-Bros regrab guard: after the player voluntarily drops
/// from a ledge, `try_start_ledge_grab` must not re-snap the same
/// lip while the cooldown is still ticking. Without this the
/// auto-snap-on-fall path fires the moment gravity pushes
/// `vel.y` past `FALL_SNAP_MIN_VY` — roughly two frames after
/// release, while the player is still inside the chin-band.
#[test]
fn voluntary_drop_arms_a_regrab_cooldown() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    let mut scratch = make_hanging_player(contact);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = crate::movement::MovementTuning::default();
    let input = InputState {
        axis_y: 1.0, // down
        ..InputState::default()
    };
    let consumed =
        tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(consumed);
    assert!(
        scratch.ledge.grab.is_none(),
        "drop should release the ledge"
    );
    assert!(
        scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
        "drop should arm the regrab cooldown, got {}",
        scratch.ledge.release_cooldown,
    );
}

/// While the regrab cooldown is live, `try_start_ledge_grab` must
/// return false even if the player is falling fast past the same
/// ledge. This is the actual fix for Jon's 2026-05-23 instant-
/// regrab bug.
#[test]
fn regrab_cooldown_blocks_auto_snap_on_fall() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    // Player is falling fast — would normally trigger the
    // Smash-style auto-snap path.
    scratch.kinematics.vel = Vec2::new(0.0, 200.0);
    scratch.ledge.release_cooldown = LEDGE_REGRAB_COOLDOWN;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(
        !latched,
        "regrab cooldown should block the auto-snap-on-fall path"
    );
    assert!(scratch.ledge.grab.is_none());
}

/// After the cooldown expires the player can grab again normally.
/// Guards against the cooldown being permanent / never decaying.
#[test]
fn regrab_cooldown_expires_and_allows_fresh_grab() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.kinematics.vel = Vec2::new(0.0, 200.0);
    // Cooldown has already expired (e.g. simulation_timers ticked
    // it down past zero between frames). Auto-snap should be free
    // to fire again.
    scratch.ledge.release_cooldown = 0.0;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(
        latched,
        "with cooldown cleared, the same fall trajectory should re-grab"
    );
}

/// Both the outward ledge-release-jump and the vertical ledge-jump
/// also arm the cooldown — any voluntary release should prevent
/// instant regrab.
#[test]
fn ledge_jump_options_also_arm_regrab_cooldown() {
    let contact = LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    };
    // Outward release (jump + away).
    {
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            axis_x: -1.0, // away from a -1 wall_normal
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(scratch.ledge.grab.is_none());
        assert!(
            scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
            "ledge release should arm cooldown",
        );
    }
    // Vertical hop (jump alone).
    {
        let mut scratch = make_hanging_player(contact);
        let mut events = crate::movement::FrameEvents::default();
        let tuning = crate::movement::MovementTuning::default();
        let input = InputState {
            jump_pressed: true,
            ..InputState::default()
        };
        let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
        assert!(scratch.ledge.grab.is_none());
        assert!(
            scratch.ledge.release_cooldown >= LEDGE_REGRAB_COOLDOWN - 0.001,
            "ledge jump should arm cooldown",
        );
    }
}

/// Grabbing a ledge grants brief intangibility via
/// ``Player::dodge_roll_timer`` so an edge-guarding hit can't
/// punish the moment of contact.
#[test]
fn ledge_grab_arms_intangibility_window() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched, "expected ledge grab to latch");
    assert!(
        scratch.dodge.roll_timer >= LEDGE_GRAB_INVULN_TIME - 0.001,
        "grab should arm at least {}s of invuln, got {}",
        LEDGE_GRAB_INVULN_TIME,
        scratch.dodge.roll_timer,
    );
}

// ---- Momentum-carry boost tests (Jon 2026-05-23 feature) ----
//
// Invariants under test:
// 1. The boost is captured at grab time and rides the LedgeGrabState.
// 2. Eligible getup options (climb, roll, attack, ledge_jump) get
//    the boost folded into their exit velocity.
// 3. The DROP and outward LEDGE-RELEASE options DO NOT get the
//    boost — those are deliberate disengage actions.
// 4. The boost decays linearly across the configured window and
//    fires zero once the window has elapsed.
// 5. Setting `LedgeMomentumTuning::OFF` (or window=0.0) fully
//    disables the mechanic — restores the original "vel zeroed
//    on grab" feel.
// 6. Only INTO-platform horizontal and UPWARD vertical momentum
//    is counted; reverse / downward components are discarded.

fn into_platform_for(contact: LedgeContact) -> f32 {
    // Helper so tests don't have to memoize the sign convention.
    into_platform_axis(contact)
}

fn rightward_ledge_contact() -> LedgeContact {
    // Wall on player's RIGHT (wall_normal_x = -1). Platform is to
    // the right of the player, so into_platform_axis = +1.
    LedgeContact {
        wall_normal_x: -1.0,
        anchor: Vec2::new(86.0, 110.0),
        climb_target: Vec2::new(115.0, 77.0),
    }
}

#[test]
fn try_start_ledge_grab_captures_incoming_velocity() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    // Player arriving at the ledge with rightward + upward momentum
    // (i.e. running up against the wall during a jump).
    scratch.kinematics.vel = Vec2::new(180.0, -240.0);
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched);
    // After grab, vel is zeroed for the hang animation...
    assert_eq!(scratch.kinematics.vel, Vec2::ZERO);
    // ...but the state retains the pre-grab velocity for the
    // boost path.
    let state = scratch.ledge.grab.unwrap();
    assert!(
        (state.momentum_at_grab - Vec2::new(180.0, -240.0)).length() < 0.01,
        "momentum_at_grab should mirror pre-grab vel, got {:?}",
        state.momentum_at_grab,
    );
}

#[test]
fn ledge_boost_decays_linearly_across_window() {
    let tuning = MovementTuning::default();
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(200.0 * into_platform_for(contact), 0.0);
    // t=0: full boost.
    let early = ledge_boost(momentum, contact, 0.0, &tuning);
    // t=window/2: roughly half.
    let mid = ledge_boost(
        momentum,
        contact,
        tuning.ledge_momentum.window * 0.5,
        &tuning,
    );
    // t=window: zero (or right at zero per the linear weight).
    let late = ledge_boost(momentum, contact, tuning.ledge_momentum.window, &tuning);
    // t>window: zero.
    let past = ledge_boost(
        momentum,
        contact,
        tuning.ledge_momentum.window * 2.0,
        &tuning,
    );
    assert!(
        early.x.abs() > mid.x.abs(),
        "early > mid: {} > {}",
        early.x,
        mid.x
    );
    assert!(
        mid.x.abs() > late.x.abs(),
        "mid > late: {} > {}",
        mid.x,
        late.x
    );
    assert!(late.x.abs() < 0.01, "late ≈ 0, got {}", late.x);
    assert_eq!(past, Vec2::ZERO);
}

#[test]
fn ledge_boost_off_disables_mechanic() {
    let mut tuning = MovementTuning::default();
    tuning.ledge_momentum = crate::movement::LedgeMomentumTuning::OFF;
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(300.0 * into_platform_for(contact), -300.0);
    let boost = ledge_boost(momentum, contact, 0.0, &tuning);
    assert_eq!(boost, Vec2::ZERO, "OFF tuning must produce zero boost");
}

#[test]
fn ledge_boost_ignores_reverse_horizontal_momentum() {
    let tuning = MovementTuning::default();
    let contact = rightward_ledge_contact();
    // Momentum AWAY from the platform — into_platform is +1 here,
    // so a leftward (negative) vel doesn't earn a boost.
    let momentum = Vec2::new(-200.0, -200.0);
    let boost = ledge_boost(momentum, contact, 0.0, &tuning);
    assert_eq!(boost.x, 0.0, "reverse momentum should produce zero x boost");
    // Upward momentum is still rewarded though.
    assert!(
        boost.y < 0.0,
        "upward momentum should still produce a y boost"
    );
}

#[test]
fn ledge_boost_ignores_downward_vertical_momentum() {
    let tuning = MovementTuning::default();
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(200.0 * into_platform_for(contact), 300.0); // falling
    let boost = ledge_boost(momentum, contact, 0.0, &tuning);
    assert_eq!(
        boost.y, 0.0,
        "downward (falling) y momentum should not boost"
    );
    // Forward horizontal momentum still counts.
    assert!(boost.x.abs() > 0.0);
}

#[test]
fn ledge_boost_clamps_at_caps() {
    let tuning = MovementTuning::default();
    let contact = rightward_ledge_contact();
    // Extreme incoming momentum (e.g. dash + air jump combo).
    let momentum = Vec2::new(2_000.0 * into_platform_for(contact), -2_000.0);
    let boost = ledge_boost(momentum, contact, 0.0, &tuning);
    assert!(
        boost.x.abs() <= tuning.ledge_momentum.x_cap + 0.01,
        "x boost should clamp to x_cap"
    );
    assert!(
        boost.y.abs() <= tuning.ledge_momentum.y_cap + 0.01,
        "y boost should clamp to y_cap"
    );
}

#[test]
fn ledge_jump_with_quick_action_carries_momentum() {
    let contact = rightward_ledge_contact();
    // Player came in with strong rightward (into-platform) momentum
    // before grabbing.
    let momentum = Vec2::new(220.0, -100.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let baseline_player = make_hanging_player_with_momentum(contact, Vec2::ZERO);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    let input = InputState {
        jump_pressed: true,
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    // Replay the same input on the zero-momentum baseline so we
    // can compare "with boost" vs "without boost" exit velocities.
    let mut baseline = baseline_player;
    let mut baseline_events = crate::movement::FrameEvents::default();
    let _ = tick_active_ledge_grab_scratch(
        &mut baseline,
        input,
        0.016,
        tuning,
        &mut baseline_events,
    );
    // The boosted exit velocity should be larger in magnitude
    // along the carried axes than the unboosted one.
    assert!(
        scratch.kinematics.vel.x.abs() > baseline.kinematics.vel.x.abs(),
        "expected boosted ledge-jump to exceed baseline x: {} vs {}",
        scratch.kinematics.vel.x,
        baseline.kinematics.vel.x,
    );
    assert!(
        scratch.kinematics.vel.y < baseline.kinematics.vel.y,
        "expected boosted ledge-jump to exceed baseline upward (more negative): {} vs {}",
        scratch.kinematics.vel.y,
        baseline.kinematics.vel.y,
    );
}

#[test]
fn drop_does_not_apply_boost() {
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(220.0, -120.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    let input = InputState {
        axis_y: 1.0, // down
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    assert!(scratch.ledge.grab.is_none());
    // After a drop the player has no claimed launch velocity —
    // the existing behavior is that vel is whatever it was when
    // the ledge released, and for `want_drop` we leave it
    // untouched (vel was ZERO from the hang). Importantly, we
    // do NOT add any boost.
    assert_eq!(
        scratch.kinematics.vel,
        Vec2::ZERO,
        "drop must not pick up momentum boost"
    );
}

#[test]
fn outward_ledge_release_does_not_apply_boost() {
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(220.0, -120.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let baseline_player = make_hanging_player_with_momentum(contact, Vec2::ZERO);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    // jump + AWAY from platform. away_from_platform here is -1
    // (left) since wall_normal_x is -1.
    let away = away_from_platform_axis(contact);
    let input = InputState {
        jump_pressed: true,
        axis_x: away,
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.016, tuning, &mut events);
    // Replay with zero momentum to confirm both produce the SAME
    // exit vel (i.e. no boost applied).
    let mut baseline = baseline_player;
    let mut baseline_events = crate::movement::FrameEvents::default();
    let _ = tick_active_ledge_grab_scratch(
        &mut baseline,
        input,
        0.016,
        tuning,
        &mut baseline_events,
    );
    assert!(
        (scratch.kinematics.vel - baseline.kinematics.vel).length() < 0.5,
        "outward release must produce identical vel with and without momentum, \
         got boosted={:?} baseline={:?}",
        scratch.kinematics.vel,
        baseline.kinematics.vel,
    );
}

#[test]
fn climb_finish_carries_momentum_when_grabbed_with_speed() {
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(220.0, -120.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    // Start the climb (Up).
    let input = InputState {
        axis_y: -1.0,
        ..InputState::default()
    };
    let _ = tick_active_ledge_grab_scratch(&mut scratch, input, 0.001, tuning, &mut events);
    // Run the full climb in one big tick.
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState::default(),
        LEDGE_CLIMB_TIME + 0.05,
        tuning,
        &mut events,
    );
    assert!(scratch.ledge.grab.is_none(), "climb should have finished");
    // Carried x velocity into platform. The forward-into is +x
    // here (right-side wall_normal, into = +1).
    assert!(
        scratch.kinematics.vel.x > 0.0,
        "expected positive x exit velocity from carry, got {}",
        scratch.kinematics.vel.x,
    );
}

/// Regression for Jon's "horizontal getup shouldn't be adding
/// vertical boost" — the player just got placed standing on the
/// platform; a residual upward vel.y would relaunch them off
/// it. Climb / roll / attack finish must zero the Y component
/// of the boost; ledge-jump (a vertical hop) still keeps both.
#[test]
fn climb_finish_does_not_carry_vertical_boost() {
    let contact = rightward_ledge_contact();
    // Strong upward incoming momentum (e.g. recovery via double-jump).
    let momentum = Vec2::new(220.0, -500.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    // Start + complete the climb in two big ticks.
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState {
            axis_y: -1.0,
            ..InputState::default()
        },
        0.001,
        tuning,
        &mut events,
    );
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState::default(),
        LEDGE_CLIMB_TIME + 0.05,
        tuning,
        &mut events,
    );
    assert!(scratch.ledge.grab.is_none());
    assert!(
        scratch.kinematics.vel.x > 0.0,
        "horizontal carry should still apply, got vx={}",
        scratch.kinematics.vel.x,
    );
    assert_eq!(
        scratch.kinematics.vel.y, 0.0,
        "climb-finish must NOT launch the player upward off the platform; got vy={}",
        scratch.kinematics.vel.y,
    );
}

/// The boost mechanic now ALSO shortens the getup transition
/// when momentum was carried. Without this, a 0.24-s climb of
/// dead-zero velocity feels sluggish — the post-transition kick
/// can't compensate. Tests that a fresh-momentum getup completes
/// in noticeably less time than a baseline getup with the
/// speedup disabled.
#[test]
fn getup_transition_completes_faster_with_momentum_carry() {
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(220.0, -120.0);
    let mut boosted = make_hanging_player_with_momentum(contact, momentum);
    let mut baseline = make_hanging_player_with_momentum(contact, momentum);
    let tuning = MovementTuning::default();
    // Disable just the speedup on the baseline so the comparison
    // isolates THIS knob.
    let mut baseline_tuning = tuning;
    baseline_tuning.ledge_momentum.getup_speedup_gain = 0.0;
    let input = InputState {
        axis_y: -1.0,
        ..InputState::default()
    };
    // Start both climbs.
    let mut events = crate::movement::FrameEvents::default();
    let _ = tick_active_ledge_grab_scratch(&mut boosted, input, 0.001, tuning, &mut events);
    let _ = tick_active_ledge_grab_scratch(
        &mut baseline,
        input,
        0.001,
        baseline_tuning,
        &mut events,
    );
    // Step both forward by exactly the BASELINE climb time. The
    // baseline should be ~done; the boosted player should be
    // OFF the ledge already (we're past their shortened duration).
    let _ = tick_active_ledge_grab_scratch(
        &mut boosted,
        InputState::default(),
        LEDGE_CLIMB_TIME * 0.6,
        tuning,
        &mut events,
    );
    let _ = tick_active_ledge_grab_scratch(
        &mut baseline,
        InputState::default(),
        LEDGE_CLIMB_TIME * 0.6,
        baseline_tuning,
        &mut events,
    );
    assert!(
        boosted.ledge.grab.is_none(),
        "boosted climb should have completed by 60% of base duration"
    );
    assert!(
        baseline.ledge.grab.is_some(),
        "baseline climb should still be in progress at 60% of base duration"
    );
}

/// `try_start_ledge_grab` now prefers `pre_wall_vel` over
/// `scratch.kinematics.vel` when the snapshot is fresh — because wall-cling
/// and wall-collision shred the actual approach velocity by the
/// time the grab fires. This was Jon's "I don't feel the boost
/// on jump option at all, even with gain > 1 and caps in
/// thousands" bug: vel was 0 at capture time, so any gain
/// multiplied to zero.
#[test]
fn grab_prefers_pre_wall_vel_when_fresh() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    // Simulate "wall-cling killed our approach velocity": current
    // vel.x is zero (collision zeroed it), but pre_wall_vel
    // still has the approach momentum from a frame ago.
    scratch.kinematics.vel = Vec2::new(0.0, 50.0);
    scratch.wall.pre_wall_vel = Vec2::new(260.0, -180.0);
    scratch.wall.pre_wall_vel_age = 0.05; // fresh
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched);
    let state = scratch.ledge.grab.unwrap();
    assert!(
        (state.momentum_at_grab - Vec2::new(260.0, -180.0)).length() < 0.01,
        "grab should snapshot the pre-wall vel, got {:?}",
        state.momentum_at_grab,
    );
}

/// Once `pre_wall_vel_age` exceeds the freshness threshold, the
/// grab falls back to `scratch.kinematics.vel` so a player who clung the
/// wall for ages can't claim a fossil approach.
#[test]
fn grab_falls_back_to_current_vel_when_pre_wall_stale() {
    let world = world_with(vec![Block::solid(
        "ledge",
        Vec2::new(100.0, 100.0),
        Vec2::new(200.0, 200.0),
    )]);
    let mut scratch = scratch_at(Vec2::new(86.0, 110.0));
    scratch.abilities.abilities.ledge_grab = true;
    scratch.kinematics.vel = Vec2::new(0.0, 50.0);
    scratch.wall.pre_wall_vel = Vec2::new(260.0, -180.0);
    scratch.wall.pre_wall_vel_age = LEDGE_REGRAB_COOLDOWN * 4.0; // very stale
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;
    let mut events = crate::movement::FrameEvents::default();
    let latched =
        try_start_ledge_grab_scratch(&world, &mut scratch, InputState::default(), &mut events);
    assert!(latched);
    let state = scratch.ledge.grab.unwrap();
    // Should fall back to current vel, NOT the stale pre_wall.
    assert!(
        (state.momentum_at_grab - Vec2::new(0.0, 50.0)).length() < 0.01,
        "stale pre_wall must be discarded; got {:?}",
        state.momentum_at_grab,
    );
}

#[test]
fn boost_decays_to_zero_outside_window() {
    // If the player lingers on the ledge past the boost window
    // and THEN climbs, the carry should be zero. Verifies the
    // window gate uses the grab-to-action time, not zero.
    let contact = rightward_ledge_contact();
    let momentum = Vec2::new(220.0, -120.0);
    let mut scratch = make_hanging_player_with_momentum(contact, momentum);
    let mut events = crate::movement::FrameEvents::default();
    let tuning = MovementTuning::default();
    // Sit on the ledge for longer than the boost window with no
    // input (so we don't auto-climb).
    let dt = tuning.ledge_momentum.window + 0.05;
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState::default(),
        dt,
        tuning,
        &mut events,
    );
    // Now climb.
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState {
            axis_y: -1.0,
            ..InputState::default()
        },
        0.001,
        tuning,
        &mut events,
    );
    let _ = tick_active_ledge_grab_scratch(
        &mut scratch,
        InputState::default(),
        LEDGE_CLIMB_TIME + 0.05,
        tuning,
        &mut events,
    );
    assert!(scratch.ledge.grab.is_none(), "climb should have finished");
    assert_eq!(
        scratch.kinematics.vel,
        Vec2::ZERO,
        "post-window climb should NOT carry momentum, got {:?}",
        scratch.kinematics.vel,
    );
}
