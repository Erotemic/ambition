//! **The combat domain's rollback schema** (Campaign 2, R3).
//!
//! Combat's authoritative state: slots, volumes, hit bookkeeping, and the message buffers a rewound tick must not replay.
//!
//! ⚠ **relocation only.** The registrations were extracted mechanically and the
//! schema baseline verifies the result is byte-identical — a retyped call is
//! exactly the mistake that would slip through review and not through the
//! baseline.
//!
//! ⚠ the owner label stays `ambition_runtime` because this module is in it, and
//! must be: `ambition_combat` sits below the runtime in the crate graph. R1's
//! recorded decision is that this is the right shape for every domain below the
//! runtime; crates above it own their schemas directly.

use bevy::prelude::App;

use super::super::AmbitionRollbackApp;
// The bespoke checksum projections these registrations name. They live beside
// the central function because several domains once shared them; a projection
// used by exactly one domain should follow it here, which is a later tidy and
// not part of a relocation commit.
use super::super::pending_player_hits_checksum;

const OWNER: &str = "ambition_runtime";

/// Register everything the combat domain needs rewound.
pub(in crate::rollback) fn register(app: &mut App) {
    app.rollback_resource_cursor::<ambition_combat::slots::CombatSlotsRes>(
        OWNER,
        "resource.combat_slot_board",
    );
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
    app.rollback_component_clone_entity_set::<ambition_combat::on_hit::HitboxOnHit>(
        OWNER,
        "combat.hitbox_on_hit",
        |on_hit| on_hit.fired_victims(),
    );
    app.rollback_map_entities::<ambition_combat::on_hit::HitboxOnHit>(OWNER, "map.hitbox_on_hit");
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
    app.rollback_component_canonical::<ambition_combat::components::ActorCooldowns>(
        OWNER,
        "actor.cooldowns",
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
    app.rollback_resource_canonical::<ambition_combat::stocks::StocksMatchSettled>(
        OWNER,
        "resource.stocks_match_settled",
    );
    app.rollback_component_canonical::<ambition_combat::components::RulesetOwnsDeath>(
        OWNER,
        "actor.ruleset_owns_death",
    );
    app.rollback_component_canonical::<ambition_combat::components::ActorIntent>(
        OWNER,
        "actor.intent",
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
    app.rollback_component_clone::<ambition_combat::on_hit::PogoTarget>(
        OWNER,
        "feature.pogo_target",
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
    app.rollback_component_clone::<ambition_combat::components::SandboxSolidContributor>(
        OWNER,
        "feature.sandbox_solid_contributor",
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
