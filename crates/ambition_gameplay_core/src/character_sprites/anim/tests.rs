//! Tests for the per-actor animation pickers: that player/enemy/NPC
//! state (ledge, shield, dodge, swim, climb, crouch, shoot, aim,
//! wall-jump, interact, aerial flight) maps to the expected
//! `CharacterAnim`, and that the action/ledge rows resolve by name and
//! loop correctly.

use super::*;
use crate::actor::BodyCombat;
use crate::player::PlayerBlinkCameraState;

/// Build a player + the three default state inputs that
/// `pick_player_anim` consumes. Tests then mutate just the
/// fields relevant to the case under test.
/// Bundle of every cluster component `pick_player_anim` reads.
/// Tests mutate just the fields relevant to the case under test.
struct PickClusters {
    kinematics: crate::actor::BodyKinematics,
    ground: crate::actor::BodyGroundState,
    wall: crate::actor::BodyWallState,
    blink: crate::actor::BodyBlinkState,
    flight: crate::actor::BodyFlightState,
    dash: crate::actor::BodyDashState,
    ledge: crate::actor::BodyLedgeState,
    body_mode: crate::actor::BodyModeState,
    env_contact: crate::actor::BodyEnvironmentContact,
    abilities: crate::actor::BodyAbilities,
    dodge: crate::actor::BodyDodgeState,
    shield: crate::actor::BodyShieldState,
}

impl PickClusters {
    fn defaults() -> Self {
        Self {
            kinematics: Default::default(),
            ground: Default::default(),
            wall: Default::default(),
            blink: Default::default(),
            flight: Default::default(),
            dash: Default::default(),
            ledge: Default::default(),
            body_mode: Default::default(),
            env_contact: Default::default(),
            abilities: Default::default(),
            dodge: Default::default(),
            shield: Default::default(),
        }
    }
}

fn pick_inputs() -> (
    PlayerAnimState,
    BodyCombat,
    PlayerBlinkCameraState,
    PickClusters,
) {
    (
        PlayerAnimState::default(),
        BodyCombat::default(),
        PlayerBlinkCameraState::default(),
        PickClusters::defaults(),
    )
}

fn pick(
    anim: &PlayerAnimState,
    combat: &BodyCombat,
    blink_cam: &PlayerBlinkCameraState,
    attack: Option<&crate::MeleeSwing>,
    c: &PickClusters,
) -> CharacterAnim {
    pick_player_anim(
        anim,
        combat,
        blink_cam,
        attack,
        &c.kinematics,
        &c.ground,
        &c.wall,
        &c.blink,
        &c.flight,
        &c.dash,
        &c.ledge,
        &c.body_mode,
        &c.env_contact,
        &c.abilities,
        &c.dodge,
        &c.shield,
    )
}

fn hang_state(getup: ae::LedgeGetupKind, climbing: bool) -> ae::LedgeGrabState {
    let contact = ae::LedgeContact {
        wall_normal_x: -1.0,
        anchor: ae::Vec2::new(86.0, 110.0),
        climb_target: ae::Vec2::new(115.0, 77.0),
    };
    let mut state = ae::LedgeGrabState::hanging(contact);
    state.elapsed = 0.1;
    state.climbing = climbing;
    state.getup_kind = getup;
    state
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
        clusters.ledge.grab = Some(hang_state(kind, false));
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
    clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Climb, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeGetup,
    );
}

/// Roll getup picks the new `LedgeRoll` row.
#[test]
fn climbing_with_roll_kind_returns_ledge_roll() {
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Roll, true));
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
    clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Attack, true));
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::LedgeGetupAttack,
    );
}

/// The non-looping list must include the two new ledge rows so
/// `CharacterAnimator` doesn't keep cycling their frames after
/// the engine transition completes. Regression guard against
/// adding new variants and forgetting the `non_looping` entry.
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
/// engine drives both with the same `dodge.roll_timer`; this pins
/// the visual gate that picks the right pose for the situation.
#[test]
fn dodge_roll_grounded_vs_ledge_getup() {
    // Grounded: no ledge state, just a dodge timer.
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.dodge.roll_timer = 0.2;
    assert_eq!(
        pick(&anim, &combat, &blink_cam, None, &clusters),
        CharacterAnim::DodgeRoll,
    );
    // Ledge roll: same timer set, plus a ledge_grab climbing roll.
    // The ledge-state branch must win.
    let (anim, combat, blink_cam, mut clusters) = pick_inputs();
    clusters.dodge.roll_timer = 0.2;
    clusters.ledge.grab = Some(hang_state(ae::LedgeGetupKind::Roll, true));
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
    use ambition_engine_core::player_state::BodyMode;
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
    use ambition_engine_core::player_state::BodyMode;
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
        ("crouch_walk", CharacterAnim::Crawl),
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
        // PCA sheet rows that previously dropped silently.
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
    }
}

/// Build a one-frame melee swing with the given intent, sitting in its startup
/// (telegraph) phase so the picker reads it as an active swing.
fn swing_with_intent(intent: crate::combat::AttackIntent) -> crate::MeleeSwing {
    crate::MeleeSwing::new(crate::combat::AttackSpec {
        intent,
        startup_seconds: 0.1,
        active_seconds: 0.1,
        recovery_seconds: 0.1,
        hitbox_offset: ae::Vec2::ZERO,
        hitbox_half_size: ae::Vec2::new(8.0, 8.0),
        self_impulse: ae::Vec2::ZERO,
        knockback: ae::Vec2::ZERO,
        damage_kind: crate::combat::DamageKind::Slash,
        can_pogo: false,
        damage_override: None,
    })
}

fn pick_actor(c: &PickClusters, swing: Option<&crate::MeleeSwing>, state: ActorAnimState) -> CharacterAnim {
    pick_actor_anim(
        &c.kinematics,
        &c.ground,
        &c.wall,
        &c.blink,
        &c.flight,
        &c.dash,
        &c.ledge,
        &c.body_mode,
        &c.env_contact,
        &c.abilities,
        &c.dodge,
        &c.shield,
        swing,
        state,
    )
}

#[test]
fn actors_animate_from_real_state_regardless_of_disposition() {
    // One actor path; disposition (hostile/peaceful) is not an animation fork —
    // every read below is the actor's REAL ECS state, not its label.

    // Flyer (parrot): Fly while moving, Idle while hovering/perched.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(40.0, -30.0);
    assert_eq!(
        pick_actor(&c, None, ActorAnimState { aerial: true, ..actor_state() }),
        CharacterAnim::Fly,
    );
    let c = PickClusters::defaults();
    assert_eq!(
        pick_actor(&c, None, ActorAnimState { aerial: true, ..actor_state() }),
        CharacterAnim::Idle,
        "a still hover / landed perch is Idle, not Fly",
    );
    // A grounded (non-aerial) actor launched upward now reads the airborne
    // Jump/Fall gate (it shares the player's full ladder) — never Fly.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(40.0, -200.0); // top-left coords: up
    assert_eq!(
        pick_actor(&c, None, actor_state()),
        CharacterAnim::Jump,
    );
    // An active melee wins over locomotion — and a PEACEFUL-disposition actor
    // that swings animates its attack too (the old NPC path dropped this read).
    // The swing's own intent picks the directional row (Forward → AttackSide,
    // which `resolve_anim` later walks down to a slash-only sheet's slash).
    let c = PickClusters::defaults();
    assert_eq!(
        pick_actor(&c, Some(&swing_with_intent(crate::combat::AttackIntent::Forward)), actor_state()),
        CharacterAnim::AttackSide,
    );
    assert_eq!(
        pick_actor(&c, Some(&swing_with_intent(crate::combat::AttackIntent::Up)), actor_state()),
        CharacterAnim::AttackUp,
        "an up-tilt swing reads the up row — actors share the player's swing map",
    );
    // Death reads from real state for ANY actor, moving or not.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(50.0, 0.0);
    assert_eq!(
        pick_actor(&c, None, ActorAnimState { alive: false, ..actor_state() }),
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
    c.dash.timer = 0.2;
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
    c.body_mode.body_mode = ambition_engine_core::player_state::BodyMode::Climbing;
    assert_eq!(pick_actor(&c, None, actor_state()), CharacterAnim::LadderClimb);

    // A hit-flashing actor reads Hit over its locomotion.
    let mut c = PickClusters::defaults();
    c.kinematics.vel = ae::Vec2::new(80.0, 0.0);
    c.ground.on_ground = true;
    assert_eq!(
        pick_actor(&c, None, ActorAnimState { hit_flash: true, ..actor_state() }),
        CharacterAnim::Hit,
    );
}
