//! Mirror persisted save state onto ECS-owned feature actors and switches.
//!
//! ⚠ A persisted DEATH and a CLEARED boss are not mirrored: construction builds
//! those bodies in their recorded state (`construction::PersistedFates`, read
//! when the commit is requested). What remains here is the provoked-NPC flip —
//! census row DUP-PERSISTED-FATE — and the switch projection.

use super::*;
use ambition_combat::components::{
    ActorAggression, ActorDisposition, ActorInteraction, AggressionMode, FeatureId,
};
use ambition_encounter::switches::{SwitchFeature, SwitchOn};
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

/// Install the save mirror in `ProgressionSet::SaveMirror`.
///
/// ⭐ THE SET IS NAMEABLE HERE. `ProgressionSet` is in
/// `ambition_platformer2d_shared_tangle`, which this crate already depends on, so
/// no dependency edge is added by taking the membership with the systems.
///
/// ⚠ The SCHEDULE stays the caller's: `app.sim_schedule()` is `Update` under one
/// host and the fixed-tick schedule under another, and only a composition knows
/// which.
pub fn install_save_mirror(
    app: &mut bevy::prelude::App,
    schedule: impl bevy::ecs::schedule::ScheduleLabel,
) {
    use bevy::prelude::IntoScheduleConfigs;
    app.add_systems(
        schedule,
        sync_ecs_actors_with_save.in_set(ambition_platformer2d_shared_tangle::schedule::ProgressionSet::SaveMirror),
    );
}

/// Flip an NPC the save says was provoked hostile, in place.
pub fn sync_ecs_actors_with_save(
    mut commands: Commands,
    // The prepared cast, so a provoked body can take its own CHARACTER's
    // answer instead of one matched out of its display name.
    prepared: Option<Res<ambition_characters::prepared::PreparedCharacterRegistry>>,
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    // A persisted-hostile NPC re-establishes its grudge against a stable player
    // slot on load (the original attacker entity doesn't survive a save round-trip;
    // single-player has exactly one slot to be angry at).
    players: Query<
        (Entity, Option<&ambition_characters::control::PlayerSlot>),
        With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &mut ActorDisposition,
            &mut ActorAggression,
            // The body's live repertoire, which is what the provoked brain
            // choice reads.
            Option<&ambition_characters::brain::ActionSet>,
            // Talkable actors (NPCs) carry the interaction payload + a persisted
            // `npc_<id>_hostile` provoke flag.
            Option<&ActorInteraction>,
            crate::actor_clusters::ActorClusterQueryData,
            // Is this body in a fight? A loaded save restores a body's read
            // model, and a combatant's attack state is part of it.
            // WHICH CHARACTER THIS BODY IS — gameplay identity, not the
            // sprite's. See `provoke_actor_in_place`.
            Option<&ambition_characters::actor::WornCharacter>,
        ),
        With<FeatureSimEntity>,
    >,
) {
    let data = save.data();
    // AMBITION_REVIEW(determinism): Bevy query iteration order is not a stable
    // multiplayer fallback. When the persisted attacker entity cannot be restored,
    // anchor hostility to the lowest PlayerSlot so save-load behavior is replay-safe.
    let stable_player_grudge = players
        .iter()
        .min_by_key(|(_, slot)| {
            slot.copied()
                .unwrap_or(ambition_characters::control::PlayerSlot::PRIMARY)
        })
        .map(|(entity, _)| entity);
    for (
        entity,
        mut disposition,
        mut aggression,
        repertoire,
        interaction,
        mut cq,
        worn,
    ) in &mut actors
    {
        let id = cq.as_actor_mut().identity.id.clone();
        if interaction.is_some() && data.flag(&super::super::npcs::npc_flag_id(&id)) {
            // Persisted-hostile NPC: flip it hostile IN PLACE on load (no cluster
            // swap), keeping its entity + sprite.
            aggression.mode = AggressionMode::Hostile;
            aggression.grudge = stable_player_grudge.map(ambition_combat::components::Grudge::Body);
            let mut em = cq.as_actor_mut();
            crate::features::ecs::actors::provoke_actor_in_place(
                &mut commands,
                entity,
                &mut em,
                &mut disposition,
                repertoire,
                worn.map(ambition_characters::actor::WornCharacter::id),
                prepared.as_deref(),
                false,
            );
        }
    }
}

/// Mirror persisted save switch state onto ECS switch components.
///
/// Encounter arming now reads `EncounterSwitchIndex`, which is rebuilt from
/// these ECS components.
pub fn sync_ecs_switches_from_save(
    save: Res<ambition_persistence::save::AmbitionGameSave>,
    mut switches: Query<(&FeatureId, &mut SwitchOn), With<SwitchFeature>>,
) {
    for (id, mut switch_on) in &mut switches {
        switch_on.0 = save.data().switch(id.as_str());
    }
}

#[cfg(test)]
mod switch_save_tests;
