//! Tests for the per-actor animation pickers: that player/enemy/NPC
//! state (ledge, shield, dodge, swim, climb, crouch, shoot, aim,
//! wall-jump, interact, aerial flight) maps to the expected
//! `CharacterAnim`, and that the action/ledge rows resolve by name and
//! loop correctly.

use super::*;
use ambition_characters::actor::BodyCombat;
use ambition_platformer2d_shared_tangle::camera_ease::PlayerBlinkCameraState;

/// Build a player + the three default state inputs that
/// `pick_player_anim` consumes. Tests then mutate just the
/// fields relevant to the case under test.
/// Bundle of every cluster component `pick_player_anim` reads.
/// Tests mutate just the fields relevant to the case under test.
struct PickClusters {
    kinematics: ae::BodyKinematics,
    ground: ae::BodyGroundState,
    /// The published maneuver projection the pickers consume (ADR 0024).
    facts: ae::BodyMotionFacts,
    flight: ae::BodyFlightState,
    body_mode: ae::BodyModeState,
    env_contact: ae::BodyEnvironmentContact,
    abilities: ae::BodyAbilities,
    shield: ae::BodyShieldState,
    /// The body's own reference basis. Defaults to ordinary down-gravity, so
    /// every pre-existing case reads exactly as it did before the frame became
    /// an input; a rotated-gravity case sets it.
    frame: ae::AccelerationFrame,
}

impl PickClusters {
    fn defaults() -> Self {
        Self {
            kinematics: Default::default(),
            ground: Default::default(),
            facts: Default::default(),
            flight: Default::default(),
            body_mode: Default::default(),
            env_contact: Default::default(),
            abilities: Default::default(),
            shield: Default::default(),
            frame: ae::AccelerationFrame::new(ae::Vec2::new(0.0, 1.0)),
        }
    }
}

fn pick_inputs() -> (
    BodyAnimFacts,
    BodyCombat,
    PlayerBlinkCameraState,
    PickClusters,
) {
    (
        BodyAnimFacts::default(),
        BodyCombat::default(),
        PlayerBlinkCameraState::default(),
        PickClusters::defaults(),
    )
}

fn pick(
    anim: &BodyAnimFacts,
    combat: &BodyCombat,
    blink_cam: &PlayerBlinkCameraState,
    attack: Option<&MeleeSwing>,
    c: &PickClusters,
) -> CharacterAnim {
    pick_player_anim(
        anim,
        combat,
        blink_cam,
        attack,
        &c.kinematics,
        &c.ground,
        &c.facts,
        &c.flight,
        &c.body_mode,
        &c.env_contact,
        &c.abilities,
        &c.shield,
        c.frame,
    )
}

fn hang_state(getup: ae::LedgeGetupKind, climbing: bool) -> ae::LedgeFacts {
    ae::LedgeFacts {
        climbing,
        getup_kind: getup,
    }
}

/// While hanging (not climbing), the picker returns the static
/// `LedgeGrab` row regardless of getup_kind. The hang is the
/// pre-action state; the getup kind is only meaningful once
/// the player commits.
#[test]
fn hang_returns_ledge_grab_regardless_of_getup_kind() {
    for kind in [
        ae::LedgeGetupKind::Climb,
        ae::LedgeGetupKind::Roll,
        ae::LedgeGetupKind::Attack,
    ] {
        let (anim, combat, blink_cam, mut clusters) = pick_inputs();
        clusters.facts.ledge = Some(hang_state(kind, false));
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &clusters),
            CharacterAnim::LedgeGrab,
            "hang with kind {:?} must read as LedgeGrab",
            kind,
        );
    }
}

/// Climb is the default getup; picker should return the
/// `LedgeGetup` row (the existing mantle pop-up animation).
#[test]
fn climbing_with_climb_kind_returns_ledge_getup() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.facts.ledge = Some(hang_state(ae::LedgeGetupKind::Climb, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeGetup,
    );
}

/// Roll getup picks the new `LedgeRoll` row.
#[test]
fn climbing_with_roll_kind_returns_ledge_roll() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.facts.ledge = Some(hang_state(ae::LedgeGetupKind::Roll, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeRoll,
    );
}

/// Attack getup picks the new `LedgeGetupAttack` row. The
/// `slash_anim_timer` happens to be 0 here so the regular
/// directional-attack branch doesn't preempt the ledge branch;
/// the next test pins that ordering.
#[test]
fn climbing_with_attack_kind_returns_ledge_getup_attack() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.facts.ledge = Some(hang_state(ae::LedgeGetupKind::Attack, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeGetupAttack,
    );
}

/// The non-looping list must include the two new ledge rows so `CharacterAnimator` doesn't keep
/// cycling their frames after the engine transition completes.
#[test]
fn new_ledge_rows_are_non_looping() {
    assert!(non_looping(CharacterAnim::LedgeRoll));
    assert!(non_looping(CharacterAnim::LedgeGetupAttack));
    // Sanity: the prior LedgeGetup also stays non-looping.
    assert!(non_looping(CharacterAnim::LedgeGetup));
}

/// `from_name` round-trips the new row names so the spritesheet
/// RON parser can resolve `"ledge_roll"` / `"ledge_getup_attack"`
/// from the generator output without dropping them silently.
#[test]
fn from_name_resolves_new_ledge_rows() {
    assert_eq!(
        CharacterAnim::from_name("ledge_roll"),
        Some(CharacterAnim::LedgeRoll),
    );
    assert_eq!(
        CharacterAnim::from_name("ledge_getup_attack"),
        Some(CharacterAnim::LedgeGetupAttack),
    );
}

/// Shield-up flag wins over slash / aim. Only fires when
/// `abilities.shield` is true — otherwise the shield cluster's
/// `active` flag is unreachable from input.
#[test]
fn shield_active_with_ability_returns_block() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.abilities.abilities.shield = true;
    clusters.shield.active = true;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Block,
    );
}

/// Grounded dodge roll picks `DodgeRoll`, but a roll that fires as
/// part of a ledge getup keeps the dedicated `LedgeRoll` row. The
/// engine drives both with the same `dodge_rolling` fact; this pins
/// the visual gate that picks the right pose for the situation.
#[test]
fn dodge_roll_grounded_vs_ledge_getup() {
    // Grounded: no ledge state, just the dodge-roll fact.
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.facts.dodge_rolling = true;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::DodgeRoll,
    );
    // Ledge roll: same fact set, plus a ledge climbing roll.
    // The ledge-state branch must win.
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.facts.dodge_rolling = true;
    clusters.facts.ledge = Some(hang_state(ae::LedgeGetupKind::Roll, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeRoll,
    );
}

/// Swim row picks when the player is in water AND has the swim
/// ability. Without the ability the picker falls back to whatever
/// locomotion the kinematics imply (gravity will fight it but at
/// least the sprite isn't trying to play a swim row the character
/// can't actually do).
#[test]
fn water_contact_with_swim_ability_returns_swim() {
    let water = ae::WaterContact {
        kind: ae::WaterKind::Clear,
        region_aabb: ae::Aabb::new(ae::Vec2::ZERO, ae::Vec2::new(64.0, 64.0)),
        surface_y: 0.0,
        submersion: 1.0,
        spec: ae::WaterVolumeSpec::default(),
    };
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.abilities.abilities.swim = true;
    clusters.env_contact.water = Some(water);
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Swim,
    );
    // Same water contact but no swim ability — picker should NOT
    // return Swim.
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.env_contact.water = Some(water);
    assert_ne!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Swim,
    );
}

/// BodyMode::Climbing picks `LadderClimb` (distinct from the
/// wall-grab path which is for solid-block wall-cling).
#[test]
fn climbing_body_mode_returns_ladder_climb() {
    use ambition_platformer2d_core::player_state::BodyMode;
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.body_mode.body_mode = BodyMode::Climbing;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LadderClimb,
    );
}

/// Crouching body mode picks `Crouch` from the locomotion
/// fallback once the airborne / cling / dash branches all fall
/// through.
#[test]
fn crouching_body_mode_returns_crouch() {
    use ambition_platformer2d_core::player_state::BodyMode;
    let (mut anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.body_mode.body_mode = BodyMode::Crouching;
    clusters.ground.on_ground = true;
    let _ = &mut anim;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Crouch,
    );
}

/// `shoot_anim_timer > 0.0` picks the `Shoot` row, and the row
/// wins over slash so a same-frame swing doesn't immediately stomp
/// the muzzle-flash pose.
#[test]
fn shoot_anim_timer_returns_shoot() {
    let (mut anim, combat, blink_cam, clusters) = pick_inputs();
    anim.shoot_anim_timer = 0.10;
    anim.slash_anim_timer = 0.10;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Shoot,
    );
}

/// `aim_anim_active` picks `Aim` only when no higher-priority
/// state (shoot, slash, shield) is set.
#[test]
fn aim_anim_active_returns_aim() {
    let (mut anim, combat, blink_cam, clusters) = pick_inputs();
    anim.aim_anim_active = true;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Aim,
    );
}

/// `wall_jump_anim_timer > 0.0` picks `WallJump` even while the
/// player is airborne moving upward. Pre-poison the result with
/// the default fall-through so a missed return trips this.
#[test]
fn wall_jump_anim_timer_returns_wall_jump_when_airborne() {
    let (mut anim, combat, blink_cam, mut clusters) = pick_inputs();
    anim.wall_jump_anim_timer = 0.15;
    clusters.ground.on_ground = false;
    clusters.kinematics.vel.y = -200.0;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::WallJump,
    );
}

/// `interact_anim_timer > 0.0` picks `Interact`. Set from
/// NPC / switch / chest open paths; held briefly while the
/// interaction commits.
#[test]
fn rolling_fact_returns_the_looping_ball_over_air_and_dash_reads() {
    // Grounded roll.
    let (mut anim, combat, blink_cam, clusters) = pick_inputs();
    anim.rolling = true;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Roll,
    );
    // A ball flying off a ramp is still a ball: rolling outranks the airborne
    // Jump/Fall gate.
    let (mut anim, combat, blink_cam, mut clusters) = pick_inputs();
    anim.rolling = true;
    clusters.ground.on_ground = false;
    clusters.kinematics.vel.y = 300.0;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Roll,
    );
    // ...and outranks a still-draining dash-startup pre-roll (the launch tick
    // can leave both facts set; the persistent curl wins).
    let (mut anim, combat, blink_cam, clusters) = pick_inputs();
    anim.rolling = true;
    anim.dash_startup_timer = 0.05;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Roll,
    );
}

#[test]
fn skidding_fact_returns_skid_over_the_locomotion_tail() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.ground.on_ground = true;
    clusters.facts.skidding = true;
    // Fast enough that the tail would otherwise read Run.
    clusters.kinematics.vel.x = 400.0;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Skid,
    );
}

#[test]
fn the_ball_loops_and_falls_back_through_the_dodge_tumble() {
    // The persistent curl must LOOP (a Sonic ball keeps spinning); the one-shot
    // dodge tumble stays held.
    assert!(!non_looping(CharacterAnim::Roll));
    assert!(non_looping(CharacterAnim::DodgeRoll));
    // Name resolution: `ball` is the loop, `roll` stays the dodge tumble.
    assert_eq!(CharacterAnim::from_name("ball"), Some(CharacterAnim::Roll));
    assert_eq!(
        CharacterAnim::from_name("roll"),
        Some(CharacterAnim::DodgeRoll)
    );
    assert_eq!(CharacterAnim::from_name("skid"), Some(CharacterAnim::Skid));
    // A sheet without a ball row shows its curl (the dodge tumble), not a run.
    assert_eq!(
        CharacterAnim::Roll.base_pose(),
        Some(CharacterAnim::DodgeRoll)
    );
    assert_eq!(CharacterAnim::Skid.base_pose(), Some(CharacterAnim::Run));
}

#[test]
fn interact_anim_timer_returns_interact() {
    let (mut anim, combat, blink_cam, clusters) = pick_inputs();
    anim.interact_anim_timer = 0.20;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::Interact,
    );
}

/// `from_name` round-trips all the new row names so the
/// spritesheet RON parser can resolve them without dropping rows
/// silently.
#[test]
fn from_name_resolves_all_new_action_rows() {
    for (name, expected) in [
        ("crouch", CharacterAnim::Crouch),
        // ⛔ this used to assert `Crawl`, and that alias was the defect: 23
        // sheets author `crouch_walk` and one authors `crawl`, so the art only
        // ever played while the body was CRAWLING and a crouch-walking body
        // drew a statue. `Crawl` now FALLS BACK to this row, so those 23 keep
        // drawing what they always drew.
        ("crouch_walk", CharacterAnim::CrouchWalk),
        ("crawl", CharacterAnim::Crawl),
        ("crouch_jump", CharacterAnim::CrouchJump),
        ("crawl", CharacterAnim::Crawl),
        ("slide", CharacterAnim::Slide),
        ("climb", CharacterAnim::LadderClimb),
        ("ladder_climb", CharacterAnim::LadderClimb),
        ("swim", CharacterAnim::Swim),
        ("shoot", CharacterAnim::Shoot),
        ("aim", CharacterAnim::Aim),
        ("charge", CharacterAnim::Charge),
        ("block", CharacterAnim::Block),
        ("shield", CharacterAnim::Block),
        ("roll", CharacterAnim::DodgeRoll),
        ("dodge_roll", CharacterAnim::DodgeRoll),
        ("wall_jump", CharacterAnim::WallJump),
        ("interact", CharacterAnim::Interact),
        ("jab", CharacterAnim::Slash),
        ("punch", CharacterAnim::Punch),
        ("special", CharacterAnim::Special),
    ] {
        assert_eq!(
            CharacterAnim::from_name(name),
            Some(expected),
            "from_name({name:?}) should map to {expected:?}",
        );
    }
}

/// A grounded, alive, not-swinging actor disposition — the inert baseline tests
/// flip one fact off.
fn actor_state() -> ActorAnimState {
    ActorAnimState {
        alive: true,
        hit_flash: false,
        aerial: false,
        ..Default::default()
    }
}

/// Build a one-frame melee swing with the given intent, sitting in its startup
/// (telegraph) phase so the picker reads it as an active swing.
fn swing_with_intent(intent: ambition_combat::AttackIntent) -> MeleeSwing {
    MeleeSwing::new(ambition_combat::AttackSpec {
        intent,
        startup_seconds: 0.1,
        active_seconds: 0.1,
        recovery_seconds: 0.1,
        hitbox_offset: ae::Vec2::ZERO,
        hitbox_half_size: ae::Vec2::new(8.0, 8.0),
        self_impulse: ae::Vec2::ZERO,
        knockback: ae::Vec2::ZERO,
        damage_kind: ambition_entity_catalog::placements::DamageKind::Slash,
        can_pogo: false,
        damage_override: None,
    })
}

/// A CPU FIGHTER AT FULL SPRINT DRAWS `run`, NOT `walk`.
///
/// the same shape ruled on — a body semantic must not depend on which control road the body
/// happens to occupy — so the fix is the published `BodyMotionFacts:running`, which is speed
/// against THIS body's own top speed rather than an absolute number no heavyweight could reach.
///
/// the second half is the floor: a body under the gait line must still be
/// `Walk`, or this would have deleted the walk instead of the run.
#[test]
fn an_actor_in_the_run_gait_is_not_drawn_walking() {
    let (_, _, _, mut c) = pick_inputs();
    c.ground.on_ground = true;
    c.kinematics = walking_in_gravity(ae::Vec2::new(0.0, 1.0), 240.0);

    c.facts.running = true;
    assert_eq!(
        pick_actor(&c, None, actor_state()),
        CharacterAnim::Run,
        "a CPU fighter in the run gait was drawn walking — the actor road is \
         capping the gait again"
    );

    c.facts.running = false;
    assert_eq!(
        pick_actor(&c, None, actor_state()),
        CharacterAnim::Walk,
        "a body below the gait line was drawn running, so the run row has \
         swallowed the walk"
    );
}

fn pick_actor(
    c: &PickClusters,
    swing: Option<&MeleeSwing>,
    state: ActorAnimState,
) -> CharacterAnim {
    pick_actor_anim(
        &c.kinematics,
        &c.ground,
        &c.facts,
        &c.flight,
        &c.body_mode,
        &c.env_contact,
        &c.abilities,
        &c.shield,
        swing,
        state,
        c.frame,
    )
}

#[test]
fn actors_show_movement_overlay_poses_like_the_player() {
    // §A9: the movement-driven overlays (wall-jump / dash-startup / landing) are
    // no longer player-only — an actor whose BodyAnimFacts armed one shows that
    // pose, through the SAME `pick_body_anim` ladder the player uses.
    // Grounded, standing still — the base ladder reads Idle with no overlay.
    let mut c = PickClusters::defaults();
    c.ground.on_ground = true;
    // Wall-jump + dash-startup are high-priority overlays (win over locomotion).
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                wall_jump: true,
                ..actor_state()
            }
        ),
        CharacterAnim::WallJump,
    );
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                dash_startup: true,
                ..actor_state()
            }
        ),
        CharacterAnim::DashStartup,
    );
    // Landing grades hard vs soft; it only shows on the ground.
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                landing: Some(true),
                ..actor_state()
            }
        ),
        CharacterAnim::LandHard,
    );
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                landing: Some(false),
                ..actor_state()
            }
        ),
        CharacterAnim::LandRecovery,
    );
    // No overlay armed → the base ladder (a grounded idle actor reads Idle).
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::Idle);
}

#[test]
fn actors_animate_from_real_state_regardless_of_disposition() {
    // One actor path; disposition (hostile/peaceful) is not an animation fork —
    // every read below is the actor's REAL ECS state, not its label.

    // Flyer (parrot): Fly while moving, Idle while hovering/perched.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(40.0, -30.0);
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                aerial: true,
                ..actor_state()
            }
        ),
        CharacterAnim::Fly,
    );
    let c = PickClusters::defaults();
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                aerial: true,
                ..actor_state()
            }
        ),
        CharacterAnim::Idle,
        "a still hover / landed perch is Idle, not Fly",
    );
    // A grounded (non-aerial) actor launched upward now reads the airborne
    // Jump/Fall gate (it shares the player's full ladder) — never Fly.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(40.0, -200.0); // top-left coords: up
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::Jump,);
    // An active melee wins over locomotion — and a PEACEFUL-disposition actor
    // that swings animates its attack too (the old NPC path dropped this read).
    // The swing's own intent picks the directional row (Forward → AttackSide,
    // which `resolve_anim` later walks down to a slash-only sheet's slash).
    let c = PickClusters::defaults();
    assert_eq!(
        pick_actor(
            &c,
            Some(&swing_with_intent(ambition_combat::AttackIntent::Forward)),
            actor_state()
        ),
        CharacterAnim::AttackSide,
    );
    assert_eq!(
        pick_actor(
            &c,
            Some(&swing_with_intent(ambition_combat::AttackIntent::Up)),
            actor_state()
        ),
        CharacterAnim::AttackUp,
        "an up-tilt swing reads the up row — actors share the player's swing map",
    );
    // Death reads from real state for ANY actor, moving or not.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(50.0, 0.0);
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                alive: false,
                ..actor_state()
            }
        ),
        CharacterAnim::Death,
    );
}

/// The whole point of the cluster wiring: an actor animates the RICH movement
/// abilities its brain drives its real `Body*` clusters into — the same clusters,
/// read through the same builder, as the player. No per-archetype branch; a
/// brain (or an LLM) flipping a cluster is all it takes.
#[test]
fn actors_animate_rich_cluster_abilities() {
    // Dash: a brain that fires the body's dash limb → Dash, mid-air or not.
    let mut c = PickClusters::defaults();
    c.facts.dashing = true;
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::Dash);

    // Flight toggled on (not the aerial archetype flag — the real flight cluster)
    // → Fly.
    let mut c = PickClusters::defaults();
    c.flight.fly_enabled = true;
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::Fly);

    // Shield raised, with the ability enabled → Block.
    let mut c = PickClusters::defaults();
    c.abilities.abilities.shield = true;
    c.shield.active = true;
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::Block);

    // Ladder climb from body mode → LadderClimb.
    let mut c = PickClusters::defaults();
    c.body_mode.body_mode = ambition_platformer2d_core::player_state::BodyMode::Climbing;
    assert_eq!(
        pick_actor(&c, None, actor_state()),
        CharacterAnim::LadderClimb
    );

    // A hit-flashing actor reads Hit over its locomotion.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(80.0, 0.0);
    c.ground.on_ground = true;
    assert_eq!(
        pick_actor(
            &c,
            None,
            ActorAnimState {
                hit_flash: true,
                ..actor_state()
            }
        ),
        CharacterAnim::Hit,
    );
}

/// `Roll`'s own fallback is `DodgeRoll`, so a sheet with one curl still animates; a sheet with both
/// shows two maneuvers.
#[test]
fn an_air_dodge_picks_its_own_row_and_falls_back_to_the_ground_roll() {
    let mut view = BodyAnimView {
        air_dodge: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&view), CharacterAnim::Roll);
    assert_eq!(
        CharacterAnim::Roll.base_pose(),
        Some(CharacterAnim::DodgeRoll),
        "a sheet without a roll row still curls"
    );
    view.dodge_roll = true;
    assert_eq!(
        pick_body_anim(&view),
        CharacterAnim::DodgeRoll,
        "the grounded roll outranks it when both are somehow set"
    );
}

/// A crouch that MOVES and a crouch that LEAVES THE GROUND each get their own
/// row, and neither could before.
///
/// The compact stance answered with one row whether the body was shuffling or
/// still, so a crouch-walk was a statue. And the airborne branch ran BEFORE the
/// compact one, so a body that jumped while ducked drew a plain jump — a crouch
/// jump is a real move in the games this borrows from and nothing could draw
/// one. Both fall back, so a sheet that authors neither is unchanged.
#[test]
fn a_crouch_that_moves_and_a_crouch_that_jumps_are_their_own_rows() {
    let still = BodyAnimView {
        compact: CompactBody::Crouch,
        idle_below: 1.0,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&still), CharacterAnim::Crouch);

    let shuffling = BodyAnimView {
        speed: 40.0,
        ..still
    };
    assert_eq!(
        pick_body_anim(&shuffling),
        CharacterAnim::CrouchWalk,
        "a crouch with speed on it is a crouch WALK"
    );

    let leaping = BodyAnimView {
        airborne: true,
        moving_up: true,
        ..still
    };
    assert_eq!(
        pick_body_anim(&leaping),
        CharacterAnim::CrouchJump,
        "the airborne branch used to run first and swallow this"
    );

    // A body that jumped WITHOUT ducking is untouched, and so is a fall.
    let plain = BodyAnimView {
        airborne: true,
        moving_up: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&plain), CharacterAnim::Jump);

    for (row, base) in [
        (CharacterAnim::CrouchWalk, CharacterAnim::Crouch),
        (CharacterAnim::CrouchJump, CharacterAnim::Jump),
        // And the row `crouch_walk` used to BE keeps drawing it.
        (CharacterAnim::Crawl, CharacterAnim::CrouchWalk),
    ] {
        assert_eq!(
            row.base_pose(),
            Some(base),
            "a sheet without {row:?} still animates"
        );
    }
}

/// A knocked-down body draws the knockdown, not the hit flash it is still
/// inside. Hitstun outlives the landing, so an ordering that read `hit` first
/// would make the whole floor game invisible.
#[test]
fn the_floor_game_outranks_the_hit_row() {
    let prone = BodyAnimView {
        knocked_down: true,
        hit: true,
        tumbling: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&prone), CharacterAnim::LandHard);

    let standing = BodyAnimView {
        getting_up: true,
        hit: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&standing), CharacterAnim::LandRecovery);

    let flying = BodyAnimView {
        tumbling: true,
        ..Default::default()
    };
    assert_eq!(
        pick_body_anim(&flying),
        CharacterAnim::Hit,
        "a launched body holds the struck pose through its arc"
    );
}

// ── GRAVITY-RELATIVE LOCOMOTION METRIC ───────────────────────────────────────
//
// Turn gravity sideways and the run axis becomes world-y, `|vel.x|` reads ~0, and a body at full
// running speed is below `idle_below`.

/// The four cardinal gravities, and a walking speed expressed in each one's own
/// run axis. `side` is `down` rotated -90°, matching `AccelerationFrame::new`.
fn walking_in_gravity(down: ae::Vec2, speed: f32) -> ae::BodyKinematics {
    let frame = ae::AccelerationFrame::new(down);
    ae::BodyKinematics {
        vel: frame.side * speed,
        ..Default::default()
    }
}

#[test]
fn a_grounded_body_walks_in_every_gravity_not_just_the_vertical_ones() {
    let (anim, combat, blink_cam, mut c) = pick_inputs();
    c.ground.on_ground = true;

    for down in [
        ae::Vec2::new(0.0, 1.0),  // ordinary floor
        ae::Vec2::new(0.0, -1.0), // ceiling: upside down
        ae::Vec2::new(1.0, 0.0),  // wall on the right — reported BROKEN
        ae::Vec2::new(-1.0, 0.0), // wall on the left  — reported BROKEN
    ] {
        c.kinematics = walking_in_gravity(down, 120.0);
        c.frame = ae::AccelerationFrame::new(down);
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &c),
            CharacterAnim::Walk,
            "a body running along its own floor under gravity {down:?} played \
             the idle row: the locomotion metric is measuring a WORLD axis \
             rather than the body's run axis"
        );
    }
}

/// the poison. Without it the fix could be "always report Walk when
/// grounded", which would pass the test above and make standing still animate.
#[test]
fn a_grounded_body_standing_still_is_idle_in_every_gravity() {
    let (anim, combat, blink_cam, mut c) = pick_inputs();
    c.ground.on_ground = true;

    for down in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        c.kinematics = walking_in_gravity(down, 0.0);
        c.frame = ae::AccelerationFrame::new(down);
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &c),
            CharacterAnim::Idle,
            "a motionless body under gravity {down:?} animated as walking"
        );
    }
}

/// Only motion along `side` counts.
#[test]
fn motion_along_the_fall_axis_is_not_walking() {
    let (anim, combat, blink_cam, mut c) = pick_inputs();
    c.ground.on_ground = true;

    for down in [ae::Vec2::new(0.0, 1.0), ae::Vec2::new(1.0, 0.0)] {
        let frame = ae::AccelerationFrame::new(down);
        c.kinematics = ae::BodyKinematics {
            vel: frame.down * 120.0,
            ..Default::default()
        };
        c.frame = ae::AccelerationFrame::new(down);
        assert_eq!(
            pick(&anim, &combat, &blink_cam, None, &c),
            CharacterAnim::Idle,
            "motion straight along the fall axis under gravity {down:?} read as \
             walking: the metric is total speed, not the run component"
        );
    }
}

/// A HELD BODY DRAWS AS HELD, WHATEVER ITS VELOCITY SAYS.
#[test]
fn a_captive_outranks_every_locomotion_read() {
    let mut v = BodyAnimView {
        held: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&v), CharacterAnim::Hit);

    // The reads a captive would otherwise win with, one at a time.
    v.airborne = true;
    assert_eq!(pick_body_anim(&v), CharacterAnim::Hit, "a held body fell");
    v.airborne = false;
    v.rolling = true;
    assert_eq!(pick_body_anim(&v), CharacterAnim::Hit, "a held body rolled");
    v.rolling = false;
    v.knocked_down = true;
    assert_eq!(
        pick_body_anim(&v),
        CharacterAnim::Hit,
        "a held body was drawn prone"
    );

    // and the floor: released, the same body goes back to its own reads.
    v.held = false;
    assert_eq!(
        pick_body_anim(&v),
        CharacterAnim::LandHard,
        "clearing the hold did not give the body its own pose back"
    );
}

/// A BROKEN GUARD READS AS REELING, NOT AS STANDING THERE.
///
/// the gap this closes: the shield became a resource that can shatter and
/// leave a body dizzy and helpless for two seconds, and the picker had no read
/// for it — so the most punishable state in the game drew the same idle pose as
/// standing safely. it draws `Hit` because no sheet owns a dizzy row; the
/// point is that it stops drawing calm.
#[test]
fn a_shattered_guard_outranks_the_guard_it_no_longer_has() {
    let broken = BodyAnimView {
        guard_broken: true,
        // the poison built in: `blocking` is set too. A body cannot be shown
        // holding a guard it just lost, so the arm order is the claim.
        blocking: true,
        ..Default::default()
    };
    assert_eq!(pick_body_anim(&broken), CharacterAnim::Hit);

    let guarding = BodyAnimView {
        blocking: true,
        ..Default::default()
    };
    assert_eq!(
        pick_body_anim(&guarding),
        CharacterAnim::Block,
        "an unbroken guard stopped drawing itself"
    );
}
