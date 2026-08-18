//! **The combat domain's rollback schema** (Campaign 2, R3).
//!
//! Combat's authoritative state: slots, volumes, hit bookkeeping, and the message buffers a rewound tick must not replay.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_platformer2d_runtime` because this module is in it, and
//! must be: `ambition_combat` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;
// The byte-writer vocabulary these projections are built from.
use ambition_platformer2d_core::snapshot::{
    checksum_bytes, put_bool, put_f32, put_i32, put_str, put_u64, put_u8, put_vec2,
};
// The bespoke checksum projections these registrations name. They live beside
// the central function because several domains once shared them; a projection
// used by exactly one domain should follow it here, which is a later tidy and
// not part of a relocation commit.

const OWNER: &str = "ambition_platformer2d_runtime";

/// Register everything the combat domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.rollback_resource_clone::<ambition_combat::targeting::FactionRelations>(
        OWNER,
        "resource.faction_relations",
    );
    app.rollback_resource_clone::<ambition_combat::targeting::FriendlyFire>(
        OWNER,
        "resource.friendly_fire",
    );
    app.rollback_resource_clone_checksum::<ambition_combat::events::PendingPlayerHitEvents>(
        OWNER,
        "resource.pending_player_hit_events",
        "bevy_ggrs clone snapshot + entity-free staged-hit checksum projection",
        pending_player_hits_checksum,
    );
    app.rollback_resource_map_entities::<ambition_combat::events::PendingPlayerHitEvents>(
        OWNER,
        "map.resource.pending_player_hit_events",
    );
    app.rollback_component_clone_entity_ref::<ambition_combat::moveset::StrikeVolume>(
        OWNER,
        "combat.strike_volume",
        |volume| volume.owner,
    );
    app.rollback_map_entities::<ambition_combat::moveset::StrikeVolume>(OWNER, "map.strike_volume");
    app.rollback_component_clone_checksum::<ambition_combat::on_hit::HitboxOnHit>(
        OWNER,
        "combat.hitbox_on_hit",
        "bevy_ggrs clone snapshot + entity-less world-contact fired-state checksum projection",
        |on_hit| if on_hit.world_fired() { 1 } else { 0 },
    );
    app.rollback_component_canonical::<ambition_combat::components::BodyMelee>(
        OWNER,
        "actor.body_melee",
    );
    app.rollback_component_canonical::<ambition_combat::components::ActorDisposition>(
        OWNER,
        "actor.disposition",
    );
    app.rollback_component_cursor::<ambition_combat::components::ActorAggression>(
        OWNER,
        "actor.aggression",
    );
    app.rollback_map_entities::<ambition_combat::components::ActorAggression>(
        OWNER,
        "map.actor_aggression",
    );
    app.rollback_component_canonical::<ambition_combat::targeting::MatchTeam>(
        OWNER,
        "actor.match_team",
    );
    app.rollback_component_canonical::<ambition_combat::components::FighterStocks>(
        OWNER,
        "entity:fighter_stocks",
    );
    app.rollback_component_canonical::<ambition_combat::stocks::FighterEliminated>(
        OWNER,
        "entity:fighter_eliminated",
    );
    // ⛔ `resource.stocks_match_settled` used to be registered here, beside the
    // stock count. It is the RULESET's verdict rather than the engine's count,
    // it is keyed to a `MatchInstance`, and it lives beside `ActiveMatch` now —
    // in [`super::actors`], where the receipt it names is registered (D147).
    app.rollback_component_canonical::<ambition_combat::components::RulesetOwnsDeath>(
        OWNER,
        "actor.ruleset_owns_death",
    );
    // **The death interlude** (ADR 0033) — the window between a participant's
    // death and its consequence, and the state that keeps the world's hands off
    // the body while it is open. Both change mid-run, so both rewind: without
    // them a rewound branch resimulates with a body the world has stopped
    // touching for a death that has not happened in that branch.
    app.rollback_component_canonical::<ambition_combat::death_rules::OutOfPlay>(
        OWNER,
        "actor.out_of_play",
    );
    app.rollback_component_canonical::<ambition_combat::death_rules::DeathInterlude>(
        OWNER,
        "actor.death_interlude",
    );
    // **Is this body IN a fight?** Registered beside the death-ownership marker
    // it was standing in for, and for the same reason that one is: elimination
    // REMOVES it, so a rewind past an elimination has to put it back or the
    // replayed branch runs with a fighter that is out of a match it has not lost
    // yet. See `ActiveCombatant`.
    app.rollback_component_canonical::<ambition_combat::components::ActiveCombatant>(
        OWNER,
        "actor.active_combatant",
    );
    app.rollback_component_cursor::<ambition_combat::components::ActorTarget>(
        OWNER,
        "actor.target",
    );
    app.rollback_map_entities::<ambition_combat::components::ActorTarget>(
        OWNER,
        "map.actor_target",
    );
    app.rollback_component_resolved::<ambition_combat::moveset::MovePlayback>(
        OWNER,
        "actor.move_playback",
    );
    app.rollback_map_entities::<ambition_combat::moveset::MovePlayback>(OWNER, "map.move_playback");
    app.rollback_component_canonical::<ambition_combat::components::BossPatternTimer>(
        OWNER,
        "boss.pattern_timer",
    );
    app.rollback_component_canonical::<ambition_combat::components::BossPhase>(OWNER, "boss.phase");
    app.rollback_component_canonical::<ambition_combat::components::BodyEnvelope>(
        OWNER,
        "actor.body_envelope",
    );
    app.rollback_component_clone::<ambition_combat::components::CombatCapabilities>(
        OWNER,
        "combat.capabilities",
    );
    app.rollback_component_clone::<ambition_combat::components::CombatTuning>(
        OWNER,
        "combat.tuning",
    );
    app.rollback_component_clone::<ambition_combat::components::ActorIdentity>(
        OWNER,
        "actor.identity",
    );
    app.rollback_component_clone::<ambition_combat::components::ActorInteraction>(
        OWNER,
        "actor.interaction",
    );
    app.rollback_component_clone::<ambition_combat::components::ActorRenderSize>(
        OWNER,
        "actor.render_size",
    );
    app.rollback_component_clone::<ambition_combat::components::ActorSpriteOffset>(
        OWNER,
        "actor.sprite_offset",
    );
    app.rollback_component_clone::<ambition_combat::components::BossDeathAnimation>(
        OWNER,
        "boss.death_animation",
    );
    app.rollback_component_clone::<ambition_combat::components::CombatKit>(OWNER, "combat.kit");
    app.rollback_component_clone::<ambition_combat::components::DamageableVolumes>(
        OWNER,
        "feature.damageable_volumes",
    );
    app.rollback_component_clone::<ambition_combat::components::FeatureId>(OWNER, "feature.id");
    app.rollback_component_clone::<ambition_combat::components::FeatureName>(OWNER, "feature.name");
    app.rollback_component_clone::<ambition_combat::components::BreakableFeature>(
        OWNER,
        "feature.breakable",
    );
    app.rollback_component_clone::<ambition_combat::components::ChestFeature>(
        OWNER,
        "feature.chest",
    );
    app.rollback_component_clone::<ambition_combat::components::Opened>(OWNER, "feature.opened");
    app.rollback_component_clone_probed::<ambition_combat::components::RespawnTimer>(
        OWNER,
        "feature.respawn_timer",
        |timer| timer.0.to_bits() as u64,
    );
    app.rollback_component_clone_probed::<ambition_combat::components::StandTimer>(
        OWNER,
        "feature.stand_timer",
        |timer| timer.0.to_bits() as u64,
    );
    app.rollback_component_clone::<ambition_combat::hazard_runtime::HazardFeature>(
        OWNER,
        "feature.hazard",
    );
    app.rollback_component_clone::<ambition_combat::components::PogoPolicy>(
        OWNER,
        "feature.pogo_policy",
    );
    app.rollback_component_clone::<ambition_combat::components::PogoTargetContributor>(
        OWNER,
        "feature.pogo_target_contributor",
    );
    app.rollback_component_clone::<ambition_combat::components::PogoTargetVolumes>(
        OWNER,
        "feature.pogo_target_volumes",
    );
    app.rollback_component_clone::<ambition_combat::held_items::HeldItem>(OWNER, "actor.held_item");
    app.rollback_component_clone::<ambition_combat::moveset::ActorMoveset>(OWNER, "actor.moveset");
    app.rollback_component_clone::<ambition_combat::moveset::MovesetMelee>(
        OWNER,
        "actor.moveset_melee",
    );
    app.rollback_component_clone::<ambition_combat::components::PickupFeature>(
        OWNER,
        "feature.pickup",
    );
    app.rollback_component_clone::<ambition_combat::components::Collected>(
        OWNER,
        "feature.collected",
    );
    app.rollback_component_clone::<ambition_combat::components::RuntimeStagedActor>(
        OWNER,
        "marker.runtime_staged_actor",
    );
    app.declare_rollback_derived_resource::<ambition_combat::rules::ResolvedCombatTuning>(
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
    app.rollback_component_clone_entity_ref::<ambition_combat::capture::CapturedBy>(
        OWNER,
        "capture.captured_by",
        |held| held.captor,
    );
    app.rollback_map_entities::<ambition_combat::capture::CapturedBy>(OWNER, "map.captured_by");
    // ⚠ the three capture REQUESTS are same-frame transients. A resimulated tick
    // re-derives them from the authored timeline it is replaying, so a buffer
    // that survived the rewind would apply a pummel twice.
    app.clear_message_on_rollback::<ambition_combat::capture::CaptureAttemptRequested>(
        OWNER,
        "message.capture_attempt_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::capture::CapturePummelRequested>(
        OWNER,
        "message.capture_pummel_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::capture::CaptureThrowRequested>(
        OWNER,
        "message.capture_throw_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::hitbox::LandedBodyHit>(
        OWNER,
        "message.landed_body_hit",
    );
    app.clear_message_on_rollback::<ambition_combat::events::HitEvent>(OWNER, "message.hit_event");
    app.clear_message_on_rollback::<ambition_combat::stocks::BodyKnockedOut>(
        OWNER,
        "message.body_knocked_out",
    );
    app.clear_message_on_rollback::<ambition_combat::stocks::FighterStockSpent>(
        OWNER,
        "message.fighter_stock_spent",
    );
    app.clear_message_on_rollback::<ambition_combat::stocks::StocksMatchDecided>(
        OWNER,
        "message.stocks_match_decided",
    );
    app.clear_message_on_rollback::<ambition_combat::on_hit::OnHitEffectMessage>(
        OWNER,
        "message.on_hit_effect",
    );
    app.clear_message_on_rollback::<ambition_combat::moveset::MoveEventMessage>(
        OWNER,
        "message.move_event",
    );
    app.clear_message_on_rollback::<ambition_combat::events::ActorStimulus>(
        OWNER,
        "message.actor_stimulus",
    );
    app.clear_message_on_rollback::<ambition_combat::events::GameplayBannerRequested>(
        OWNER,
        "message.gameplay_banner_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::events::GameplaySfxRequested>(
        OWNER,
        "message.gameplay_sfx_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::events::ResetRoomFeaturesEvent>(
        OWNER,
        "message.reset_room_features",
    );
    app.clear_message_on_rollback::<ambition_combat::events::SetFlagRequested>(
        OWNER,
        "message.set_flag_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::events::ActorStimulus>(
        OWNER,
        "message.actor_stimulus",
    );
    app.clear_message_on_rollback::<ambition_combat::events::GameplayBannerRequested>(
        OWNER,
        "message.gameplay_banner_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::events::GameplaySfxRequested>(
        OWNER,
        "message.gameplay_sfx_requested",
    );
    app.clear_message_on_rollback::<ambition_combat::events::ResetRoomFeaturesEvent>(
        OWNER,
        "message.reset_room_features",
    );
    app.clear_message_on_rollback::<ambition_combat::events::SetFlagRequested>(
        OWNER,
        "message.set_flag_requested",
    );
}

/// Entity-free canonical projection of the staged victim-hit FIFO.
///
/// The exact `Entity` handles (`attacker`, pre-resolved targets) stay out —
/// the stable-id contract keeps allocator-local values out of every checksum —
/// but everything that decides what the hit DOES participates, so a diverged
/// queue surfaces as a sync-test mismatch at the staging frame instead of one
/// frame later as mystery damage.
fn pending_player_hits_checksum(pending: &ambition_combat::events::PendingPlayerHitEvents) -> u64 {
    use ambition_combat::events::{HitKnockbackMagnitude, HitMode, HitSource, HitTarget};
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
