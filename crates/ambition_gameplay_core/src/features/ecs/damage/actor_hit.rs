//! Applying a hit to an actor: peaceful actors accumulate strikes/barks (and
//! provoke via `ActorStimulus`); hostile actors take damage/knockback/death.
//! Branches on `ActorDisposition`, not an actor type — every actor is the same
//! unified cluster.

use bevy::prelude::Entity;

use super::super::super::{util::midpoint, NPC_HOSTILE_STRIKE_THRESHOLD};
use super::super::damage_drops::{
    drop_currency_coin, drop_health_pickup, id_drops_health, spawn_death_explosion,
    spawn_split_offspring,
};
use super::super::{ae, ActorDisposition, GameplayBanner, HitEvent, HitSource, SetFlagRequested};
// Only the exploding-mite blast test pins this drop tuning constant; the drop
// tests query `PickupFeature` directly. Both are test-only now that the drop
// spawners live in `damage_drops`.
use crate::audio::SfxMessage;
use crate::features::ActorStimulus;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use super::*;

/// Peel-off speed (px/s) applied along the surface normal when a struck
/// surface-walker loses its cling. Enough to visibly pop off a wall/ceiling
/// before gravity takes over; tuned well under the patrol speed's order so it
/// reads as a knock, not a launch.
const CLING_DETACH_POP_SPEED: f32 = 180.0;

/// Apply one landed attacker-side hit to a single actor and emit its per-actor
/// feedback. A PEACEFUL actor accumulates strikes + barks and emits a
/// retaliation `ActorStimulus` (the flip to hostile lands later via
/// `apply_actor_stimuli`); it does NOT take health damage (it has 1 HP). A
/// HOSTILE actor takes the full damage/knockback/death path.
///
/// Returns `true` when the actor took the hit, so the caller drives the shared
/// landed-hit feedback (hitstop + Hit SFX) and re-syncs the read-models. A dead
/// hostile actor returns `false` (no-op).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_actor_hit(
    event: &HitEvent,
    actor_entity: Entity,
    disposition: ActorDisposition,
    em: &mut super::super::actor_clusters::ActorMut<'_>,
    // The body's combat state — the ONE post-hit i-frame authority for every
    // body (the player gates re-hits on the same `BodyCombat.damage_invuln_timer`).
    combat: &mut crate::actor::BodyCombat,
    aggression: Option<&mut crate::features::ActorAggression>,
    interactable: Option<&ambition_interaction::Interactable>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    // Knockback feel values (§A2 step 6) — the same tuning the player's
    // knockback resolution reads.
    feel: crate::time::feel::SandboxFeelTuning,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    if disposition.is_peaceful() {
        // Body-generic post-hit i-frame — the same consume-time gate
        // `resolve_body_hit` applies to a hostile body: a body that registered
        // a hit within the last `ACTOR_DAMAGE_IFRAME_S` ignores further hits,
        // collapsing a sustained 60 fps overlap (lingering attack volume, body
        // contact, dialog-pinned body next to an enemy) to one hit per window.
        // Returns false so the caller's `actor_hit_this_event` stays unset
        // (no sfx/hitstop, no per-swing dedup record).
        if !combat.vulnerable() {
            return false;
        }
        // Peaceful actor (talkable NPC): accumulate strikes + barks and emit a
        // retaliation stimulus. No health damage — the flip to hostile is the
        // consequence, handled by `apply_actor_stimuli`.
        let pos = em.kin.pos;
        let bark_anchor = em.bark_anchor();
        combat.hit_flash = 0.18;
        combat.damage_invuln_timer = super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S;
        let impact = midpoint(event.volume.center(), pos);
        writers.vfx.write(VfxMessage::Impact { pos: impact });
        writers.actor_stimuli.write(ActorStimulus::DamagedBy {
            actor: actor_entity,
            source: event.attacker,
            damage: event.damage,
        });
        if let Some(aggression) = aggression {
            aggression.strikes = aggression.strikes.saturating_add(1);
            if let Some(interactable) = interactable {
                if aggression.strikes >= NPC_HOSTILE_STRIKE_THRESHOLD {
                    writers.set_flag.write(SetFlagRequested {
                        id: super::super::super::npcs::npc_flag_id(&em.config.id),
                        on: true,
                    });
                    writers.vfx.write(VfxMessage::SpeechBubble {
                        pos: bark_anchor,
                        text: super::super::super::npcs::npc_hostile_bark_line(
                            interactable,
                            &em.config.name,
                            &em.config.id,
                        )
                        .to_string(),
                    });
                    writers.vfx.write(VfxMessage::Burst {
                        pos,
                        count: 16,
                        speed: 230.0,
                        color: [0.84, 0.95, 1.0, 0.82],
                        kind: ParticleKind::Spark,
                    });
                    banner.show(format!("{} turns hostile", em.config.name), 2.6);
                } else {
                    writers.vfx.write(VfxMessage::SpeechBubble {
                        pos: bark_anchor,
                        text: super::super::super::npcs::npc_hit_bark_line(
                            interactable,
                            &em.config.name,
                            &em.config.id,
                            aggression.strikes,
                        )
                        .to_string(),
                    });
                }
            }
        }
        true
    } else {
        // Combat banter — decided BEFORE the resolver mutates state: the bark
        // dedups on a near-zero hit_flash (first non-overlapping hit) and its
        // line index reads pre-damage HP. A blocked hit barks too (the body
        // was struck), matching the resolver's "registered hit" notion.
        let should_bark = combat.hit_flash < 0.05;
        let strikes = (em.health.max() - em.health.current()).max(0) as u32;
        let gravity_dir = -em
            .surface
            .surface_normal
            .normalize_or(ae::Vec2::new(0.0, -1.0));
        let caps = em.caps.clone();
        // THE shared victim-side mechanics (§A2): consume-time i-frame gate,
        // the reactive shield block (the body's RESOLVED guard — a possessing
        // human and an AI brain block identically, invariants I2/I3; the same
        // frame-agnostic directional rule the player uses), damage, death
        // flag, and hit-flash/i-frame arming. Actors pass multiplier 1.0 —
        // difficulty scaling is player policy.
        let resolution = crate::combat::damage::resolve_body_hit(
            combat,
            Some(&mut *em.health),
            em.shield.active,
            em.kin.facing,
            em.kin.pos,
            event.volume.center(),
            gravity_dir,
            event.damage,
            1.0,
            caps.never_dies,
            crate::combat::damage::BodyHitFeel {
                hit_flash: 0.16,
                damage_invuln_time: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S,
                block_hit_flash: 0.16,
                block_invuln_floor: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S,
            },
        );
        if resolution == crate::combat::damage::BodyHitResolution::Ignored {
            return false;
        }
        if should_bark {
            // Catalog-first: resolve the enemy's catalog id from its display
            // name (the identity every actor carries) and read its `on_hit`
            // pool. TEMP fallback to the CombatBanterRegistry until enemy rows
            // are populated (then drop the registry + its content installers).
            let line = crate::character_roster::character_id_for_display_name(&em.config.name)
                .and_then(|cid| {
                    crate::character_roster::bark_line_for_character_id(
                        cid,
                        ambition_characters::actor::character_catalog::BarkSituation::OnHit,
                        strikes,
                    )
                })
                .or_else(|| {
                    combat_banter.and_then(|reg| reg.pick_hit_bark(&em.config.name, strikes))
                });
            if let Some(line) = line {
                writers.vfx.write(VfxMessage::SpeechBubble {
                    pos: em.bark_anchor(),
                    text: line.to_string(),
                });
            }
        }
        if resolution == crate::combat::damage::BodyHitResolution::Blocked {
            // The guard costs nothing but consumes the hit: no damage, no
            // knockback, just a clang. A blocked hit still counts as "took the
            // hit" (returns true) so the caller plays the shared hitstop.
            let impact = midpoint(event.volume.center(), em.kin.pos);
            writers.sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_ROCK_HIT,
                pos: em.kin.pos,
            });
            writers.vfx.write(VfxMessage::Impact { pos: impact });
            writers.vfx.write(VfxMessage::Burst {
                pos: impact,
                count: 8,
                speed: 160.0,
                color: [0.78, 0.90, 1.0, 0.90],
                kind: ParticleKind::Spark,
            });
            return true;
        }
        let killed = matches!(
            resolution,
            crate::combat::damage::BodyHitResolution::Damaged { died: true, .. }
        );
        // §A2 step 6 (FEEL-BLIND): a struck actor rides the SAME feel-tuned,
        // frame-agnostic knockback resolution the player does — side away from
        // the source, rise against ITS gravity — replacing the old inline
        // `local.y - 90 max -280` pop. The data comes from the event's
        // `HitKnockback` (attached by hitboxes / body-contact / hazards); a
        // slash carries its impulse as `knock_x`, folded into the same
        // resolution as a dir + standard strength. A hit with neither leaves
        // the velocity alone (as before).
        let knockback = match (&event.source, event.knockback.as_ref()) {
            (_, Some(k)) => Some(k.clone()),
            (HitSource::PlayerSlash { knock_x }, None) if *knock_x != 0.0 => {
                Some(crate::features::HitKnockback {
                    dir: knock_x.signum(),
                    strength: 1.0,
                    source_pos: event.volume.center(),
                    impact_pos: event.volume.center(),
                })
            }
            _ => None,
        };
        if let Some(k) = knockback {
            let boss_hit = matches!(
                event.source,
                HitSource::BossBody | HitSource::BossAttack
            );
            // §A2 step 7 (FEEL-BLIND): the launch also arms the shared stagger
            // (hitstun / recoil-lock / hitstop on `BodyCombat`), consumed by
            // the actor driver's post-hit input gate + hitstop dt beat — an
            // actor staggers exactly like the player.
            let pos = em.kin.pos;
            let facing = em.kin.facing;
            crate::combat::damage::apply_body_hit_reaction(
                &mut em.kin.vel,
                combat,
                pos,
                facing,
                gravity_dir,
                boss_hit,
                Some(&k),
                feel,
            );
        }
        let impact = midpoint(event.volume.center(), em.kin.pos);
        writers.vfx.write(VfxMessage::Impact { pos: impact });
        // Cling-break: a struck surface-walker (puppy-slug) is knocked off
        // its surface — it peels away along the surface normal and falls with
        // gravity until it lands and re-attaches (handled by the surface-walk
        // integration's `fall_until_landed`). Archetypes authored with
        // `cling_breaks_on_hit: false` hold on when hit. Keep the last surface
        // normal while airborne; `fall_until_landed` reorients it relative to
        // the active acceleration frame on the next support contact.
        if !killed && em.config.tuning.surface_walker && em.config.tuning.cling_breaks_on_hit {
            let peel = em.surface.surface_normal * CLING_DETACH_POP_SPEED;
            em.ground.on_ground = false;
            em.kin.vel += peel;
        }
        if killed {
            // `health.damage` already zeroed HP → `alive()` is false; no flag to flip.
            if let Some(respawn_s) = caps.respawn_in_place_seconds {
                em.status.respawn_timer = respawn_s;
                banner.show(format!("{} dropped; respawning", em.config.name), 2.6);
            } else {
                banner.show(format!("defeated {}", em.config.name), 2.6);
                // Earn-side: a defeated enemy drops a collectible coin so the
                // player can fund the merchant / ability shop from combat, and
                // ~1 in 4 enemy kinds also drops a heart (combat sustain).
                drop_currency_coin(
                    &mut writers.commands,
                    &em.config.id,
                    em.kin.pos,
                    ENEMY_BOUNTY,
                );
                // Volatile archetypes detonate on death — a sizable
                // Enemy-faction blast at the corpse, so a point-blank kill is
                // punished (the read: kill it at range / sidestep the body).
                if caps.explodes_on_death {
                    spawn_death_explosion(&mut writers.commands, actor_entity, em.kin.pos);
                    writers.vfx.write(VfxMessage::Explosion {
                        pos: em.kin.pos,
                        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
                        scale: 0.85,
                    });
                }
                // Replicating blobs divide on death into two fast offspring.
                if caps.divides_on_death {
                    spawn_split_offspring(&mut writers.commands, &em.config.id, em.kin.pos);
                }
                if id_drops_health(&em.config.id) {
                    drop_health_pickup(
                        &mut writers.commands,
                        &em.config.id,
                        em.kin.pos + ae::Vec2::new(18.0, 0.0),
                        ENEMY_HEALTH_DROP,
                    );
                }
                // Steal the enemy's weapon: a defeated enemy that was wielding
                // a held item drops it as a `GroundItem` the player can grab +
                // wield (e.g. a pirate's gun-sword), via the existing pickup path.
                if let Some(spec) = caps.drops_held_item.clone() {
                    writers.commands.spawn((
                        crate::items::pickup::GroundItem {
                            spec,
                            pos: em.kin.pos + ae::Vec2::new(-14.0, 0.0),
                            vel: ae::Vec2::ZERO,
                            half_extent: ae::Vec2::splat(16.0),
                        },
                        bevy::prelude::Name::new("Dropped weapon"),
                    ));
                }
                if !em.config.id.starts_with("encounter:") && !em.config.tuning.is_sandbag {
                    use crate::features::EnemyRespawnPolicy as P;
                    let flag_id = match caps.respawn_policy {
                        P::OnRoomReenter => None,
                        P::OnRest => Some(format!(
                            "enemy_{}{}",
                            em.config.id,
                            crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                        )),
                        P::Never => Some(format!("enemy_{}_dead", em.config.id)),
                    };
                    if let Some(id) = flag_id {
                        writers.set_flag.write(SetFlagRequested { id, on: true });
                    }
                }
            }
            writers.vfx.write(VfxMessage::Burst {
                pos: em.kin.pos,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            writers.debris.write(DebrisBurstMessage {
                pos: em.kin.pos,
                cue: PhysicsDebrisCue::EnemyRagdoll,
            });
            writers.sfx.write(SfxMessage::Death { pos: em.kin.pos });
        }
        true
    }
}
