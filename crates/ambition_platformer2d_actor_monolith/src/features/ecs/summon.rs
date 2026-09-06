//! Summoning: the one spawn road that PLANS at runtime.
//!
//! ⭐⭐ IT LIVES HERE RATHER THAN IN `spawn_actors` BECAUSE IT IS A DIFFERENT
//! LAYER, and the two sharing a file is what made the actor crate's construction
//! cycle look unbreakable. Measured 2026-09-06: `construction/mod.rs` names 15
//! things in `crate::features`, and **every one of them is defined in
//! `spawn_actors.rs`** — the spawn PRIMITIVES (`spawn_*_into`, `is_limbed_host`,
//! `giant_hand_plans`, `SpawnActorKind`, `SpawnActorRequest`). In the other
//! direction `features` names `construction` 30 times, and in `spawn_actors.rs`
//! **all five of those references were inside this one function**.
//!
//! ⇒ So the `construction ↔ features` cycle was not two authorities depending on
//! each other. It was ONE FILE holding two layers: the spawn primitives that
//! construction legitimately consumes, and one orchestration system that
//! legitimately consumes construction. Splitting them is what lets the primitives
//! move BELOW the construction domain without the cycle reappearing in a new
//! spelling — see `docs/planning/engine/actor-monolith-work-frontier.md`, F1.
//!
//! ⚠ WHAT THIS FILE DOES NOT DO IS BREAK THE CYCLE. After this move
//! `spawn_actors.rs` names no `crate::construction` at all, which is the
//! PREREQUISITE; the cycle itself closes when the primitives leave `features`.
//! Stating that here so the next reader does not read a single-layer file as a
//! finished carve.

use super::*;
// ⭐ NAMED EXPLICITLY RATHER THAN INHERITED. `spawn_actors.rs` pulls these in at
// file scope, so while this system lived there its real dependencies were
// invisible — which is part of how the two layers stayed indistinguishable. Four
// types is the whole of what the summon road needs from outside its own module.
use ambition_boss_encounter::BossCatalog;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_platformer2d_shared_tangle::lifecycle::{ActiveSessionScope, SessionSpawnScope};

/// Lib-side executor for `Effect::Summon`: the runtime-dynamic origin of the
/// three the construction planner covers.
///
/// Lives next to the spawner (not in `effects::apply_effects`) so the
/// `ambition_vfx` crate stays free of the enemy-roster substrate.
///
/// ## Why a summon is planned at all
///
/// One minion is a small plan, and running it through the same planner as a room's contents is
/// the point rather than an overhead: it is what gives a summoned body a real dynamic identity
/// (`SimId::spawned` under its summoner, taken from the summoner's own `SimIdCounter`) and an
/// explicit [`SpawnOrigin::Dynamic`] naming its parent.
///
/// A summon without a summoner `SimId` is skipped because dynamic identities
/// require an explicit parent provenance.
/// One summoner's reserved stretch of its own identity sequence.
///
/// Carries the value planning READ as well as the value it wants to write, so
/// applying the reservation can tell "nothing moved" from "someone else spent
/// these ids while this batch was in flight".
struct SummonerSequenceReservation {
    summoner: ambition_platformer2d_shared_tangle::sim_id::SimId,
    /// What the counter held when this batch planned against it.
    expected: u64,
    /// What it must hold afterwards — `expected` plus one per summon reserved.
    next: u64,
}

impl SummonerSequenceReservation {
    /// Whether this summoner's counter still holds what planning assumed.
    fn still_valid(
        &self,
        counter: Option<&ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
    ) -> bool {
        counter.is_some_and(|counter| counter.0 == self.expected)
    }
}

pub fn apply_summon_effects(
    mut commands: bevy::prelude::Commands,
    mut requests: bevy::prelude::MessageReader<ambition_vfx::EffectRequest>,
    character_catalog: bevy::prelude::Res<CharacterCatalog>,
    authored_sheets: bevy::prelude::Res<ambition_sprite_sheet::character::sheets::AuthoredSheets>,
    // `Option` like every other reader of it: a composition with no registered characters is
    // ordinary, not degraded.
    prepared_characters: Option<
        bevy::prelude::Res<ambition_characters::prepared::PreparedCharacterRegistry>,
    >,
    boss_catalog: bevy::prelude::Res<BossCatalog>,
    recipes: bevy::prelude::Res<crate::construction::ActorConstructionRegistry>,
    active_session: Option<bevy::prelude::Res<ActiveSessionScope>>,
    identities: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    // Read-only: the advance is a queued command, not a direct write.
    counters: bevy::prelude::Query<&ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
) {
    use ambition_platformer2d_shared_tangle::construction::{ConstructionPlan, ConstructionScope};

    let Some(session_scope) =
        SessionSpawnScope::for_optional_active_session(active_session.as_deref())
    else {
        requests.clear();
        return;
    };

    // Sequence numbers are RESERVED here and applied only as part of the commit.
    // `SimIdCounter` is snapshot-registered authoritative state, so advancing it
    // while assembling requests would mean a rejected batch had already consumed
    // dynamic identities that no entity was ever built for — a mutation that
    // survives into the next snapshot.
    //
    // Each reservation records the value it read, so applying it can verify the
    // counter is still what planning assumed rather than blindly overwriting.
    //  This is an ordered command, NOT rollback atomicity: the commands are
    // applied in sequence at the next flush, and nothing un-applies the earlier
    // ones if a later one finds its precondition violated. What it buys is that
    // a REFUSAL costs nothing and a violation is loud instead of silent.
    let mut reservations: std::collections::BTreeMap<
        bevy::prelude::Entity,
        SummonerSequenceReservation,
    > = std::collections::BTreeMap::new();
    let mut planned = Vec::new();
    // (rider, the mount's derived identity) for the summons that asked to be
    // ridden. Resolved to entities only after the commit flush.
    let mut board_after_commit: Vec<(
        bevy::prelude::Entity,
        ambition_platformer2d_shared_tangle::sim_id::SimId,
        ambition_vfx::SummonedRide,
    )> = Vec::new();
    for req in requests.read() {
        let ambition_vfx::Effect::Summon(s) = &req.effect else {
            continue;
        };
        let (Ok(summoner), Ok(counter)) = (identities.get(req.owner), counters.get(req.owner))
        else {
            // Loud, not silent: every body carrying a `FeatureId` is identified
            // at the head of the tick, so reaching this means the emitter is
            // outside the identity migration and its summons would have no
            // reconstructable provenance.
            bevy::log::warn!(
                target: "ambition_platformer2d::construction",
                "summon `{}` skipped: its emitter has no simulation identity to descend from",
                s.id,
            );
            continue;
        };
        // Successive summons from one summoner in a single batch each advance
        // the reserved value, so two adds never claim one identity.
        let reservation =
            reservations
                .entry(req.owner)
                .or_insert_with(|| SummonerSequenceReservation {
                    summoner: summoner.clone(),
                    expected: counter.0,
                    next: counter.0,
                });
        let taken = reservation.next;
        reservation.next += 1;
        // ⭐ THE MOUNT'S IDENTITY IS KNOWN BEFORE IT EXISTS. `SimId::spawned` is
        // what the request below derives, so a summon that asked to be ridden
        // can name its mount now and look it up after the commit — no channel,
        // no follow-up tick, and nothing that could name a DIFFERENT body.
        if let Some(ride) = s.ridden_by_summoner {
            board_after_commit.push((
                req.owner,
                ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(summoner, taken),
                ride,
            ));
        }
        planned.push(crate::construction::summoned_minion_request(
            summoner,
            taken,
            crate::construction::SummonedMinionParams {
                health: s.health,
                keeps_contact_damage: s.keeps_contact_damage,
                feature_id: s.id.clone(),
                name: s.name.clone(),
                pos: s.pos,
                half_size: s.half_size,
                character_id: s.character_id.clone(),
                encounter_id: s.encounter_id.clone(),
                faction: ambition_combat::actor_faction_from_hit_side(s.faction),
            },
        ));
    }
    if planned.is_empty() {
        return;
    }

    let scope = ConstructionScope {
        // A summon is not a content artifact. It says so explicitly rather than
        // by writing the same zero epoch a reset and a fixture also wrote, which
        // is what made the three indistinguishable to a commit boundary.
        binding: ambition_platformer2d_shared_tangle::construction::ContentBinding::RuntimeDynamic,
        room: None,
    };
    let services = crate::construction::ActorConstructionServices {
        context: {
            let context = crate::world::placements::ActorPlacementContext::new(
                &character_catalog,
                &authored_sheets,
            );
            match prepared_characters.as_deref() {
                Some(prepared) => context.with_prepared(prepared),
                None => context,
            }
        },
        boss_catalog: boss_catalog.clone(),
    };

    // Every minion's body is proved buildable before the batch is planned.
    // A summon that resolves nothing REFUSES — and after AC6 that refusal is the
    // only outcome, because there is no generic body left to settle for. It
    // belongs here rather than inside the recipe: a rejected batch has spent
    // nothing, where a recipe-time refusal is a panic with rows already built.
    if let Err(error) =
        crate::construction::preflight_planned_bodies(&planned, prepared_characters.as_deref())
    {
        bevy::log::error!(
            target: "ambition_platformer2d::construction",
            "summon batch rejected before mutation: {error}"
        );
        return;
    }
    // Planning stays out here, against the App's own registry, and stays pure:
    // a rejected batch has spent nothing and built nothing.
    let live: std::collections::BTreeSet<_> = identities.iter().cloned().collect();
    let plan = match ConstructionPlan::prepare(scope.clone(), planned, &live, &recipes) {
        Ok(plan) => plan,
        Err(error) => {
            bevy::log::error!(
                target: "ambition_platformer2d::construction",
                "summon batch rejected before mutation: {error}"
            );
            return;
        }
    };

    // The counter check, the construction, and the advance then happen inside
    // ONE exclusive-world command, so nothing can spend this summoner's
    // identities between the check and the spawn.
    //
    //  Atomicity of DECISION, not rollback. Bevy commands do not un-apply. There is
    // consequently no `max()` recovery path: by the time the advance runs, the value it is
    // replacing has just been read under the same lock.
    commands.queue(move |world: &mut bevy::prelude::World| {
        use ambition_platformer2d_shared_tangle::sim_id::SimIdCounter;

        for (owner, reservation) in &reservations {
            let counter = world.get::<SimIdCounter>(*owner);
            if !reservation.still_valid(counter) {
                bevy::log::error!(
                    target: "ambition_platformer2d::construction",
                    "summon batch refused: summoner `{}` no longer holds the counter value {} \
                     this batch reserved against (now {:?}). Nothing was built.",
                    reservation.summoner,
                    reservation.expected,
                    counter.map(|counter| counter.0),
                );
                return;
            }
        }

        {
            let mut commands = world.commands();
            let mut ctx = ambition_platformer2d_shared_tangle::construction::ConstructionExecCtx {
                commands: &mut commands,
                scope: &scope,
                session: session_scope,
                services: &services,
            };
            plan.commit(&mut ctx);
        }
        world.flush();

        // ⭐⭐ CONSTRUCTION RESERVES THE MOUNT; IT DOES NOT BOARD IT. This used
        // to weld the rider here, inside the same exclusive command that built
        // the body — the only moment both were in hand — and install the lease
        // in the same breath. That was right while a summoned mount appeared on
        // top of its summoner and wrong the moment it has to travel to them.
        //
        // ⛔ AND IT WAS NEVER THE ATOMIC TRANSACTION ITS COMMENT CLAIMED. A
        // refused board left the freshly-built mount standing in the world with
        // no `MountSlot`, which every cleanup path filters on, so nothing could
        // see it, and Jon hit it in play. Now the
        // reservation is the whole of what construction owes: it either becomes
        // a ride or becomes a `RideRefused`, and `board_reserved_mounts` owns
        // both endings.
        //
        // ⚠ THE SUMMONER'S IDENTITY IS KNOWN BEFORE THE BODY EXISTS —
        // `SimId::spawned` is what the request below derives — so the
        // reservation can name its rider without a channel or a follow-up tick.
        for (rider, mount_id, ride) in board_after_commit {
            let mount = {
                let mut q = world.query::<(
                    bevy::prelude::Entity,
                    &ambition_platformer2d_shared_tangle::sim_id::SimId,
                )>();
                q.iter(world)
                    .find(|(_, id)| **id == mount_id)
                    .map(|(entity, _)| entity)
            };
            match mount {
                Some(mount) => {
                    bevy::log::info!(
                        target: "ambition::mount",
                        "summon built, reserved for its summoner: mount={mount:?} rider={rider:?}",
                    );
                    world
                        .entity_mut(mount)
                        .insert(ambition_mount::MountReservedFor {
                            rider,
                            lease_seconds: ride.seconds,
                            board_within: ride.board_within,
                            expires_in: ride.board_deadline_s,
                        });
                }
                None => bevy::log::warn!(
                    target: "ambition_platformer2d::construction",
                    "summon `{mount_id:?}` asked to be ridden but no body with that identity \
                     exists after the commit",
                ),
            }
        }

        for (owner, reservation) in reservations {
            if let Some(mut counter) = world.get_mut::<SimIdCounter>(owner) {
                counter.0 = reservation.next;
            }
        }
    });
}
