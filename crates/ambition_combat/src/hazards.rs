//! Hazard tick: patrol motion and contact damage. The contact hit's feedback
//! (its strike sound + the victim's hurt spray) is published by the ONE
//! victim-side reaction (`emit_hit_feedback`), keyed off `HitEvent::strike_sfx`
//! that this system stamps — not emitted here (CM8).

use super::util::hazard_sfx_id;
use super::*;

/// The set `advance_hazards` runs in.
///
/// two `ambition_content` plugins (`bosses`, `intro`) order against this
/// function by name across a crate boundary. Same shape as
/// `crate::strike::EffectExecutionSet` and
/// `ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet`: a
/// general crate consumed by content owed its consumers a name to order against
/// and did not have one. ⭐ That one is nameable from here now — it moved down
/// out of the actor kernel on 2026-09-03, partly BECAUSE this comment and its
/// twin in `ambition_damage` had to describe it in prose.
///
/// ONE member, so `.before(HazardTickSet)` orders against hazard MOTION only.
/// Contacts are a separate system in `BodyPathSet::Contacts`, because they read
/// the tick's settled travelled path.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HazardTickSet;

/// Advance every hazard's patrol and publish its volume.
pub fn advance_hazards(
    world_time: Res<WorldTime>,
    mut hazards: Query<(&mut CenteredAabb, &mut HazardFeature), With<FeatureSimEntity>>,
) {
    // Sim clock: patrolling damage volumes must slow in bullet-time
    // so the player can route around them. ADR 0010.
    let dt = world_time.sim_dt();
    for (mut aabb, mut feature) in &mut hazards {
        let hazard = &mut feature.hazard;
        hazard.update(dt);
        aabb.center = hazard.pos;
        aabb.half_size = hazard.size * 0.5;
    }
}

/// Did this body touch `target` this tick? The live footprint overlapping it,
/// or the tick's travelled path crossing it.
///
/// CC2 (the sweep law): a fast body (dash, Sanic run) must not tunnel a thin
/// spike between frames. The path is the §3.1 SweepSample read whole —
/// `prev → curr` swept with the shape it was taken in — and only when it ends
/// where the body is: a body a raw teleport moved has no path to here, and one
/// with no sample travelled nothing. Splicing a stale delta onto the live box
/// would invent a segment the body never took.
fn body_touches(
    hurtbox: &CenteredAabb,
    live_pos: Option<ae::Vec2>,
    sweep: Option<&ae::SweepSample>,
    target: ae::Aabb,
) -> bool {
    ae::cast::aabb_path_contacts(hurtbox.center, hurtbox.half_size, ae::Vec2::ZERO, target)
        || live_pos
            .and_then(|pos| sweep?.ending_at(pos))
            .is_some_and(|path| path.touches(target))
}

/// Publish hazard damage for every body the hazard touched.
pub fn apply_hazard_contacts(
    mut hit_events: MessageWriter<HitEvent>,
    player: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            Option<&ae::SweepSample>,
            &CenteredAabb,
            &ambition_characters::actor::BodyHealth,
            &ambition_platformer2d_core::BodyMotionFacts,
            &ambition_platformer2d_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            // The victim's per-tick resolved frame (ADR 0024): the knockback
            // side is a fact of the VICTIM's own frame.
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        ),
        (
            With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Without<FeatureSimEntity>,
        ),
    >,
    // Every OTHER body with a published footprint burns too (fable review
    // §A4): hazards are relational-agnostic world danger — an NPC
    // in lava takes the hit, a boss can be lured into spikes. Deliberately NOT
    // faction-gated (unified-actors guardrail 4).
    actor_victims: Query<
        (
            Entity,
            Option<&ambition_platformer2d_core::BodyKinematics>,
            Option<&ae::SweepSample>,
            &CenteredAabb,
            &ambition_platformer2d_core::BodyMotionFacts,
            &ambition_platformer2d_core::BodyShieldState,
            &ambition_characters::actor::BodyCombat,
            &ambition_characters::actor::BodyHealth,
        ),
        (
            With<FeatureSimEntity>,
            Without<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Without<HazardFeature>,
        ),
    >,
    hazards: Query<&HazardFeature, With<FeatureSimEntity>>,
) {
    for feature in &hazards {
        let hazard = &feature.hazard;
        if !hazard.active() {
            continue;
        }
        // Iterate every player so each overlapping player takes damage
        // independently — a future co-op build wants hazards to bite
        // every player in the volume, not implicitly the primary one.
        for (
            player_entity,
            kin,
            sweep,
            hurtbox,
            victim_health,
            facts,
            shield,
            combat,
            resolved_frame,
        ) in &player
        {
            if !crate::util::body_vulnerable(
                victim_health.health.invulnerable,
                facts.evading(),
                shield,
                combat,
            ) || !body_touches(hurtbox, Some(kin.pos), sweep, hazard.aabb())
            {
                continue;
            }
            let pos = kin.pos;
            // Knockback side in the victim's LOCAL frame (§B11), from its own
            // per-tick resolved frame.
            let side = resolved_frame.basis().side;
            let knockback_dir = (pos - hazard.pos).dot(side).signum();
            hit_events.write(HitEvent {
                strike_sfx: Some(hazard_sfx_id(&hazard.name)),
                volume: hazard.aabb().into(),
                damage: hazard.volume.damage.amount.max(1),
                source: HitSource::Hazard,
                attacker: None,
                // Hazards iterate every overlapping player; tag the
                // event with the player who actually overlapped so
                // the reader lands the hit on the right one.
                target: HitTarget::Body(player_entity),
                mode: hazard.mode,
                knockback: Some(HitKnockback {
                    // A hazard is a hit: it stuns.
                    reaction: ae::hit_response::HitReaction::Strike,
                    dir: knockback_dir,
                    magnitude: HitKnockbackMagnitude::FeelScale(1.0),
                    source_pos: hazard.pos,
                    impact_pos: pos,
                    launch_dir: None,
                    follow: None,
                }),
                ignored_targets: Vec::new(),
                            attacker_move_instance: None,
            });
        }
        // Non-player bodies: same hazard, same rule, pre-resolved victim.
        // Knockback is left to the victim consumer (actor knockback rides the
        // resolver, not the event — see §A2).
        for (victim, kin, sweep, hurtbox, facts, shield, combat, health) in &actor_victims {
            if health.current() <= 0
                || !crate::util::body_vulnerable(
                    health.health.invulnerable,
                    facts.evading(),
                    shield,
                    combat,
                )
                || !body_touches(hurtbox, kin.map(|k| k.pos), sweep, hazard.aabb())
            {
                continue;
            }
            // CM8: same hazard strike sound, carried to the victim-side reaction;
            // the actor's knockback + feedback position derive from the event.
            hit_events.write(HitEvent {
                strike_sfx: Some(hazard_sfx_id(&hazard.name)),
                volume: hazard.aabb().into(),
                damage: hazard.volume.damage.amount.max(1),
                source: HitSource::Hazard,
                attacker: None,
                target: HitTarget::Body(victim),
                mode: hazard.mode,
                knockback: None,
                ignored_targets: Vec::new(),
                            attacker_move_instance: None,
            });
        }
    }
}

#[cfg(test)]
mod tests;
