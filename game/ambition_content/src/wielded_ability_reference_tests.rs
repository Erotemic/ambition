//! The NATIVE wielded abilities (shockwave, beam, volley, meteor, sentry,
//! vortex), kept as the
//! reference traces of their procedural modules (`ambition_content_modules`).
//! Test-only: the game runs the modules. `wielded_ability_parity_tests` holds
//! each module to its reference on the linked and the WASM road.


pub mod shockwave {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;

    /// Held-item id of the shockwave gauntlet.
    pub const SHOCKWAVE_ID: &str = "shockwave";

    /// Mana per use (out of 100).
    const SHOCKWAVE_MANA_COST: f32 = 25.0;

    /// Player gauntlet tuning. A boss authors its own `DamageBox` values where it
    /// emits.
    const SHOCKWAVE_HALF: ae::Vec2 = ae::Vec2::new(120.0, 52.0);
    const SHOCKWAVE_DAMAGE: i32 = 4;
    const SHOCKWAVE_LIFETIME_S: f32 = 0.18;
    const SHOCKWAVE_KNOCKBACK: f32 = 1.3;

    /// `Attack` while holding the shockwave gauntlet emits a `DamageBox` effect
    /// from the wielding body. Plain Attack only; `Shield + Attack` is the
    /// throw/drop gesture (`item_pickup::throw_held_item_system` excludes this id
    /// from throw-on-plain-Attack).
    ///
    /// Body-generic: reads the body's resolved intent ([`ActorControl`], the same
    /// frame an NPC brain writes), not raw input, for every wielder. Mana is the
    /// gate, and a body has Mana only when its experience declared the pool.
    pub fn fire_shockwave_system(
        mut wielders: Query<(
            Entity,
            &ActorControl,
            &HeldItem,
            &BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        )>,
        mut effects: MessageWriter<ambition_vfx::EffectRequest>,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for (entity, control, held, kin, resolved_frame, mut mana) in &mut wielders {
            if !control.0.melee_pressed || control.0.shield_held {
                continue;
            }
            if held.spec.id != SHOCKWAVE_ID {
                continue;
            }
            // Costs mana; with too little, no slam.
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), SHOCKWAVE_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let half_extent = resolved_frame.basis().to_world_half(SHOCKWAVE_HALF);
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                    center: kin.pos,
                    faction: ambition_vfx::HitSide::Player,
                    half_extent,
                    damage: SHOCKWAVE_DAMAGE,
                    knockback: SHOCKWAVE_KNOCKBACK,
                    lifetime_s: SHOCKWAVE_LIFETIME_S,
                    name: Some("Shockwave AOE"),
                }),
            });
            sfx.write_for(
                entity,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                },
            );
        }
    }
}


pub mod beam {
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;

    /// Held-item id of the focus-beam gauntlet.
    pub const BEAM_ID: &str = "beam";

    /// Mana per zap (out of 100). Expensive, because it is a strong,
    /// long-reach, line-clearing hit.
    const BEAM_MANA_COST: f32 = 30.0;

    /// Beam length (px) along the aim axis: how far forward it reaches.
    const BEAM_LENGTH: f32 = 300.0;
    /// Beam thickness (px) across the aim axis.
    const BEAM_WIDTH: f32 = 30.0;
    const BEAM_DAMAGE: i32 = 5;
    const BEAM_LIFETIME_S: f32 = 0.12;
    const BEAM_KNOCKBACK: f32 = 1.1;

    /// The beam's axis-aligned geometry from an aim vector. Snaps to the dominant
    /// axis and returns `(center_offset_from_player, half_extent)`, reaching
    /// `BEAM_LENGTH` forward. A zero aim uses `facing` (a forward horizontal
    /// lance), so a plain Attack still fires.
    fn beam_geometry(aim: ae::Vec2, facing: f32) -> (ae::Vec2, ae::Vec2) {
        let half_len = BEAM_LENGTH * 0.5;
        let half_wid = BEAM_WIDTH * 0.5;
        // Pick the dominant axis; default to horizontal-facing on a null aim.
        let horizontal = if aim == ae::Vec2::ZERO {
            true
        } else {
            aim.x.abs() >= aim.y.abs()
        };
        if horizontal {
            let dir = if aim.x.abs() > 0.001 {
                aim.x.signum()
            } else {
                facing.signum()
            };
            (
                ae::Vec2::new(dir * half_len, 0.0),
                ae::Vec2::new(half_len, half_wid),
            )
        } else {
            let dir = aim.y.signum();
            (
                ae::Vec2::new(0.0, dir * half_len),
                ae::Vec2::new(half_wid, half_len),
            )
        }
    }

    /// `Attack` while holding the beam gauntlet fires an aimed `Player`-faction
    /// line [`Hitbox`] along the dominant aim axis. Plain Attack only;
    /// `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
    /// throw-on-plain-Attack in `throw_held_item_system`).
    pub fn fire_beam_system(
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut players: Query<(
            Entity,
            &ActorControl,
            &HeldItem,
            &BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        )>,
        mut effects: MessageWriter<ambition_vfx::EffectRequest>,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for subject in driven.entities() {
            let Ok((entity, control, held, kin, resolved_frame, mut mana)) = players.get_mut(subject)
            else {
                continue;
            };
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != BEAM_ID {
                continue;
            }
            // Costs mana; with too little, no beam.
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), BEAM_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let frame = resolved_frame.basis();
            let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
            let (offset_local, half_local) = beam_geometry(aim, kin.facing);
            let offset = frame.to_world(offset_local);
            let half_extent = frame.to_world_half(half_local);
            effects.write(ambition_vfx::EffectRequest {
                owner: entity,
                effect: ambition_vfx::Effect::DamageBox(ambition_vfx::DamageBoxEffect {
                    center: kin.pos + offset,
                    faction: ambition_vfx::HitSide::Player,
                    half_extent,
                    damage: BEAM_DAMAGE,
                    knockback: BEAM_KNOCKBACK,
                    lifetime_s: BEAM_LIFETIME_S,
                    name: Some("Focus Beam"),
                }),
            });
            // G1: the beam is this body's ability, so it speaks in this body's voice.
            sfx.write_for(
                entity,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                },
            );
        }
    }
}


pub mod volley {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

    /// Held-item id of the volley gauntlet.
    pub const VOLLEY_ID: &str = "volley";

    /// Mana per fan (out of 100). Cheaper than the shockwave slam.
    const VOLLEY_MANA_COST: f32 = 18.0;

    /// Bolts per volley.
    const VOLLEY_SHOT_COUNT: usize = 5;
    /// Total fan spread (degrees), centered on the aim direction.
    const VOLLEY_SPREAD_DEG: f32 = 40.0;
    const VOLLEY_SPEED: f32 = 460.0;
    const VOLLEY_DAMAGE: i32 = 2;
    const VOLLEY_LIFETIME: f32 = 1.6;
    const VOLLEY_HALF: ae::Vec2 = ae::Vec2::new(8.0, 8.0);

    fn volley_origin_local_offset(aim_local: ae::Vec2, body_size: ae::Vec2) -> ae::Vec2 {
        let dir = aim_local.normalize_or_zero();
        if dir == ae::Vec2::ZERO {
            return ae::Vec2::ZERO;
        }
        let half = body_size * 0.5;
        let body_extent_along_aim = half.x * dir.x.abs() + half.y * dir.y.abs();
        dir * (body_extent_along_aim + 8.0)
    }

    fn volley_origin_world(
        player_pos: ae::Vec2,
        body_size: ae::Vec2,
        aim_local: ae::Vec2,
        frame: ae::AccelerationFrame,
    ) -> ae::Vec2 {
        player_pos + frame.to_world(volley_origin_local_offset(aim_local, body_size))
    }

    /// `Attack` while holding the volley gauntlet fires a fan of player-faction
    /// bolts along the body's aim direction (`ActorControl` aim, locomotion, or
    /// facing). Plain Attack only; `Shield + Attack` drops the item (the id is
    /// excluded from throw-on-plain-Attack in `throw_held_item_system`).
    pub fn fire_volley_system(
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut players: Query<(
            Entity,
            &ActorControl,
            &BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        )>,
        mut projectiles: MessageWriter<ProjectileSpawnRequest>,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for subject in driven.entities() {
            let Ok((entity, control, kin, resolved_frame, held, mut mana)) = players.get_mut(subject)
            else {
                continue;
            };
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != VOLLEY_ID {
                continue;
            }
            // Costs mana; with too little, no volley.
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), VOLLEY_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let frame = resolved_frame.basis();
            let aim_local = ambition_held_items::ability_aim_local(&c, kin.facing);
            let aim = frame.to_world(aim_local).normalize_or_zero();
            if aim == ae::Vec2::ZERO {
                continue;
            }
            let base_angle = aim.y.atan2(aim.x);
            let origin = volley_origin_world(kin.pos, kin.size, aim_local, frame);
            let spread = VOLLEY_SPREAD_DEG.to_radians();
            for i in 0..VOLLEY_SHOT_COUNT {
                // Centered fan: t in [-0.5, 0.5].
                let t = if VOLLEY_SHOT_COUNT > 1 {
                    i as f32 / (VOLLEY_SHOT_COUNT - 1) as f32 - 0.5
                } else {
                    0.0
                };
                let angle = base_angle + t * spread;
                let dir = ae::Vec2::new(angle.cos(), angle.sin());
                projectiles.write(ProjectileSpawnRequest::open(
                    // The firing actor owns every bolt, so a kill is credited to it
                    // (materialization stamps `ProjectileOwner` from this).
                    entity,
                    ProjectileSpawn {
                        origin,
                        dir,
                        speed: VOLLEY_SPEED,
                        damage: VOLLEY_DAMAGE,
                        max_lifetime: VOLLEY_LIFETIME,
                        half_extent: VOLLEY_HALF,
                        gravity: 0.0,
                        visual_id: String::new(),
                        // Straight volley: this ability authors no bounce.
                        bounces: 0,
                        bounce_on_world_contact: false,
                        splash_half_extent: 0.0,
                        boomerang_return_s: None,
                    },
                    ProjectileStart::StepThisTick,
                ));
            }
            sfx.write_for(
                entity,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                },
            );
        }
    }
}


pub mod meteor {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};

    /// Held-item id of the meteor gauntlet.
    pub const METEOR_ID: &str = "meteor";

    /// Mana per cast (out of 100): the most expensive wielded attack (a multi-hit
    /// zone strike).
    const METEOR_MANA_COST: f32 = 32.0;

    /// How many meteors fall per cast.
    const METEOR_COUNT: usize = 5;
    /// How far ahead of the player (along the aim's horizontal) the strike zone centers.
    const METEOR_RANGE: f32 = 190.0;
    /// Horizontal width (px) the meteors are spread across.
    const METEOR_SPREAD: f32 = 220.0;
    /// How far above the player's level each meteor spawns (it falls from here).
    const METEOR_DROP_HEIGHT: f32 = 270.0;
    /// Initial downward speed (px/s); gravity accelerates it from there.
    const METEOR_SPEED: f32 = 140.0;
    /// Downward acceleration (px/s^2) — a fast, readable fall.
    const METEOR_GRAVITY: f32 = 950.0;
    /// Damage per meteor (the area comes from count and spread, not large hits).
    const METEOR_DAMAGE: i32 = 2;
    const METEOR_LIFETIME: f32 = 2.0;
    const METEOR_HALF: ae::Vec2 = ae::Vec2::new(9.0, 9.0);

    /// The spawn origins of one cast: `METEOR_COUNT` points spread evenly across
    /// `METEOR_SPREAD`, centered `METEOR_RANGE` ahead along the aim's horizontal
    /// (default `facing`), all `METEOR_DROP_HEIGHT` above the player so they fall
    /// onto the zone. Pure, so the geometry is testable without the projectile
    /// pool.
    fn meteor_strike_origins(
        player_pos: ae::Vec2,
        aim_local: ae::Vec2,
        facing: f32,
        gravity_dir: ae::Vec2,
    ) -> [ae::Vec2; METEOR_COUNT] {
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let dir_x = if aim_local.x.abs() > 0.001 {
            aim_local.x.signum()
        } else {
            facing.signum()
        };
        let zone = player_pos + frame.to_world(ae::Vec2::new(dir_x * METEOR_RANGE, 0.0));
        let spawn_center = zone + frame.to_world(ae::Vec2::new(0.0, -METEOR_DROP_HEIGHT));
        let mut origins = [ae::Vec2::ZERO; METEOR_COUNT];
        for (i, slot) in origins.iter_mut().enumerate() {
            // Spread evenly across [-0.5, 0.5] * SPREAD along local side.
            let frac = (i as f32) / ((METEOR_COUNT - 1) as f32) - 0.5;
            *slot = spawn_center + frame.to_world(ae::Vec2::new(frac * METEOR_SPREAD, 0.0));
        }
        origins
    }

    /// `Attack` while holding the meteor gauntlet drops [`METEOR_COUNT`] falling
    /// `Player`-faction projectiles onto the zone ahead. Plain Attack only;
    /// `Shield + Attack` drops the item (the id is `UseSystem`).
    pub fn fire_meteor_system(
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut players: Query<(
            Entity,
            &ActorControl,
            &BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
        )>,
        mut projectiles: MessageWriter<ProjectileSpawnRequest>,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for subject in driven.entities() {
            let Ok((entity, control, kin, resolved_frame, held, mut mana)) = players.get_mut(subject)
            else {
                continue;
            };
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != METEOR_ID {
                continue;
            }
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), METEOR_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let gravity_dir = resolved_frame.down();
            let aim = ambition_held_items::ability_aim_local(&c, kin.facing);
            for origin in meteor_strike_origins(kin.pos, aim, kin.facing, gravity_dir) {
                projectiles.write(ProjectileSpawnRequest::open(
                    // The firing actor owns every meteor, so a kill is credited to
                    // it (materialization stamps `ProjectileOwner` from this).
                    entity,
                    ProjectileSpawn {
                        origin,
                        // Toward local down; gravity accelerates it the same way.
                        dir: gravity_dir,
                        speed: METEOR_SPEED,
                        damage: METEOR_DAMAGE,
                        max_lifetime: METEOR_LIFETIME,
                        half_extent: METEOR_HALF,
                        gravity: METEOR_GRAVITY,
                        visual_id: String::new(),
                        // Straight volley: this ability authors no bounce.
                        bounces: 0,
                        bounce_on_world_contact: false,
                        splash_half_extent: 0.0,
                        boomerang_return_s: None,
                    },
                    ProjectileStart::StepThisTick,
                ));
            }
            sfx.write_for(
                entity,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                },
            );
        }
    }
}

/// The native sentry: the deploy, the turret and its tick. The module road
/// is `ambition_content_modules::sentry` on the module-entity ports.
pub mod sentry {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_combat::components::{ActorFaction, CenteredAabb};
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
    };
    use ambition_projectiles::{ProjectileSpawn, ProjectileSpawnRequest, ProjectileStart};
    use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use ambition_platformer2d_shared_tangle::sim_selection::winner_by;

    /// Held-item id of the sentry gauntlet.
    pub const SENTRY_ID: &str = "sentry";

    /// Mana the sentry spends per deploy (out of 100).
    const SENTRY_MANA_COST: f32 = 28.0;

    /// How long (s) a deployed sentry lives.
    const SENTRY_LIFETIME_S: f32 = 5.0;
    /// Seconds between shots.
    const SENTRY_FIRE_INTERVAL_S: f32 = 0.55;
    /// Targeting range (px) — enemies beyond this are ignored.
    const SENTRY_RANGE: f32 = 480.0;
    const SENTRY_BOLT_SPEED: f32 = 430.0;
    /// `pub` so the kernel's end-to-end bolt damage test (which chains two kernel
    /// projectile systems) can name this value.
    pub const SENTRY_BOLT_DAMAGE: i32 = 2;
    const SENTRY_BOLT_LIFETIME: f32 = 1.4;
    const SENTRY_BOLT_HALF: ae::Vec2 = ae::Vec2::new(7.0, 7.0);

    /// A deployed sentry: lives at `pos`, fires when `fire_cooldown` hits zero.
    #[derive(Component, Debug, Clone, Copy)]
    pub struct Sentry {
        pub pos: ae::Vec2,
        pub remaining_s: f32,
        pub fire_cooldown: f32,
    }

    /// `Attack` while holding the sentry gauntlet drops a [`Sentry`] at the
    /// wielder's feet. Plain Attack only; `Shield + Attack` drops the item (the id
    /// is `UseSystem`).
    ///
    /// Body-generic: gated on the body's resolved intent ([`ActorControl`], the
    /// same frame an NPC brain writes) for every wielder, so a possessed or robot
    /// body deploys through this path. Mana is the gate; a body has it only when
    /// its experience declares the pool.
    pub fn fire_sentry_system(
        mut wielders: Query<(
            Entity,
            &ActorControl,
            &BodyKinematics,
            &HeldItem,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
            Option<&SessionScopedEntity>,
            // The deployer's combat side, copied onto the turret. `Option` because
            // only fixtures lack one.
            Option<&ActorFaction>,
            // The driver too: possession keeps a possessed NPC's faction as
            // `Enemy` and moves its side through the driving relationship
            // (`targeting::effective_faction`). The authored faction alone would
            // make a player's sentry shoot the player.
            Option<&ambition_characters::control::DrivingParticipant>,
            Option<&ambition_combat::targeting::MatchTeam>,
            // The deployer's identity and mint stream. Fixtures carry neither.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
        )>,
        mut commands: Commands,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for (
            wielder,
            control,
            kin,
            held,
            mut mana,
            owner,
            side,
            driver,
            team,
            deployer_id,
            mut deployer_counter,
        ) in &mut wielders
        {
            if !control.0.melee_pressed || control.0.shield_held {
                continue;
            }
            if held.spec.id != SENTRY_ID {
                continue;
            }
            // Refuse before spending (ADR 0030): a dynamic entity that cannot name
            // its spawner does not spawn, and its bolts, which mint under the
            // turret, would be skipped by `mint_spawned_sim_ids`. The refusal must
            // stay above `try_spend`, or it takes mana and deploys nothing.
            let (Some(deployer), Some(counter)) = (deployer_id, deployer_counter.as_mut()) else {
                warn!(
                    "a sentry deploy was refused: the deployer carries no SimId or no \
                     SimIdCounter, so the turret could not be named and its bolts \
                     could not mint under it"
                );
                continue;
            };
            // The turret is a dynamically spawned sim entity, and its bolts mint
            // under it.
            let id = Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
                deployer,
                counter.next(),
            ));
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), SENTRY_MANA_COST) {
                continue;
            }
            // G1: the turret inherits its summoner's presentation source, so its
            // shots sound like the placing character, even after it leaves.
            let inherited = sfx.source_of(wielder);
            deploy_sentry(
                &mut commands,
                SessionSpawnScope::new(owner.map(|owner| owner.0)),
                kin.pos,
                ambition_combat::targeting::effective_faction(
                    side.copied().unwrap_or(ActorFaction::Player),
                    driver,
                ),
                team.cloned(),
                inherited,
                id,
            );
            sfx.write_for(
                wielder,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: kin.pos,
                },
            );
        }
    }

    /// Place one turret. The only way a sentry enters the world, so tests build
    /// the same turret production does, and bolt provenance is decided once.
    ///
    /// The turret carries its deployer's combat side. A bolt's allegiance is
    /// stamped from its `ProjectileOwner` (the turret); with no `ActorFaction` on
    /// the turret, `can_hit` is false against every victim and the bolts do
    /// nothing.
    ///
    /// It also carries an identity: a bolt's `SimId` is
    /// `SimId::spawned(owner, ..)` with the turret as owner, so an unnamed turret
    /// makes bolts `mint_spawned_sim_ids` skips.
    ///
    /// Both are frozen at deploy, not looked up at fire time, because the turret
    /// outlives its deployer. The presentation source is inherited for the same
    /// reason.
    pub fn deploy_sentry(
        commands: &mut Commands,
        scope: SessionSpawnScope,
        pos: ae::Vec2,
        side: ActorFaction,
        team: Option<ambition_combat::targeting::MatchTeam>,
        inherited_presentation: Option<ambition_sfx::PresentationSourceId>,
        id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
    ) -> Entity {
        let mut turret = commands.spawn_session_scoped(
            scope,
            (
                Sentry {
                    pos,
                    remaining_s: SENTRY_LIFETIME_S,
                    // A short arm delay before the first shot.
                    fire_cooldown: 0.25,
                },
                Name::new("Sentry turret"),
                side,
            ),
        );
        if let Some(team) = team {
            turret.insert(team);
        }
        if let Some(source) = inherited_presentation {
            turret.insert(ambition_sfx::BodyPresentationSource(source));
        }
        if let Some(id) = id {
            turret.insert(id);
        }
        turret.id()
    }

    /// Tick every sentry: age it out, and when its cadence is ready, fire one
    /// player-faction bolt at the nearest Enemy-faction actor within range. Runs on
    /// `scaled_dt` (bullet-time slows the turret with everything else).
    ///
    /// The outer loop order is a gameplay decision. Two turrets firing on one
    /// tick write two `ProjectileSpawnRequest`s, and the materializer assigns the
    /// global `ProjectileSeq` in request order, which decides each bolt's
    /// identity. So turrets are ordered by their own state, then identity, like
    /// [`update_vortex_wells`]. Position and the two timers fully decide a
    /// turret's action, so two that tie emit identical requests.
    pub fn update_sentries(
        world_time: Res<ambition_time::WorldTime>,
        mut commands: Commands,
        mut sentries: Query<(Entity, &mut Sentry)>,
        // Tie-break authority for the outer loop, read separately so a turret
        // with no id still fires.
        ids: Query<&SimId>,
        enemies: Query<
            (
                &CenteredAabb,
                &ActorFaction,
                Option<&ambition_characters::actor::BodyHealth>,
                // A body out of play, or behind the playable plane, is not a target.
                (
                    bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
                    Option<&ambition_platformer2d_core::DepthPlane>,
                ),
                // Tie-break authority: two equidistant enemies are common, and
                // query order must not decide.
                Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
                // Whether a participant drives this body, which decides its
                // effective side. See the filter below.
                Option<&ambition_characters::control::DrivingParticipant>,
            ),
            With<FeatureSimEntity>,
        >,
        mut projectiles: MessageWriter<ProjectileSpawnRequest>,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        let dt = world_time.scaled_dt;
        if dt <= 0.0 {
            return;
        }
        let mut order: Vec<(ae::Vec2, f32, f32, Option<SimId>, Entity)> = sentries
            .iter()
            .map(|(entity, sentry)| {
                (
                    sentry.pos,
                    sentry.remaining_s,
                    sentry.fire_cooldown,
                    ids.get(entity).ok().cloned(),
                    entity,
                )
            })
            .collect();
        order.sort_by(|a, b| {
            a.0.x
                .total_cmp(&b.0.x)
                .then_with(|| a.0.y.total_cmp(&b.0.y))
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| a.2.total_cmp(&b.2))
                .then_with(|| a.3.cmp(&b.3))
        });
        for (_, _, _, _, entity) in order {
            let Ok((entity, mut sentry)) = sentries.get_mut(entity) else {
                continue;
            };
            sentry.remaining_s -= dt;
            if sentry.remaining_s <= 0.0 {
                if let Ok(mut ec) = commands.get_entity(entity) {
                    ec.despawn();
                }
                continue;
            }
            sentry.fire_cooldown -= dt;
            if sentry.fire_cooldown > 0.0 {
                continue;
            }
            // Nearest enemy, with a named tie-break. `min_by` on distance alone
            // keeps the first minimum, so query order would pick between
            // equidistant enemies, and that changes who dies.
            let target = winner_by(
                enemies
                    .iter()
                    // A dead enemy is an intangible corpse; skip it.
                    // Use the effective faction, not the authored one: a possessed
                    // NPC keeps `ActorFaction::Enemy` and fights as a Player
                    // through its driver. Same answer as the strike resolver.
                    // Not widened to `can_damage`: which classes a sentry engages
                    // (Enemy, not Npc/Boss/Neutral) is a separate design question.
                    .filter(|(_, f, health, (out_of_play, plane), _, driver)| {
                        ambition_combat::targeting::effective_faction(**f, *driver)
                            == ActorFaction::Enemy
                            && !ambition_combat::util::body_is_untouchable(*health, *out_of_play, *plane)
                    })
                    .filter(|(aabb, _, _, _, _, _)| aabb.center.distance(sentry.pos) <= SENTRY_RANGE),
                |(aabb, _, _, _, _, _)| aabb.center.distance_squared(sentry.pos),
                |(_, _, _, _, id, _)| *id,
            )
            .map(|(aabb, _, _, _, _, _)| aabb.center);
            let Some(target) = target else {
                // No target: idle, with the cadence ready to fire as soon as an
                // enemy arrives.
                sentry.fire_cooldown = 0.0;
                continue;
            };
            let dir = (target - sentry.pos).normalize_or_zero();
            if dir == ae::Vec2::ZERO {
                continue;
            }
            projectiles.write(ProjectileSpawnRequest::open(
                entity,
                ProjectileSpawn {
                    origin: sentry.pos,
                    dir,
                    speed: SENTRY_BOLT_SPEED,
                    damage: SENTRY_BOLT_DAMAGE,
                    max_lifetime: SENTRY_BOLT_LIFETIME,
                    half_extent: SENTRY_BOLT_HALF,
                    gravity: 0.0,
                    visual_id: String::new(),
                    // Straight volley: this ability authors no bounce.
                    bounces: 0,
                    bounce_on_world_contact: false,
                    splash_half_extent: 0.0,
                    boomerang_return_s: None,
                },
                ProjectileStart::StepThisTick,
            ));
            sentry.fire_cooldown = SENTRY_FIRE_INTERVAL_S;
            // The turret fires, with the source it inherited at spawn.
            sfx.write_for(
                entity,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::WORLD_ROCK_HIT,
                    pos: sentry.pos,
                },
            );
        }
    }
}

/// The native vortex: the cast and the well. The module road is
/// `ambition_content_modules::vortex` on the module-entity ports.
pub mod vortex {
    use ambition_characters::control::ActorControl;
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_combat::components::ActorFaction;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::body_clusters::BodyKinematics;
    use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;
    use ambition_platformer2d_shared_tangle::lifecycle::{
        SessionScopedEntity, SessionSpawnScope, SpawnSessionScopedExt,
    };
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

    /// Held-item id of the vortex gauntlet.
    pub const VORTEX_ID: &str = "vortex";

    /// Mana per cast (out of 100).
    const VORTEX_MANA_COST: f32 = 22.0;

    /// How far in front of the player (along aim) the singularity spawns.
    const VORTEX_RANGE: f32 = 200.0;
    /// Radius (px) within which enemies get dragged toward the center.
    const VORTEX_RADIUS: f32 = 220.0;
    /// Pull rate (1/s): the fraction of the remaining gap closed per second
    /// (`lerp` factor `rate * dt`). Higher gathers faster.
    const VORTEX_PULL_RATE: f32 = 5.0;
    /// How long (s) the singularity persists pulling.
    const VORTEX_LIFETIME_S: f32 = 0.9;

    /// A live vortex singularity: pulls enemies toward `center` until `remaining_s`
    /// hits zero.
    #[derive(Component, Debug, Clone, Copy)]
    pub struct VortexWell {
        pub center: ae::Vec2,
        pub remaining_s: f32,
    }

    /// `Attack` while holding the vortex gauntlet spawns a [`VortexWell`] ahead of
    /// the player along the aim. Plain Attack only; `Shield + Attack` drops the
    /// item (the id is `UseSystem`, excluded from throw-on-plain-Attack).
    pub fn fire_vortex_system(
        // Every driven body, not only the primary seat's `ControlledSubject`, so a
        // possessed body or a second seat can cast.
        driven: ambition_held_items::DrivenBodies,
        mut bodies: Query<(
            &ActorControl,
            &BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            Option<&mut ambition_platformer2d_core::resources::ActorResources>,
            Option<&SessionScopedEntity>,
            // The caster's identity and mint stream. `Option` because fixtures
            // carry neither; production bodies get them from `ensure_sim_id` or
            // their spawn site.
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter>,
        )>,
        mut commands: Commands,
        mut sfx: ambition_sfx::BodySfxWriter,
    ) {
        for subject in driven.entities() {
            let Ok((
                control,
                kin,
                resolved_frame,
                held,
                mut mana,
                owner,
                caster_id,
                mut caster_counter,
            )) = bodies.get_mut(subject)
            else {
                continue;
            };
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != VORTEX_ID {
                continue;
            }
            // Refuse before spending (ADR 0030): a refusal after `try_spend`
            // would take mana and open nothing.
            let (Some(caster), Some(counter)) = (caster_id, caster_counter.as_mut()) else {
                warn!(
                    "a vortex cast was refused: the caster carries no SimId or no \
                     SimIdCounter, so the well could not be named"
                );
                continue;
            };
            // N3.1: a dynamically spawned sim entity is `SimId::spawned(caster,
            // counter.next())`. The counter lives on the caster, so casters never
            // share a stream; taking a number is snapshot state.
            let id = Some(ambition_platformer2d_shared_tangle::sim_id::SimId::spawned(
                caster,
                counter.next(),
            ));
            if !ambition_platformer2d::abilities::mana::spend(mana.as_deref_mut(), VORTEX_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let gravity_dir = resolved_frame.down();
            let aim = ambition_held_items::ability_aim_world(&c, kin.facing, gravity_dir)
                .normalize_or_zero();
            if aim == ae::Vec2::ZERO {
                continue;
            }
            let center = kin.pos + aim * VORTEX_RANGE;
            open_vortex_well(
                &mut commands,
                SessionSpawnScope::new(owner.map(|owner| owner.0)),
                center,
                id,
            );
            sfx.write_for(
                subject,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_BLINK,
                    pos: center,
                },
            );
        }
    }

    /// Open one singularity. The only way a vortex well enters the world.
    ///
    /// One place, like `module_entity::spawn_module_entity`, so tests can spawn the entity the way
    /// production does. An archetype spawned only inside a system that needs a
    /// held gauntlet, mana, and an aim would be unreachable by coverage sweeps.
    ///
    /// `id` is `Option`: a well cast by a named caster gets `SimId::spawned`; a
    /// fixture well has no caster. It never decides the order (see
    /// [`update_vortex_wells`]).
    ///
    /// `remaining_s` is authoritative simulation state: the well pulls every body
    /// in radius while it counts down. The component and entity anchor are
    /// declared in the actor crate's `register_rollback_state`.
    pub fn open_vortex_well(
        commands: &mut Commands,
        scope: SessionSpawnScope,
        center: ae::Vec2,
        id: Option<ambition_platformer2d_shared_tangle::sim_id::SimId>,
    ) -> Entity {
        let mut well = commands.spawn_session_scoped(
            scope,
            (
                VortexWell {
                    center,
                    remaining_s: VORTEX_LIFETIME_S,
                },
                Name::new("Vortex singularity"),
            ),
        );
        if let Some(id) = id {
            well.insert(id);
        }
        well.id()
    }

    /// Drag every Enemy-faction actor within [`VORTEX_RADIUS`] of each live well
    /// toward its center (a position lerp; the actor's `step_motion` next tick
    /// resolves walls), then age the wells out. Runs on `scaled_dt`, so
    /// bullet-time slows the gather.
    ///
    /// Overlapping wells do not commute. Each well lerps a fraction `f` toward
    /// its own center, so A-then-B ends `f²·(B−A)` away from B-then-A (about
    /// 1.3px per tick for wells 200px apart at 60 Hz). Rollback registration does
    /// not fix the order, so wells are sorted by their own state (center,
    /// remaining life), then by identity. Wells that tie on state are the same
    /// pull, and fixtures without ids stay repeatable.
    pub fn update_vortex_wells(
        world_time: Res<ambition_time::WorldTime>,
        mut commands: Commands,
        mut wells: Query<(Entity, &mut VortexWell)>,
        // Tie-break authority, read separately so a well with no id still
        // applies.
        ids: Query<&SimId>,
        mut actors: Query<
            (
                &mut BodyKinematics,
                Option<&mut ae::SweepSample>,
                &ActorFaction,
                Option<&ambition_characters::actor::BodyHealth>,
                // A body out of play, or behind the playable plane, is not a target.
                (
                    bevy::prelude::Has<ambition_combat::death_rules::OutOfPlay>,
                    Option<&ambition_platformer2d_core::DepthPlane>,
                ),
                // Whether a participant drives this body, which decides its
                // effective side. See the filter below.
                Option<&ambition_characters::control::DrivingParticipant>,
            ),
            With<FeatureSimEntity>,
        >,
    ) {
        let dt = world_time.scaled_dt;
        if dt <= 0.0 {
            return;
        }
        let factor = (VORTEX_PULL_RATE * dt).min(1.0);
        let mut order: Vec<(ae::Vec2, f32, Option<SimId>, Entity)> = wells
            .iter()
            .map(|(entity, well)| {
                (
                    well.center,
                    well.remaining_s,
                    ids.get(entity).ok().cloned(),
                    entity,
                )
            })
            .collect();
        order.sort_by(|a, b| {
            a.0.x
                .total_cmp(&b.0.x)
                .then_with(|| a.0.y.total_cmp(&b.0.y))
                .then_with(|| a.1.total_cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        for (_, _, _, entity) in order {
            let Ok((entity, mut well)) = wells.get_mut(entity) else {
                continue;
            };
            for (mut kin, mut sweep, faction, health, (out_of_play, plane), driver) in &mut actors {
                // Use the effective faction, not the authored one: a possessed NPC
                // keeps `ActorFaction::Enemy` and moves its side through the
                // driver, so the authored field would pull the player's own body.
                // Not widened past the `Enemy` class (as the sentry: `ambition.world.module_entity_tick`'s `nearest_enemy`).
                // A dead enemy is an intangible corpse; the well does not drag it.
                if ambition_combat::targeting::effective_faction(*faction, driver)
                    != ActorFaction::Enemy
                    || ambition_combat::util::body_is_untouchable(health, out_of_play, plane)
                {
                    continue;
                }
                if kin.pos.distance(well.center) <= VORTEX_RADIUS {
                    // The well is an external kinematic constraint (ADR 0024): it
                    // moves the body toward the center by this tick's pull delta.
                    let delta = kin.pos.lerp(well.center, factor) - kin.pos;
                    ae::movement::carry_body(&mut kin, sweep.as_deref_mut(), delta);
                }
            }
            well.remaining_s -= dt;
            if well.remaining_s <= 0.0 {
                if let Ok(mut ec) = commands.get_entity(entity) {
                    ec.despawn();
                }
            }
        }
    }
}
