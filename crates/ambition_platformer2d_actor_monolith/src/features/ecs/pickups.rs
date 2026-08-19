//! Player → pickup collection on the ECS feature path.

use super::*;
use crate::features::SetFlagRequested;
use ambition_sfx::{SfxMessage, SfxWriter};

/// **THE BODIES THAT COLLECT WHAT THEY TOUCH — one population, three systems.**
///
/// Touch-collection existed twice with two different answers to "who collects",
/// and the two were wrong in OPPOSITE directions:
///
/// * `collect_ecs_pickups` / [`magnetize_pickups`] claimed `With<PlayerEntity>`
///   — every body in the player population, which is right for a couch (four
///   seats, four collectors, each credited on its own wallet) and **excludes a
///   possessed body**, because possession moves `Brain::Player` onto an ACTOR
///   and an actor carries no `PlayerEntity`. A possessed body walked through
///   coins.
/// * [`collect_world_items`](crate::items::world_item::collect_world_items)
///   read the `ControlledSubject` resource — ONE body — which includes the
///   possessed body and **excludes couch seats 2..N**. Seat two walked through
///   mushrooms. That half was not in the report; it is the same defect seen
///   from the other side, and it is why "just unify onto `ControlledSubject`"
///   would have been a regression.
///
/// Neither existing population is the right one, so the union is spelled once
/// here and read by all three systems. It is a strict SUPERSET of both: no body
/// that collected before stops collecting.
///
/// ⚠ **the filter is only half of the answer.** `TemporaryControl` rides on
/// EVERY autonomous actor (its `Default` is `Autonomous`), so `With` it selects
/// the whole cast; [`body_collects_on_touch`] applies the value test. The filter
/// exists to keep the iteration off bodies that can never qualify, not to decide
/// anything.
///
/// ⛔ **`pickup_held_item_system` is deliberately NOT in this population**, and
/// grouping it with these was the report's third mis-reading. Grabbing a weapon
/// off the floor SPENDS AN ATTACK PRESS on one body's `ActorControl`; walking
/// into a coin spends nothing. Its `ControlledSubject` is "the body whose press
/// this is", which is a different question from "who is standing on the loot".
pub type TouchCollectorFilter = bevy::prelude::Or<(
    bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    bevy::prelude::With<crate::features::TemporaryControl>,
)>;

/// The value half of [`TouchCollectorFilter`]: a body collects on touch if it is
/// in the player population, or if a player is currently driving it through
/// possession.
///
/// ⚠ **a home avatar whose brain has been vacated keeps collecting**, because it
/// keeps `PlayerEntity`. That is deliberate and is NOT the possession case
/// leaking: possession is scoped to slot 0, and narrowing this to "carries a
/// live player brain" would silently drop any body whose brain arrives later
/// than its markers do. Widening is safe here; narrowing is what costs a game
/// its coins.
pub fn body_collects_on_touch(
    in_player_population: bool,
    control: Option<&crate::features::TemporaryControl>,
) -> bool {
    in_player_population
        || matches!(
            control,
            Some(crate::features::TemporaryControl::Player { .. })
        )
}

/// **A pickup that comes to you.** Absent by default — a pickup with no
/// `PickupMagnet` sits where it landed and is collected by touching it.
///
/// ⛔ **this used to be an engine DEFAULT and two hardcoded constants**, applied
/// to every pickup in every game. Jon, 2026-08-03: *"Maryo coins and sanic rings
/// should not be magnetic to the player."* That is not a tuning request — the
/// engine had decided for all content, and the only way for a game to say "my
/// coins stay put" was to not use pickups.
///
/// ⚠ **and the old rule named the PLAYER** (`With<PrimaryPlayer>`), which is the
/// first core value inverted: on a couch, every coin in the room flew at seat
/// one. The attraction is toward the NEAREST collector now — the same population
/// `collect_ecs_pickups` already claims with — so four players work without this
/// module knowing what a protagonist is.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct PickupMagnet {
    /// Distance within which this pickup starts drifting toward a collector.
    pub range: f32,
    /// How fast it closes (px/s).
    pub speed: f32,
}

impl PickupMagnet {
    /// The behaviour every pickup used to get: 130px reach, 340px/s.
    ///
    /// Named rather than `Default` on purpose: a game asking for the classic
    /// loot magnet should have to say so, and "default" reads like "what a
    /// pickup is", which is exactly the assumption this component removes.
    pub fn classic() -> Self {
        Self {
            range: 130.0,
            speed: 340.0,
        }
    }
}

/// A pickup that is temporarily NEITHER magnetizable NOR collectible — a piece
/// of loot mid-toss that has not settled yet (Sanic's scattered rings burst from
/// the body and must not be reeled straight back or credited the instant they
/// spawn on top of the player). Both [`magnetize_pickups`] and
/// [`collect_ecs_pickups`] skip a pickup carrying this, so a game can throw loot
/// outward and make it collectible only once it removes the lock. It is
/// authoritative sim state (it changes whether a pickup is collected this frame),
/// so it is rollback-registered beside the other pickup components.
#[derive(bevy::prelude::Component, Debug, Clone, Copy, Default)]
pub struct PickupCollectLock;

/// The animated sheet a pickup is DRAWN with, carried on the sim entity.
///
/// The authored `PickupSpec.sprite` id used to be read only at room-load, off
/// the room spec, by the pass that spawns static visuals. That works right up
/// until something spawns a pickup at RUNTIME — Sanic's scattered rings — at
/// which point the spec is long gone and the pickup's art is unrecoverable: it
/// simulates, magnetizes and credits perfectly while drawing nothing at all.
///
/// Putting the id on the ENTITY makes a pickup self-describing, so the
/// dynamic-visual pass can bind the same spinning sheet the authored pass binds
/// without needing the room spec that no longer applies.
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct PickupArt(pub String);

/// **The set [`magnetize_pickups`] runs in — loot is pulled here.**
///
/// The opening half of the pickup window; see [`PickupCollect`] for what the
/// pair is for. ONE member: this is the whole of the attract step.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupMagnetize;

/// Pull nearby uncollected pickups toward the player. Runs before
/// [`collect_ecs_pickups`], which still does the actual overlap grant — a pickup
/// pulled into overlap is collected the same frame.
pub fn magnetize_pickups(
    time: Res<ambition_time::WorldTime>,
    // The SAME population `collect_ecs_pickups` claims with, so a pickup cannot
    // be pulled toward a body that is not allowed to pick it up. Now spelled
    // ONCE, in `TouchCollectorFilter`, instead of restated per system.
    collectors: Query<
        (
            &ambition_platformer2d_core::BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&crate::features::TemporaryControl>,
        ),
        TouchCollectorFilter,
    >,
    mut pickups: Query<
        (&mut CenteredAabb, &PickupMagnet),
        (
            With<PickupFeature>,
            Without<Collected>,
            // A tossed pickup mid-flight owns its own motion (its game's toss
            // system) and must not be reeled in until it settles.
            Without<PickupCollectLock>,
        ),
    >,
) {
    let dt = time.scaled_dt;
    for (mut aabb, magnet) in &mut pickups {
        // NEAREST collector, not the first one the query yields: iteration order
        // is not a gameplay fact, and on a couch "whoever the query happened to
        // return" would be a coin flip between two players.
        let Some((to_collector, dist)) = collectors
            .iter()
            .filter(|(_, is_player, control)| body_collects_on_touch(*is_player, *control))
            .map(|(body, _, _)| {
                let delta = body.pos - aabb.center;
                (delta, delta.length())
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            return;
        };
        if dist > 1.0 && dist < magnet.range {
            aabb.center += to_collector.normalize() * (magnet.speed * dt).min(dist);
        }
    }
}

/// **The set [`collect_ecs_pickups`] runs in — loot is claimed here.**
///
/// The closing half of the pickup window. Together with [`PickupMagnetize`] it
/// names the gap a game inserts custom loot motion into: after the magnet has
/// pulled, before collection claims. Sanic's scattered-ring burst owns each
/// ring's position during exactly that gap, so the magnet cannot reclaim it and
/// collect sees it out at its arc rather than on top of the knocked-back body.
///
/// ⚠ ONE member. `apply_player_heal_requests` is chained after and is a
/// CONSUMER of what collection produced, not part of claiming it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupCollect;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    // See `TouchCollectorFilter`: the possessed body used to be missing from
    // this population entirely, so a body you had taken over walked through
    // coins while it could still pick up axes and mushrooms.
    collectors: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&crate::features::TemporaryControl>,
        ),
        TouchCollectorFilter,
    >,
    pickups: Query<
        (
            Entity,
            &FeatureName,
            &CenteredAabb,
            &PickupFeature,
            Option<&Collected>,
        ),
        // A locked (mid-toss) pickup is not collectible yet, exactly as it is not
        // magnetizable — the two guards MUST agree or a ring the magnet ignores
        // could still be collected on overlap.
        (With<FeatureSimEntity>, Without<PickupCollectLock>),
    >,
    mut heals: MessageWriter<crate::avatar::PlayerHealRequested>,
    mut wallets: Query<&mut ambition_characters::actor::BodyWallet>,
    mut sfx: SfxWriter,
    mut vfx: MessageWriter<VfxMessage>,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut owned: Option<ResMut<crate::items::OwnedItems>>,
) {
    // ⛔ the whole-system `if player.is_empty() { return; }` that used to stand
    // here is gone. With a population expressed as a filter plus a value test it
    // would no longer mean "nobody can collect" — `TouchCollectorFilter` matches
    // every autonomous actor — and a system-wide return on a population guess is
    // exactly the shape that has switched whole subsystems off in this repo. The
    // per-pickup `find` below already yields nothing when nobody qualifies.
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() {
            continue;
        }
        // Find the first overlapping collector. The heal is then routed
        // to that specific body via `PlayerHealRequested::target` so
        // a non-primary collector still actually heals themselves
        // (OVERNIGHT-TODO #17.6 bridge). Single-player behavior is
        // unchanged: the iterator has one entity, and the target ==
        // primary fallback path lands the heal on the same player.
        let Some((collector_entity, ..)) =
            collectors.iter().find(|(_, kin, is_player, control)| {
                body_collects_on_touch(*is_player, *control)
                    && aabb.aabb().strict_intersects(kin.aabb())
            })
        else {
            continue;
        };
        commands.entity(entity).insert(Collected);
        banner.show(format!("picked up {}", name.0.as_str()), 2.6);
        grant_pickup(
            &pickup.pickup.kind,
            collector_entity,
            &mut heals,
            &mut wallets,
            &mut set_flag,
            owned.as_deref_mut(),
        );
        let pos = aabb.center;
        vfx.write(VfxMessage::Burst {
            pos,
            count: 16,
            speed: 230.0,
            color: [0.84, 0.95, 1.0, 0.82],
            kind: ParticleKind::Spark,
        });
        let id = match &pickup.pickup.kind {
            ambition_interaction::PickupKind::Health { .. } => {
                ambition_sfx::ids::WORLD_HEALTH_COLLECT
            }
            ambition_interaction::PickupKind::Currency { .. } => {
                ambition_sfx::ids::WORLD_COIN_PICKUP
            }
            _ => ambition_sfx::ids::WORLD_PICKUP_GENERIC,
        };
        sfx.write(SfxMessage::Play { id, pos });
    }
}

/// **Give `collector` what this pickup is worth.** THE grant authority for a
/// [`PickupKind`](ambition_interaction::PickupKind), for every road that hands
/// one to somebody.
///
/// ⛔⛔ **it exists because a SECOND road had the payload and no way to spend
/// it.** `ChestFeature::reward()` — an `Option<PickupKind>` filled by all three
/// chest authors (LDtk's `spawn_static`, the mob encounter's reward chest, and
/// the boss's `DropChest` profile) — had **zero callers**. Every chest in the
/// game opened, sparked, played its sound and announced *"opened X"*, and the
/// authored reward was parsed, lowered onto the live component, and never
/// granted to anybody.
///
/// ⭐ **the fix was not to teach chests how to grant.** The walk-over pickup
/// already knew, in a `match` inlined in the middle of its collection system, so
/// the chest road would have had to grow a second copy that agrees with it about
/// four payload kinds forever. Lifting the arm out is what makes "a chest's
/// reward is a pickup" true in the code rather than only in the data.
///
/// ⚠ **grant only — no banner, no spark, no sound.** Those belong to the road
/// the reward arrived by, and a chest already has its own.
pub fn grant_pickup(
    kind: &ambition_interaction::PickupKind,
    collector: bevy::prelude::Entity,
    heals: &mut MessageWriter<crate::avatar::PlayerHealRequested>,
    wallets: &mut Query<&mut ambition_characters::actor::BodyWallet>,
    set_flag: &mut MessageWriter<SetFlagRequested>,
    mut owned: Option<&mut crate::items::OwnedItems>,
) {
    match kind {
        ambition_interaction::PickupKind::Health { amount } => {
            heals.write(crate::avatar::PlayerHealRequested::for_target(
                *amount, collector,
            ));
        }
        ambition_interaction::PickupKind::Currency { amount } => {
            // Credit the collecting player's wallet (HUD money meter).
            if let Ok(mut wallet) = wallets.get_mut(collector) {
                wallet.add(*amount);
            }
        }
        ambition_interaction::PickupKind::Ability { ability_id } => {
            // Grant the ability into the player's catalog so it shows up in
            // the OoT inventory and can be equipped (wired abilities) — the
            // Metroidvania "learn a power from a boss" beat.
            if let Some(owned) = owned.as_deref_mut() {
                if let Some(item) = crate::items::Item::from_dialog_id(ability_id) {
                    owned.grant(item, 1);
                }
            }
        }
        ambition_interaction::PickupKind::StoryFlag { flag } => {
            // PickupSpawn entities with `kind: "flag:<id>"` set
            // the named flag in the save layer and emit a
            // QuestAdvanceEvent::FlagSet via apply_flag_effects.
            // Mirrors the LockWall/Switch flag-setting pattern so
            // intro-v1 cartography pickups and similar narrative
            // story-flag drops just work without per-pickup wiring.
            set_flag.write(SetFlagRequested {
                id: flag.clone(),
                on: true,
            });
        }
        // ⛔⛔ **A PAYLOAD THIS CANNOT SPEND IS A LOUD FAILURE, NOT A NO-OP.**
        // `PickupKind::Custom` is an opaque authored string with no reader
        // anywhere in the engine, so reaching here means somebody authored a
        // reward that is granted to nobody — and a silent `_ => {}` is how the
        // eight shipped boss chests came to be authored `Custom("pirate_hoard")`,
        // `Custom("gnu_scroll")` and six more relics whose ids appear in
        // `boss_profiles.ron` and in NO catalog, item table or flag. Measured
        // 2026-08-19. ⚠ bounded by opens/collects, not per frame.
        ambition_interaction::PickupKind::Custom(id) => {
            bevy::log::warn!(
                target: "ambition_platformer2d::pickups",
                "pickup payload `Custom({id})` reached the grant and the engine has no \
                 vocabulary for it, so {id} was awarded to nobody: author it as a \
                 health/currency/ability/flag reward, or teach the catalog what it is",
            );
        }
    }
}

#[cfg(test)]
mod tests;
