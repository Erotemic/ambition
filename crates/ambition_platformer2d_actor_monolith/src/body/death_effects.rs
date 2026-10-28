//! Attacks and offspring produced by a body death.

use ambition_platformer2d_core as ae;
use ambition_platformer2d_shared_tangle::lifecycle::SessionSpawnScope;
use bevy::prelude::*;

/// Half-extent (px) of an `ExplodingMite`'s death blast — a wide, readable boom.
const EXPLODER_BLAST_HALF: f32 = 64.0;

/// Damage the blast deals (more than the mite's contact, so a point-blank kill
/// genuinely punishes).
pub(super) const EXPLODER_BLAST_DAMAGE: i32 = 3;

const EXPLODER_BLAST_KNOCKBACK: f32 = 1.6;

/// A brief flash — the box exists just long enough to register one hit.
const EXPLODER_BLAST_LIFETIME_S: f32 = 0.14;

/// Spawn the death blast of a volatile mite: a one-shot Enemy-faction
/// [`Hitbox`](ambition_combat::hitbox::Hitbox) centered on the corpse. Enemy faction, so
/// `apply_hitbox_damage` routes it at the *player* (not other enemies — the blast
/// doesn't chain), and the player's shield/parry can still negate it. `owner` is
/// the dying mite (moot for ignore-self, since the blast never hits its own side).
///
/// Calls the executor DIRECTLY (not via `Effect::DamageBox`) on purpose: this
/// runs in the hit-resolution stage, AFTER `apply_effects`, so a fire-and-forget
/// `EffectRequest` would land a frame late. Spawning the box here keeps it
/// same-frame (and replay-identical).
pub(super) fn spawn_death_explosion(
    commands: &mut Commands,
    session_scope: SessionSpawnScope,
    owner: Entity,
    pos: ae::Vec2,
) {
    let entity = ambition_combat::strike::spawn_damage_box(
        commands,
        owner,
        ambition_vfx::HitSide::Enemy,
        pos,
        ambition_combat::strike::DamageBox {
            half_extent: ae::Vec2::splat(EXPLODER_BLAST_HALF),
            shape: None,
            damage: EXPLODER_BLAST_DAMAGE,
            knockback: EXPLODER_BLAST_KNOCKBACK,
            lifetime_s: EXPLODER_BLAST_LIFETIME_S,
            name: Some("Exploding mite blast"),
        },
    );
    let mut entity_commands = commands.entity(entity);
    session_scope.apply_to(&mut entity_commands);
}

/// Lateral offset (px) each split offspring spawns from the parent's corpse.
const SPLIT_OFFSET_X: f32 = 30.0;

/// Half-size of a split offspring.
const SPLIT_OFFSPRING_HALF: ae::Vec2 = ae::Vec2::new(15.0, 20.0);

/// A `DividingMite` splits into two offspring on death — one to each side —
/// through the runtime-minion spawner. The children do not divide, so the split
/// is exactly one level deep: no runaway recursion, just "kill the slow parent,
/// then handle two quick children."
///
/// THE CHILDREN ARE PUPPY SLUGS. Skitters are Puppy Slug."*
///
/// the repo had already answered this once: the proving grounds' placement
/// literally named `pg_skitter` is cast as `npc_puppy_slug` today. The split was
/// the site that had not caught up.
///
/// an engine module still names an Ambition creature, and that is AC5.4's
/// remainder rather than this line's. What a character splits into is a CONTENT
/// fact and belongs on the parent's definition; casting it correctly first is
/// what makes moving it a move rather than a decision.
pub(super) fn spawn_split_offspring(
    commands: &mut Commands,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    authored_sheets: &ambition_sprite_sheet::character::sheets::AuthoredSheets,
    // The offspring are a CHARACTER when one is registered for them — the same
    // resolution every other spawn road does.
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
    session_scope: SessionSpawnScope,
    parent_id: &str,
    pos: ae::Vec2,
    // AC5.4: WHAT it splits into, from the parent character's own `divides_into`.
    offspring: &str,
) {
    let cast = match ambition_platformer2d_actor_spawn::summon_cast(prepared, offspring) {
        Ok(cast) => cast,
        Err(refusal) => {
            bevy::log::error!("{refusal}");
            return;
        }
    };
    for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        ambition_platformer2d_actor_spawn::spawn_runtime_minion(
            commands,
            character_catalog,
            authored_sheets,
            cast,
            session_scope,
            format!("{parent_id}:split{i}"),
            pos + ae::Vec2::new(side * SPLIT_OFFSET_X, 0.0),
            SPLIT_OFFSPRING_HALF,
            offspring,
            format!("{parent_id}:split"),
            ambition_characters::actor::ActorFaction::Enemy,
            ambition_combat::components::ActorAggression::hostile(),
        );
    }
}
