//! Attack-phase support: brain-output → engine-input translation, the shared
//! post-hit stagger gates, the moveset down-air's world-orb pogo, and the
//! debug-overlay hitbox source.
//!
//! The melee LIFECYCLE lives entirely on the moveset runtime now
//! (`combat::moveset`): a body's swing is a `"attack"`-verb move started by
//! `trigger_moveset_moves`, advanced by `advance_move_playback`, and projected
//! back into `BodyMelee` for the anim/HUD/telegraph read-model by
//! `project_moveset_melee_to_body_melee`. The former flat player+actor melee
//! driver (`start_body_melee`/`advance_body_melee`/`start_attack`/`advance_attack`
//! and the single-hitbox `spawn_melee_strike`) is gone — there is ONE melee path.
//!
//! Pure sim + message emission — it writes `SfxMessage`/`VfxMessage` *facts* and
//! holds no render dependency.
//!
//! ⭐ MOVED OUT OF THE ACTOR MONOLITH 2026-08-28 (D33). 604 lines reaching it
//! through no `crate::` path, no `super::` path and no glob — the melee lifecycle
//! had already left for `combat::moveset`, and what stayed behind was support for
//! a path that lives here. The one dependency it brought is
//! `ambition_platformer2d_world`, a clean downward edge.

use bevy::prelude::{Entity, Query, Res};

use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_platformer2d_core::{self as ae, AabbExt};

use crate::BodyMelee;
use crate::{AttackIntent, AttackView};

use crate::feel::Platformer2dFeelTuningMonolith;
use ambition_platformer2d_shared_tangle::frame_env as physics;
use ambition_sfx::SfxMessage;

/// Build the engine's `InputState` purely from `ActorControl` —
/// the player's brain output is the single source of truth for
/// every input verb the simulation consumes. The polarity flip is
/// now complete: raw `ControlFrame` is no longer consulted inside
/// the player simulation phases.
///
/// The drop-through gesture is no longer precomputed here — the engine forms it
/// gravity-relatively (`movement::wants_drop_through`) from `axis_y + jump`, so
/// it flips correctly under inverted gravity.
///
/// Two post-hit gates apply to the FINAL `InputState`:
/// - `hard_lock_timer`: every verb, including the movement/flight steering
///   axis, is zeroed so the body cannot act at all. two different facts
///   produce it and the caller takes the longer: the brief recoil throw at
///   the front of a knockback (`recoil_lock_timer`), and the authored landing
///   lag an aerial owes for touching down mid-move (`landing_lag_timer`). They
///   are kept as separate fields on purpose — a trace that cannot tell "thrown"
///   from "landed badly" cannot explain why a fighter stood still — and joined
///   only here, where the question is the narrower one of whether any verb
///   survives this frame.
/// - `hitstun_timer` (the longer, softer window once recoil clears): movement
///   authority is reduced and jump/dash/blink are suppressed, but the ATTACK
///   verb is preserved so the player can swing back the instant recoil ends —
///   even while still inside a boss and flashing (Hollow-Knight feel).
pub fn engine_input_from_actor_control(
    actor: ActorControlFrame,
    feel: Platformer2dFeelTuningMonolith,
    combat: &ambition_characters::actor::BodyCombat,
    shield: &ae::BodyShieldState,
    control_dt: f32,
    // Whether this body is tumbling, for the tech exemption. See
    // [`apply_post_hit_input_gates`].
    tumbling: bool,
    // Whether this body is HELPLESS — see [`body_is_helpless`], which derives it
    // from facts the caller already holds.
    helpless: bool,
) -> ae::InputState {
    // what is genuinely this function's own is the two lines below it. The
    // frame carries no clock — `to_input_state` leaves `control_dt` at zero on
    // purpose, because a brain runs at sim time — so a human's responsive-aim
    // window is stamped here; and the post-hit gates are the body's, not the
    // frame's.
    let mut input = actor.to_input_state();
    input.control_dt = control_dt;
    apply_post_hit_input_gates(&mut input, feel, combat, shield, tumbling, helpless);
    input
}

/// The two post-hit gates applied to ANY body's FINAL [`ae::InputState`]
/// (fable review §A2 step 7): the ONE stagger rule, so a knocked actor loses
/// authority exactly the way the knocked player does — the player's input
/// bridge and the actor's `integrate_body` both call this.
pub fn apply_post_hit_input_gates(
    input: &mut ae::InputState,
    feel: Platformer2dFeelTuningMonolith,
    // the BODY, so the two gates read the same authority and neither caller
    // can spell one of them differently. See `engine_input_from_actor_control`.
    combat: &ambition_characters::actor::BodyCombat,
    // A broken guard is the third fact that removes control outright, and it is
    // kept beside the other two rather than folded into either: a shield break
    // and a knockback recoil read identically to a gate and differently to a
    // trace, and the punish they open is the whole reason to break a shield.
    shield: &ae::BodyShieldState,
    // IS THIS BODY TUMBLING? — the published projection
    // (`ae::BodyMotionFacts::tumbling`), never the model's private maneuver
    // state. See the tech exemption below for why this gate needs it.
    tumbling: bool,
    // IS THIS BODY HELPLESS — a spent recovery, still in the air, and the move
    // that spent it over? See [`body_is_helpless`], which is where the fact is
    // DERIVED; this takes the answer so both roads read one rule.
    helpless: bool,
) {
    // THE TECH PRESS SURVIVES THE STAGGER, and it is the one press that must.
    //
    // Every gate below strips the BURST edge, and a tumble is nothing but
    // stagger — so a body falling out of a launch had its tech press deleted
    // before `tick_knockdown` could read it, and teching was unreachable for
    // EVERY body in the game, a human one included. The kernel's own tech tests
    // pass because they hand it a synthetic `InputState` and never cross this
    // gate. Measured in a real match: nine presses during one fall, nothing
    // armed.
    //
    // Exempt only while TUMBLING, not through hitstun generally: hitstun with
    // Burst open is a body that air-dodges out of being hit. And while
    // tumbling the press has exactly one meaning — `tick_knockdown` consumes it
    // for the tech and returns the input `without_evade`, so it cannot also
    // spend an aerial evade.
    //
    // The same shape as the fly-toggle exemption below: an INTENT the stagger
    // has no business eating, while the movement authority around it stays
    // stripped.
    let tech_press = tumbling.then(|| input.movement.get(ae::MovementAction::Burst));
    let hitstun_timer = combat.hitstun_timer;
    // Three facts that remove control outright: the knockback/landing locks the
    // body owns, the dizzy a broken guard owes, and the shieldstun a blocked hit
    // charges. `shield_held` is not among the verbs stripped below, so a stunned
    // blocker keeps its guard up — which is the difference between shieldstun
    // and a break.
    let hard_lock_timer = combat
        .hard_lock_timer()
        .max(shield.break_timer)
        .max(shield.stun_timer)
        // SHIELD DROP LAG — the fourth fact that removes control outright, and
        // beside the other three for the same reason: it is what LETTING GO of
        // a guard costs, which is a different cause from the shieldstun a
        // blocked hit charges and reads differently in a trace. `0.0` for a
        // game that declares no drop rule.
        .max(shield.drop_lag_timer);
    // The FLY TOGGLE is exempt from both gates: it is a mode-switch INTENT, not
    // movement authority (the axes are still stripped, so a toggled flyer can't
    // steer until the stagger clears). Eating an edge-triggered toggle corrupts
    // an open-loop brain's mode state (it believes it toggled), and toggling
    // flight to arrest a launch is a legitimate recovery tech for every body.
    if hard_lock_timer > 0.0 {
        // Recoil throw: NO authority. Zero everything (including the movement /
        // flight steering axis) so the knockback carries the body out and it
        // can't steer back in or act until it clears.
        input.axes = ae::LocalAxes::ZERO;
        // Strip all locomotion authority (fly-toggle is exempt — see above).
        input.movement.set(ae::MovementAction::Jump, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Burst, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::FastFall, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Blink, ae::Edge::NONE);
        input.attack_pressed = false;
        input.pogo_pressed = false;
        input.interact_pressed = false;
    } else if hitstun_timer > 0.0 {
        // Post-recoil hitstun: reduced movement authority and no
        // jump/burst/blink, but the attack verb (and its pogo sibling) is
        // PRESERVED — you can fight back, and damage a boss you're standing in,
        // the instant the recoil lock ends while i-frames are still ticking.
        let scale = feel.hitstun_control_scale.clamp(0.0, 1.0);
        input.axes = ae::LocalAxes::new(input.axes.x * scale, input.axes.y * scale);
        // No jump/burst/blink, but PRESERVE an in-progress jump's held/released
        // (only the press EDGE is eaten, matching the pre-re-key behavior).
        input.movement.set_pressed(ae::MovementAction::Jump, false);
        input
            .movement
            .set(ae::MovementAction::Burst, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::FastFall, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Blink, ae::Edge::NONE);
        input.interact_pressed = false;
    }
    // ⭐⭐ HELPLESS, and it is the ONLY gate here that is not a clock. A fighter
    // that has spent its recovery and is still airborne has nothing left: no
    // attack, no special, no jump, no air dodge. It keeps its DRIFT, which is
    // the whole of what it can still do about where it lands — and that is the
    // genre's edgeguard game, the reason spending a recovery early is a
    // decision rather than a button.
    //
    // ⛔ AFTER the two gates above, deliberately: a helpless body that is ALSO
    // in hitstun is both, and the strictest answer is the honest one.
    //
    // ⛔⛔ BUT IT NO LONGER RETURNS BEFORE THE TECH IS RESTORED. That ordering
    // was justified as *"a helpless body has no floor game to tech into — it has
    // not been hit"*, and the premise is false exactly where it matters:
    // `tech_press` is `Some` ONLY while TUMBLING, and a tumbling body has been
    // hit by definition. So a fighter that spent its recovery, went helpless,
    // and was then launched into a wall lost the tech — punished twice for one
    // decision, and specifically because it had already spent recovery.
    if helpless {
        input.movement.set(ae::MovementAction::Jump, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Burst, ae::Edge::NONE);
        input
            .movement
            .set(ae::MovementAction::Blink, ae::Edge::NONE);
        input.attack_pressed = false;
        input.pogo_pressed = false;
        // ⛔ FAST FALL SURVIVES, and it belongs to the same idea the drift does:
        // choosing to come down faster is a decision about where you land, not
        // an action. Every game in the genre keeps it.
        //
        // ⭐ AND SO DOES THE TECH, for the reason above: this value exists only
        // while tumbling, so restoring it here cannot hand a Burst to a body
        // that was not hit.
        if let Some(edge) = tech_press {
            input.movement.set(ae::MovementAction::Burst, edge);
        }
        return;
    }
    if let Some(edge) = tech_press {
        input.movement.set(ae::MovementAction::Burst, edge);
    }
}

/// Tick the remaining `BodyMelee` cooldown floors on simulation time.
/// `ranged_cooldown` gates refire; `cooldown` still feeds the AI telegraph.
/// TODO(compat-remove): move the remaining melee recovery floor off `BodyMelee`
/// once all telegraph readers use moveset playback state.
pub fn tick_body_melee_cooldowns(
    world_time: Res<ambition_time::WorldTime>,
    mut bodies: Query<&mut BodyMelee>,
) {
    let dt = world_time.sim_dt();
    for mut melee in &mut bodies {
        melee.cooldown = (melee.cooldown - dt).max(0.0);
        melee.ranged_cooldown = (melee.ranged_cooldown - dt).max(0.0);
    }
}

fn pogo_target_for_attack_hitbox(world: &ae::World, attack: ae::Aabb) -> Option<ae::Aabb> {
    world
        .blocks
        .iter()
        .find(|block| block.kind.is_pogo_target() && attack.strict_intersects(block.aabb))
        .map(|block| block.aabb)
}

/// World-surface pogo for moveset down-airs.
///
/// This path is deliberately ONLY for genuine collision-world `PogoOrb` /
/// `Rebound` surfaces, which have no victim entity. Body contacts travel through
/// `LandedBodyHit` -> `dispatch_landed_hit_effects` -> `apply_pogo_bounce` and
/// never enter the block world. Keeping those domains separate preserves body
/// identity and makes self-pogo through an anonymous body projection impossible.
pub fn pogo_moveset_off_world_orbs(
    // The composed collision read-API rather than its three ingredients.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    mut hitboxes: Query<(
        Entity,
        &crate::strike::Hitbox,
        &mut crate::on_hit::HitboxOnHit,
    )>,
    boxes: Query<&ae::CenteredAabb>,
    mut owners: Query<(
        &physics::ResolvedMotionFrame,
        &mut ae::BodyKinematics,
        &mut ambition_platformer2d_core::BodyGroundState,
    )>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    // A world surface has no victim Entity, so it cannot use `HitboxHits`'
    // entity-keyed dedup. `HitboxOnHit::world_fired` is the explicit one-bit
    // state for that one exceptional domain: at most one world rebound per strike.
    // (hitbox, owner, world box, rise, the cue the effect authored)
    let pogo: Vec<(Entity, Entity, ae::Aabb, f32, Option<ambition_sfx::SfxId>)> = hitboxes
        .iter()
        .filter(|(_, _, on_hit)| on_hit.effect.key == crate::on_hit::POGO_BOUNCE_KEY)
        .filter(|(_, _, on_hit)| !on_hit.world_fired())
        .filter_map(|(hb_entity, hitbox, on_hit)| {
            let owner_box = boxes.get(hitbox.owner).ok()?;
            let world_box = hitbox.world_volume(owner_box.center).bounds();
            Some((
                hb_entity,
                hitbox.owner,
                world_box,
                crate::on_hit::pogo_rise_from(&on_hit.effect),
                crate::on_hit::pogo_sfx_from(&on_hit.effect),
            ))
        })
        .collect();
    if pogo.is_empty() {
        return;
    }
    let Some(assembled) = collision.solids() else {
        return;
    };
    for (hb_entity, owner, world_box, rise, cue) in pogo {
        if pogo_target_for_attack_hitbox(&assembled, world_box).is_none() {
            continue;
        }
        let Ok((resolved_frame, mut kin, mut ground)) = owners.get_mut(owner) else {
            continue;
        };
        // The owner's per-tick resolved frame: the pogo launches opposite ITS
        // down, the same value its movement integrated under.
        let gdir = resolved_frame.down();
        let pos = kin.pos;
        ae::movement::set_jump_velocity(&mut kin.vel, gdir, rise);
        ground.on_ground = false;
        // The bounce is the OWNER's technique, so it is the owner's cue (G1) —
        // both the authored override and the generic pogo.
        sfx.write_for(
            owner,
            match cue {
                Some(id) => SfxMessage::Play { id, pos },
                None => SfxMessage::Pogo { pos },
            },
        );
        // One world rebound per strike. Body contacts dedup independently through
        // `HitboxHits`; no Entity sentinel is mixed into this state.
        if let Ok((_, _, mut on_hit)) = hitboxes.get_mut(hb_entity) {
            on_hit.mark_world_fired();
        }
    }
}

/// Source the body's melee hitbox from the sprite manifest — the box authored
/// and shown by `debug-hitboxes` — so the debug overlay draws the visible blade
/// through the same data-driven path bosses use via the App-local
/// `AuthoredAttackVolumeResolver`. Returns `None` when the current swing's
/// animation has no authored hitbox, so the overlay falls back to the hardcoded
/// `AttackSpec` volume. Consumed by `dev/debug_overlay/gizmos.rs`.
pub fn player_attack_hitbox(
    character_catalog: &CharacterCatalog,
    authored_volumes: &crate::authored_volumes::AuthoredAttackVolumeResolver,
    sprite_character_id: Option<&str>,
    view: &AttackView,
    intent: AttackIntent,
    gravity_dir: ae::Vec2,
) -> Option<ae::CombatVolume> {
    let animation = attack_intent_animation(intent);
    // The resolver answers BODY-LOCAL; the overlay wants the box where this
    // body is standing right now, so it places it — the same placement a
    // spawned strike volume applies per query, so the drawn box and the damage
    // box cannot disagree about which way the swing points.
    Some(
        authored_volumes
            .resolve(
                character_catalog,
                sprite_character_id,
                animation,
                view.size,
                None,
            )?
            .place_body_local(view.pos, view.facing, gravity_dir),
    )
}

/// Map the attack intent to its sprite animation row (mirrors the renderer's
/// `directional_attack_anim`). Only rows with an authored manifest hitbox
/// resolve to a box; the rest fall back to the spec volume. Today only
/// `attack_side` is authored — the others are placeholders for when their
/// per-row hitboxes land.
fn attack_intent_animation(intent: AttackIntent) -> &'static str {
    match intent {
        AttackIntent::Up => "attack_up",
        AttackIntent::Down => "attack_down",
        AttackIntent::AirUp => "air_up",
        AttackIntent::AirDown => "air_down",
        AttackIntent::AirForward => "air_forward",
        AttackIntent::AirBack => "air_back",
        _ => "attack_side",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_attack_box() -> ae::Aabb {
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 16.0))
    }

    /// Review-required AI-body coverage for the `InputState` re-key: an actor /
    /// AI brain drives movement by writing `ActorControlFrame` verbs, and the
    /// ⭐⭐ A SPENT RECOVERY LEAVES A FIGHTER WITH DRIFT AND NOTHING ELSE.
    ///
    /// The genre's edgeguard game rests on it: once you have thrown your
    /// recovery you cannot attack, jump, or dodge your way home, and the only
    /// thing left is where you steer while you fall. Without it a fighter offstage
    /// keeps every option it had, and going offstage after somebody costs nothing.
    ///
    /// ⛔ THE DRIFT AND THE FAST FALL SURVIVE, and asserting that is half the
    /// test: a gate that stripped the axes would be a body that cannot influence
    /// where it lands, which is a stun rather than helplessness. A test that only
    /// checked "the attack is gone" would pass against that.
    #[test]
    fn a_helpless_fighter_keeps_its_drift_and_loses_everything_else() {
        use crate::feel::Platformer2dFeelTuningMonolith;
        use ambition_characters::actor::control::ActorControlFrame;

        let mut frame = ActorControlFrame::neutral();
        frame.locomotion = ae::LocalAxes::new(1.0, 0.0);
        frame.jump_pressed = true;
        frame.burst_pressed = true;
        frame.melee_pressed = true;
        frame.fast_fall_pressed = true;

        let gated = |helpless: bool| {
            engine_input_from_actor_control(
                frame,
                Platformer2dFeelTuningMonolith::default(),
                &ambition_characters::actor::BodyCombat::default(),
                &ae::BodyShieldState::default(),
                1.0 / 60.0,
                false,
                helpless,
            )
        };

        // The control: an ordinary airborne fighter keeps all four.
        let free = gated(false);
        assert!(
            free.jump_pressed() && free.burst_pressed() && free.attack_pressed,
            "the fixture's presses do not survive an UNGATED frame, so the \
             assertions below measure the fixture rather than the gate"
        );

        let spent = gated(true);
        assert!(!spent.jump_pressed(), "a helpless fighter jumped home");
        assert!(
            !spent.burst_pressed(),
            "a helpless fighter air-dodged, which is the recovery the spent one \
             was supposed to have been"
        );
        assert!(!spent.attack_pressed, "a helpless fighter swung");
        assert_eq!(
            spent.axes.x, 1.0,
            "the drift went with everything else — a helpless fighter that \
             cannot steer is not helpless, it is frozen"
        );
        assert!(
            spent.fast_fall_pressed(),
            "fast fall was stripped, and it belongs to the same idea the drift \
             does: choosing to come down faster is about WHERE you land"
        );
    }

    /// bridge must route them onto the re-keyed `movement` edges the kernel now
    /// consumes — including the edge-granular post-hit stagger gates.
    #[test]
    fn ai_body_movement_routes_through_action_edges_and_gates() {
        use crate::feel::Platformer2dFeelTuningMonolith;
        use ambition_characters::actor::control::ActorControlFrame;
        let dt = 1.0 / 60.0;

        // Normal: jump held + burst + blink-release route to the movement edges.
        let mut frame = ActorControlFrame::neutral();
        frame.jump_pressed = true;
        frame.jump_held = true;
        frame.burst_pressed = true;
        frame.blink_released = true;
        let input = engine_input_from_actor_control(
            frame,
            Platformer2dFeelTuningMonolith::default(),
            &ambition_characters::actor::BodyCombat::default(),
            &ae::BodyShieldState::default(),
            dt,
            false,
            // Not helpless: these fixtures are about the stagger gates.
            false,
        );
        assert!(input.jump_pressed() && input.jump_held());
        assert!(input.burst_pressed());
        assert!(input.blink_released() && !input.blink_pressed());

        // Recoil lock: ALL locomotion stripped for the AI body too.
        let mut recoiled = ActorControlFrame::neutral();
        recoiled.jump_pressed = true;
        recoiled.burst_pressed = true;
        let input = engine_input_from_actor_control(
            recoiled,
            Platformer2dFeelTuningMonolith::default(),
            &ambition_characters::actor::BodyCombat {
                recoil_lock_timer: 1.0,
                ..Default::default()
            },
            &ae::BodyShieldState::default(),
            dt,
            false,
            // Not helpless: these fixtures are about the stagger gates.
            false,
        );
        assert!(!input.jump_pressed() && !input.burst_pressed());

        // Hitstun: eats only the jump PRESS; an in-progress jump keeps held.
        let mut stunned = ActorControlFrame::neutral();
        stunned.jump_pressed = true;
        stunned.jump_held = true;
        let input = engine_input_from_actor_control(
            stunned,
            Platformer2dFeelTuningMonolith::default(),
            &ambition_characters::actor::BodyCombat {
                hitstun_timer: 1.0,
                ..Default::default()
            },
            &ae::BodyShieldState::default(),
            dt,
            false,
            // Not helpless: these fixtures are about the stagger gates.
            false,
        );
        assert!(!input.jump_pressed(), "hitstun eats the jump press");
        assert!(input.jump_held(), "but an in-progress jump keeps held");
    }

    #[test]
    fn attack_phase_pogo_rejects_ground_and_one_way_targets() {
        let attack = test_attack_box();
        let min = attack.center() - attack.half_size();
        let size = attack.half_size() * 2.0;
        let world = ae::World::new(
            "pogo attack reject test",
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::ZERO,
            vec![
                ae::Block::solid("floor", min, size),
                ae::Block::one_way("one-way", min, size),
                ae::Block::blink_wall("blink-wall", min, size, ae::BlinkWallTier::Soft),
            ],
        );

        assert_eq!(pogo_target_for_attack_hitbox(&world, attack), None);
    }

    #[test]
    fn attack_phase_pogo_accepts_authored_pogo_targets() {
        let attack = test_attack_box();
        let min = attack.center() - attack.half_size();
        let size = attack.half_size() * 2.0;
        let orb = ae::Block::pogo_orb("orb", attack.center(), 12.0);
        let rebound = ae::Block::rebound(
            "rebound",
            min + ae::Vec2::new(60.0, 0.0),
            size,
            ae::Vec2::new(0.0, 180.0),
        );
        let world = ae::World::new(
            "pogo attack accept test",
            ae::Vec2::new(400.0, 300.0),
            ae::Vec2::ZERO,
            vec![ae::Block::solid("floor", min, size), orb.clone(), rebound],
        );

        assert_eq!(
            pogo_target_for_attack_hitbox(&world, attack),
            Some(orb.aabb)
        );
    }

    /// A HELPLESS BODY THAT IS TUMBLING KEEPS ITS TECH.
    ///
    /// ⛔⛔ THE ORDERING WAS JUSTIFIED BY A PREMISE THAT IS FALSE WHERE IT
    /// MATTERS. The helpless branch returned BEFORE restoring the preserved tech
    /// edge, on the reasoning that *"a helpless body has no floor game to tech
    /// into — it has not been hit"*. But the preserved edge only exists while
    /// TUMBLING, and a tumbling body has been hit by definition. So a fighter
    /// that spent its recovery, went helpless, and was then launched into a wall
    /// lost the tech — punished twice for one decision, and specifically for
    /// having already spent recovery.
    #[test]
    fn a_helpless_body_in_a_tumble_still_reaches_its_tech() {
        let dt = 1.0 / 60.0;
        let mut frame = ActorControlFrame::neutral();
        frame.burst_pressed = true;
        // Jump is stripped by helplessness and must stay stripped — without this
        // the arm below would pass on a gate that stopped refusing anything.
        frame.jump_pressed = true;

        let input = engine_input_from_actor_control(
            frame,
            Platformer2dFeelTuningMonolith::default(),
            // TUMBLING is what makes the Burst a tech rather than a dodge.
            &ambition_characters::actor::BodyCombat::default(),
            &ae::BodyShieldState::default(),
            dt,
            true,
            // HELPLESS: the recovery is spent.
            true,
        );
        assert!(
            input.burst_pressed(),
            "a helpless body in a TUMBLE lost its tech press — it has been hit, \
             which is the one situation the preserved edge exists for"
        );
        assert!(
            !input.jump_pressed(),
            "helplessness stopped refusing the jump, so this fixture no longer \
             measures a helpless body at all"
        );
    }
}
