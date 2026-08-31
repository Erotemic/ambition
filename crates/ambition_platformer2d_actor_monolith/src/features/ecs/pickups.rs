//! Player → pickup collection on the ECS feature path.

use super::*;
use ambition_combat::components::{CenteredAabb, Collected, FeatureName, PickupFeature};
use ambition_combat::events::GameplayBanner;
use ambition_combat::events::SetFlagRequested;
use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
use ambition_platformer2d_shared_tangle::sim_selection::winner_by;
use ambition_sfx::{SfxMessage, SfxWriter};

/// Bodies eligible for passive touch collection.
///
/// This includes all `PlayerEntity` bodies and actors currently driven through
/// `TemporaryControl::Player`. [`body_collects_on_touch`] performs the value
/// check because every autonomous actor also carries `TemporaryControl`.
/// Action-driven held-item pickup uses a separate control path.
pub type TouchCollectorFilter = bevy::prelude::Or<(
    bevy::prelude::With<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
    bevy::prelude::With<ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>,
)>;

/// A body collects on touch when it belongs to the player population or is
/// currently driven through possession. `PlayerEntity` remains sufficient even
/// while that body's brain is temporarily absent.
pub fn body_collects_on_touch(
    in_player_population: bool,
    control: Option<&ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>,
) -> bool {
    in_player_population
        || matches!(
            control,
            Some(
                ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl::Player { .. }
            )
        )
}

/// Attraction targets the nearest eligible touch collector.
#[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
pub struct PickupMagnet {
    /// Distance within which this pickup starts drifting toward a collector.
    pub range: f32,
    /// How fast it closes (px/s).
    pub speed: f32,
}

impl PickupMagnet {
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
/// That works right up until something spawns a pickup at RUNTIME — Sanic's scattered rings — at
/// which point the spec is long gone and the pickup's art is unrecoverable: it simulates,
/// magnetizes and credits perfectly while drawing nothing at all.
///
/// Putting the id on the ENTITY makes a pickup self-describing, so the
/// dynamic-visual pass can bind the same spinning sheet the authored pass binds
/// without needing the room spec that no longer applies.
#[derive(bevy::prelude::Component, Debug, Clone)]
pub struct PickupArt(pub String);

/// The set [`magnetize_pickups`] runs in — loot is pulled here.
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
            Option<&ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>,
            // ⭐ THE TIE-BREAK. Two collectors equidistant from a pickup is the
            // ordinary couch arrangement, and `min_by` on distance alone answered
            // it with whichever body the query happened to yield first — which is
            // archetype order, not a gameplay rule.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
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
        let Some((to_collector, dist, _)) =
            ambition_platformer2d_shared_tangle::sim_selection::winner_by(
                collectors
                    .iter()
                    .filter(|(_, is_player, control, _)| {
                        body_collects_on_touch(*is_player, *control)
                    })
                    .map(|(body, _, _, id)| {
                        let delta = body.pos - aabb.center;
                        (delta, delta.length(), id)
                    }),
                |(_, dist, _)| *dist,
                |(_, _, id)| *id,
            )
        else {
            return;
        };
        if dist > 1.0 && dist < magnet.range {
            aabb.center += to_collector.normalize() * (magnet.speed * dt).min(dist);
        }
    }
}

/// The set [`collect_ecs_pickups`] runs in — loot is claimed here.
///
/// The closing half of the pickup window. Together with [`PickupMagnetize`] it
/// names the gap a game inserts custom loot motion into: after the magnet has
/// pulled, before collection claims. Sanic's scattered-ring burst owns each
/// ring's position during exactly that gap, so the magnet cannot reclaim it and
/// collect sees it out at its arc rather than on top of the knocked-back body.
///
/// ONE member. `apply_player_heal_requests` is chained after and is a
/// CONSUMER of what collection produced, not part of claiming it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PickupCollect;

/// Collect ECS-owned pickups after the player simulation has advanced.
pub fn collect_ecs_pickups(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    collectors: Query<
        (
            Entity,
            &ambition_platformer2d_core::BodyKinematics,
            bevy::prelude::Has<ambition_platformer2d_shared_tangle::markers::PlayerEntity>,
            Option<&ambition_platformer2d_shared_tangle::temporary_control::TemporaryControl>,
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
    mut owned: Option<ResMut<ambition_items::OwnedItems>>,
    // The tie-break's authority. Read through a lookup rather than joined onto
    // the collector query so a body without one still competes on distance —
    // it just cannot win a tie, which is what `winner_by` documents.
    sim_ids: Query<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
) {
    // With a population expressed as a filter plus a value test it would no longer mean "nobody can
    // collect" — `TouchCollectorFilter` matches every autonomous actor — and a system-wide return
    // on a population guess is exactly the shape that has switched whole subsystems off in this
    // repo. The per-pickup `find` below already yields nothing when nobody qualifies.
    for (entity, name, aabb, pickup, collected) in &pickups {
        if collected.is_some() {
            continue;
        }
        // ⛔⛔ THE NEAREST OVERLAPPING COLLECTOR, NOT THE FIRST. This was
        // `collectors.iter().find(..)`, and its own comment said so — "find the
        // first overlapping collector" — which reads like a rule and is not one:
        // "first" is Bevy query order, i.e. archetype order, which a resimulated
        // tick can present differently. With two players standing on one ring
        // that decided who healed, who banked the currency, and who took the
        // flag, and it decided it unrepeatably.
        //
        // ⭐ NEAREST-CENTRE IS A RULE A PLAYER CAN SEE, and `SimId` is what makes
        // the answer the same on both peers when two bodies are equidistant. The
        // heal is still routed to that specific body via
        // `PlayerHealRequested::target`, so single-player behaviour is unchanged:
        // one candidate wins by being the only one.
        let Some((collector_entity, ..)) = winner_by(
            collectors.iter().filter(|(_, kin, is_player, control)| {
                body_collects_on_touch(*is_player, *control)
                    && aabb.aabb().strict_intersects(kin.aabb())
            }),
            |(_, kin, _, _)| kin.pos.distance_squared(aabb.center),
            |(entity, _, _, _)| sim_ids.get(*entity).ok(),
        ) else {
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

/// Give `collector` what this pickup is worth. THE grant authority for a
/// [`PickupKind`](ambition_interaction::PickupKind), for every road that hands
/// one to somebody.
///
/// it exists because a SECOND road had the payload and no way to spend
/// it. `ChestFeature::reward()` — an `Option<PickupKind>` filled by all three
/// chest authors (LDtk's `spawn_static`, the mob encounter's reward chest, and
/// the boss's `DropChest` profile) — had zero callers. Every chest in the
/// game opened, sparked, played its sound and announced *"opened X"*, and the
/// authored reward was parsed, lowered onto the live component, and never
/// granted to anybody.
///
/// Lifting the arm out is what makes "a chest's reward is a pickup" true in the code rather
/// than only in the data.
///
/// grant only — no banner, no spark, no sound. Those belong to the road
/// the reward arrived by, and a chest already has its own.
pub fn grant_pickup(
    kind: &ambition_interaction::PickupKind,
    collector: bevy::prelude::Entity,
    heals: &mut MessageWriter<crate::avatar::PlayerHealRequested>,
    wallets: &mut Query<&mut ambition_characters::actor::BodyWallet>,
    set_flag: &mut MessageWriter<SetFlagRequested>,
    mut owned: Option<&mut ambition_items::OwnedItems>,
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
                if let Some(item) = ambition_items::Item::from_dialog_id(ability_id) {
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
        // A PAYLOAD THIS CANNOT SPEND IS A LOUD FAILURE, NOT A NO-OP. `PickupKind::Custom` is
        // an opaque authored string with no reader anywhere in the engine, so reaching here means
        // somebody authored a reward that is granted to nobody — and a silent `_ => {}` is how the
        // eight shipped boss chests came to be authored `Custom("pirate_hoard")`,
        // `Custom("gnu_scroll")` and six more relics whose ids appear in `boss_profiles.ron` and in
        // NO catalog, item table or flag.
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
