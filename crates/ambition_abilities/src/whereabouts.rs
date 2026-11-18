//! The whereabouts of what a scope's records name (P5 step 3): the supplier
//! of `ambition.world.whereabouts`.
//!
//! Each word comes from the owner of its fact, read as it stands: a live
//! entity with the identity (and its custody and room), the occurrence
//! ledger's row for an identity with none ([`AuthoredOccurrences`]), and the
//! save's death record for an authored body. The supplier decides nothing;
//! it says which of those facts holds.

use ambition_combat_port::{Standing, Whereabouts, WhereaboutsPort};
use ambition_extension_host::{BodyRecords, ExtensionAppExt, SessionRecords};
use ambition_extension_sdk::phases::ACTOR_THINK;
use ambition_extension_sdk::Value;
use ambition_platformer2d_shared_tangle::lifecycle::{
    AuthoredOccurrences, InCustodyOf, InRoomInstance, LiveRoomInstance, OccurrenceWhereabouts, RoomInstanceRoot,
};
use ambition_platformer2d_shared_tangle::sim_id::SimId;
use bevy::prelude::*;
use std::collections::BTreeSet;

/// Offer the whereabouts where a controller decides (`actor_think`): a group
/// keeps its claims between ticks, and a claim on a coconut another room now
/// holds is not a claim on a coconut that was eaten.
pub fn install(app: &mut App) {
    app.install_extension_observation::<WhereaboutsPort>(ACTOR_THINK, "ambition_abilities", whereabouts_of);
}

/// Every identity a reference field of `scope`'s records holds, and where
/// each is. A scope with no records is told nothing (an empty table).
pub fn whereabouts_of(world: &World, scope: Entity) -> Option<Whereabouts> {
    let mut named = BTreeSet::new();
    let sets = [
        world.get::<BodyRecords>(scope).map(|records| &records.0),
        world.get::<SessionRecords>(scope).map(|records| &records.0),
    ];
    for stored in sets.into_iter().flatten().flat_map(|set| set.records()) {
        for value in stored.record.values() {
            collect_refs(value, &mut named);
        }
    }
    if named.is_empty() {
        return Some(Whereabouts::default());
    }
    let here = room_of(world, scope);
    let mut live = std::collections::BTreeMap::new();
    let mut entities = world.try_query::<(Entity, &SimId)>()?;
    for (entity, sim_id) in entities.iter(world) {
        if named.contains(sim_id.as_str()) {
            live.insert(sim_id.as_str().to_owned(), entity);
        }
    }
    let ledger = world.get_resource::<AuthoredOccurrences>();
    let save = world.get_resource::<ambition_persistence::save::AmbitionGameSave>();
    let of = named
        .into_iter()
        .map(|id| {
            let standing = match live.get(&id) {
                Some(&entity) => live_standing(world, entity, here),
                // A lookup key equal to a stored identity, never a new one.
                None => absent_standing(&SimId::from_snapshot(id.clone()), ledger, save.map(|save| save.data())),
            };
            (id, standing.to_owned())
        })
        .collect();
    Some(Whereabouts { of })
}

/// The identities a value names, in any option, sequence or record.
fn collect_refs(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Ref(id) => {
            out.insert(id.clone());
        }
        Value::Opt(Some(inner)) => collect_refs(inner, out),
        Value::Seq(items) => items.iter().for_each(|item| collect_refs(item, out)),
        Value::Record(record) => record.values().iter().for_each(|item| collect_refs(item, out)),
        _ => {}
    }
}

/// A live entity: in hand, here, or away. In hand comes first: a carried
/// coconut is wherever its carrier is, and a reader that waits for it to be
/// put down asks a different question from one that goes to fetch it.
fn live_standing(world: &World, entity: Entity, here: Option<LiveRoomInstance>) -> &'static str {
    if world.get::<InCustodyOf>(entity).is_some() {
        return Standing::CARRIED;
    }
    match (room_of(world, entity), here) {
        (Some(theirs), Some(ours)) if theirs == ours => Standing::HERE,
        // With one live room an unstamped entity is in it (`LiveRooms::of`).
        (None, _) | (_, None) if live_room_count(world) <= 1 => Standing::HERE,
        _ => Standing::AWAY,
    }
}

/// An identity with no live entity, by what the world remembers of it.
fn absent_standing(
    id: &SimId,
    ledger: Option<&AuthoredOccurrences>,
    save: Option<&ambition_persistence::save_data::AmbitionGameSaveData>,
) -> &'static str {
    match ledger.and_then(|ledger| ledger.whereabouts(id)) {
        Some(OccurrenceWhereabouts::Consumed | OccurrenceWhereabouts::Spent) => return Standing::ENDED,
        // Put down in a room that is not loaded, or in the hands of a body
        // that is not: it comes back with its room or its carrier.
        Some(OccurrenceWhereabouts::Placed { .. } | OccurrenceWhereabouts::InCustody) => return Standing::DORMANT,
        None => {}
    }
    let Some(placement) = id.placement_key() else {
        // A mint with no row: nothing builds it again.
        return Standing::GONE;
    };
    let dead = save.is_some_and(|save| {
        use ambition_persistence::save_data as flags;
        save.flag(&flags::enemy_dead_flag(&placement)) || save.flag(&flags::enemy_dead_until_rest_flag(&placement))
    });
    if dead { Standing::ENDED } else { Standing::DORMANT }
}

/// The live room an entity is in, by `LiveRooms::of`'s rule without the sole
/// room: its stamp, else its own root identity.
fn room_of(world: &World, entity: Entity) -> Option<LiveRoomInstance> {
    world
        .get::<InRoomInstance>(entity)
        .map(|stamp| stamp.0)
        .or_else(|| world.get::<RoomInstanceRoot>(entity).and(world.get::<LiveRoomInstance>(entity).copied()))
}

fn live_room_count(world: &World) -> usize {
    world
        .try_query_filtered::<(), (With<RoomInstanceRoot>, With<LiveRoomInstance>)>()
        .map_or(0, |mut roots| roots.iter(world).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_extension_sdk::{Attachment, FieldDecl, FieldKind, FieldRef, RefKind, SaveEligibility, SchemaKey, StateSchema};
    use std::collections::BTreeMap;

    /// Each word of the port, from the owner of its fact: a live entity here,
    /// away and in hand; a ledger row put down and eaten; a recorded death; an
    /// authored identity with no row (its room is not loaded); a mint with none.
    #[test]
    fn each_reference_is_told_where_the_world_says_it_is() {
        let mut world = World::new();
        let ours = LiveRoomInstance::ACTIVATION;
        let theirs = ours.next();
        world.spawn((RoomInstanceRoot, ours));
        world.spawn((RoomInstanceRoot, theirs));
        let ids = [
            "placement:nut_here", "placement:nut_away", "slot:0/3", "placement:put_down", "placement:eaten",
            "placement:killed", "placement:sleeping", "slot:0/9",
        ];
        let schema = StateSchema {
            key: SchemaKey::new("test", "claims", 1),
            attachment: Attachment::Body,
            save: SaveEligibility::Transient,
            fields: vec![FieldDecl::new(1, "memory", FieldKind::Record(vec![
                FieldDecl::new(1, "goals", FieldKind::seq(FieldKind::opt(FieldKind::Ref(RefKind::Item)), 16)),
                FieldDecl::new(2, "label", FieldKind::Text { max_bytes: 128 }),
            ]))],
        };
        let mut record = schema.initial_record();
        record
            .set(FieldRef(0), Value::Record(ambition_extension_sdk::Record::from_values(vec![
                Value::Seq(ids.iter().map(|id| Value::some(Value::Ref((*id).into()))).collect()),
                Value::Text("placement:only_a_label".into()),
            ])))
            .unwrap();
        let mut records = BodyRecords::default();
        records.put(schema.key.clone(), schema.shape_digest(), record);
        let scope = world.spawn((records, InRoomInstance(ours))).id();
        world.spawn((SimId::placement("nut_here"), InRoomInstance(ours)));
        world.spawn((SimId::placement("nut_away"), InRoomInstance(theirs)));
        let carrier = world.spawn(InRoomInstance(theirs)).id();
        world.spawn((
            SimId::from_snapshot("slot:0/3".into()),
            InCustodyOf { custodian: carrier, durability: ambition_platformer2d_shared_tangle::lifecycle::CustodyDurability::Restored },
        ));
        let mut ledger = AuthoredOccurrences::default();
        ledger.adopt_rows(BTreeMap::from([
            (SimId::placement("put_down"), OccurrenceWhereabouts::Placed { room: "hub".into(), at: Vec2::ZERO }),
            (SimId::placement("eaten"), OccurrenceWhereabouts::Consumed),
        ]));
        world.insert_resource(ledger);
        let mut save = ambition_persistence::save::AmbitionGameSave::default();
        save.data_mut().set_flag(ambition_persistence::save_data::enemy_dead_flag("killed"), true);
        world.insert_resource(save);

        let told: BTreeMap<String, String> = whereabouts_of(&world, scope).expect("a table").of.into_iter().collect();
        let expect = [
            ("placement:nut_here", Standing::HERE),
            ("placement:nut_away", Standing::AWAY),
            ("slot:0/3", Standing::CARRIED),
            ("placement:put_down", Standing::DORMANT),
            ("placement:eaten", Standing::ENDED),
            ("placement:killed", Standing::ENDED),
            ("placement:sleeping", Standing::DORMANT),
            ("slot:0/9", Standing::GONE),
        ];
        for (id, standing) in expect {
            assert_eq!(told.get(id).map(String::as_str), Some(standing), "{id}: {told:?}");
        }
        assert_eq!(told.len(), expect.len(), "only what the records name");
    }
}
