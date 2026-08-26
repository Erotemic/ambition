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

/// A dead hostile actor returns `false` (no-op).
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_actor_hit(
    event: &HitEvent,
    catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    // AD8: the prepared cast, for the voice floor under the two bark paths that
    // did not have one.
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    actor_entity: Entity,
    disposition: ActorDisposition,
    // Does a RULESET own this body's death? A match fighter's KO belongs to the
    // match, not to the world's exploration economy.
    ruleset_owns_death: bool,
    active_combatant: bool,
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
    feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
    // The struck actor's held locomotion (local frame) for DI (CM2) — the SAME
    // `ActorControl` the brain writes, so a brain/RL victim DIs like a human.
    di_input_local: ae::Vec2,
    // CM8: how THIS body reacts to being hurt (its `CombatTuning.hurt_feedback`,
    // the ENEMY default today). The victim owns its spray/debris; the attack owns
    // only the strike sound.
    hurt: ambition_vfx::HurtFeedback,
    // Does this hit come off a HEAVY attacker? — the heavier launch and the
    // longer hitstun (`feel.boss_*`).
    //
    // That is a source-specific formula for a fact about the ATTACKER, and it could only ever be
    // true for a body the vocabulary happened to have a word for — so a heavy NPC, a possessed
    // boss, or a match fighter with boss-class weight was unrepresentable. The caller asks the
    // attacker entity now.
    heavy_attacker: bool,
    // Is this body inside an EVADE? The published maneuver fact
    // (`BodyMotionFacts::evading`), resolved by the system that holds the
    // queries — the same shape `heavy_attacker` above already uses, and for the
    // same reason: the actor cluster is at Bevy's column ceiling and this is a
    // read of one component by entity.
    evading: bool,
    // ⭐ MAY THIS HIT SPEAK? — drawn by the caller, for the same reason
    // `heavy_attacker` and `evading` above are resolved there: the draw needs
    // the tick and the match, and this resolver holds neither.
    //
    // Jon, 2026-08-24: *"not have barks happen every time a character is hit.
    // Make it a more rare event."* `true` is every world that declared no rate.
    bark_allowed: bool,
    writers: &mut FeatureHitWriters<'_, '_>,
) -> bool {
    let session_scope = writers.session_spawn_scope();
    // THE QUESTION IS COMBAT STANDING, NOT SOCIAL MOOD. This asked
    // `disposition.is_peaceful()`, which made
    // `ActorDisposition` answer two things at once: *how does this actor regard
    // combat* and *may this body be hurt*. A fighter somebody entered into a
    // match had to stay `Hostile` merely to be damageable — and two
    // participant-driven fighters hold no AI target, so both stood down and neither
    // could hurt the other.
    //
    // being IN a fight is the stated decision, and it outranks whatever this
    // body's brain currently thinks. The provoke-before-damage behaviour a town
    // NPC needs is unchanged, because a town NPC is in no fight.
    //
    // and it is `ActiveCombatant`, not `RulesetOwnsDeath` — this asked the
    // death-ownership marker, which correlates and does not mean the same thing.
    // An eliminated fighter's body keeps standing, its death still belongs to
    // the match, and it is not fighting; the correlation breaks exactly there.
    if !ambition_combat::components::CombatStanding::of(disposition, active_combatant).takes_damage()
    {
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
                            prepared,
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
                            prepared,
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
        // ⭐ TWO CONDITIONS, AND THEY ANSWER DIFFERENT QUESTIONS. The flash is
        // DEDUP — is this the first non-overlapping hit — and the draw is RATE.
        // Folding them together would make a rare bark also a broken dedup.
        let should_bark = combat.hit_flash < 0.05 && bark_allowed;
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
        // What the body is actually holding, captured beside `caps` and for
        // the same reason: the death branch below runs after the resolver has
        // borrowed `em` mutably, and this is a read of pre-death state.
        let held_at_death = em.held_item.map(|held| held.spec.clone());
        // THE shared victim-side mechanics (§A2): consume-time i-frame gate,
        // the reactive shield block (the body's RESOLVED guard — a possessing
        // human and an AI brain block identically, invariants I2/I3; the same
        // frame-agnostic directional rule the player uses), damage, death
        // flag, and hit-flash/i-frame arming. Actors pass multiplier 1.0 —
        // difficulty scaling is player policy.
        // Resolved BEFORE the shared mechanics, because it decides whether they
        // apply at all: the blast zone is not a hit anything can defend against.
        let left_the_world = matches!(event.source, HitSource::LeftTheWorld);
        let shield_tuning = motion_model.shield_tuning();
        // `Copy` reads taken before the guard borrows the velocity.
        let victim_facing = em.kin.facing;
        let victim_pos = em.kin.pos;
        let victim_size = em.kin.size;
        let resolution = crate::features::ecs::damage_apply::resolve_body_hit(
            combat,
            Some(&mut *em.health),
            // No actor archetype wears equipment armor today; the resolver
            // supports it generically, but nothing threads a `WornEquipment` here.
            None,
            wallet_shield,
            crate::features::ecs::damage_apply::GuardUnderFire::offered_to(
                event.knockback.as_ref(),
                &mut *em.shield,
                shield_tuning,
                &mut em.kin.vel,
                victim_size,
            ),
            victim_facing,
            victim_pos,
            event.volume.center(),
            gravity_dir,
            event.damage,
            1.0,
            caps.never_dies,
            crate::features::ecs::damage_apply::BodyHitFeel {
                hit_flash: 0.16,
                // SCALED BY THE MATCH'S RULE, so a game whose moves author
                // their own multi-hit cadence can say it has no blanket window
                // — see `DeclaredCombatRules::hit_repeat_window_scale`. An
                // undeclared world multiplies by `1.0`.
                damage_invuln_time: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S
                    * feel.hit_repeat_window_scale,
                block_hit_flash: 0.16,
                block_invuln_floor: super::super::actor_clusters::ACTOR_DAMAGE_IFRAME_S
                    * feel.hit_repeat_window_scale,
                armor_hitstop_time: 0.070,
            },
            evading,
            left_the_world,
        );
        // The resolver's decision, announced for the inspector — BEFORE the
        // early return, because `Ignored` ("i-framed, or already dead") is one
        // of the answers somebody comes here looking for. A publisher placed
        // after this line would explain every hit except the ones that did
        // nothing, which are the puzzling ones.
        #[cfg(feature = "causal")]
        if let Some(resolutions) = writers.resolutions.as_mut() {
            resolutions.write(crate::features::ecs::damage_apply::BodyHitResolved {
                body: actor_entity,
                resolution,
                source: event.source.clone(),
                raw_damage: event.damage,
            });
        }
        if resolution == crate::features::ecs::damage_apply::BodyHitResolution::Ignored {
            return false;
        }
        // CM1 death policy: an `Unbounded` (smash-percent) body never dies from
        // its meter — the blast-zone/OOB gate owns its death — so a meter-kill
        // is suppressed. `HpDepleted` (the default) kills as before: parity.
        // Computed HERE, before the bark, so a LETHAL hit does not also speak a
        // hit line: a dying body presents its death (the Death SFX + burst +
        // debris below), not an "ow!".
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
            ) && em.health.policy().kills_at_max());
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
        // ⭐ A HIT TAKES THE HANG. Jon, 2026-08-24: *"A character can just stay
        // on the ledge, and there is no way to knock them off. If you get hit
        // you should fall off the ledge at least."*
        //
        // ⛔⛔ THE RULE EXISTED AND ONLY THE PLAYER ROAD CALLED IT. Both of
        // `damage_apply`'s `HitMode::Knockback` arms knock the hanging PLAYER
        // off through `knock_off_ledge`; this generic actor road never did, so
        // every CPU fighter — and in the arena, every fighter, because a
        // platform fighter's roster is actors — hung through an edge-guard
        // untouched. The typed op arms the re-grab lockout with it, so the body
        // falls with the knockback instead of re-latching on the next frame.
        //
        // Before the knockback below and outside its `Option`: a damaging hit
        // that authored no launch still ends the hang, exactly as it does for
        // the player. A blocked or armored hit returned above and never reaches
        // here.
        // §A2 step 6 (FEEL-BLIND): a struck actor rides the SAME feel-tuned,
        // frame-agnostic knockback resolution the player does — side away from
        // the source, rise against ITS gravity — replacing the old inline
        // `local.y - 90 max -280` pop. The data comes from the event's
        // `HitKnockback` (attached by hitboxes / body-contact / hazards). A hit
        // with none leaves the velocity alone.
        //
        // One producer used the channel (the dive corridor, at 1.4) and its magnitude never reached
        // a victim. Both the field and this arm are deleted: a producer that wants a shove attaches
        // a `HitKnockback`, and there is nowhere left to put a magnitude that silently evaporates.
        //
        // ⛔⛔ AND IT IS NOT CONDITIONAL ON A LAUNCH. This whole block sat inside
        // `if let Some(k) = knockback`, so a damage-only hit — a hazard, a chip,
        // a poison tick — skipped the shared reaction outright and with it every
        // fact of having been hit: no hitlag, no ledge drop, no dodge back. The
        // player road ran it and erased the body's velocity instead. Two roads,
        // wrong in opposite directions, which is D203's whole subject.
        //
        // ⇒ every ACCEPTED damaging hit runs the reaction, and the reaction
        // decides what a missing launch means.
        let knockback = event.knockback.clone();
        {
            let k = knockback;
            let boss_hit = heavy_attacker;
            // §A2 step 7 (FEEL-BLIND): the launch also arms the shared stagger
            // (hitstun / recoil-lock / hitstop on `BodyCombat`), consumed by
            // the actor driver's post-hit input gate + hitstop dt beat — an
            // actor staggers exactly like the player.
            let pos = em.kin.pos;
            let facing = em.kin.facing;
            let reaction = crate::features::ecs::damage_apply::apply_body_hit_reaction(
                &mut em.kin.vel,
                &mut em.flight,
                combat,
                pos,
                facing,
                gravity_dir,
                boss_hit,
                k.as_ref(),
                event.damage,
                di_input_local,
                crate::features::ecs::damage_apply::VictimStance {
                    grounded: em.ground.on_ground,
                    crouching: em.body_mode.body_mode == ae::BodyMode::Crouching,
                },
                // ⭐ THE AIR DODGE A HIT GIVES BACK — one resource, and the
                // reaction's rather than each road's (D203). ⛔ NOT the double
                // jump: a spent second jump stays spent through an ordinary
                // edge-guard hit, which is what makes taking one worth doing.
                Some(&mut *em.dodge),
                // …and the helpless EPISODE the hit ends. Not a resource: the
                // spent recovery charge stays spent.
                Some(&mut *em.jump),
                Some((motion_model, em.ledge)),
                feel,
            );
            // ⭐ THE HIT'S RESULT, for the simulation — the actor road's half of
            // the fact the match freeze reads. Beside the reaction because THAT
            // is where the hitlag exists: publishing beside the resolution
            // instead reported zero for every hit.
            crate::features::ecs::damage_apply::publish_resolved_hit(
                writers.resolved.as_mut(),
                actor_entity,
                combat.hitstop_timer,
                event.source.clone(),
            );
            #[cfg(feature = "causal")]
            if let Some(reactions) = writers.reactions.as_mut() {
                reactions.write(crate::features::ecs::damage_apply::BodyReactionApplied {
                    body: actor_entity,
                    reaction,
                });
            }
            #[cfg(not(feature = "causal"))]
            let _ = reaction;
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
            // THE KO, announced (S4). This branch is exactly where a match
            // rather than the world takes over, so it is where the stocks loop
            // is told. Health is the wrong signal for it: an `Unbounded`
            // fighter's pool is FULL at the moment it is thrown off the stage,
            // so a rule watching `alive()` would watch a healthy fighter fall
            // out of the world forever.
            writers
                .knockouts
                .write(ambition_combat::stocks::BodyKnockedOut {
                    body: actor_entity,
                    cause: event.source.clone(),
                });
            // A RULESET owns this body's death (`RulesetOwnsDeath`). Health is
            // already zero and stays zero, and NONE of the world's death
            // consequences run: no bounty coin, no heart, no death explosion, no
            // split offspring, no held-item drop, no in-place respawn timer.
            //
            // Those are an exploration economy. An arena has no economy, and a
            // round that funds the player's wallet and detonates the loser is
            // not a round.
        } else if killed && kill_disposition(&event.source, em.config.tuning.respawn).is_gone() {
            // GONE — see [`kill_disposition`]. No respawn timer, and no
            // exploration payout either: a bounty coin, a heart, or a death
            // explosion dropped at this corpse would land in the void, somewhere
            // no player can ever walk to.
            //
            // The type's own doc claims it is *"a decision, extracted so it can be stated and
            // tested rather than inferred from the order of an `if`-chain"* — and the chain went on
            // inferring it. Asking the decision makes the extraction load-bearing instead of
            // decorative.
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
                // Whose death this loot fell out of. Resolved once for both
                // drops below: each states it as its provenance, and without it
                // no render family claims the pickup.
                let parent = super::drop_parent(writers, actor_entity, "actor", &em.config.id);
                // Earn-side: a defeated enemy drops a collectible coin so the
                // player can fund the merchant / ability shop from combat, and
                // ~1 in 4 enemy kinds also drops a heart (combat sustain).
                if let Some(parent) = &parent {
                    drop_currency_coin(
                        &mut writers.commands,
                        session_scope,
                        parent,
                        &em.config.id,
                        em.kin.pos,
                        ENEMY_BOUNTY,
                    );
                }
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
                    writers.vfx.write(VfxMessage::Effect {
                        pos: em.kin.pos,
                        fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                        scale: 0.85,
                        pose: ambition_vfx::FxPose::UPRIGHT,
                    });
                }
                if let Some(offspring) = caps.divides_into.as_deref() {
                    spawn_split_offspring(
                        &mut writers.commands,
                        catalog,
                        authored_sheets,
                        prepared,
                        session_scope,
                        &em.config.id,
                        em.kin.pos,
                        offspring,
                    );
                }
                if let (true, Some(parent)) = (id_drops_health(&em.config.id), &parent) {
                    drop_health_pickup(
                        &mut writers.commands,
                        session_scope,
                        parent,
                        &em.config.id,
                        em.kin.pos + ae::Vec2::new(18.0, 0.0),
                        ENEMY_HEALTH_DROP,
                    );
                }
                // Steal the enemy's weapon: a defeated body that drops what it
                // wields leaves it as a `GroundItem` the player can grab and use
                // (e.g. a pirate's gun-sword), via the existing pickup path.
                //
                // `held_items`' own module doc had already named this consumer: *"future item
                // drops can read the same component without adding archetype-specific Rust
                // branches."*
                if let (true, Some(spec), Some(parent)) =
                    (caps.drops_held_item, held_at_death.clone(), &parent)
                {
                    super::super::damage_drops::drop_held_weapon(
                        &mut writers.commands,
                        session_scope,
                        parent,
                        em.kin.pos + ae::Vec2::new(-14.0, 0.0),
                        spec,
                        ae::Vec2::splat(16.0),
                        "Dropped weapon",
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
        // A round that ends with no sound is a round nobody notices, and the KO is the whole payoff
        // of a platform fighter.
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
            // A body dies in its OWN voice.
            writers.sfx.write_for_body(
                victim_source.as_ref(),
                SfxMessage::Death { pos: em.kin.pos },
            );
        }
        true
    }
}
