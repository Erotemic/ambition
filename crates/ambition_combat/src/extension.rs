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

use ambition_combat_port::{DamageBoxPort, HeldDamageBoxPort};
use ambition_extension_host::{ExtensionAppExt, ExtensionOutbox};
use ambition_extension_sdk::phases::TECHNIQUE_EXECUTION;
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use crate::components::ActorFaction;

/// Install the request port in `technique_execution`, before the effect
/// executor.
pub fn install(app: &mut App) {
    app.install_extension_request::<DamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_damage_boxes,
    );
    app.install_extension_request::<HeldDamageBoxPort, _>(
        TECHNIQUE_EXECUTION,
        "ambition_combat",
        lower_held_damage_boxes,
    );
}

fn lower_damage_boxes(
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
fn lower_held_damage_boxes(
    mut commands: Commands,
    mut outbox: ResMut<ExtensionOutbox>,
    mut holders: Query<(Entity, &mut HeldDamageBoxes)>,
    factions: Query<(
        &ActorFaction,
        Option<&ambition_characters::control::DrivingParticipant>,
    )>,
) {
    let submitted = outbox.drain::<HeldDamageBoxPort>();
    let asked = |owner: Entity, r: &HeldDamageBoxRecord| {
        submitted.iter().any(|s| {
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
