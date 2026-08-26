//! Authoritative live strike volume and lifecycle state.
//!
//! Simulation truth for damage, knockback, ownership, and per-strike hit
//! deduplication lives here. Presentation reads the combat geometry view instead
//! of depending on live strike components.

use bevy::prelude::{Commands, Component, Entity, MessageReader, Name};

use ambition_platformer2d_core as ae;

pub use ambition_vfx::HitSide;
use ambition_vfx::{Effect, EffectRequest};

/// Unit-bearing knockback payload carried by a live [`Hitbox`].
///
/// World damage boxes use a dimensionless multiplier over the victim's normal
/// reaction. Authored melee volumes use an absolute launch speed and may add
/// smash-style speed growth from the victim's accumulated damage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitboxKnockback {
    /// Dimensionless multiplier over the victim's feel-tuned launch.
    FeelScale(f32),
    /// Absolute engine-unit launch speed plus speed growth per damage point.
    ///
    /// `growth: None` means the volume did not decide and the RULESET's growth
    /// applies. `Some(0.0)` is FIXED knockback — the same launch at 0% and at
    /// 200% — and it is a real authoring choice, so it must survive the trip
    /// from the volume to [`resolved_hitbox_knockback_magnitude`] distinct from
    /// "unspecified". A plain `f32` collapsed the two.
    LaunchSpeed { base: f32, growth: Option<f32> },
}

/// One in-flight strike's damage volume. Spawned on the windup → active edge of
/// an attack (or by a `DamageBox` effect); despawned when its [`HitboxLifetime`]
/// expires. Damage resolution (`apply_hitbox_damage`) lives in the game lib.
#[derive(Component, Clone, Debug)]
pub struct Hitbox {
    /// Entity that spawned the hitbox (skip self-hits; look up the follow
    /// anchor's world position each tick).
    pub owner: Entity,
    /// Whose attack is this? Picks the target query in damage resolution.
    pub source: HitSide,
    /// `FollowOwner` re-resolves the AABB each tick from the owner's
    /// authoritative position; `World` is a fixed world-space rectangle.
    pub anchor: HitboxAnchor,
    pub half_extent: ae::Vec2,
    /// Optional non-box strike shape (rotated box, disc, convex arc), in local
    /// space (`+x` = forward) and placed at the anchor center. `None` falls back
    /// to the `half_extent` box, so existing strikes are unchanged.
    pub shape: Option<ae::VolumeShape>,
    /// Facing baked at spawn — mirrors an asymmetric `shape`. `1.0` for boxes /
    /// discs (mirror-invariant).
    pub facing: f32,
    pub damage: i32,
    /// Explicitly unit-bearing knockback. Do not collapse feel multipliers and
    /// authored engine-unit speeds back into a bare scalar.
    pub knockback: HitboxKnockback,
    /// Authored launch direction in the victim's gravity frame: `x` is lateral
    /// and mirrored away from the source; `y` is toward the feet (`+y` is
    /// gravity-down). Thus `(0, -1)` launches up and `(0, 1)` spikes down.
    /// `None` uses the standard feel diagonal at the authored speed.
    pub launch_dir: Option<ae::Vec2>,
    /// The volume's REACTION override, carried verbatim from the authored
    /// volume. `None` is an ordinary hit, which is nearly every hitbox.
    ///
    /// ⭐ ONE FIELD BECAUSE THE AUTHORED SIDE IS ONE FIELD: autolink HOLDS and
    /// windbox SHOVES, so a hitbox carrying both was never a state the game
    /// could mean. Targeting, faction and contact resolve identically for
    /// either and for an ordinary hit; only the reaction differs.
    ///
    /// ⚠ the autolink arm's ANCHOR is authored but the attacker's velocity at
    /// the pulse is not — the runtime samples it, because that is a fact about
    /// the moment and not about the move.
    pub reaction: Option<ambition_entity_catalog::VolumeReaction>,
    pub frame_down: ae::Vec2,
    /// Authored STRIKE SOUND identity (CM8): the sound THIS attack makes when it
    /// lands, carried from the volume's `hit_sfx` tag so a sword and a goblin
    /// swipe sound different even when they hit the same body. `None` = the
    /// victim's own [`HurtFeedback`] default sound. A `Copy` `u64` id, so it
    /// rides the GGRS-snapshotted strike-volume entity for free.
    pub strike_sfx: Option<ambition_sfx::SfxId>,
}

impl Hitbox {
    /// The autolink this hitbox carries, if its reaction is one.
    pub fn autolink(&self) -> Option<ambition_entity_catalog::AutolinkVolume> {
        match self.reaction {
            Some(ambition_entity_catalog::VolumeReaction::Autolink(link)) => Some(link),
            _ => None,
        }
    }

    /// The windbox this hitbox carries, if its reaction is one.
    pub fn windbox(&self) -> Option<ambition_entity_catalog::WindboxVolume> {
        match self.reaction {
            Some(ambition_entity_catalog::VolumeReaction::Windbox(wind)) => Some(wind),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum HitboxAnchor {
    /// Melee swing — the hitbox tracks the owner's `pos` each tick with a
    /// per-strike local offset baked at spawn (facing encoded in
    /// `local_offset.x`'s sign, so a flipped attacker needs no re-spawn).
    FollowOwner { local_offset: ae::Vec2 },
    #[allow(dead_code)]
    World { center: ae::Vec2 },
}

#[derive(Component, Clone, Copy, Debug)]
pub struct HitboxLifetime {
    pub remaining_s: f32,
}

/// Hit-once set: targets the hitbox already damaged this strike, so a long
/// active window can't re-hit a stationary target every frame.
///
/// `Clone` + [`MapEntities`](bevy::ecs::entity::MapEntities) because this set
/// IS sim truth under rollback: a re-simulated strike volume restored without
/// its hit-once memory re-hits every victim it already struck (the Phase-5
/// second-hit desync). The whole strike-volume entity family snapshots
/// through GGRS — see the `entity:hitbox` registration in `ambition_platformer2d_runtime`.
#[derive(Component, Default, Debug, Clone)]
pub struct HitboxHits {
    pub hit: std::collections::HashSet<Entity>,
}

impl bevy::ecs::entity::MapEntities for Hitbox {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.owner = mapper.get_mapped(self.owner);
    }
}

impl bevy::ecs::entity::MapEntities for HitboxHits {
    fn map_entities<M: bevy::ecs::entity::EntityMapper>(&mut self, mapper: &mut M) {
        self.hit = std::mem::take(&mut self.hit)
            .into_iter()
            .map(|entity| mapper.get_mapped(entity))
            .collect();
    }
}

impl Hitbox {
    /// Re-resolve this hitbox's world-space volume. Computed every tick rather
    /// than mirrored on the entity so a moving owner needs no per-frame update.
    /// Uses the authored `shape` when present (placed + facing-mirrored at the
    /// anchor center); otherwise the `half_extent` box.
    pub fn world_volume(&self, owner_pos: ae::Vec2) -> ae::CombatVolume {
        let center = match self.anchor {
            HitboxAnchor::FollowOwner { local_offset } => owner_pos + local_offset,
            HitboxAnchor::World { center } => center,
        };
        match &self.shape {
            Some(shape) => shape.place_at(center, self.facing, self.frame_down),
            None => ae::CombatVolume::aabb(ae::Aabb::new(center, self.half_extent)),
        }
    }

    /// Conservative axis-aligned bounds of [`Self::world_volume`] — for the
    /// debug overlay and any broad-phase callers.
    pub fn world_aabb(&self, owner_pos: ae::Vec2) -> ae::Aabb {
        self.world_volume(owner_pos).bounds()
    }
}

// ===================================================================
// DamageBox primitive — spawn a world-anchored Hitbox.
// ===================================================================

/// The `DamageBox` effect primitive: a world-anchored, time-limited damage
/// volume. `owner` + `source` faction are supplied at spawn, so one shape serves
/// player AOEs, boss hazards, and enemy death blasts.
pub struct DamageBox {
    pub half_extent: ae::Vec2,
    /// Optional non-box shape (a disc for an explosion, a convex blast cone).
    /// `None` = the `half_extent` box. The world-anchored counterpart to the
    /// player/enemy melee `shape`.
    pub shape: Option<ae::VolumeShape>,
    pub damage: i32,
    /// Dimensionless multiplier over the victim's standard feel-tuned launch.
    pub knockback: f32,
    /// Final lifetime in seconds — callers pass the already-clamped value.
    pub lifetime_s: f32,
    /// Optional inspector/debug name. `None` matches the call sites that spawn
    /// no `Name` (kept exact so the spawned archetype — and replay — is unchanged).
    pub name: Option<&'static str>,
}

/// Spawn a world-anchored [`DamageBox`] at `center`, owned by `owner` and tagged
/// with `source` faction. Returns the entity so callers that track the box (the
/// rotating-cross arms) can despawn it later. The single spawn point for
/// world-anchored damage boxes.
pub fn spawn_damage_box(
    commands: &mut Commands,
    owner: Entity,
    source: HitSide,
    center: ae::Vec2,
    dbox: DamageBox,
) -> Entity {
    let mut e = commands.spawn((
        Hitbox {
            strike_sfx: None,
            // A `DamageBox` effect is a hit by construction — the name is the
            // contract. A gust is authored on a move's volume, never here.
            owner,
            source,
            anchor: HitboxAnchor::World { center },
            half_extent: dbox.half_extent,
            shape: dbox.shape.clone(),
            facing: 1.0,
            damage: dbox.damage,
            // DamageBox knockback is authored as a feel multiplier (values such
            // as 1.0 or 1.6), not an engine-unit melee launch speed.
            knockback: HitboxKnockback::FeelScale(dbox.knockback),
            launch_dir: None,
            // World-anchored volumes are authored in world space (arena
            // hazards); screen-down IS their frame.
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
        },
        HitboxLifetime {
            remaining_s: dbox.lifetime_s,
        },
        HitboxHits::default(),
    ));
    if let Some(name) = dbox.name {
        e.insert(Name::new(name));
    }
    e.id()
}

/// Generic effect executor: drains [`EffectRequest`]s and spawns each
/// `DamageBox`. Pure executor — every effect carries its own geometry, so this
/// needs no actor queries. Reads in message order (unsorted) to match the
/// per-consumer behavior it replaces.
/// The set `apply_effects` runs in, so a caller can order against a NAME
/// rather than against this function.
///
///  this crate has no `Plugin` and deliberately keeps none: it is an effect
/// VOCABULARY plus one executor, and the host decides when to run it. A set is
/// the smaller thing that makes the host's decision expressible — it says WHERE
/// the executor sits without claiming when the host should install it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectExecutionSet;

pub fn apply_effects(mut commands: Commands, mut requests: MessageReader<EffectRequest>) {
    for req in requests.read() {
        match &req.effect {
            Effect::DamageBox(d) => {
                spawn_damage_box(
                    &mut commands,
                    req.owner,
                    d.faction,
                    d.center,
                    DamageBox {
                        half_extent: d.half_extent,
                        shape: None,
                        damage: d.damage,
                        knockback: d.knockback,
                        lifetime_s: d.lifetime_s,
                        name: d.name,
                    },
                );
            }
            // Materialized by the actor-construction executor beside its substrate.
            Effect::Summon(_) => {}
        }
    }
}

#[cfg(test)]
mod hitbox_shape_tests {
    use super::*;

    #[test]
    fn shaped_hitbox_resolves_its_shape_at_the_anchor() {
        // A disc FollowOwner hitbox resolves to a Circle centered at owner+offset,
        // not the half_extent box.
        let hb = Hitbox {
            strike_sfx: None,
            owner: Entity::PLACEHOLDER,
            source: HitSide::Player,
            anchor: HitboxAnchor::FollowOwner {
                local_offset: ae::Vec2::new(20.0, 0.0),
            },
            half_extent: ae::Vec2::splat(10.0),
            shape: Some(ae::VolumeShape::circle(15.0)),
            facing: 1.0,
            // Gravity-down direction the hitbox shape orients against (added to
            // `Hitbox` in e56cd830 for frame-correct spawn); the default is +Y.
            frame_down: ae::Vec2::new(0.0, 1.0),
            damage: 1,
            knockback: HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            reaction: None,
        };
        match hb.world_volume(ae::Vec2::new(100.0, 50.0)) {
            ae::CombatVolume::Circle { center, radius } => {
                assert_eq!(center, ae::Vec2::new(120.0, 50.0));
                assert_eq!(radius, 15.0);
            }
            other => panic!("expected Circle, got {other:?}"),
        }
    }

    #[test]
    fn unshaped_hitbox_falls_back_to_the_half_extent_box() {
        let hb = Hitbox {
            strike_sfx: None,
            owner: Entity::PLACEHOLDER,
            source: HitSide::Enemy,
            anchor: HitboxAnchor::World {
                center: ae::Vec2::new(5.0, 5.0),
            },
            half_extent: ae::Vec2::new(8.0, 12.0),
            shape: None,
            facing: 1.0,
            // Gravity-down direction the hitbox shape orients against (added to
            // `Hitbox` in e56cd830 for frame-correct spawn); the default is +Y.
            frame_down: ae::Vec2::new(0.0, 1.0),
            damage: 1,
            knockback: HitboxKnockback::FeelScale(0.0),
            launch_dir: None,
            reaction: None,
        };
        assert!(matches!(
            hb.world_volume(ae::Vec2::ZERO),
            ae::CombatVolume::Aabb(_)
        ));
    }
}
