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

/// How long a split offspring cannot be hurt, from its birth: longer than any
/// strike's live window, so a strike already live where the halves appear
/// (the next swing at the dividing body) does not end a half on the tick it
/// is born.
const SPLIT_GRACE_S: f32 = 0.45;

/// How hard a death blast shakes the view: a little under a bomb's 9 px, as
/// its blast is a little smaller.
const DEATH_BLAST_SHAKE_PX: f32 = 7.0;

/// The death rows that have played to their end lead into what their bodies
/// become: a volatile body's blast, a dividing body's two halves. They come at
/// the end of the row and not at the kill, because the row draws the lead-in
/// (the fuse burning down, the lobes pulling apart), and a blast before its
/// fuse is a blast with no warning.
///
/// Only a death that ran the world's death consequences is due a finale
/// (`DeathBeat::finale_due`): a ruleset's KO and a fall out of the world are
/// not. Runs after `advance_death_beats` in the same tick, so the finale and a
/// `Gone` body leaving the view land on one tick.
pub fn run_death_row_finales(
    sprite_content: Res<ambition_sprite_sheet::SpriteContent>,
    catalogs: super::damage::FeatureHitCatalogs,
    mut bodies: Query<(
        Entity,
        &mut ambition_combat::components::DeathBeat,
        &ambition_combat::CombatCapabilities,
        &ae::BodyKinematics,
        &ambition_combat::components::ActorIdentity,
    )>,
    mut writers: super::damage::FeatureHitWriters,
    // The blast's burst, drawn with its own paired sound. `Option`: a
    // composition with no presentation draws no burst.
    mut bursts: Option<MessageWriter<ambition_vfx::FxRequest>>,
) {
    for (entity, mut beat, caps, kin, identity) in &mut bodies {
        if !beat.finale_due || !beat.row_ended() {
            continue;
        }
        beat.finale_due = false;
        let scope = writers.spawn_scope_from(entity);
        if caps.explodes_on_death {
            detonate_corpse(&mut writers, bursts.as_mut(), entity, kin.pos, scope.room());
        }
        if let (Some(offspring), Some(generation)) = (caps.divides_into.as_deref(), catalogs.generation.as_deref()) {
            spawn_split_offspring(
                &sprite_content,
                &mut writers.commands,
                &catalogs.characters,
                &generation.sheets,
                catalogs.prepared.get(),
                scope,
                &identity.id,
                kin.pos,
                kin.size(),
                kin.facing,
                offspring,
            );
        }
    }
}

/// The blast of a volatile body at its corpse: the hitbox, the burst drawn as
/// big as the area it hurts (the way a bomb's blast is: its 80 px blast draws
/// a 2.2 burst, and one small burst over a 128 px blast read as no explosion
/// at all) with the burst's own sound, and a shake.
fn detonate_corpse(
    writers: &mut super::damage::FeatureHitWriters,
    bursts: Option<&mut MessageWriter<ambition_vfx::FxRequest>>,
    owner: Entity,
    pos: ae::Vec2,
    room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
) {
    spawn_death_explosion(&mut writers.damage_boxes, owner, pos);
    if let Some(bursts) = bursts {
        let mut burst = ambition_vfx::FxRequest::classic(room, pos).with_scale(1.75);
        if let Some(source) = writers.source_of(Some(owner)) {
            burst = burst.from_source(source);
        }
        bursts.write(burst);
    }
    let mut room_vfx = writers.vfx.for_room(room);
    for (fx, scale) in [(ambition_vfx::fx::ids::SMOKE_BURST, 1.45), (ambition_vfx::fx::ids::SHOCKWAVE, 1.3)] {
        room_vfx.write(ambition_vfx::vfx::VfxMessage::Effect { pos, fx, scale, pose: ambition_vfx::FxPose::UPRIGHT });
    }
    if let Some(shake) = writers.shake.as_mut() {
        shake.write(ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest {
            amplitude_px: DEATH_BLAST_SHAKE_PX,
            room,
        });
    }
}

/// A dividing body splits into two offspring at the end of its death row, one
/// where each half of it stood: a quarter of its width out from its middle,
/// the front half on its front, each half facing away from the other, as its
/// death row leaves them. Through the runtime-minion spawner. What it splits
/// into is the parent's own `divides_into` (a Dividing Mite into its two
/// halves). The offspring do not divide, so the split is exactly one level
/// deep: kill the slow parent, then handle two quick children.
///
/// Each child is born in its post-hit grace ([`SPLIT_GRACE_S`]).
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
    parent_size: ae::Vec2,
    parent_facing: f32,
    // AC5.4: WHAT it splits into, from the parent character's own `divides_into`.
    offspring: &str,
) {
    let forward = if parent_facing < 0.0 { -1.0 } else { 1.0 };
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
            pos + ae::Vec2::new(side * forward * parent_size.x * 0.25, 0.0),
            // Half the parent's box, so a half stands on the parent's feet
            // until its own sheet sizes it.
            ae::Vec2::new(parent_size.x * 0.25, parent_size.y * 0.5),
            offspring,
            format!("{parent_id}:split"),
            ambition_characters::actor::ActorFaction::Enemy,
            ambition_combat::components::ActorAggression::hostile(),
        );
        // Queued after the spawn's own commands, so it finds the built body.
        commands.entity(child).queue(move |mut child: EntityWorldMut| {
            if let Some(mut combat) = child.get_mut::<ambition_characters::actor::BodyCombat>() {
                combat.damage_invuln_timer = combat.damage_invuln_timer.max(SPLIT_GRACE_S);
            }
            if let Some(mut kin) = child.get_mut::<ae::BodyKinematics>() {
                kin.facing = side * forward;
            }
        });
    }
}
