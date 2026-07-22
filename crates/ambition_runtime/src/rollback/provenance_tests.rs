//! **Provenance-driven reconstruction: the identity pair, end to end.**
//!
//! `mint_spawned_sim_ids` stamps a dynamically-spawned entity's identity AND its
//! [`SpawnOrigin`]; `heal_projectile_owners` re-derives the one `Entity` handle
//! the projectile family carries from that origin. Those two systems used to be
//! joined by a string: the minter spelled the parent into the id, and the healer
//! read it back out with `rsplit_once('/')`.
//!
//! Nothing tested that seam before this file, which is why the swap could look
//! green against 3400 other tests while proving nothing about the mechanism it
//! replaced.
//!
//! **Two of these tests discriminate between the mechanisms; four do not.**
//! Verified by running the whole file against the pre-change implementation:
//! `minting_a_spawned_id_also_states_its_parent` and
//! `without_provenance_the_owner_cannot_be_healed` FAIL there, and are what
//! prove the swap actually happened. The rest — healing a lost handle,
//! repointing a stale one, leaving a healthy one alone — pass either way,
//! because *healing works* was true before too. They are regression protection
//! for the behaviour, not evidence about the mechanism, and it is worth being
//! explicit about which is which rather than calling the whole file poisoned.

use bevy::prelude::*;

use ambition_platformer_primitives::construction::SpawnOrigin;
use ambition_platformer_primitives::sim_id::{SimId, SimIdCounter};
use ambition_projectiles::{LiveProjectile, ProjectileOwner, ProjectileSeq};

/// A world holding one identified firer and one freshly-spawned projectile that
/// has not been through the identity pass yet.
fn world_with_a_shot() -> (World, Entity, Entity) {
    let mut world = World::new();
    let firer = world
        .spawn((SimId::placement("duel_pca"), SimIdCounter::default()))
        .id();
    let shot = world
        .spawn((LiveProjectile, ProjectileOwner(firer), ProjectileSeq(0)))
        .id();
    (world, firer, shot)
}

fn mint(world: &mut World) {
    world
        .run_system_cached(super::mint_spawned_sim_ids)
        .expect("minting runs");
}

fn heal(world: &mut World) {
    world
        .run_system_cached(super::heal_projectile_owners)
        .expect("healing runs");
}

/// Minting states the parent instead of spelling it. This is the fact the
/// healer now depends on, so it is asserted directly rather than inferred from
/// the healer working.
#[test]
fn minting_a_spawned_id_also_states_its_parent() {
    let (mut world, firer, shot) = world_with_a_shot();
    mint(&mut world);

    assert_eq!(
        world.get::<SimId>(shot),
        Some(&SimId::spawned(&SimId::placement("duel_pca"), 0)),
        "the projectile takes a spawned identity under its firer"
    );
    assert_eq!(
        world.get::<SpawnOrigin>(shot),
        Some(&SpawnOrigin::Dynamic {
            parent: Some(SimId::placement("duel_pca")),
            sequence: 0,
        }),
        "and states that firer as data"
    );
    assert_eq!(
        world.get::<SpawnOrigin>(shot).and_then(SpawnOrigin::parent),
        world.get::<SimId>(firer),
        "the stated parent IS the firer's identity"
    );
}

/// The reconstruction case: a blob-rebuilt projectile has no `Entity` handle,
/// because N3.1 forbids serializing one. It is re-derived from provenance.
#[test]
fn a_projectile_that_lost_its_owner_handle_heals_from_its_provenance() {
    let (mut world, firer, shot) = world_with_a_shot();
    mint(&mut world);

    // What a restore leaves behind: state, identity, provenance — no handle.
    world.entity_mut(shot).remove::<ProjectileOwner>();
    assert!(world.get::<ProjectileOwner>(shot).is_none());

    heal(&mut world);

    assert_eq!(
        world.get::<ProjectileOwner>(shot).map(|owner| owner.0),
        Some(firer),
        "the owner handle came back from the declared parent"
    );
}

/// Poison test for the above: strip the provenance and healing must NOT happen.
/// Without this, the previous test would still pass if healing silently fell
/// back to some other fact — including the id string it used to parse.
#[test]
fn without_provenance_the_owner_cannot_be_healed() {
    let (mut world, _firer, shot) = world_with_a_shot();
    mint(&mut world);
    world.entity_mut(shot).remove::<ProjectileOwner>();

    // The id still SPELLS the parent (`placement:duel_pca/0`). Only the
    // declared origin is gone.
    assert!(world.get::<SimId>(shot).unwrap().as_str().contains('/'));
    world.entity_mut(shot).remove::<SpawnOrigin>();

    heal(&mut world);

    assert!(
        world.get::<ProjectileOwner>(shot).is_none(),
        "healing read the declared provenance, not the id's spelling — a shot \
         whose origin is missing stays unhealed instead of being silently \
         re-parented by string surgery"
    );
}

/// The case the whole mechanism exists for: the firer itself was rebuilt, so it
/// is a DIFFERENT `Entity` carrying the same `SimId`. A stale handle must be
/// replaced, not trusted.
#[test]
fn a_stale_owner_handle_is_repointed_at_the_rebuilt_firer() {
    let (mut world, firer, shot) = world_with_a_shot();
    mint(&mut world);

    // The rollback: the firer is despawned and recreated in a new slot.
    world.entity_mut(firer).despawn();
    let rebuilt = world
        .spawn((SimId::placement("duel_pca"), SimIdCounter::default()))
        .id();
    assert_ne!(rebuilt, firer, "the rebuilt firer is a different entity");

    heal(&mut world);

    assert_eq!(
        world.get::<ProjectileOwner>(shot).map(|owner| owner.0),
        Some(rebuilt),
        "the dangling handle was repointed at the rebuilt firer by identity"
    );
}

/// A live, resolvable handle is left alone — healing is repair, not a rewrite
/// every tick.
#[test]
fn a_healthy_owner_handle_is_left_alone() {
    let (mut world, firer, shot) = world_with_a_shot();
    mint(&mut world);
    heal(&mut world);
    assert_eq!(
        world.get::<ProjectileOwner>(shot).map(|owner| owner.0),
        Some(firer)
    );
}

/// Provenance survives a snapshot round trip, which is what makes healing
/// possible at all for a blob-rebuilt entity: nothing else in the world can
/// still say where it came from.
#[test]
fn spawn_origin_round_trips_through_its_snapshot_codec() {
    use super::codec::SnapshotState;

    let cases = [
        SpawnOrigin::Authored {
            source: "hall".into(),
            instance: "pickup_a".into(),
        },
        SpawnOrigin::ProviderStaged {
            provider: "duel".into(),
            room: "hall".into(),
            instance: "duel_red".into(),
        },
        SpawnOrigin::Dynamic {
            parent: Some(SimId::placement("boss_1")),
            sequence: 9,
        },
        SpawnOrigin::Dynamic {
            parent: None,
            sequence: 0,
        },
    ];
    for origin in cases {
        let mut bytes = Vec::new();
        origin.encode(&mut bytes);
        let mut reader = super::codec::Reader::new(&bytes);
        assert_eq!(
            SpawnOrigin::decode(&mut reader).as_ref(),
            Some(&origin),
            "provenance must survive the blob it is rebuilt from"
        );
    }
}
