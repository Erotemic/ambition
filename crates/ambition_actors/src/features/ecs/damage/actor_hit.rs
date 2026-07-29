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
use crate::features::ActorStimulus;
use ambition_platformer_primitives::lifecycle::SpawnSessionScopedExt;
use ambition_sfx::SfxMessage;
use ambition_vfx::vfx::{DebrisBurstMessage, PhysicsDebrisCue};
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

use super::*;

/// Peel-off speed (px/s) applied along the surface normal when a struck
/// surface-walker loses its cling. Enough to visibly pop off a wall/ceiling
/// before gravity takes over; tuned well under the patrol speed's order so it
/// reads as a knock, not a launch.
const CLING_DETACH_POP_SPEED: f32 = 180.0;

/// What a kill does to the dead body's own lifecycle.
///
/// A decision, extracted so it can be stated and tested rather than inferred
/// from the order of an `if`-chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum KillDisposition {
    /// The authored `RespawnPolicy::InPlace` arm: this body comes back where it
    /// fell, after this many seconds.
    RespawnInPlace(f32),
    /// Defeated in the world: the exploration economy pays out (bounty coin,
    /// heart, death explosion, split offspring) and the body stays down.
    Defeated,
    /// Gone. The body left the world, and there is no "in place" for it to come
    /// back to — its position is OUTSIDE the room, so an in-place respawn would
    /// put it straight back where the blast gate is waiting. It would die again
    /// on the next tick, respawn again, and the room would acquire a body whose
    /// whole behaviour is dying, each death arming a hitstop, which is a global
    /// clock beat paid by every other body in the room.
    ///
    /// An enemy that walks into a pit is simply gone, which is what the genre
    /// has always done with them.
    GoneFromTheWorld,
}

impl KillDisposition {
    pub(crate) fn is_gone(self) -> bool {
        matches!(self, Self::GoneFromTheWorld)
    }
}

/// Decide [`KillDisposition`] from what killed the body and what its archetype
/// authored. Leaving the world OUTRANKS the authored respawn policy, because
/// the policy's own precondition — that there is somewhere to come back to —
/// is exactly what leaving the world destroys.
pub(crate) fn kill_disposition(
    source: &HitSource,
    respawn: ambition_entity_catalog::placements::RespawnPolicy,
) -> KillDisposition {
    if matches!(source, HitSource::LeftTheWorld) {
        return KillDisposition::GoneFromTheWorld;
    }
    match respawn {
        ambition_entity_catalog::placements::RespawnPolicy::InPlace(seconds) => {
            KillDisposition::RespawnInPlace(seconds)
        }
        _ => KillDisposition::Defeated,
    }
}

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
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_actor_hit(
    event: &HitEvent,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    roster: &crate::features::CharacterRoster,
    actor_entity: Entity,
    disposition: ActorDisposition,
    // Does a RULESET own this body's death? A match fighter's KO belongs to the
    // match, not to the world's exploration economy.
    ruleset_owns_death: bool,
    em: &mut super::super::actor_clusters::ActorMut<'_>,
    // The body's explicit movement policy, for typed policy operations (the
    // crawler cling-break detach).
    motion_model: &mut crate::features::MotionModel,
    // The body's combat state — the ONE post-hit i-frame authority for every
    // body (the player gates re-hits on the same `BodyCombat.damage_invuln_timer`).
    combat: &mut ambition_characters::actor::BodyCombat,
    wallet_shield: Option<crate::features::ecs::damage_apply::WalletArmor<'_>>,
    aggression: Option<&mut crate::features::ActorAggression>,
    interactable: Option<&ambition_interaction::Interactable>,
    banner: &mut GameplayBanner,
    combat_banter: Option<&crate::features::banter::CombatBanterRegistry>,
    // Knockback feel values (§A2 step 6) — the same tuning the player's
    // knockback resolution reads.
    feel: crate::time::feel::SandboxFeelTuning,
    // The struck actor's held locomotion (local frame) for DI (CM2) — the SAME
    // `ActorControl` the brain writes, so a brain/RL victim DIs like a human.
    di_input_local: ae::Vec2,
    // CM8: how THIS body reacts to being hurt (its `CombatTuning.hurt_feedback`,
    // the ENEMY default today). The victim owns its spray/debris; the attack owns
    // only the strike sound.
    hurt: ambition_vfx::HurtFeedback,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    let session_scope = writers.session_spawn_scope();
    if disposition.is_peaceful() {
        // Body-generic post-hit i-frame — the same consume-time gate
        // `resolve_body_hit` applies to a hostile body: a body that registered
        // a hit within the last `ACTOR_DAMAGE_IFRAME_S` ignores further hits,
        // collapsing a sustained 60 fps overlap (lingering attack volume, body
        // contact, dialog-pinned body next to an enemy) to one hit per window.
        // An i-framed repeat returns false so the caller's
        // `actor_hit_this_event` stays unset (no feedback or hit bookkeeping).
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
        // A13: the authored strike sound is the ATTACKER's cue; the hurt fallback is
        // the VICTIM's, so both are resolved before the emitter borrows the writers.
        let attacker_source = writers.source_of(event.attacker);
        let victim_source = writers.source_of(Some(actor_entity));
        crate::combat::util::emit_hit_feedback(
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
                            catalog,
                            interactable,
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
                            catalog,
                            interactable,
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
        // G1: resolved once for the whole branch, because every reaction below is
        // one of the two bodies' own — this actor's block clang, its hurt spray, its
        // death — and each was previously attributed to whoever owned the session.
        let attacker_source = writers.source_of(event.attacker);
        let victim_source = writers.source_of(Some(actor_entity));
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
        // Resolved BEFORE the shared mechanics, because it decides whether they
        // apply at all: the blast zone is not a hit anything can defend against.
        let left_the_world = matches!(event.source, HitSource::LeftTheWorld);
        let resolution = crate::features::ecs::damage_apply::resolve_body_hit(
            combat,
            Some(&mut *em.health),
            // No actor archetype wears equipment armor today; the resolver
            // supports it generically, but nothing threads a `WornEquipment` here.
            None,
            wallet_shield,
            em.shield.active,
            em.kin.facing,
            em.kin.pos,
            event.volume.center(),
            gravity_dir,
            event.damage,
            1.0,
            caps.never_dies,
            crate::features::ecs::damage_apply::BodyHitFeel {
                hit_flash: 0.16,
                damage_invuln_time: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S,
                block_hit_flash: 0.16,
                block_invuln_floor: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S,
            },
            left_the_world,
        );
        if resolution == crate::features::ecs::damage_apply::BodyHitResolution::Ignored {
            return false;
        }
        // CM1 death policy: an `Unbounded` (smash-percent) body never dies from
        // its meter — the blast-zone/OOB gate owns its death — so a meter-kill
        // is suppressed. `HpDepleted` (the default) kills as before: parity.
        // Computed HERE, before the bark, so a LETHAL hit does not also speak a
        // hit line: a dying body presents its death (the Death SFX + burst +
        // debris below), not an "ow!" (Jon 2026-07-22: dead things don't bark).
        //
        // The blast zone is the OTHER half of that sentence, and it kills
        // whatever it catches. `Unbounded` says "the meter never kills me, the
        // world does"; filtering the world's own kill through the meter's
        // policy would leave such a body immortal, which is exactly why nothing
        // in production had ever selected `Unbounded`.
        let killed = left_the_world
            || (matches!(
                resolution,
                crate::features::ecs::damage_apply::BodyHitResolution::Damaged { died: true, .. }
            ) && em.config.tuning.death_policy.kills_at_max());
        if should_bark && !killed {
            // Catalog-first: the actor seed carries the stable authored
            // character id through spawn. Display names remain presentation and
            // are never reverse-resolved into identity.
            let line = em
                .config
                .sprite_character_id
                .as_deref()
                .and_then(|cid| {
                    catalog.bark_line(
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
        if resolution == crate::features::ecs::damage_apply::BodyHitResolution::Blocked {
            // The guard costs nothing but consumes the hit: no damage, no
            // knockback, just a clang. A blocked hit still counts as "took the
            // hit" (returns true) so the caller plays the shared hitstop.
            let impact = midpoint(event.volume.center(), em.kin.pos);
            // The guard is the VICTIM's, so the clang is the victim's cue: a
            // shielded Sanic clangs out of Sanic's bank.
            writers.sfx.write_for_body(
                victim_source.as_ref(),
                SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: em.kin.pos,
                },
            );
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
        if let crate::features::ecs::damage_apply::BodyHitResolution::WalletShielded { spent } =
            resolution
        {
            writers.wallet_shield_spent.write(
                crate::features::ecs::damage_apply::WalletShieldSpent {
                    victim: actor_entity,
                    amount: spent,
                    pos: em.kin.pos,
                },
            );
        }
        if resolution == crate::features::ecs::damage_apply::BodyHitResolution::Armored {
            // A worn armor row absorbed the hit (no actor wears one today; kept
            // exhaustive so the generic resolver stays honest). Took the hit, no
            // damage, no death, no knockback.
            return true;
        }
        // §A2 step 6 (FEEL-BLIND): a struck actor rides the SAME feel-tuned,
        // frame-agnostic knockback resolution the player does — side away from
        // the source, rise against ITS gravity — replacing the old inline
        // `local.y - 90 max -280` pop. The data comes from the event's
        // `HitKnockback` (attached by hitboxes / body-contact / hazards); a
        // slash carries its impulse as `knock_x`, folded into the same
        // resolution as a dir + standard feel scale. A hit with neither leaves
        // the velocity alone (as before).
        let knockback = match (&event.source, event.knockback.as_ref()) {
            (_, Some(k)) => Some(k.clone()),
            (HitSource::PlayerSlash { knock_x }, None) if *knock_x != 0.0 => {
                Some(crate::features::HitKnockback {
                    dir: knock_x.signum(),
                    magnitude: crate::features::HitKnockbackMagnitude::FeelScale(1.0),
                    source_pos: event.volume.center(),
                    impact_pos: event.volume.center(),
                    launch_dir: None,
                })
            }
            _ => None,
        };
        if let Some(k) = knockback {
            let boss_hit = matches!(event.source, HitSource::BossBody | HitSource::BossAttack);
            // §A2 step 7 (FEEL-BLIND): the launch also arms the shared stagger
            // (hitstun / recoil-lock / hitstop on `BodyCombat`), consumed by
            // the actor driver's post-hit input gate + hitstop dt beat — an
            // actor staggers exactly like the player.
            let pos = em.kin.pos;
            let facing = em.kin.facing;
            crate::features::ecs::damage_apply::apply_body_hit_reaction(
                &mut em.kin.vel,
                &mut em.flight,
                combat,
                pos,
                facing,
                gravity_dir,
                boss_hit,
                Some(&k),
                di_input_local,
                feel,
            );
        }
        // CM8: THE one victim-side reaction — the striking attack's `strike_sfx`
        // (a sword vs a goblin claw) over this body's own `HurtFeedback` spray.
        // An ordinary actor's profile is `ENEMY` (plain tick, no red burst), so
        // this body never borrows the player's "you got hurt" cue. A killed body
        // still gets its death drama below, layered on this landing reaction.
        let impact = midpoint(event.volume.center(), em.kin.pos);
        // A13: the authored strike sound is the ATTACKER's cue; the hurt fallback is
        // the VICTIM's. Both were resolved at the top of this branch, before the
        // emitters borrow the writers.
        crate::combat::util::emit_hit_feedback(
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
        // Cling-break: a struck crawler (puppy-slug) is knocked off its
        // surface — the TYPED detach operation on its movement policy plus a
        // peel impulse on shared velocity. It falls under the live frame until
        // its own contact rule re-attaches it. Archetypes authored with
        // `cling_breaks_on_hit: false` hold on when hit.
        if !killed && em.config.tuning.cling_breaks_on_hit {
            if let crate::features::MotionModel::AdhesiveCrawler(crawler) = motion_model {
                let peel = em.surface.surface_normal * CLING_DETACH_POP_SPEED;
                crawler.detach();
                em.ground.on_ground = false;
                em.kin.vel += peel;
            }
        }
        if killed && ruleset_owns_death {
            // A RULESET owns this body's death (`RulesetOwnsDeath`). Health is
            // already zero and stays zero, and NONE of the world's death
            // consequences run: no bounty coin, no heart, no death explosion, no
            // split offspring, no held-item drop, no in-place respawn timer.
            //
            // Those are an exploration economy. An arena has no economy, and a
            // round that funds the player's wallet and detonates the loser is
            // not a round (GPT 5.6, 2026-07-27).
        } else if killed && kill_disposition(&event.source, em.config.tuning.respawn).is_gone() {
            // GONE — see [`kill_disposition`]. No respawn timer, and no
            // exploration payout either: a bounty coin, a heart, or a death
            // explosion dropped at this corpse would land in the void, somewhere
            // no player can ever walk to.
            //
            // ⚠ this branch used to re-test the local `left_the_world` bool, so
            // `KillDisposition::is_gone` had NO CALLER and the compiler said so.
            // The type's own doc claims it is *"a decision, extracted so it can be
            // stated and tested rather than inferred from the order of an
            // `if`-chain"* — and the chain went on inferring it. Asking the
            // decision makes the extraction load-bearing instead of decorative
            // (2026-07-29).
            banner.show(format!("{} fell out of the world", em.config.name), 2.2);
        } else if killed {
            // `health.damage` already zeroed HP → `alive()` is false; no flag to
            // flip. ONE death path, matched on the ONE authored policy (ADR 0022).
            if let KillDisposition::RespawnInPlace(respawn_s) =
                kill_disposition(&event.source, em.config.tuning.respawn)
            {
                em.status.respawn_timer = respawn_s;
                banner.show(format!("{} dropped; respawning", em.config.name), 2.6);
            } else {
                banner.show(format!("defeated {}", em.config.name), 2.6);
                // Earn-side: a defeated enemy drops a collectible coin so the
                // player can fund the merchant / ability shop from combat, and
                // ~1 in 4 enemy kinds also drops a heart (combat sustain).
                drop_currency_coin(
                    &mut writers.commands,
                    session_scope,
                    &em.config.id,
                    em.kin.pos,
                    ENEMY_BOUNTY,
                );
                // Volatile archetypes detonate on death — a sizable
                // Enemy-faction blast at the corpse, so a point-blank kill is
                // punished (the read: kill it at range / sidestep the body).
                if caps.explodes_on_death {
                    spawn_death_explosion(
                        &mut writers.commands,
                        session_scope,
                        actor_entity,
                        em.kin.pos,
                    );
                    writers.vfx.write(VfxMessage::Explosion {
                        pos: em.kin.pos,
                        kind: ambition_vfx::vfx::ExplosionKind::ClassicBurst,
                        scale: 0.85,
                    });
                }
                // Replicating blobs divide on death into two fast offspring.
                if caps.divides_on_death {
                    spawn_split_offspring(
                        &mut writers.commands,
                        catalog,
                        authored_sheets,
                        roster,
                        session_scope,
                        &em.config.id,
                        em.kin.pos,
                    );
                }
                if id_drops_health(&em.config.id) {
                    drop_health_pickup(
                        &mut writers.commands,
                        session_scope,
                        &em.config.id,
                        em.kin.pos + ae::Vec2::new(18.0, 0.0),
                        ENEMY_HEALTH_DROP,
                    );
                }
                // Steal the enemy's weapon: a defeated enemy that was wielding
                // a held item drops it as a `GroundItem` the player can grab +
                // wield (e.g. a pirate's gun-sword), via the existing pickup path.
                if let Some(spec) = caps.drops_held_item.clone() {
                    writers.commands.spawn_session_scoped(
                        session_scope,
                        (
                            crate::items::pickup::GroundItem {
                                spec,
                                pos: em.kin.pos + ae::Vec2::new(-14.0, 0.0),
                                vel: ae::Vec2::ZERO,
                                half_extent: ae::Vec2::splat(16.0),
                            },
                            bevy::prelude::Name::new("Dropped weapon"),
                        ),
                    );
                }
                // Persist the death per the authored policy (ADR 0022).
                // `encounter:*` ids keep their own state machine; `InPlace`
                // is unreachable here (the timer arm above returned).
                if !em.config.id.starts_with("encounter:") {
                    use crate::features::RespawnPolicy as P;
                    let flag_id = match em.config.tuning.respawn {
                        P::OnRoomReenter | P::InPlace(_) => None,
                        P::OnRest => Some(format!(
                            "enemy_{}{}",
                            em.config.id,
                            crate::features::ENEMY_DEAD_UNTIL_REST_SUFFIX,
                        )),
                        P::DeadStaysDead => Some(format!("enemy_{}_dead", em.config.id)),
                    };
                    if let Some(id) = flag_id {
                        writers.set_flag.write(SetFlagRequested { id, on: true });
                    }
                }
            }
        }
        // THE DEATH DRAMA, for every body that died — which arm it took decides
        // the ECONOMY, not whether anybody notices.
        //
        // This used to live inside the defeat arm, so two kinds of death were
        // SILENT: a body the ruleset owns (every fighter in a versus round
        // carries `RulesetOwnsDeath`) and a body that left the world. A round
        // that ends with no sound is a round nobody notices, and the KO is the
        // whole payoff of a platform fighter.
        //
        // The `RulesetOwnsDeath` arm's own comment lists what an arena must not
        // have and it is all economy — bounty coin, heart, death explosion,
        // split offspring, held-item drop, respawn timer. A body dying in its
        // own voice is not on that list and never was.
        if killed {
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
            // A body dies in its OWN voice. This was the session's until G1: a
            // Badnik and a Goomba died to the same sample even in a crossover.
            writers.sfx.write_for_body(
                victim_source.as_ref(),
                SfxMessage::Death { pos: em.kin.pos },
            );
        }
        true
    }
}
