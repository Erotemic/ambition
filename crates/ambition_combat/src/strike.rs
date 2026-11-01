//! Authoritative live strike volume and lifecycle state.
//!
//! Simulation truth for damage, knockback, ownership, and per-strike hit
//! deduplication lives here. Presentation reads the combat geometry view instead
//! of depending on live strike components.

use bevy::prelude::{Commands, Component, Entity, Name};

use ambition_platformer2d_core as ae;

pub use ambition_vfx::HitSide;

/// Unit-bearing knockback payload carried by a live [`Hitbox`].
///
/// A strike can use a multiplier over the victim's normal reaction or an
/// absolute launch speed. A launch speed can grow with accumulated damage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitboxKnockback {
    /// A feel multiplier with an explicit side in the victim's frame.
    DirectedFeelScale { scale: f32, dir: f32 },
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
/// expires. Damage resolution runs in [`crate::hitbox::apply_hitbox_damage`].
#[derive(Component, Clone, Debug)]
#[require(FixedStrikeFeaturePulse)]
pub struct Hitbox {
    /// Entity that spawned the hitbox (skip self-hits; look up the follow
    /// anchor's world position each tick).
    pub owner: Entity,
    /// Source policy: Environment permits self-hits and ignores factions.
    /// Neutral does not damage bodies. Other sources use the common relation.
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
    /// The authored launch direction, with the axes it is written on
    /// ([`HitboxLaunch`]). The damage resolver lowers it to world space when
    /// the volume lands ([`Self::launch_world`]), with this volume's own
    /// `facing` and `frame_down`: the facing and the frame of the body that
    /// threw it. `None` uses the standard feel diagonal at the authored speed.
    pub launch_dir: Option<HitboxLaunch>,
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
    /// How this strike meets a raised guard, from the volume (S0.3).
    pub guard: ambition_entity_catalog::VolumeGuard,
    pub frame_down: ae::Vec2,
    /// Authored STRIKE SOUND identity (CM8): the sound THIS attack makes when it
    /// lands, carried from the volume's `hit_sfx` tag so a sword and a goblin
    /// swipe sound different even when they hit the same body. `None` = the
    /// victim's own [`HurtFeedback`] default sound. A `Copy` `u64` id, so it
    /// rides the GGRS-snapshotted strike-volume entity for free.
    pub strike_sfx: Option<ambition_sfx::SfxId>,
}

/// An authored launch direction and the axes it is written on.
///
/// Each is lowered ONCE, by the damage resolver, to a world direction
/// (`ae::hit_response::WorldLaunchDir`). The victim's reaction gets the world
/// direction and does not ask which of these it was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitboxLaunch {
    /// A direction on world axes, independent of either body's frame or facing.
    World(ae::Vec2),
    /// On the owner's own axes, as a move volume authors it
    /// (`HitVolume::launch_dir`): `+x` is the way the owner faced when it
    /// threw the strike (`Hitbox::facing`), `+y` is toward its feet
    /// (`Hitbox::frame_down`). `(0, -1)` launches up and `(0, 1)` spikes.
    /// A body the volume catches BEHIND its owner is launched the way the
    /// owner faces, as the volume says, and not back the other way.
    OwnerLocal(ae::Vec2),
    /// `x` is away from the strike's source, `y` is toward
    /// `Hitbox::frame_down`: a hit that pushes out from where it came from,
    /// whichever way its owner looks (a buck off a boss's back, a volume a
    /// conducted boss swings).
    AwayFromSource(ae::Vec2),
}

impl Hitbox {
    /// The authored launch direction of this strike in world space, for a
    /// victim at `victim_pos` struck from `source_pos`. THE one lowering of a
    /// hitbox's launch: the damage resolver calls it while it has the volume.
    pub fn launch_world(
        &self,
        source_pos: ae::Vec2,
        victim_pos: ae::Vec2,
    ) -> Option<ae::hit_response::WorldLaunchDir> {
        Some(match self.launch_dir? {
            HitboxLaunch::World(direction) => ae::hit_response::WorldLaunchDir::from_world(direction),
            HitboxLaunch::OwnerLocal(local) => {
                ae::hit_response::WorldLaunchDir::from_source_local(local, self.facing, self.frame_down)
            }
            HitboxLaunch::AwayFromSource(local) => ae::hit_response::WorldLaunchDir::away_from_source(
                local,
                source_pos,
                victim_pos,
                self.frame_down,
                self.facing,
            ),
        })
    }

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

    /// Can a guard meet this strike at all (block or parry)? Not a windbox,
    /// and not an unblockable strike.
    pub fn meets_guard(&self) -> bool {
        self.windbox().is_none() && self.guard != ambition_entity_catalog::VolumeGuard::Unblockable
    }

    /// The pulse's guard policy, lowered from the volume's.
    pub fn hit_guard(&self) -> ae::hit_response::HitGuard {
        match self.guard {
            ambition_entity_catalog::VolumeGuard::Blockable { extra_shield_damage } => {
                ae::hit_response::HitGuard::Blockable { extra_shield_damage }
            }
            ambition_entity_catalog::VolumeGuard::Unblockable => ae::hit_response::HitGuard::Unblockable,
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

/// This strike volume is DRAWN BY ITS OWNER: the owner's own pose shows the
/// attack — a god's noodles are its sting, its lash row is its lash — so the
/// renderer's stand-in for an unauthored attack must not draw over it.
///
/// A fact about the volume, not the character: the same body can swing one
/// volume its art shows and throw another it does not.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DepictedByOwner;

/// This strike volume is a RAY: the line from its anchor along `dir`,
/// `length` long (unfolded: the portal pairs it goes through cut the volume
/// as they cut any), `width` wide. Its look is the presentation's, by name:
/// it is drawn from this geometry, so the renderer's stand-in for an
/// unauthored attack does not draw it.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct RayStrike {
    pub dir: ae::Vec2,
    pub length: f32,
    pub width: f32,
    pub look: std::sync::Arc<str>,
}

impl RayStrike {
    /// The volume of the ray about its start (the hitbox anchor): the band
    /// from the start along `dir`, `width` across.
    pub fn shape(&self) -> ae::VolumeShape {
        let across = self.dir.perp() * (self.width * 0.5);
        let end = self.dir * self.length.max(1.0);
        ae::VolumeShape::Convex { points: vec![across, end + across, end - across, -across] }
    }
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
    pub hit: std::collections::BTreeSet<Entity>,
}

/// A fixed strike publishes one feature pulse. Body contacts have their own ledger.
#[derive(Component, Default, Clone, Copy, Debug)]
pub struct FixedStrikeFeaturePulse {
    pub(crate) emitted: bool,
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
#[derive(Clone)]
pub struct DamageBox {
    pub half_extent: ae::Vec2,
    /// Optional non-box shape (a disc for an explosion, a convex blast cone).
    /// `None` = the `half_extent` box. The world-anchored counterpart to the
    /// player/enemy melee `shape`.
    pub shape: Option<ae::VolumeShape>,
    pub damage: i32,
    /// Launch strength and optional direction.
    pub knockback: HitboxKnockback,
    /// Final lifetime in seconds — callers pass the already-clamped value.
    pub lifetime_s: f32,
    /// Optional inspector/debug name. `None` matches the call sites that spawn
    /// no `Name` (kept exact so the spawned archetype — and replay — is unchanged).
    pub name: Option<&'static str>,
}

/// Upper bound for a plausible feel multiplier. Absolute speeds use `LaunchSpeed`.
pub const MAX_PLAUSIBLE_FEEL_SCALE: f32 = 8.0;

/// Does this `DamageBox` knockback read as a feel multiplier rather than as a
/// launch speed somebody put in the wrong field?
///
/// ⚠ A HEURISTIC, AND HONEST ABOUT IT. Nothing can prove intent from one `f32`;
/// this only says the value is in the range the field's contract describes.
pub fn plausible_feel_scale(knockback: f32) -> bool {
    knockback.is_finite() && (0.0..=MAX_PLAUSIBLE_FEEL_SCALE).contains(&knockback)
}

/// Spawn a world-anchored [`DamageBox`] at `center`, owned by `owner` and tagged
/// with `source` faction. Returns the entity so callers that track the box (the
/// rotating-cross arms) can despawn it later. The single spawn point for
/// world-anchored damage boxes.
///
/// This volume keeps its world position. Use [`spawn_body_strike`] for a
/// volume that follows its owner.
pub(crate) fn spawn_damage_box(
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
            shape: dbox.shape,
            facing: 1.0,
            damage: dbox.damage,
            knockback: dbox.knockback,
            launch_dir: None,
            // World-anchored volumes are authored in world space (arena
            // hazards); screen-down IS their frame.
            frame_down: ae::Vec2::new(0.0, 1.0),
            reaction: None,
            guard: Default::default(),
        },
        HitboxLifetime {
            remaining_s: dbox.lifetime_s,
        },
        HitboxHits::default(),
    ));
    if let Some(name) = dbox.name {
        e.insert(Name::new(name));
    }
    // ⛔ WARN RATHER THAN PANIC: this is authored CONTENT, and a ruleset that
    // ships one absurd blast should still run so somebody can see it happen and
    // say so — which is exactly how the bomb was found. A panic here would turn
    // a visible, reportable bug into a crash on somebody else's machine.
    let feel_scale = match dbox.knockback {
        HitboxKnockback::FeelScale(scale) | HitboxKnockback::DirectedFeelScale { scale, .. } => Some(scale),
        HitboxKnockback::LaunchSpeed { .. } => None,
    };
    if let Some(scale) = feel_scale.filter(|scale| !plausible_feel_scale(*scale)) {
        bevy::prelude::warn!("damage-box feel multiplier {scale} exceeds the plausible range; use LaunchSpeed for an absolute speed");
    }
    e.id()
}

/// Spawn a strike that follows its owner. The common body resolver supplies
/// damage permission. Offsets and half extents use the owner's frame; convert
/// them at spawn so each caller uses the same placement rule.
pub fn spawn_body_strike(
    commands: &mut Commands,
    owner: Entity,
    // In the OWNER'S OWN SPACE: `+x` is toward their facing, `+y` toward
    // their feet. Rotated into the world below.
    local_offset: ae::Vec2,
    facing: f32,
    body_frame: ae::AccelerationFrame,
    // Also body-local. A 90° frame swaps width for height; an off-axis frame
    // gets `to_world_half`'s bounding over-approximation.
    half_extent: ae::Vec2,
    damage: i32,
    feel_scale: f32,
    lifetime_s: f32,
    // What THIS attack sounds like when it lands, or `None` for the victim's own
    // default hurt sound.
    //
    // ⭐⭐ A TECHNIQUE'S HIT CAN BE VOICED BY ITS AUTHOR — this was `None`,
    // hard-coded, and it made every technique-spawned strike in the game sound
    // identical. `resolve_strike_sfx` falls back to the victim's material when
    // this is `None`, so the old behaviour was not silence: it was a
    // swordfighter's counter and a brawler's ground shock heard as one event.
    strike_sfx: Option<ambition_sfx::SfxId>,
) -> Entity {
    // Same contract, same check, same reason as the world-anchored road above.
    if !plausible_feel_scale(feel_scale) {
        bevy::prelude::warn!(
            "a body strike was spawned with knockback {feel_scale} — that field              is a FEEL MULTIPLIER (the shipped band is 1.1–1.6, ceiling {}), not              an engine-unit launch speed.",
            MAX_PLAUSIBLE_FEEL_SCALE,
        );
    }
    commands
        .spawn((
            Hitbox {
                strike_sfx,
                owner,
                source: HitSide::Player,
                anchor: HitboxAnchor::FollowOwner {
                    local_offset: body_frame.to_world(local_offset),
                },
                half_extent: body_frame.to_world_half(half_extent),
                shape: None,
                facing,
                damage,
                knockback: HitboxKnockback::FeelScale(feel_scale),
                launch_dir: None,
                frame_down: body_frame.down,
                reaction: None,
                guard: Default::default(),
            },
            HitboxLifetime {
                remaining_s: lifetime_s,
            },
            HitboxHits::default(),
        ))
        .id()
}

/// Materialization boundary for damage boxes and summoned bodies.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectExecutionSet;

#[cfg(test)]
mod hitbox_shape_tests {
    use super::*;

    /// ⛔⛔ A LAUNCH SPEED IN THE FEEL-MULTIPLIER FIELD IS THE BUG THIS CATCHES,
    /// and it shipped three times before anybody noticed — as a player noticing,
    /// not as a test.
    ///
    /// Jon, 2026-09-05: *"does the projectile polygon bomb just straight up kill
    /// the other player?"* It did. `DamageBox::knockback` is dimensionless
    /// and the shipped band is 1.1–1.6; the bomb authored `blast_radius * 2.4` =
    /// 134.4, the mine 124.8, the steered bolt 92.0.
    ///
    /// ⭐ BOTH ENDS ASSERTED. A predicate that rejected everything would satisfy
    /// the second half alone while making every blast unauthorable, and one that
    /// accepted everything satisfies the first.
    #[test]
    fn a_launch_speed_in_the_feel_field_is_not_plausible() {
        // The whole shipped band, including the two the smash demo now uses.
        for ok in [0.0, 1.1, 1.3, 1.6, 1.7, 2.0, 8.0] {
            assert!(
                plausible_feel_scale(ok),
                "{ok} is inside the authored band and was rejected — a units \
                 guard that refuses real designs becomes a tuning argument"
            );
        }
        // The three that shipped wrong, and a negative.
        for bad in [92.0, 124.8, 134.4, -1.0, f32::NAN] {
            assert!(
                !plausible_feel_scale(bad),
                "{bad} passed as a feel multiplier. That is an engine-unit \
                 launch speed: it throws at roughly {bad}x the intended force \
                 and kills at any percent"
            );
        }
    }

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
            guard: Default::default(),
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
            guard: Default::default(),
        };
        assert!(matches!(
            hb.world_volume(ae::Vec2::ZERO),
            ae::CombatVolume::Aabb(_)
        ));
    }
}
