//! Applying a hit to a boss: mutating the boss ENTITY's HP + phase directly.
//!
//! Boss HP/phase authority is entity-local (`BossEncounter.health` +
//! `BossEncounter.encounter: ActorPhaseState`). Player damage mutates the entity in
//! place via [`apply_entity_boss_damage`]; the death CONSEQUENCES that aren't
//! immediate VFX (save Cleared + quest + music restore) are resolved by
//! `update_boss_encounters` once the death outro elapses.

use super::super::ae;
use super::super::damage_drops::{drop_ability_pickup, drop_currency_coin, drop_health_pickup};
use ambition_combat::events::{GameplayBanner, HitEvent, HitSource};
use ambition_combat::util::midpoint;
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
use ambition_boss_encounter::BossEncounter;
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::{DebrisBurstMessage, ParticleKind, PhysicsDebrisCue, VfxMessage};

use super::*;

/// Apply damage to entity-local boss health and phase authority.
///
/// Boss phase policy rejects hits while invulnerable. Bosses currently use no
/// post-hit i-frame and no body shield. Returns `(applied, killed, wallet_spent)`;
/// a kill forces the phase to `Death`.
pub(crate) fn apply_entity_boss_damage(
    status: &mut BossEncounter,
    health: &mut ambition_characters::actor::BodyHealth,
    combat: &mut ambition_characters::actor::BodyCombat,
    wallet_shield: Option<ambition_damage::WalletArmor<'_>>,
    amount: i32,
) -> (bool, bool, Option<i32>) {
    // Phase-invuln is boss POLICY, gated before the shared mechanics.
    let invulnerable = status
        .encounter
        .as_ref()
        .map_or(false, |phase| phase.boss_invulnerable());
    if invulnerable || amount <= 0 {
        return (false, false, None);
    }
    // THE shared victim-side mechanics. The guard is `None` — a boss carries no
    // `BodyShieldState`, so there is nothing to block with and nothing to spend.
    let resolution = ambition_damage::resolve_body_hit(
        combat,
        Some(health),
        // Bosses wear no equipment; armor is inert here.
        None,
        wallet_shield,
        None,
        0.0,
        ae::Vec2::ZERO,
        ae::Vec2::ZERO,
        ae::Vec2::new(0.0, 1.0),
        amount,
        1.0,
        false,
        ambition_damage::BodyHitFeel {
            hit_flash: 0.18,
            damage_invuln_time: 0.0,
            block_hit_flash: 0.0,
            block_invuln_floor: 0.0,
            armor_hitstop_time: 0.070,
        },
        // A boss carries no `BodyMotionFacts` and has no dodge to be inside, so
        // there is no evade for the eligibility gate to honour.
        false,
        // NOT unstoppable, because this path cannot tell. It takes an `amount`
        // and never sees the `HitEvent`, so it has no source to match on — and
        // the blast zone's own hit is stamped `HitTarget::Body`, which routes
        // through the actor consumer rather than here. If a boss ever needs to
        // be blasted out of a stage, the source has to reach this function
        // first; guessing `true` here would make every boss hit unblockable.
        false,
    );
    match resolution {
        // Already dead (raced past the caller's liveness check) — no hit.
        ambition_damage::BodyHitResolution::Ignored => (false, false, None),
        // No shield component  the resolver never returns Blocked for a boss.
        ambition_damage::BodyHitResolution::Blocked => (false, false, None),
        // No `WornEquipment`  the resolver never returns Armored for a boss.
        ambition_damage::BodyHitResolution::Armored => (true, false, None),
        ambition_damage::BodyHitResolution::WalletShielded { spent } => (true, false, Some(spent)),
        ambition_damage::BodyHitResolution::Damaged { died, .. } => {
            if died {
                if let Some(phase) = status.encounter.as_mut() {
                    let _ = phase.kill();
                }
            }
            (true, died, None)
        }
    }
}

/// Mutates the boss ENTITY's HP + phase directly via [`apply_entity_boss_damage`] (the entity is
/// the source of truth). Cut-rope puzzle bosses give honest local impact feedback but take no HP
/// damage from ordinary player hits.
///
/// Early-returns `false` for a dead boss, a miss against the live damageable volumes, or an
/// invulnerable-phase swallow.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_boss_hit(
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    event: &HitEvent,
    boss_entity: bevy::prelude::Entity,
    boss: ambition_boss_encounter::BossMut<'_>,
    // The boss's shared body components (§A1): `BodyHealth` is the HP
    // authority, `BodyCombat.hit_flash` the one damage-blink.
    health: &mut ambition_characters::actor::BodyHealth,
    combat: &mut ambition_characters::actor::BodyCombat,
    wallet_shield: Option<ambition_damage::WalletArmor<'_>>,
    attack_state: &ambition_characters::brain::BossAttackState,
    animation_frame: Option<&ambition_boss_encounter::attack_geometry::BossAnimationFrameSample>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&ambition_conversation::banter::CombatBanterRegistry>,
    // CM8: how this boss reacts to being hurt (its `CombatTuning.hurt_feedback`,
    // ENEMY by default). The attack contributes only its strike sound.
    hurt: ambition_vfx::HurtFeedback,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    let session_scope = writers.session_spawn_scope();
    if !health.alive() {
        return false;
    }
    if boss.config.behavior.environmental_kill_only
        && matches!(event.source, HitSource::Melee | HitSource::Projectile)
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
        let damageable = ambition_boss_encounter::attack_geometry::damageable_volumes(
            &ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
                boss_catalog,
                boss.as_ref(),
                attack_state,
            )
            .with_animation_frame(animation_frame),
        );
        if let Some(hit_aabb) = damageable.iter().find(|part| event.volume.intersects(part)) {
            combat.hit_flash = 0.18;
            let impact = midpoint(event.volume.center(), hit_aabb.center());
            // CM8: an honest strike clang + spark even though this puzzle boss
            // takes no HP from the hit.
            // A13: the authored strike sound is the ATTACKER's cue; the hurt fallback is
            // the VICTIM's, so both are resolved before the emitter borrows the writers.
            let attacker_source = writers.source_of(event.attacker);
            let victim_source = writers.source_of(Some(boss_entity));
            ambition_combat::util::emit_hit_feedback(
                &mut writers.sfx,
                &mut writers.vfx,
                &mut writers.debris,
                hurt,
                event.strike_sfx,
                event.damage,
                impact,
                attacker_source.as_ref(),
                victim_source.as_ref(),
            );
            return true;
        }
        return false;
    }
    // Damageable volumes read from BossAttackState (the
    // brain's source of truth for which strike profile is
    // live) so GNU-ton's head-descent vulnerability window
    // and the standard whole-body hurtbox agree on a single
    // attack-state source.
    let damageable = ambition_boss_encounter::attack_geometry::damageable_volumes(
        &ambition_boss_encounter::attack_geometry::BossVolumeContext::from_ref(
            boss_catalog,
            boss.as_ref(),
            attack_state,
        )
        .with_animation_frame(animation_frame),
    );
    let Some(hit_aabb) = damageable.iter().find(|part| event.volume.intersects(part)) else {
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
    let (applied, killed, wallet_spent) =
        apply_entity_boss_damage(boss.status, health, combat, wallet_shield, amount);
    if !applied {
        // Invulnerable phase swallowed the damage. Skip the
        // hit VFX / GameplayEffect signal so the player sees
        // the boss as a hard wall during the beat instead of
        // a fake impact.
        return false;
    }
    if let Some(spent) = wallet_spent {
        writers
            .wallet_shield_spent
            .write(ambition_damage::WalletShieldSpent {
                victim: boss_entity,
                amount: spent,
                pos: boss.kin.pos,
            });
    }
    let impact = midpoint(event.volume.center(), hit_aabb.center());
    // CM8: THE one victim-side reaction (strike sound over the boss's own hurt
    // spray); the killed branch layers its death drama on top.
    // A13: attacker's cue vs victim's fallback, resolved before the borrows.
    let attacker_source = writers.source_of(event.attacker);
    let victim_source = writers.source_of(Some(boss_entity));
    ambition_combat::util::emit_hit_feedback(
        &mut writers.sfx,
        &mut writers.vfx,
        &mut writers.debris,
        hurt,
        event.strike_sfx,
        event.damage,
        impact,
        attacker_source.as_ref(),
        victim_source.as_ref(),
    );
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
        // The boss dies in its own voice (G1), like every other body.
        writers.sfx.write_for_body(
            victim_source.as_ref(),
            SfxMessage::Death { pos: boss.kin.pos },
        );
        // Whose death this loot fell out of — the provenance every drop below
        // states, and without which no render family claims the pickup.
        let parent = super::drop_parent(writers, boss_entity, "boss", &boss.config.behavior.id);
        // A jackpot of coins + a heal for the hardest fight, on top of the ability.
        if let Some(parent) = &parent {
            drop_currency_coin(
                &mut writers.commands,
                session_scope,
                parent,
                &boss.config.behavior.id,
                boss.kin.pos,
                BOSS_BOUNTY,
            );
            drop_health_pickup(
                &mut writers.commands,
                session_scope,
                parent,
                &boss.config.behavior.id,
                boss.kin.pos + ae::Vec2::new(24.0, 0.0),
                3,
            );
        }
        // North star: "every boss a failed objective function, every upgrade a
        // theorem" — a defeated boss drops the ability it embodies, so combat
        // (not just the merchant) teaches the player new verbs.
        if let (Some(ability_id), Some(parent)) =
            (boss.config.behavior.reward_ability.as_deref(), &parent)
        {
            if let Some(item) = ambition_items::Item::from_dialog_id(ability_id) {
                drop_ability_pickup(
                    &mut writers.commands,
                    session_scope,
                    parent,
                    &boss.config.behavior.id,
                    boss.kin.pos,
                    ability_id,
                    item.display_name(),
                );
            }
        }
        // …and its signature wielded attack drops as a ground-item gauntlet the
        // player picks up + uses (the player literally wields the boss's move).
        if let (Some(gauntlet_id), Some(parent)) =
            (boss.config.behavior.signature_gauntlet.as_deref(), &parent)
        {
            if let Some(spec) = ambition_characters::brain::held_item_by_id(gauntlet_id) {
                super::super::damage_drops::drop_held_weapon(
                    &mut writers.commands,
                    session_scope,
                    parent,
                    // Offset from the ability pickup so the two drops don't stack.
                    boss.kin.pos + ae::Vec2::new(36.0, 0.0),
                    spec,
                    ae::Vec2::splat(18.0),
                    "Boss signature gauntlet",
                );
            }
        }
    }
    true
}

// `begin_ecs_breakable_respawn` / `emit_breakable_destroyed` moved to
// the combat kit (`ambition_combat::breakables`) — they are
// generic breakable side-effect helpers shared by the typed-damage
// path here and the kit's stand-to-break path.
pub(crate) use ambition_combat::breakables::{
    begin_ecs_breakable_respawn, emit_breakable_destroyed,
};

#[cfg(test)]
mod entity_damage_tests {
    //! The entity-local boss damage contract for `apply_entity_boss_damage`:
    //! vulnerable phases take damage, lethal damage forces `Death`, invulnerable
    //! phases swallow the hit.
    use super::*;
    use ambition_boss_encounter::test_support::test_boss_status;
    use ambition_boss_encounter::BossEncounterPhase;

    fn boss(
        hp: i32,
        phase: BossEncounterPhase,
    ) -> (BossEncounter, ambition_characters::actor::BodyHealth) {
        test_boss_status(hp, phase)
    }

    #[test]
    fn damage_decreases_hp_in_a_vulnerable_phase() {
        let (mut s, mut health) = boss(10, BossEncounterPhase::Phase1);
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let (applied, killed, _) =
            apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 3);
        assert!(applied);
        assert!(!killed);
        assert_eq!(health.current(), 7);
    }

    #[test]
    fn lethal_damage_kills_and_sets_death_phase() {
        let (mut s, mut health) = boss(4, BossEncounterPhase::Phase1);
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let (applied, killed, _) =
            apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 10);
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
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let (applied, killed, _) =
            apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 5);
        assert!(!applied);
        assert!(!killed);
        assert_eq!(health.current(), 10);
    }

    #[test]
    fn already_dead_boss_does_not_refire_killed() {
        let (mut s, mut health) = boss(4, BossEncounterPhase::Phase1);
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let _ = apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 10);
        // The first hit kills and forces the Death phase.
        let (applied, killed, _) =
            apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 5);
        // Death is invulnerable → the follow-up hit is swallowed, killed stays false.
        assert!(!applied);
        assert!(!killed);
        assert_eq!(health.current(), 0);
    }

    #[test]
    fn boss_has_no_post_hit_i_frame_so_back_to_back_hits_both_land() {
        // §A1 slice 2 FEEL invariant: unlike an actor (0.2s i-frame) or the
        // player (0.75s), a boss's `BodyHitFeel.damage_invuln_time` is 0.0, so
        // `vulnerable()` never gates — two hits in the same window both deal
        // damage (player DPS against bosses is unchanged by the resolver).
        let (mut s, mut health) = boss(10, BossEncounterPhase::Phase1);
        let mut combat = ambition_characters::actor::BodyCombat::default();
        let (a1, _, _) = apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 3);
        let (a2, _, _) = apply_entity_boss_damage(&mut s, &mut health, &mut combat, None, 3);
        assert!(a1 && a2, "both hits apply — no i-frame swallows the second");
        assert_eq!(health.current(), 4, "both hits dealt full damage");
        assert!(
            combat.vulnerable(),
            "the boss never enters an i-frame window (damage_invuln_time 0.0)"
        );
    }
}
