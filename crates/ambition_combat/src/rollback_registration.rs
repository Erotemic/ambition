//! Rollback declaration owned by `ambition_combat`.
//!
//! This module names this domain's concrete rewindable state while the host
//! supplies the backend through [`RollbackRegistrar`]. It deliberately contains
//! no `bevy_ggrs` dependency and no host/composition logic.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
use ambition_platformer2d_core::snapshot::{
    checksum_bytes, put_bool, put_f32, put_i32, put_str, put_u64, put_u8, put_vec2,
};

const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register everything the combat domain needs rewound.
pub fn register_rollback_state<R>(registrar: &mut R)
where
    R: RollbackRegistrar,
{
    registrar.rollback_resource_clone::<crate::targeting::FactionRelations>(
        OWNER,
        "resource.faction_relations",
    );
    registrar.rollback_resource_clone::<crate::targeting::FriendlyFire>(
        OWNER,
        "resource.friendly_fire",
    );
    registrar.rollback_resource_clone_checksum::<crate::events::PendingPlayerHitEvents>(
        OWNER,
        "resource.pending_player_hit_events",
        "entity-free staged-hit checksum projection",
        pending_player_hits_checksum,
    );
    registrar.rollback_resource_map_entities::<crate::events::PendingPlayerHitEvents>(
        OWNER,
        "map.resource.pending_player_hit_events",
    );
    registrar.rollback_component_clone_entity_ref::<crate::moveset::StrikeVolume>(
        OWNER,
        "combat.strike_volume",
        |volume| volume.owner,
    );
    registrar.rollback_map_entities::<crate::moveset::StrikeVolume>(OWNER, "map.strike_volume");
    registrar.rollback_component_clone_checksum::<crate::on_hit::HitboxOnHit>(
        OWNER,
        "combat.hitbox_on_hit",
        "entity-less world-contact fired-state checksum projection",
        |on_hit| if on_hit.world_fired() { 1 } else { 0 },
    );
    registrar.rollback_component_canonical::<crate::components::BodyMelee>(
        OWNER,
        "actor.body_melee",
    );
    registrar.rollback_component_canonical::<crate::components::ActorDisposition>(
        OWNER,
        "actor.disposition",
    );
    registrar.rollback_component_cursor::<crate::components::ActorAggression>(
        OWNER,
        "actor.aggression",
    );
    registrar.rollback_map_entities::<crate::components::ActorAggression>(
        OWNER,
        "map.actor_aggression",
    );
    registrar.rollback_component_canonical::<crate::targeting::MatchTeam>(
        OWNER,
        "actor.match_team",
    );
    registrar.rollback_component_canonical::<crate::components::FighterStocks>(
        OWNER,
        "entity:fighter_stocks",
    );
    registrar.rollback_component_canonical::<crate::stocks::FighterEliminated>(
        OWNER,
        "entity:fighter_eliminated",
    );
    // ⛔ `resource.stocks_match_settled` used to be registered here, beside the
    // stock count. It is the RULESET's verdict rather than the engine's count,
    // it is keyed to a `MatchInstance`, and it lives beside `ActiveMatch` now —
    // in [`super::actors`], where the receipt it names is registered (D147).
    registrar.rollback_component_canonical::<crate::components::RulesetOwnsDeath>(
        OWNER,
        "actor.ruleset_owns_death",
    );
    // **The death interlude** (ADR 0033) — the window between a participant's
    // death and its consequence, and the state that keeps the world's hands off
    // the body while it is open. Both change mid-run, so both rewind: without
    // them a rewound branch resimulates with a body the world has stopped
    // touching for a death that has not happened in that branch.
    registrar.rollback_component_canonical::<crate::death_rules::OutOfPlay>(
        OWNER,
        "actor.out_of_play",
    );
    registrar.rollback_component_canonical::<crate::death_rules::DeathInterlude>(
        OWNER,
        "actor.death_interlude",
    );
    // **Is this body IN a fight?** Registered beside the death-ownership marker
    // it was standing in for, and for the same reason that one is: elimination
    // REMOVES it, so a rewind past an elimination has to put it back or the
    // replayed branch runs with a fighter that is out of a match it has not lost
    // yet. See `ActiveCombatant`.
    registrar.rollback_component_canonical::<crate::components::ActiveCombatant>(
        OWNER,
        "actor.active_combatant",
    );
    registrar.rollback_component_cursor::<crate::components::ActorTarget>(
        OWNER,
        "actor.target",
    );
    registrar.rollback_map_entities::<crate::components::ActorTarget>(
        OWNER,
        "map.actor_target",
    );
    registrar.rollback_component_resolved::<crate::moveset::MovePlayback>(
        OWNER,
        "actor.move_playback",
    );
    registrar.rollback_map_entities::<crate::moveset::MovePlayback>(OWNER, "map.move_playback");
    registrar.rollback_component_canonical::<crate::components::BossPatternTimer>(
        OWNER,
        "boss.pattern_timer",
    );
    registrar.rollback_component_canonical::<crate::components::BossPhase>(OWNER, "boss.phase");
    registrar.rollback_component_canonical::<crate::components::BodyEnvelope>(
        OWNER,
        "actor.body_envelope",
    );
    registrar.rollback_component_clone::<crate::components::CombatCapabilities>(
        OWNER,
        "combat.capabilities",
    );
    registrar.rollback_component_clone::<crate::components::CombatTuning>(
        OWNER,
        "combat.tuning",
    );
    registrar.rollback_component_clone::<crate::components::ActorIdentity>(
        OWNER,
        "actor.identity",
    );
    registrar.rollback_component_clone::<crate::components::ActorInteraction>(
        OWNER,
        "actor.interaction",
    );
    registrar.rollback_component_clone::<crate::components::ActorRenderSize>(
        OWNER,
        "actor.render_size",
    );
    registrar.rollback_component_clone::<crate::components::ActorSpriteOffset>(
        OWNER,
        "actor.sprite_offset",
    );
    registrar.rollback_component_clone::<crate::components::BossDeathAnimation>(
        OWNER,
        "boss.death_animation",
    );
    registrar.rollback_component_clone::<crate::components::CombatKit>(OWNER, "combat.kit");
    registrar.rollback_component_clone::<crate::components::DamageableVolumes>(
        OWNER,
        "feature.damageable_volumes",
    );
    registrar.rollback_component_clone::<crate::components::FeatureId>(OWNER, "feature.id");
    registrar.rollback_component_clone::<crate::components::FeatureName>(OWNER, "feature.name");
    registrar.rollback_component_clone::<crate::components::BreakableFeature>(
        OWNER,
        "feature.breakable",
    );
    registrar.rollback_component_clone::<crate::components::ChestFeature>(
        OWNER,
        "feature.chest",
    );
    registrar.rollback_component_clone::<crate::components::Opened>(OWNER, "feature.opened");
    registrar.rollback_component_clone_probed::<crate::components::RespawnTimer>(
        OWNER,
        "feature.respawn_timer",
        |timer| timer.0.to_bits() as u64,
    );
    registrar.rollback_component_clone_probed::<crate::components::StandTimer>(
        OWNER,
        "feature.stand_timer",
        |timer| timer.0.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::hazard_runtime::HazardFeature>(
        OWNER,
        "feature.hazard",
    );
    registrar.rollback_component_clone::<crate::components::PogoPolicy>(
        OWNER,
        "feature.pogo_policy",
    );
    registrar.rollback_component_clone::<crate::components::PogoTargetContributor>(
        OWNER,
        "feature.pogo_target_contributor",
    );
    registrar.rollback_component_clone::<crate::components::PogoTargetVolumes>(
        OWNER,
        "feature.pogo_target_volumes",
    );
    registrar.rollback_component_clone::<crate::held_items::HeldItem>(OWNER, "actor.held_item");
    registrar.rollback_component_clone::<crate::moveset::ActorMoveset>(OWNER, "actor.moveset");
    registrar.rollback_component_clone::<crate::moveset::MovesetMelee>(
        OWNER,
        "actor.moveset_melee",
    );
    registrar.rollback_component_clone::<crate::components::PickupFeature>(
        OWNER,
        "feature.pickup",
    );
    registrar.rollback_component_clone::<crate::components::Collected>(
        OWNER,
        "feature.collected",
    );
    registrar.rollback_component_clone::<crate::components::RuntimeStagedActor>(
        OWNER,
        "marker.runtime_staged_actor",
    );
    registrar.declare_rollback_derived_resource::<crate::rules::ResolvedCombatTuning>(
        OWNER,
        "derived.resolved_combat_tuning",
        "refolded from DeclaredCombatRules over the world baseline every WorldPrep",
    );
    // **CAPTURE: the relationship is state; the requests are not.**
    //
    // `CapturedBy` is authoritative sim state — a rewind past a grab must undo
    // the grab, and a rewind past a THROW must put the captive back in the hold.
    // Cloned rather than blob-encoded because it carries an `Entity`, which N3.1
    // forbids in a blob; the `map_entities` pass below re-points that handle the
    // way `RidingOn`'s does. Same shape, for the same reason: a component on the
    // dependent body naming the other one.
    registrar.rollback_component_clone_entity_ref::<crate::capture::CapturedBy>(
        OWNER,
        "capture.captured_by",
        |held| held.captor,
    );
    registrar.rollback_map_entities::<crate::capture::CapturedBy>(OWNER, "map.captured_by");
    // ⚠ the three capture REQUESTS are same-frame transients. A resimulated tick
    // re-derives them from the authored timeline it is replaying, so a buffer
    // that survived the rewind would apply a pummel twice.
    registrar.clear_message_on_rollback::<crate::capture::CaptureAttemptRequested>(
        OWNER,
        "message.capture_attempt_requested",
    );
    registrar.clear_message_on_rollback::<crate::capture::CapturePummelRequested>(
        OWNER,
        "message.capture_pummel_requested",
    );
    registrar.clear_message_on_rollback::<crate::capture::CaptureThrowRequested>(
        OWNER,
        "message.capture_throw_requested",
    );
    registrar.clear_message_on_rollback::<crate::hitbox::LandedBodyHit>(
        OWNER,
        "message.landed_body_hit",
    );
    registrar.clear_message_on_rollback::<crate::events::HitEvent>(OWNER, "message.hit_event");
    registrar.clear_message_on_rollback::<crate::stocks::BodyKnockedOut>(
        OWNER,
        "message.body_knocked_out",
    );
    registrar.clear_message_on_rollback::<crate::stocks::FighterStockSpent>(
        OWNER,
        "message.fighter_stock_spent",
    );
    registrar.clear_message_on_rollback::<crate::stocks::StocksMatchDecided>(
        OWNER,
        "message.stocks_match_decided",
    );
    registrar.clear_message_on_rollback::<crate::on_hit::OnHitEffectMessage>(
        OWNER,
        "message.on_hit_effect",
    );
    registrar.clear_message_on_rollback::<crate::moveset::MoveEventMessage>(
        OWNER,
        "message.move_event",
    );
    registrar.clear_message_on_rollback::<crate::events::ActorStimulus>(
        OWNER,
        "message.actor_stimulus",
    );
    registrar.clear_message_on_rollback::<crate::events::GameplayBannerRequested>(
        OWNER,
        "message.gameplay_banner_requested",
    );
    registrar.clear_message_on_rollback::<crate::events::GameplaySfxRequested>(
        OWNER,
        "message.gameplay_sfx_requested",
    );
    registrar.clear_message_on_rollback::<crate::events::ResetRoomFeaturesEvent>(
        OWNER,
        "message.reset_room_features",
    );
    registrar.clear_message_on_rollback::<crate::events::SetFlagRequested>(
        OWNER,
        "message.set_flag_requested",
    );
    registrar.clear_message_on_rollback::<crate::events::ActorStimulus>(
        OWNER,
        "message.actor_stimulus",
    );
    registrar.clear_message_on_rollback::<crate::events::GameplayBannerRequested>(
        OWNER,
        "message.gameplay_banner_requested",
    );
    registrar.clear_message_on_rollback::<crate::events::GameplaySfxRequested>(
        OWNER,
        "message.gameplay_sfx_requested",
    );
    registrar.clear_message_on_rollback::<crate::events::ResetRoomFeaturesEvent>(
        OWNER,
        "message.reset_room_features",
    );
    registrar.clear_message_on_rollback::<crate::events::SetFlagRequested>(
        OWNER,
        "message.set_flag_requested",
    );
    // Strike entities and hit-once bookkeeping are combat state even when their
    // presentation is VFX-driven.
    registrar.require_rollback::<crate::strike::Hitbox>(OWNER, "entity:hitbox");
    registrar.rollback_component_clone_entity_ref::<crate::strike::Hitbox>(
        OWNER,
        "combat.hitbox",
        |hitbox| hitbox.owner,
    );
    registrar.rollback_map_entities::<crate::strike::Hitbox>(OWNER, "map.hitbox");
    registrar.rollback_component_clone_entity_set::<crate::strike::HitboxHits>(
        OWNER,
        "combat.hitbox_hits",
        |hits| hits.hit.iter().copied().collect(),
    );
    registrar.rollback_map_entities::<crate::strike::HitboxHits>(OWNER, "map.hitbox_hits");
    registrar.rollback_component_clone_probed::<crate::strike::HitboxLifetime>(
        OWNER,
        "combat.hitbox_lifetime",
        |lifetime| lifetime.remaining_s.to_bits() as u64,
    );

}

/// Entity-free canonical projection of the staged victim-hit FIFO.
///
/// The exact `Entity` handles (`attacker`, pre-resolved targets) stay out —
/// the stable-id contract keeps allocator-local values out of every checksum —
/// but everything that decides what the hit DOES participates, so a diverged
/// queue surfaces as a sync-test mismatch at the staging frame instead of one
/// frame later as mystery damage.
fn pending_player_hits_checksum(pending: &crate::events::PendingPlayerHitEvents) -> u64 {
    use crate::events::{HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
    let mut bytes = Vec::new();
    put_u64(&mut bytes, pending.0.len() as u64);
    for event in &pending.0 {
        let bounds = event.volume.bounds();
        put_vec2(&mut bytes, bounds.min);
        put_vec2(&mut bytes, bounds.max);
        put_i32(&mut bytes, event.damage);
        // ⚠ **the tags are the CAUSE vocabulary's, and the old direction-spelled
        // tags 5-11 are retired.** Nine variants folded into four causes when the
        // player-versus-world half of each name stopped carrying routing, so the
        // spread is now 0-3 plus 10. Tags are checksum input, not a persisted
        // wire format, so renumbering costs a checksum change and nothing else —
        // but keep `LeftTheWorld` at 10 rather than compacting it, because a
        // gratuitous renumber of a surviving variant is a diff nobody can review.
        //
        // The `f32` beside each tag was `PlayerSlash`'s own impulse channel. That
        // channel is gone — knockback has one representation — so it is a
        // constant here. The slot stays because the layout is shared.
        let (source_tag, source_payload) = match event.source {
            HitSource::Melee => (0u8, 0.0),
            HitSource::Projectile => (1, 0.0),
            HitSource::Pogo => (2, 0.0),
            HitSource::Hazard => (3, 0.0),
            HitSource::Contact => (4, 0.0),
            HitSource::LeftTheWorld => (10, 0.0),
        };
        put_u8(&mut bytes, source_tag);
        put_f32(&mut bytes, source_payload);
        put_bool(&mut bytes, event.attacker.is_some());
        put_u8(
            &mut bytes,
            match event.target {
                HitTarget::Volume => 0,
                HitTarget::Body(_) => 1,
                HitTarget::OrbMatch => 3,
                HitTarget::UnresolvedFeatures => 4,
            },
        );
        put_u8(
            &mut bytes,
            match event.mode {
                HitMode::Knockback => 0,
                HitMode::SafeRespawn => 1,
            },
        );
        match &event.knockback {
            None => put_bool(&mut bytes, false),
            Some(kb) => {
                put_bool(&mut bytes, true);
                put_f32(&mut bytes, kb.dir);
                match kb.magnitude {
                    HitKnockbackMagnitude::FeelScale(value) => {
                        put_u8(&mut bytes, 0);
                        put_f32(&mut bytes, value);
                    }
                    HitKnockbackMagnitude::LaunchSpeed(value) => {
                        put_u8(&mut bytes, 1);
                        put_f32(&mut bytes, value);
                    }
                }
                put_vec2(&mut bytes, kb.source_pos);
                put_vec2(&mut bytes, kb.impact_pos);
                match kb.launch_dir {
                    None => put_bool(&mut bytes, false),
                    Some(dir) => {
                        put_bool(&mut bytes, true);
                        put_vec2(&mut bytes, dir);
                    }
                }
            }
        }
        for key in &event.ignored_targets {
            put_str(&mut bytes, key);
        }
    }
    checksum_bytes(&bytes)
}
