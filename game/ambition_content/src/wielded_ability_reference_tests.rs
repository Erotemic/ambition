//! The NATIVE wielded abilities (shockwave, beam, volley, meteor, sentry,
//! vortex, blink, dive, mark/recall, grapple), kept as the
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
    use ambition_combat::components::CenteredAabb;
    use ambition_characters::actor::ActorFaction;
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
    /// `sim_dt` (bullet-time slows the turret with everything else).
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
        let dt = world_time.sim_dt();
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
    use ambition_characters::actor::ActorFaction;
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
    /// resolves walls), then age the wells out. Runs on `sim_dt`, so
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
        let dt = world_time.sim_dt();
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

pub mod blink {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

    /// Held-item id of the blink.
    pub const BLINK_ID: &str = "blink";

    /// How far a blink carries the player along the aim direction, walls permitting.
    const BLINK_DISTANCE: f32 = 150.0;

    /// Cooldown between blinks, so it is a deliberate reposition, not spam.
    const BLINK_COOLDOWN_S: f32 = 0.45;

    /// Half-extent of the arrival shockwave that lets you blink offensively into a
    /// cluster of enemies.
    const BLINK_SHOCKWAVE_HALF: f32 = 36.0;
    /// Shockwave damage: modest; Blink is mobility first, a light strike second.
    const BLINK_SHOCKWAVE_DAMAGE: i32 = 2;

    /// `Attack` while holding the Blink ability teleports the player up to
    /// [`BLINK_DISTANCE`] along the aim direction, stopping a body-half short of the
    /// first solid wall so the teleport never lands inside geometry.
    pub fn blink_system(
        world: ambition_platformer2d_world::collision::CollisionWorld,
        mut commands: Commands,
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut bodies: Query<(
            Entity,
            ae::BodyClusterQueryData,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            &ActorControl,
            Option<&mut ambition_platformer2d::abilities::ability_cooldown::AbilityCooldown>,
            &mut ambition_platformer2d_core::movement::MotionModel,
            // The live room the body is in: it blinks against that room's walls.
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        )>,
        mut sfx: ambition_sfx::BodySfxWriter,
        mut vfx: ambition_vfx::vfx::VfxWriter,
        mut hits: MessageWriter<ambition_combat::events::HitEvent>,
        // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
        // blinks.
        mut class_b: Option<ResMut<ClassBRemapLog>>,
    ) {
        for subject in driven.entities() {
            let Ok((
                player,
                mut cluster_item,
                resolved_frame,
                held,
                control,
                mut cooldown,
                mut motion_model,
                room,
            )) = bodies.get_mut(subject)
            else {
                continue;
            };
            if !matches!(
                *motion_model,
                ambition_platformer2d_core::movement::MotionModel::AxisSwept(_)
            ) {
                continue;
            }
            let c = control.0;
            // Plain Attack blinks; Shield+Attack throws the item away.
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != BLINK_ID {
                continue;
            }
            // Aim from the brain-resolved frame (aim stick, then movement stick,
            // then facing), rotated to world. Uses the body's per-tick resolved
            // frame (ADR 0024).
            let gravity_dir = resolved_frame.down();
            let facing = cluster_item.kinematics.facing;
            let dir =
                ambition_held_items::ability_aim_world(&c, facing, gravity_dir).normalize_or_zero();
            if dir == ae::Vec2::ZERO {
                continue;
            }
            // Check the shared movement-ability cooldown only after a real blink
            // is confirmed, so an aimless press does not use it.
            if !ambition_platformer2d::abilities::ability_cooldown::try_use_ability(
                &mut cooldown,
                &mut commands,
                player,
                BLINK_COOLDOWN_S,
            ) {
                continue;
            }
            let mut clusters = cluster_item.as_clusters_mut();
            let from = clusters.kinematics.pos;
            // The box the body has: turned to the DOWN of its resolved frame, as
            // the kernel turns it for the step.
            let half = clusters.kinematics.half_oriented(gravity_dir);
            // One collision view (moving platforms and ECS solids included) for
            // the clamp raycast and the embed check in `blink_target`: the walls
            // of the body's own live room, not of "the" room.
            let collision = world.room(room).and_then(|room| room.solids());
            let target = match collision.as_ref() {
                Some(w) => ambition_platformer2d::abilities::traversal::blink::blink_target(&**w, from, dir, BLINK_DISTANCE, half),
                // No collision world (tests): blink the full distance.
                None => from + dir * BLINK_DISTANCE,
            };
            // The discrete-transit authority: arrive with momentum kept, and
            // reconcile departure contacts and attachment (ADR 0024).
            ae::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                target,
                ae::movement::TransitVelocity::Keep,
            );
            // Class-B transit (`docs/concepts/movement-collision.md`): a
            // traversal ability that moves a body is a scripted teleport, ranked
            // weakest, so dying mid-blink is a death, not a blink.
            if let Some(log) = class_b.as_mut() {
                log.record(player, ClassBRemap::ScriptedTeleport);
            }
            // Offensive blink: a small player-side shockwave at the arrival point,
            // so you can blink into enemies to hit them (PlayerSlash spares the
            // player).
            hits.write(ambition_combat::events::HitEvent {
                strike_sfx: None,
                volume: ae::CombatVolume::circle(target, BLINK_SHOCKWAVE_HALF),
                damage: BLINK_SHOCKWAVE_DAMAGE,
                source: ambition_combat::events::HitSource::Melee,
                attacker: Some(player),
                room: None,
                target: ambition_combat::events::HitTarget::Volume,
                mode: ambition_combat::events::HitMode::Knockback,
                knockback: None,
                ignored_targets: Vec::new(),
                        attacker_move_instance: None,
            });
            sfx.write_for(
                player,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_BLINK,
                    pos: target,
                },
            );
            // A wisp where you left, a flash where you arrive, in the live room
            // of the body.
            let mut vfx = vfx.for_room(room.map(|stamp| stamp.0));
            vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                pos: from,
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.35,
                pose: ambition_vfx::FxPose::UPRIGHT,
            });
            vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                pos: target,
                fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                scale: 0.5,
                pose: ambition_vfx::FxPose::UPRIGHT,
            });
        }
    }
}

pub mod dive {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core::{self as ae, AabbExt};
    use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

    /// Held-item id of the dive gauntlet.
    pub const DIVE_ID: &str = "dive";

    /// Mana per lunge (out of 100), so it cannot be spammed across a room.
    const DIVE_MANA_COST: f32 = 26.0;

    /// How far (px) the player lunges along the aim, absent a wall.
    const DIVE_LUNGE: f32 = 140.0;
    /// Half-thickness (px) of the damaging corridor swept by the lunge.
    const DIVE_WIDTH: f32 = 48.0;
    /// Damage dealt to everything in the corridor.
    const DIVE_DAMAGE: i32 = 4;
    /// Horizontal shove imparted to struck enemies (signed by the lunge direction).
    const DIVE_KNOCKBACK: f32 = 1.4;

    /// Snap an aim and facing to a lunge direction (a unit vector on the
    /// dominant axis). A null aim uses `facing`, so a plain Attack still lunges;
    /// the blink, in contrast, needs an explicit aim.
    fn dive_dir(aim: ae::Vec2, facing: f32) -> ae::Vec2 {
        let horizontal = if aim == ae::Vec2::ZERO {
            true
        } else {
            aim.x.abs() >= aim.y.abs()
        };
        if horizontal {
            let s = if aim.x.abs() > 0.001 {
                aim.x.signum()
            } else {
                facing.signum()
            };
            ae::Vec2::new(s, 0.0)
        } else {
            ae::Vec2::new(0.0, aim.y.signum())
        }
    }

    /// The damaging corridor from `from` to `to`: an axis-aligned box around both
    /// endpoints, padded by a body width. For a snapped lunge this is a thin
    /// rectangle.
    fn dive_corridor(from: ae::Vec2, to: ae::Vec2) -> ae::Aabb {
        let center = (from + to) * 0.5;
        let half = ae::Vec2::new(
            (to.x - from.x).abs() * 0.5 + DIVE_WIDTH * 0.5,
            (to.y - from.y).abs() * 0.5 + DIVE_WIDTH * 0.5,
        );
        ae::Aabb::new(center, half)
    }

    /// `Attack` while holding the dive gauntlet lunges the player along the aim
    /// and emits a one-shot `Player`-faction hit over the corridor. Plain Attack
    /// only; `Shield + Attack` drops the item (the id is `UseSystem`, excluded from
    /// throw-on-plain-Attack in `throw_held_item_system`).
    pub fn fire_dive_system(
        world: ambition_platformer2d_world::collision::CollisionWorld,
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can act.
        driven: ambition_held_items::DrivenBodies,
        mut players: Query<(
            Entity,
            &ActorControl,
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            // The live room the body is in: it lunges against that room's walls.
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        )>,
        mut sfx: ambition_sfx::BodySfxWriter,
        mut hits: MessageWriter<ambition_combat::events::HitEvent>,
        // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
        // dives.
        mut class_b: Option<ResMut<ClassBRemapLog>>,
    ) {
        for subject in driven.entities() {
            let Ok((player, control, mut cluster_item, mut motion_model, resolved_frame, held, room)) =
                players.get_mut(subject)
            else {
                continue;
            };
            let mut clusters = cluster_item.as_clusters_mut();
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != DIVE_ID {
                continue;
            }
            if !ambition_platformer2d::abilities::mana::spend(clusters.resources.as_deref_mut(), DIVE_MANA_COST) {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let frame = resolved_frame.basis();
            let facing = clusters.kinematics.facing;
            let local_aim = ambition_held_items::ability_aim_local(&c, facing);
            let local_dir = dive_dir(local_aim, facing).normalize_or_zero();
            let dir = frame.to_world(local_dir).normalize_or_zero();
            let from = clusters.kinematics.pos;
            // Stop a body-half short of the wall so the lunge never embeds. Use
            // the body's extent in the lunge direction (half-height for a vertical
            // dive), as the blink does, or a downward dive embeds in the floor
            // and trips the OOB detector.
            // The half is of the box the body has: turned to the DOWN of its last
            // step. For a body the axis arm moves that is the DOWN of its
            // resolved frame. A crawler on a wall lies along the wall, and only
            // the record of its step says so; the dive does not turn it.
            let down = ae::SweepSample::down_or(clusters.sweep.as_deref(), resolved_frame.down());
            let half = clusters.kinematics.half_oriented(down);
            let margin = (half.x * dir.x.abs() + half.y * dir.y.abs()) + 2.0;
            // One collision view for the clamp raycast and the embed check, so
            // moving platforms and ECS solids also stop the lunge.
            let collision = world.room(room).and_then(|room| room.solids());
            let mut target = match collision.as_ref().and_then(|w| {
                ambition_platformer2d_core::cast::raycast_solids(
                    &**w,
                    from,
                    dir,
                    DIVE_LUNGE + margin,
                    false,
                )
            }) {
                Some((hit, _normal)) => hit - dir * margin,
                None => from + dir * DIVE_LUNGE,
            };
            // Safety net: if the landing AABB still overlaps a solid (a corner
            // the center ray missed), stay at the start instead of embedding.
            if let Some(w) = collision.as_ref() {
                let landing = ae::Aabb::new(target, half);
                let embeds = w.blocks.iter().any(|b| {
                    ae::collision_semantics::is_full_collision_surface(b.kind) && landing.strict_intersects(b.aabb)
                });
                if embeds {
                    target = from;
                }
            }
            // The discrete-transit authority: arrive with momentum kept, and
            // reconcile departure contacts and attachment (ADR 0024).
            ae::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                target,
                ae::movement::TransitVelocity::Keep,
            );
            // Class-B transit (`docs/concepts/movement-collision.md`): a
            // traversal ability that moves a body is a scripted teleport, ranked
            // weakest, so dying mid-dive is a death, not a dive.
            if let Some(log) = class_b.as_mut() {
                log.record(player, ClassBRemap::ScriptedTeleport);
            }
            if local_dir.x.abs() > 0.001 {
                clusters.kinematics.facing = local_dir.x.signum();
            }
            // The corridor hits everything between start and landing: a one-shot
            // PlayerSlash volume that spares the player and pushes enemies along
            // the dash. The push uses `DIVE_KNOCKBACK` above.
            let corridor: ambition_platformer2d_core::CombatVolume = dive_corridor(from, target).into();
            let corridor_center = corridor.center();
            hits.write(ambition_combat::events::HitEvent {
                strike_sfx: None,
                volume: corridor,
                damage: DIVE_DAMAGE,
                source: ambition_combat::events::HitSource::Melee,
                attacker: Some(player),
                room: None,
                target: ambition_combat::events::HitTarget::Volume,
                mode: ambition_combat::events::HitMode::Knockback,
                knockback: Some(ambition_combat::events::HitKnockback {
                    // An ordinary hit: it stuns.
                    reaction: ambition_platformer2d_core::hit_response::HitReaction::Strike,
                    dir: local_dir.x.signum(),
                    magnitude: ambition_combat::events::HitKnockbackMagnitude::FeelScale(
                        DIVE_KNOCKBACK,
                    ),
                    source_pos: corridor_center,
                    impact_pos: corridor_center,
                    launch_dir: None,
                    follow: None,
                }),
                ignored_targets: Vec::new(),
                        attacker_move_instance: None,
            });
            sfx.write_for(
                player,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_BLINK,
                    pos: target,
                },
            );
        }
    }
}

pub mod mark_recall {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

    /// The held-item id the Mark/Recall ability grants (see `brain::action_set`
    /// `HELD_ITEMS` and `items::Item::held_item_id`).
    pub const MARK_RECALL_ID: &str = "mark_recall";

    /// Half-extent of the recall-strike shockwave at the mark.
    const RECALL_SHOCKWAVE_HALF: f32 = 36.0;
    /// Recall-strike damage: modest, like Blink's arrival shockwave.
    const RECALL_SHOCKWAVE_DAMAGE: i32 = 2;

    /// While holding the Mark/Recall item: a plain `Attack` drops or moves the
    /// mark at the player's feet, and `Blink` recalls to the mark if set. A frame
    /// that drops a mark does not also recall, so a simultaneous press means "set
    /// the mark here".
    pub fn mark_recall_system(
        mut commands: Commands,
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut players: Query<(
            Entity,
            &ActorControl,
            ae::BodyClusterQueryData,
            &mut ambition_platformer2d_core::movement::MotionModel,
            &HeldItem,
            Option<&mut ambition_platformer2d::abilities::traversal::mark_recall::PlayerMark>,
            // The live room the body is in: its effects are drawn there.
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        )>,
        mut sfx: ambition_sfx::BodySfxWriter,
        mut vfx: ambition_vfx::vfx::VfxWriter,
        mut hits: MessageWriter<ambition_combat::events::HitEvent>,
        // Optional diagnostic Class-B ledger (§3.2), so a minimal test app still
        // recalls.
        mut class_b: Option<ResMut<ClassBRemapLog>>,
    ) {
        for subject in driven.entities() {
            let Ok((player, control, mut cluster_item, mut motion_model, held, mut mark, room)) =
                players.get_mut(subject)
            else {
                continue;
            };
            let room = room.map(|stamp| stamp.0);
            let mut vfx = vfx.for_room(room);
            let mut clusters = cluster_item.as_clusters_mut();
            let c = control.0;
            if held.spec.id != MARK_RECALL_ID {
                continue;
            }

            // Plain Attack drops or moves the mark. Shield+Attack throws the item
            // away, so a shielded frame does not mark.
            if c.melee_pressed && !c.shield_held {
                let pos = clusters.kinematics.pos;
                let dropped = ambition_platformer2d::abilities::traversal::mark_recall::PlayerMark { pos: Some(pos), room };
                match mark.as_deref_mut() {
                    Some(existing) => *existing = dropped,
                    None => {
                        commands.entity(player).insert(dropped);
                    }
                }
                sfx.write_for(
                    player,
                    ambition_sfx::SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_DASH,
                        pos,
                    },
                );
                vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                    pos,
                    fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                    scale: 0.4,
                    pose: ambition_vfx::FxPose::UPRIGHT,
                });
                continue;
            }

            // Blink recalls to the mark, if one is set in the room the body is in.
            if c.blink_pressed {
                if let Some(target) = mark.filter(|m| m.room == room).and_then(|m| m.pos) {
                    // The discrete-transit authority: momentum kept, departure
                    // contacts and attachment reconciled (ADR 0024).
                    ae::movement::transit_body(
                        &mut motion_model,
                        &mut clusters,
                        target,
                        ae::movement::TransitVelocity::Keep,
                    );
                    // Class-B transit (`docs/concepts/movement-collision.md`):
                    // the recall moves the body, so it is a scripted teleport.
                    if let Some(log) = class_b.as_mut() {
                        log.record(player, ClassBRemap::ScriptedTeleport);
                    }
                    // Recall strike: a player-side shockwave at the mark, so you
                    // can lure enemies onto it and recall in to hit them.
                    hits.write(ambition_combat::events::HitEvent {
                        strike_sfx: None,
                        volume: ae::CombatVolume::circle(target, RECALL_SHOCKWAVE_HALF),
                        damage: RECALL_SHOCKWAVE_DAMAGE,
                        source: ambition_combat::events::HitSource::Melee,
                        attacker: Some(player),
                        room: None,
                        target: ambition_combat::events::HitTarget::Volume,
                        mode: ambition_combat::events::HitMode::Knockback,
                        knockback: None,
                        ignored_targets: Vec::new(),
                                        attacker_move_instance: None,
                    });
                    sfx.write_for(
                        player,
                        ambition_sfx::SfxMessage::Play {
                            id: ambition_sfx::ids::PLAYER_BLINK,
                            pos: target,
                        },
                    );
                    vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
                        pos: target,
                        fx: ambition_vfx::fx::ids::CLASSIC_BURST,
                        scale: 0.6,
                        pose: ambition_vfx::FxPose::UPRIGHT,
                    });
                }
            }
        }
    }
}

pub mod grapple {
    use bevy::prelude::*;

    use ambition_combat::held_items::HeldItem;
    use ambition_characters::control::ActorControl;
    use ambition_platformer2d_core as ae;
    use ambition_platformer2d_core::BodyKinematics;

    /// Held-item id of the grapple.
    pub const GRAPPLE_ID: &str = "grapple";

    /// How far the grapple line reaches for a solid surface.
    const GRAPPLE_RANGE: f32 = 300.0;

    /// Speed of the burst yank toward a grappled surface.
    const GRAPPLE_PULL_SPEED: f32 = 620.0;

    /// Cooldown between successful pulls, so grappling is deliberate.
    const GRAPPLE_COOLDOWN_S: f32 = 0.55;

    /// `Attack` while holding the Grapple ability casts along the aim direction; on
    /// hitting a solid within [`GRAPPLE_RANGE`] it yanks the player toward the hit.
    pub fn grapple_system(
        world: ambition_platformer2d_world::collision::CollisionWorld,
        mut commands: Commands,
        // Every driven body, not only the primary seat's `ControlledSubject`, so
        // a possessed body or a second seat can use it.
        driven: ambition_held_items::DrivenBodies,
        mut bodies: Query<(
            Entity,
            &ActorControl,
            &mut BodyKinematics,
            &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
            &HeldItem,
            Option<&mut ambition_platformer2d::abilities::ability_cooldown::AbilityCooldown>,
            // The live room the body is in: the hook catches that room's walls.
            Option<&ambition_platformer2d_shared_tangle::lifecycle::InRoomInstance>,
        )>,
        mut sfx: ambition_sfx::BodySfxWriter,
        mut vfx: ambition_vfx::vfx::VfxWriter,
    ) {
        for subject in driven.entities() {
            let Ok((player, control, mut kin, resolved_frame, held, mut cooldown, room)) =
                bodies.get_mut(subject)
            else {
                continue;
            };
            let c = control.0;
            if !c.melee_pressed || c.shield_held {
                continue;
            }
            if held.spec.id != GRAPPLE_ID {
                continue;
            }
            // The body's per-tick resolved frame (ADR 0024 frame law).
            let gravity_dir = resolved_frame.down();
            let dir = ambition_held_items::ability_aim_world(&c, kin.facing, gravity_dir)
                .normalize_or_zero();
            if dir == ae::Vec2::ZERO {
                continue;
            }
            let from = kin.pos;
            // Raycast against the composited collision world, so the grapple can
            // latch a moving platform or ECS solid.
            let Some((hit, _normal)) = world.room(room).and_then(|room| room.solids()).and_then(|w| {
                ambition_platformer2d_core::cast::raycast_solids(&*w, from, dir, GRAPPLE_RANGE, false)
            }) else {
                // Grapple into empty space: a fizzle, no pull, no cooldown used.
                sfx.write_for(
                    player,
                    ambition_sfx::SfxMessage::Play {
                        id: ambition_sfx::ids::PLAYER_DASH,
                        pos: from,
                    },
                );
                continue;
            };
            // Only a successful latch uses the cooldown; a miss costs nothing.
            if !ambition_platformer2d::abilities::ability_cooldown::try_use_ability(
                &mut cooldown,
                &mut commands,
                player,
                GRAPPLE_COOLDOWN_S,
            ) {
                continue;
            }
            // Pull toward the latched surface (collision settles the player
            // there). A burst velocity, not a teleport, so it reads as a pull.
            let pull = (hit - from).normalize_or_zero();
            kin.vel = pull * GRAPPLE_PULL_SPEED;
            sfx.write_for(
                player,
                ambition_sfx::SfxMessage::Play {
                    id: ambition_sfx::ids::PLAYER_DASH,
                    pos: from,
                },
            );
            // Draw the grapple line as a tan spark trail from the player to the
            // latch point, so the ability reads as a rope pulling you in (#53).
            const GRAPPLE_LINE_SEGMENTS: i32 = 8;
            // The line and its end are drawn in the live room of the body.
            let mut vfx = vfx.for_room(room.map(|stamp| stamp.0));
            for i in 1..GRAPPLE_LINE_SEGMENTS {
                let p = from.lerp(hit, i as f32 / GRAPPLE_LINE_SEGMENTS as f32);
                vfx.write(ambition_vfx::vfx::VfxMessage::Burst {
                    pos: p,
                    count: 2,
                    speed: 28.0,
                    color: [0.86, 0.78, 0.48, 0.95],
                    kind: ambition_vfx::vfx::ParticleKind::Spark,
                });
            }
            vfx.write(ambition_vfx::vfx::VfxMessage::Impact { pos: hit });
        }
    }
}
