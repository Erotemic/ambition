//! Applying a hit to a boss: routing damage into the encounter registry + phase.

use crate::engine_core::AabbExt;

use super::super::super::util::midpoint;
use super::super::damage_drops::{drop_ability_pickup, drop_currency_coin, drop_health_pickup};
use super::super::{ae, GameplayBanner, HitEvent, HitSource};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
use crate::audio::SfxMessage;
use crate::boss_encounter::{record_boss_damage, BossEncounterRegistry};
use crate::cutscene_trigger::CutsceneTriggerQueue;
use crate::encounter::BossEncounterMusicRequest;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use super::*;

/// Apply one landed attacker-side hit to a single boss and emit its
/// feedback. Routes damage through `record_boss_damage` (engine
/// `BossEncounterState` is the source of truth) when the encounter
/// resources are present, falling back to direct runtime mutation for
/// test fixtures without the encounter machine. Cut-rope puzzle bosses
/// give honest local impact feedback but take no HP damage from
/// ordinary player hits.
///
/// Returns `true` when the boss took the hit (so the caller drives the
/// shared landed-hit feedback). Early-returns `false` for a dead boss,
/// a miss against the live damageable volumes, or an invulnerable-phase
/// swallow — matching the previous `continue` behavior.
///
/// Extracted from `apply_feature_hit_events` per ecs-cleanup-plan.md #4.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_boss_hit(
    event: &HitEvent,
    boss: super::super::boss_clusters::BossMut<'_>,
    attack_state: &crate::brain::BossAttackState,
    animation_frame: Option<&crate::features::BossAnimationFrameSample>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    writers: &mut FeatureHitWriters<'_, '_>,
    boss_registry: Option<&mut BossEncounterRegistry>,
    music_request: Option<&mut BossEncounterMusicRequest>,
    cutscene_queue: Option<&mut CutsceneTriggerQueue>,
) -> bool {
    if !boss.status.alive {
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
            .find(|part| event.volume.strict_intersects(**part))
        {
            boss.status.hit_flash = 0.18;
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
        .find(|part| event.volume.strict_intersects(**part))
    else {
        return false;
    };
    // Speech bubble bark when player lands a hit, debounced by hit_flash.
    let should_bark = boss.status.hit_flash < 0.05;
    boss.status.hit_flash = 0.18;
    if should_bark {
        if let Some(reg) = combat_banter {
            let strikes = boss.status.health.max - boss.status.health.current;
            if let Some(line) = reg.pick_hit_bark(&boss.config.name, strikes.max(0) as u32) {
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: boss.bark_anchor(),
                    text: line.to_string(),
                });
            }
        }
    }
    let amount = event.damage.max(1);
    // Boss encounter authoritative state (OVERNIGHT-TODO #8).
    // Apply damage to engine state via the registry; the
    // outcome tells us whether the hit actually landed (false
    // during invulnerable phases) and whether it killed the
    // boss. Mirror the new HP back to the runtime so
    // downstream readers (HUD health bar, bark count, etc.)
    // see it on the same tick instead of one frame late.
    //
    // If any of the boss-encounter resources is missing (test
    // fixtures that don't install the encounter machine) we
    // fall back to the pre-inversion direct mutation so the
    // runtime still takes damage and the test exercises the
    // hit path.
    let outcome = match (boss_registry, music_request, cutscene_queue) {
        (Some(registry), Some(music), Some(cutscene)) => record_boss_damage(
            registry,
            music,
            cutscene,
            banner,
            boss.config.id.as_str(),
            amount,
        ),
        _ => None,
    };
    let (applied, killed) = match outcome {
        Some(outcome) => {
            boss.status.health.current = outcome.hp_remaining;
            (outcome.applied, outcome.killed)
        }
        // No engine encounter / missing test resource. Fall
        // back to the pre-inversion direct mutation so the
        // runtime still takes damage.
        None => {
            let died = boss.status.health.damage(amount);
            (true, died)
        }
    };
    if !applied {
        // Invulnerable phase swallowed the damage. Skip the
        // hit VFX / GameplayEffect signal so the player sees
        // the boss as a hard wall during the beat instead of
        // a fake impact.
        return false;
    }
    let impact = midpoint(event.volume.center(), hit_aabb.center());
    writers.vfx.write(VfxMessage::Impact { pos: impact });
    // Boss HP is applied directly via `record_boss_damage`
    // above, so the engine BossEncounterState is the source of
    // truth; the old no-op GameplayEffect::DamageBoss bus hook
    // has been removed.
    if killed {
        boss.status.alive = false;
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
            if let Some(spec) = crate::brain::held_item_by_id(gauntlet_id) {
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
