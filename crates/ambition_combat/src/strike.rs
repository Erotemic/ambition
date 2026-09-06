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

/// The largest FEEL MULTIPLIER a `DamageBox` can plausibly mean.
///
/// ⛔⛔ THIS CONSTANT EXISTS BECAUSE `f32` CANNOT TELL A MULTIPLIER FROM A
/// SPEED, AND THREE SHIPPED BLASTS GOT IT WRONG. `DamageBoxEffect::knockback` is
/// a dimensionless multiplier over the victim's feel-tuned launch — the band
/// every long-standing author sits in is 1.1 (beam), 1.3 (shockwave) and 1.6
/// (boss, exploder). The smash demo authored engine-unit LAUNCH SPEEDS into it:
/// the bomb 134.4, the mine 124.8, the steered bolt 92.0. A 134× launch kills
/// at any percent, which is how it was noticed — by a player, not by a test.
///
/// ⭐ 8.0 IS DELIBERATELY LOOSE. It is not a balance ceiling and must not become
/// one: a blast four times harder than the boss's is a design choice somebody
/// may want. What it catches is the ORDER-OF-MAGNITUDE error — a number that
/// only makes sense as pixels per second — and a tight bound here would turn a
/// units guard into a tuning argument.
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
/// ⚠ AND "WORLD-ANCHORED" IS THE WHOLE SCOPE OF THAT SENTENCE. For a swing that
/// belongs to a BODY — hurts the people in front of it and not the fighter who
/// threw it — see [`spawn_body_strike`], which is the sibling road and exists
/// because no spelling of a world-anchored box can express that.
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
            //
            // ⛔ AND THE SENTENCE ABOVE WAS NOT ENOUGH — see
            // `MAX_PLAUSIBLE_FEEL_SCALE`. Three shipped blasts read it as a
            // launch speed anyway, so the contract is now checked here, at the
            // ONE place every `DamageBox` in the workspace becomes a `Hitbox`.
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
    // ⛔ WARN RATHER THAN PANIC: this is authored CONTENT, and a ruleset that
    // ships one absurd blast should still run so somebody can see it happen and
    // say so — which is exactly how the bomb was found. A panic here would turn
    // a visible, reportable bug into a crash on somebody else's machine.
    if !plausible_feel_scale(dbox.knockback) {
        bevy::prelude::warn!(
            "a DamageBox was spawned with knockback {} — that field is a FEEL \
             MULTIPLIER (the shipped band is 1.1–1.6, ceiling {}), not an \
             engine-unit launch speed. A value this size launches at roughly \
             {}x the intended force and will kill at any percent.",
            dbox.knockback,
            MAX_PLAUSIBLE_FEEL_SCALE,
            dbox.knockback.round() as i64
        );
    }
    e.id()
}

/// Spawn an ordinary BODY strike: player-sided, anchored to `owner`, resolved by
/// the same melee path a move's own volume takes. Returns the entity.
///
/// ⛔⛔ THE SIBLING OF [`spawn_damage_box`], AND THE DIFFERENCE IS NOT COSMETIC.
/// A `DamageBox` is anchored `World`, and the resolver's table reads
/// `(HitSide::Player, HitboxAnchor::World { .. }) => None` — a player-sided world
/// box takes NO melee path and damages nobody. The only side that reaches bodies
/// from a world anchor is `Environment`, which by design consults **no
/// self-exclusion**: *"your own bomb hurts you, and you still placed it."*
/// ⇒ So a technique that wants "a swing that hurts THEM and not ME" cannot be
/// spelled as a `DamageBox` at all. It needs this: `Player` + `FollowOwner`,
/// where the owner is excluded by identity and `damage_lands_between` still
/// decides teams, factions and friendly fire.
///
/// ⚠ WHICH IS WHY IT EXISTS. The first draft of the riposte technique authored a
/// world box on `Environment` and would have cut the fighter who threw it — a
/// counter that damages its own owner — and the mistake was invisible until the
/// citation in `a_hazard_hits_bystander_and_owner_alike_where_a_neutral_box_hits_neither`
/// was actually read.
pub fn spawn_body_strike(
    commands: &mut Commands,
    owner: Entity,
    local_offset: ae::Vec2,
    facing: f32,
    frame_down: ae::Vec2,
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
                anchor: HitboxAnchor::FollowOwner { local_offset },
                half_extent,
                shape: None,
                facing,
                damage,
                knockback: HitboxKnockback::FeelScale(feel_scale),
                launch_dir: None,
                frame_down,
                reaction: None,
            },
            HitboxLifetime {
                remaining_s: lifetime_s,
            },
            HitboxHits::default(),
        ))
        .id()
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

    /// ⛔⛔ A LAUNCH SPEED IN THE FEEL-MULTIPLIER FIELD IS THE BUG THIS CATCHES,
    /// and it shipped three times before anybody noticed — as a player noticing,
    /// not as a test.
    ///
    /// Jon, 2026-09-05: *"does the projectile polygon bomb just straight up kill
    /// the other player?"* It did. `DamageBoxEffect::knockback` is dimensionless
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
