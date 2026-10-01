//! The combat domain's extension request adapters.
//!
//! A module's damage box enters the effect executor's one road
//! (`EffectRequest` → `Effect::DamageBox`). A module's HELD damage box is an
//! entity this adapter spawns, keeps while the module asks for it, and
//! despawns ([`HeldDamageBoxes`]). Each box is owned by the body the
//! invocation ran for and is on that body's EFFECTIVE faction
//! (`targeting::effective_faction`: a driven body fights as the player). See
//! the port cards on `ambition_combat_port`.

use std::sync::Arc;

use ambition_combat_port::{DamageBoxPort, HeldDamageBoxPort, RidingHitboxPort, RidingKnockback};
use ambition_extension_host::{AdmittedExtensions, ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::{BOSS_CONDUCT, TECHNIQUE_EXECUTION, WIELDED_USE};
use ambition_extension_sdk::Phase;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::components::ActorFaction;

/// Install the request port in `technique_execution`, before the effect
/// executor.
pub fn install(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_damage_boxes::<InTechniqueExecution>,
    );
    app.install_extension_request::<HeldDamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_held_damage_boxes::<InTechniqueExecution>,
    );
}

/// Install the held damage box and the riding hitbox in `boss_conduct`.
pub fn install_for_boss_conduct(app: &mut App) {
    app.install_extension_request::<HeldDamageBoxPort, _>(
        BOSS_CONDUCT,
        "ambition_combat",
        lower_held_damage_boxes::<InBossConduct>,
    );
    app.install_extension_request::<RidingHitboxPort, _>(BOSS_CONDUCT, "ambition_combat", lower_riding_hitboxes);
}

/// Install the damage box port in `wielded_use` (a held item's use), before
/// the effect executor as in `technique_execution`.
pub fn install_for_wielded_use(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        WIELDED_USE,
        "ambition_combat",
        lower_damage_boxes::<InWieldedUse>,
    );
}

/// The phase a request adapter instance lowers for. One port offered in two
/// phases has two adapter systems, one after each phase's invocations; the
/// marker makes them two named systems, not one system registered twice.
pub struct InTechniqueExecution;
/// See [`InTechniqueExecution`].
pub struct InWieldedUse;
/// See [`InTechniqueExecution`].
pub struct InBossConduct;

/// The phase an adapter instance lowers for, for an adapter whose state spans
/// phases: the held boxes of one phase's entries are not the other's to
/// release.
pub trait LowersIn: Send + Sync + 'static {
    const PHASE: Phase;
}
impl LowersIn for InTechniqueExecution {
    const PHASE: Phase = TECHNIQUE_EXECUTION;
}
impl LowersIn for InBossConduct {
    const PHASE: Phase = BOSS_CONDUCT;
}

fn lower_damage_boxes<Phase: Send + Sync + 'static>(
    mut outbox: ResMut<ExtensionOutbox>,
    mut effects: MessageWriter<ambition_vfx::EffectRequest>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    for submitted in outbox.drain::<DamageBoxPort>() {
        // ⛔ SUBMITTED IS NOT APPLIED. A body with no faction cannot say whom
        // its box hurts, and the module may not say it either.
        let Ok((authored, driver)) = factions.get(submitted.scope) else {
            warn!(
                "extension entry {} asked for a damage box for {:?}, which has no faction; refused",
                submitted.entry, submitted.scope
            );
            continue;
        };
        let faction = crate::targeting::effective_faction(*authored, driver);
        let b = submitted.value;
        effects.write(ambition_vfx::EffectRequest {
            owner: submitted.scope,
            effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                center: ae::Vec2::from(b.center),
                faction: crate::hit_side_from_actor_faction(faction),
                half_extent: ae::Vec2::from(b.half_extent),
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
                name: None,
            }),
        });
    }
}

/// The held damage boxes of one body: the box entities the held-box adapter
/// spawned for it, by entry and slot. The module never sees these entities.
///
/// Rollback state: a rewind must restore which box each slot holds, or the
/// replay spawns a second box. The entity references are mapped
/// (`map.combat.held_damage_boxes`); the checksum reads the slots and
/// generations, not the allocator-local entities.
#[derive(Component, Clone, Debug, Default)]
pub struct HeldDamageBoxes(pub Vec<HeldDamageBoxRecord>);

#[derive(Clone, Debug)]
pub struct HeldDamageBoxRecord {
    /// `provider::module/entry`: slots are per entry.
    pub entry: Arc<str>,
    pub slot: u32,
    pub generation: u32,
    /// The box. It can be gone already: a box that ran out of lifetime is not
    /// spawned again for the same generation.
    pub entity: Entity,
}

impl bevy::ecs::entity::MapEntities for HeldDamageBoxes {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        for record in &mut self.0 {
            record.entity = mapper.get_mapped(record.entity);
        }
    }
}

impl ae::snapshot::SnapshotCursor for HeldDamageBoxes {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        ae::snapshot::put_u32(out, self.0.len() as u32);
        for record in &self.0 {
            ae::snapshot::put_str(out, &record.entry);
            ae::snapshot::put_u32(out, record.slot);
            ae::snapshot::put_u32(out, record.generation);
        }
    }
}

/// See the port card on `HeldDamageBoxPort`: release what was not asked for
/// this tick, then spawn what is asked for and not held.
fn lower_held_damage_boxes<P: LowersIn>(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    admitted: Res<AdmittedExtensions>,
    mut holders: Query<(Entity, &mut HeldDamageBoxes)>,
    mut hitboxes: Query<&mut crate::strike::Hitbox>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    let submitted = outbox.drain::<HeldDamageBoxPort>();
    // Only this phase's entries' boxes: the other phase's adapter releases
    // its own.
    let mine: Vec<&str> = admitted
        .0
        .entries
        .iter()
        .filter(|e| e.descriptor.phase == P::PHASE)
        .map(|e| e.path.as_str())
        .collect();
    let asked = |owner: Entity, r: &HeldDamageBoxRecord| {
        !mine.contains(&&*r.entry)
            || submitted.iter().any(|s| {
                s.scope == owner
                    && *s.entry == *r.entry
                    && s.value.slot == r.slot
                    && s.value.generation == r.generation
            })
    };
    for (owner, mut held) in &mut holders {
        if held.0.iter().all(|r| asked(owner, r)) {
            continue;
        }
        held.0.retain(|r| {
            let keep = asked(owner, r);
            if !keep {
                if let Ok(mut e) = commands.get_entity(r.entity) {
                    e.try_despawn();
                }
            }
            keep
        });
    }

    // New boxes, per owner, in submit order. An owner that holds nothing yet
    // gets the component with them.
    let mut new: Vec<(Entity, Vec<HeldDamageBoxRecord>)> = Vec::new();
    for s in &submitted {
        let held = holders.get(s.scope).ok().map(|(_, h)| h);
        let pending = new.iter().find(|(o, _)| *o == s.scope).map(|(_, r)| r);
        let is_held = held
            .into_iter()
            .flat_map(|h| h.0.iter())
            .chain(pending.into_iter().flatten())
            .any(|r| *r.entry == *s.entry && r.slot == s.value.slot && r.generation == s.value.generation);
        if is_held {
            // Held: it follows its submitted centre (version 2).
            let entity = held
                .into_iter()
                .flat_map(|h| h.0.iter())
                .find(|r| *r.entry == *s.entry && r.slot == s.value.slot && r.generation == s.value.generation)
                .map(|r| r.entity);
            if let Some(mut hitbox) = entity.and_then(|e| hitboxes.get_mut(e).ok()) {
                hitbox.anchor = crate::strike::HitboxAnchor::World {
                    center: ae::Vec2::from(s.value.center),
                };
            }
            continue;
        }
        // ⛔ SUBMITTED IS NOT APPLIED: a body with no faction cannot say whom
        // its box hurts.
        let Ok((authored, driver)) = factions.get(s.scope) else {
            warn!(
                "extension entry {} asked for a held damage box for {:?}, which has no faction; refused",
                s.entry, s.scope
            );
            continue;
        };
        let faction = crate::targeting::effective_faction(*authored, driver);
        let b = &s.value;
        let entity = crate::strike::spawn_damage_box(
            &mut commands,
            s.scope,
            crate::hit_side_from_actor_faction(faction),
            ae::Vec2::from(b.center),
            crate::strike::DamageBox {
                half_extent: ae::Vec2::from(b.half_extent),
                shape: None,
                damage: b.damage,
                knockback: b.knockback,
                lifetime_s: b.lifetime_s,
                name: None,
            },
        );
        let record = HeldDamageBoxRecord {
            entry: s.entry.clone(),
            slot: b.slot,
            generation: b.generation,
            entity,
        };
        match new.iter_mut().find(|(o, _)| *o == s.scope) {
            Some((_, records)) => records.push(record),
            None => new.push((s.scope, vec![record])),
        }
    }
    for (owner, records) in new {
        // A same-slot box of another generation was released above.
        if let Ok((_, mut held)) = holders.get_mut(owner) {
            held.0.extend(records);
        } else {
            commands.entity(owner).insert(HeldDamageBoxes(records));
        }
    }
}

/// Lower `ambition.combat.riding_hitbox`: a hitbox that follows its owner, on
/// the owner's effective side, shown by the owner's own art.
fn lower_riding_hitboxes(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    for s in outbox.drain::<RidingHitboxPort>() {
        // ⛔ SUBMITTED IS NOT APPLIED: a body with no faction cannot say whom
        // its hitbox hurts.
        let Ok((authored, driver)) = factions.get(s.scope) else {
            warn!(
                "extension entry {} asked for a riding hitbox for {:?}, which has no faction; refused",
                s.entry, s.scope
            );
            continue;
        };
        let h = s.value;
        let side = crate::hit_side_from_actor_faction(crate::targeting::effective_faction(*authored, driver));
        commands.spawn((
            crate::strike::Hitbox {
                strike_sfx: None,
                owner: s.scope,
                source: side,
                anchor: crate::strike::HitboxAnchor::FollowOwner {
                    local_offset: ae::Vec2::from(h.offset),
                },
                half_extent: ae::Vec2::from(h.half_extent),
                shape: h.circle_radius.map(|radius| ae::VolumeShape::Circle { radius }),
                facing: 1.0,
                damage: h.damage,
                knockback: match h.knockback {
                    RidingKnockback::FeelScale(f) => crate::strike::HitboxKnockback::FeelScale(f),
                    RidingKnockback::LaunchSpeed { base, growth } => {
                        crate::strike::HitboxKnockback::LaunchSpeed { base, growth }
                    }
                },
                launch_dir: h.launch_dir.map(ae::Vec2::from),
                frame_down: ae::Vec2::new(0.0, 1.0),
                reaction: None,
            },
            crate::strike::HitboxLifetime { remaining_s: h.lifetime_s },
            crate::strike::HitboxHits::default(),
            crate::strike::DepictedByOwner,
            Name::new(h.name),
        ));
    }
}
