//! Applying a hit to a boss: mutating the boss ENTITY's HP + phase directly.
//!
//! Boss HP/phase authority is entity-local (`BossStatus.health` +
//! `BossStatus.encounter: BossPhaseState`). Player damage mutates the entity in
//! place via [`apply_entity_boss_damage`]; the death CONSEQUENCES that aren't
//! immediate VFX (save Cleared + quest + music restore) are resolved by
//! `update_boss_encounters` once the death outro elapses.

use ambition_engine_core::AabbExt;

use super::super::super::util::midpoint;
use super::super::damage_drops::{drop_ability_pickup, drop_currency_coin, drop_health_pickup};
use super::super::{ae, GameplayBanner, HitEvent, HitSource};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
use crate::audio::SfxMessage;
use crate::combat::boss_clusters::BossStatus;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use super::*;

/// Apply player damage to a boss ENTITY (the entity is the source of truth:
/// HP on the shared `BodyHealth`, phase on `BossStatus.encounter` — §A1).
///
/// Returns `(applied, killed)`: `applied` is false when an invulnerable phase
/// (Intro / Transition / the `transition_lock` tell / Dormant / Death) swallows
/// the hit, so the caller can suppress hit VFX; `killed` is true on the hit that
/// drives HP to zero. On a kill the phase is forced to `Death` (the death outro
/// + save/quest resolution run in `update_boss_encounters`).
pub(crate) fn apply_entity_boss_damage(
    status: &mut BossStatus,
    health: &mut crate::actor::BodyHealth,
    amount: i32,
) -> (bool, bool) {
    let invulnerable = status
        .encounter
        .as_ref()
        .map_or(false, |phase| phase.boss_invulnerable());
    if invulnerable || amount <= 0 || !health.alive() {
        return (false, false);
    }
    let killed = health.damage(amount);
    if killed {
        if let Some(phase) = status.encounter.as_mut() {
            let _ = phase.kill();
        }
    }
    (true, killed)
}

/// Apply one landed attacker-side hit to a single boss and emit its
/// feedback. Mutates the boss ENTITY's HP + phase directly via
/// [`apply_entity_boss_damage`] (the entity is the source of truth).
/// Cut-rope puzzle bosses give honest local impact feedback but take no HP
/// damage from ordinary player hits.
///
/// Returns `true` when the boss took the hit (so the caller drives the
/// shared landed-hit feedback). Early-returns `false` for a dead boss,
/// a miss against the live damageable volumes, or an invulnerable-phase
/// swallow.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_boss_hit(
    event: &HitEvent,
    boss: super::super::boss_clusters::BossMut<'_>,
    // The boss's shared body components (§A1): `BodyHealth` is the HP
    // authority, `BodyCombat.hit_flash` the one damage-blink.
    health: &mut crate::actor::BodyHealth,
    combat: &mut crate::actor::BodyCombat,
    attack_state: &ambition_characters::brain::BossAttackState,
    animation_frame: Option<&crate::features::BossAnimationFrameSample>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    if !health.alive() {
        return false;
    }
    if boss.config.behavior.environmental_kill_only
        && matches!(
            event.source,
            HitSource::PlayerSlash { .. } | HitSource::PlayerProjectile { .. }
        )
    {
        // Environmental puzzle bosses (e.g. the Smirking Behemoth) take
        // no HP from ordinary player hits; those should give honest local
        // feedback only when they overlap the body hurtbox. The authored
        // environmental rule (the rope/anvil trap in
        // `ambition_content::bosses::cut_rope`) owns the only kill
        // condition. This is data-driven via `environmental_kill_only`
        // so core never names a specific boss. Keep this before the
        // generic damage branch so harmless feedback cannot accidentally
        // route through `record_boss_damage`.
        let damageable = crate::features::damageable_volumes(
            &crate::features::BossVolumeContext::from_ref(boss.as_ref(), attack_state)
                .with_animation_frame(animation_frame),
        );
        if let Some(hit_aabb) = damageable
            .iter()
            .find(|part| event.volume.intersects_aabb(**part))
        {
            combat.hit_flash = 0.18;
            let impact = midpoint(event.volume.center(), hit_aabb.center());
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            return true;
        }
        return false;
    }
    // Damageable volumes read from BossAttackState (the
    // brain's source of truth for which strike profile is
    // live) so GNU-ton's head-descent vulnerability window
    // and the standard whole-body hurtbox agree on a single
    // attack-state source.
    let damageable = crate::features::damageable_volumes(
        &crate::features::BossVolumeContext::from_ref(boss.as_ref(), attack_state)
            .with_animation_frame(animation_frame),
    );
    let Some(hit_aabb) = damageable
        .iter()
        .find(|part| event.volume.intersects_aabb(**part))
    else {
        return false;
    };
    // Speech bubble bark when player lands a hit, debounced by hit_flash.
    let should_bark = combat.hit_flash < 0.05;
    combat.hit_flash = 0.18;
    if should_bark {
        if let Some(reg) = combat_banter {
            let strikes = health.max() - health.current();
            if let Some(line) = reg.pick_hit_bark(&boss.config.name, strikes.max(0) as u32) {
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: boss.bark_anchor(),
                    text: line.to_string(),
                });
            }
        }
    }
    let amount = event.damage.max(1);
    // The boss ENTITY is the source of truth: mutate its HP + phase in place.
    // `applied` is false during invulnerable phases (Intro / Transition / the
    // transition_lock tell) so we suppress the hit VFX; `killed` flags the lethal
    // hit. The death CONSEQUENCES that aren't immediate feedback (save Cleared +
    // quest + music restore) are resolved by `update_boss_encounters` once the
    // death outro elapses.
    let (applied, killed) = apply_entity_boss_damage(boss.status, health, amount);
    if !applied {
        // Invulnerable phase swallowed the damage. Skip the
        // hit VFX / GameplayEffect signal so the player sees
        // the boss as a hard wall during the beat instead of
        // a fake impact.
        return false;
    }
    let impact = midpoint(event.volume.center(), hit_aabb.center());
    writers.vfx.write(VfxMessage::Impact { pos: impact });
    if killed {
        banner.show(format!("defeated boss {}", boss.config.name), 2.6);
        writers.vfx.write(VfxMessage::Burst {
            pos: boss.kin.pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        writers.debris.write(DebrisBurstMessage {
            pos: boss.kin.pos,
            cue: PhysicsDebrisCue::BossRagdoll,
        });
        writers.sfx.write(SfxMessage::Death { pos: boss.kin.pos });
        // A jackpot of coins + a heal for the hardest fight, on top of the ability.
        drop_currency_coin(
            &mut writers.commands,
            &boss.config.behavior.id,
            boss.kin.pos,
            BOSS_BOUNTY,
        );
        drop_health_pickup(
            &mut writers.commands,
            &boss.config.behavior.id,
            boss.kin.pos + ae::Vec2::new(24.0, 0.0),
            3,
        );
        // North star: "every boss a failed objective function, every upgrade a
        // theorem" — a defeated boss drops the ability it embodies, so combat
        // (not just the merchant) teaches the player new verbs.
        if let Some(ability_id) = boss.config.behavior.reward_ability.as_deref() {
            if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                drop_ability_pickup(
                    &mut writers.commands,
                    &boss.config.behavior.id,
                    boss.kin.pos,
                    ability_id,
                    item.display_name(),
                );
            }
        }
        // …and its signature wielded attack drops as a ground-item gauntlet the
        // player picks up + uses (the player literally wields the boss's move).
        if let Some(gauntlet_id) = boss.config.behavior.signature_gauntlet.as_deref() {
            if let Some(spec) = ambition_characters::brain::held_item_by_id(gauntlet_id) {
                writers.commands.spawn((
                    crate::items::pickup::GroundItem {
                        spec,
                        // Offset from the ability pickup so the two drops don't stack.
                        pos: boss.kin.pos + ae::Vec2::new(36.0, 0.0),
                        vel: ae::Vec2::ZERO,
                        half_extent: ae::Vec2::splat(18.0),
                    },
                    bevy::prelude::Name::new("Boss signature gauntlet"),
                ));
            }
        }
    }
    true
}

// `begin_ecs_breakable_respawn` / `emit_breakable_destroyed` moved to
// the combat kit (`crate::combat::breakables`) — they are
// generic breakable side-effect helpers shared by the typed-damage
// path here and the kit's stand-to-break path.
pub(crate) use crate::combat::breakables::{begin_ecs_breakable_respawn, emit_breakable_destroyed};

#[cfg(test)]
mod entity_damage_tests {
    //! The entity-local boss damage contract for `apply_entity_boss_damage`:
    //! vulnerable phases take damage, lethal damage forces `Death`, invulnerable
    //! phases swallow the hit.
    use super::*;
    use crate::boss_encounter::BossEncounterPhase;
    use crate::combat::boss_clusters::test_support::test_boss_status;

    fn boss(hp: i32, phase: BossEncounterPhase) -> (BossStatus, crate::actor::BodyHealth) {
        test_boss_status(hp, phase)
    }

    #[test]
    fn damage_decreases_hp_in_a_vulnerable_phase() {
        let (mut s, mut health) = boss(10, BossEncounterPhase::Phase1);
        let (applied, killed) = apply_entity_boss_damage(&mut s, &mut health, 3);
        assert!(applied);
        assert!(!killed);
        assert_eq!(health.current(), 7);
    }

    #[test]
    fn lethal_damage_kills_and_sets_death_phase() {
        let (mut s, mut health) = boss(4, BossEncounterPhase::Phase1);
        let (applied, killed) = apply_entity_boss_damage(&mut s, &mut health, 10);
        assert!(applied);
        assert!(killed);
        assert_eq!(health.current(), 0);
        assert!(!health.alive());
        assert_eq!(
            s.encounter.as_ref().unwrap().phase,
            BossEncounterPhase::Death
        );
    }

    #[test]
    fn invulnerable_phase_swallows_damage() {
        // Transition is invulnerable in the phase vocabulary.
        let (mut s, mut health) = boss(10, BossEncounterPhase::Transition);
        let (applied, killed) = apply_entity_boss_damage(&mut s, &mut health, 5);
        assert!(!applied);
        assert!(!killed);
        assert_eq!(health.current(), 10);
    }

    #[test]
    fn already_dead_boss_does_not_refire_killed() {
        let (mut s, mut health) = boss(4, BossEncounterPhase::Phase1);
        let _ = apply_entity_boss_damage(&mut s, &mut health, 10); // kills → Death
        let (applied, killed) = apply_entity_boss_damage(&mut s, &mut health, 5);
        // Death is invulnerable → the follow-up hit is swallowed, killed stays false.
        assert!(!applied);
        assert!(!killed);
        assert_eq!(health.current(), 0);
    }
}
