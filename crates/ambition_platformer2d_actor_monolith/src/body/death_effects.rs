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
/// Hit resolution follows queued materialization. Construct now so the
/// snapshot for this tick includes the death blast.
pub(super) fn spawn_death_explosion(
    damage_boxes: &mut ambition_combat::damage_box::DamageBoxSpawner,
    owner: Entity,
    pos: ae::Vec2,
) {
    damage_boxes.spawn(owner, owner, ambition_vfx::HitSide::Enemy, pos,
        ambition_combat::strike::DamageBox {
            cause: ambition_combat::events::HitSource::Melee,
            move_instance: None,
            half_extent: ae::Vec2::splat(EXPLODER_BLAST_HALF), shape: None,
            damage: EXPLODER_BLAST_DAMAGE, knockback: ambition_combat::strike::HitboxKnockback::FeelScale(EXPLODER_BLAST_KNOCKBACK),
            lifetime_s: EXPLODER_BLAST_LIFETIME_S, name: Some("Exploding mite blast"),
        });
}

/// Lateral offset (px) each split offspring spawns from the parent's corpse.
const SPLIT_OFFSET_X: f32 = 30.0;

/// Half-size of a split offspring.
const SPLIT_OFFSPRING_HALF: ae::Vec2 = ae::Vec2::new(15.0, 20.0);

/// How long a split offspring cannot be hurt, from its birth: longer than any
/// strike's live window, so the blow that split its parent, still live, does
/// not kill the halves it made.
const SPLIT_GRACE_S: f32 = 0.45;

/// A dividing body splits into two offspring on death — one to each side —
/// through the runtime-minion spawner. What it splits into is the parent's own
/// `divides_into` (a Dividing Mite into its two halves). The offspring do not
/// divide, so the split is exactly one level deep: kill the slow parent, then
/// handle two quick children.
///
/// Each child is born in its post-hit grace ([`SPLIT_GRACE_S`]): the strike
/// that killed the parent is still live where the children appear, and a new
/// body is not on its list of bodies already struck.
pub(super) fn spawn_split_offspring(sprite_content: &ambition_sprite_sheet::SpriteContent,
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
        let child = ambition_platformer2d_actor_spawn::spawn_runtime_minion(&sprite_content,
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
        // Queued after the spawn's own commands, so it finds the built body.
        commands.entity(child).queue(|mut child: EntityWorldMut| {
            if let Some(mut combat) = child.get_mut::<ambition_characters::actor::BodyCombat>() {
                combat.damage_invuln_timer = combat.damage_invuln_timer.max(SPLIT_GRACE_S);
            }
        });
    }
}
