//! The individual debug-overlay layers — world bounds/grid/camera, entity
//! hitboxes, health bars, feature/projectile/portal debug draws.
//!
//! Split out of the former 1001-line `debug_overlay.rs` (2026-06-15).

use super::*;

/// Draw each in-flight held-item shot (gun-sword bolt / Fireball): its solid
/// **contact box** (the box that registers a hit — `HeldProjectile::contact_aabb`)
/// and, for a Fireball, the fainter **splash box** it detonates with on contact.
///
/// These were previously invisible: the shot rendered as a thin lasersword
/// sprite while a wider contact box (and a much wider splash) did the hitting,
/// so a Fireball read as "hitting gnuton before the bolt touches him". Sharing
/// `contact_aabb` / `splash_aabb` with the collision system keeps the drawn box
/// identical to the box that hits.
#[cfg(feature = "input")]
pub(crate) fn draw_held_projectiles<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    projectiles: impl Iterator<
        Item = (
            &'a ambition::actors::actor::BodyKinematics,
            &'a ambition::actors::items::pickup::HeldProjectile,
        ),
    >,
    developer_tools: &DeveloperTools,
) {
    use ambition::actors::items::pickup::HeldProjectile;
    let contact_color = Color::srgba(0.35, 0.85, 1.00, 0.90); // light blue (player-side)
    let splash_color = Color::srgba(1.00, 0.55, 0.20, 0.45); // faint orange (AOE)
    for (kin, proj) in projectiles {
        if let Some(splash) = proj.splash_aabb(kin.pos) {
            draw_aabb_styled(gizmos, world, splash, splash_color, developer_tools);
        }
        draw_aabb_styled(
            gizmos,
            world,
            HeldProjectile::contact_aabb(kin.pos),
            contact_color,
            developer_tools,
        );
    }
}

/// Draw each portal's capture AABB (the box that warps the player) plus a short
/// outward normal tick, so the portal's collision can be eyeballed in the
/// debug overlay (it's otherwise invisible — only the thin sprite shows).
#[cfg(feature = "input")]
#[cfg(feature = "portal")]
pub(crate) fn draw_portals<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    portals: impl Iterator<Item = &'a ambition::portal::PlacedPortal>,
) {
    for portal in portals {
        let color = portal.channel.display().0.with_alpha(0.95);
        draw_aabb(
            gizmos,
            world,
            ae::Aabb::new(portal.pos, portal.half_extent),
            color,
        );
        // Outward normal tick from the portal face into the room.
        let base = w2(world, portal.pos);
        let tip = w2(world, portal.pos + portal.normal * 22.0);
        gizmos.line_2d(base, tip, color);
        // The along-surface TANGENT (the "second normal" — which way is "right"
        // along the doorway). The portal map preserves this component, so it sets
        // whether your along-surface direction is kept or mirrored. Drawn in green
        // as a single-headed tick so its sign is visible.
        let tangent = ambition::portal::pieces::portal_tangent(portal.normal);
        gizmos.line_2d(
            base,
            w2(world, portal.pos + tangent * 18.0),
            Color::srgb(0.4, 1.0, 0.5),
        );
    }
}

#[cfg(feature = "input")]
#[derive(SystemParam)]
pub struct FeatureDebugQueries<'w, 's> {
    pub bosses: Query<
        'w,
        's,
        (
            ambition::actors::features::BossClusterRef,
            &'static ambition::characters::actor::BodyHealth,
            &'static ambition::characters::brain::BossAttackState,
            Option<&'static ambition::actors::features::BossAnimationFrameSample>,
        ),
        With<ambition::actors::features::FeatureSimEntity>,
    >,
    pub actors: Query<
        'w,
        's,
        (
            &'static ambition::actors::features::ActorDisposition,
            &'static ambition::actors::features::ActorAggression,
            &'static ambition::actors::features::CenteredAabb,
            Option<&'static ambition::actors::features::BodyKinematics>,
            Option<&'static ambition::actors::features::BodyMelee>,
            Option<&'static ambition::actors::features::ActorSurfaceState>,
        ),
        With<ambition::actors::features::FeatureSimEntity>,
    >,
    pub breakables: Query<
        'w,
        's,
        &'static ambition::actors::features::CenteredAabb,
        (
            With<ambition::actors::features::FeatureSimEntity>,
            With<ambition::actors::features::BreakableFeature>,
        ),
    >,
    pub chests: Query<
        'w,
        's,
        &'static ambition::actors::features::CenteredAabb,
        (
            With<ambition::actors::features::FeatureSimEntity>,
            With<ambition::actors::features::ChestFeature>,
        ),
    >,
    pub hazards: Query<
        'w,
        's,
        &'static ambition::actors::features::HazardFeature,
        With<ambition::actors::features::FeatureSimEntity>,
    >,
    /// All live `Hitbox` entities (melee swings, World-anchored
    /// hazards like the Gradient Sentinel's PitTrap pit /
    /// RotatingCross arms / HazardColumn column). Drawn so the debug
    /// view answers "what just hit me?" — without this pass the
    /// World-anchored boss specials are invisible even though they
    /// deal damage.
    pub hitboxes: Query<'w, 's, &'static ambition::actors::features::Hitbox>,
    /// CenteredAabb lookup for resolving `FollowOwner` hitboxes to
    /// their current world-space rectangle. World-anchored
    /// hitboxes don't need this — their AABB is fixed at spawn.
    pub hitbox_owners: Query<'w, 's, &'static ambition::actors::features::CenteredAabb>,
    /// In-flight held-item shots (gun-sword bolt / Fireball). Their
    /// contact + splash boxes were previously undrawn, so a Fireball
    /// read as "hitting before it touches the visible box". Lives in
    /// this bundle (not a top-level param) to keep `draw_debug_overlay`
    /// under Bevy's 16-system-param ceiling.
    /// `Without<PlayerEntity>` keeps this read of `BodyKinematics` disjoint from
    /// the `&mut` player query (a held shot is never the player) — B0001.
    pub held_projectiles: Query<
        'w,
        's,
        (
            &'static ambition::actors::actor::BodyKinematics,
            &'static ambition::actors::items::pickup::HeldProjectile,
        ),
        Without<ambition::actors::actor::PlayerEntity>,
    >,
    /// The player's resolved gravity, so the player debug box can rotate to
    /// match its (now gravity-oriented) collision box + sprite. Lives in this
    /// bundle (not a top-level param) to keep `draw_debug_overlay` under Bevy's
    /// 16-system-param ceiling.
    pub gravity: Option<Res<'w, ambition::actors::physics::GravityField>>,
    /// App-local character authority and attack-volume bridge used by the combat
    /// preview. Keeping them in this bundle preserves the top-level system's
    /// parameter budget.
    pub character_catalog:
        Res<'w, ambition::characters::actor::character_catalog::CharacterCatalog>,
    pub boss_catalog: Res<'w, ambition::actors::boss_encounter::BossCatalog>,
    pub authored_attack_volumes:
        Res<'w, ambition::actors::combat::authored_volumes::AuthoredAttackVolumeResolver>,
    /// In-flight player projectiles (ECS entities). Bundled here (rather than a
    /// top-level param) so `draw_debug_overlay` has a slot free for the
    /// debug-label buffer while staying under the 16-param ceiling.
    /// `Without<PlayerEntity>` proves disjointness from the `&mut` player query.
    pub player_projectiles: Query<
        'w,
        's,
        &'static ambition::actors::actor::BodyKinematics,
        (
            With<ambition::projectiles::PlayerProjectile>,
            Without<ambition::actors::actor::PlayerEntity>,
        ),
    >,
    /// In-flight enemy projectiles (ECS entities); see `player_projectiles`.
    pub enemy_projectiles: Query<
        'w,
        's,
        &'static ambition::actors::actor::BodyKinematics,
        (
            With<ambition::projectiles::enemy::EnemyProjectile>,
            Without<ambition::actors::actor::PlayerEntity>,
        ),
    >,
}

pub(crate) fn draw_room_bounds(gizmos: &mut Gizmos, world: &ae::World) {
    let room = ae::aabb_from_min_size(ae::Vec2::ZERO, world.size);
    draw_aabb(gizmos, world, room, white_dim());
}

pub(crate) fn draw_micro_grid(gizmos: &mut Gizmos, world: &ae::World, minor: f32, major: f32) {
    if minor <= 0.0 || major <= 0.0 {
        return;
    }
    let minor_color = Color::srgba(0.45, 0.55, 0.70, 0.13);
    let major_color = Color::srgba(0.70, 0.80, 1.00, 0.23);
    let cols = (world.size.x / minor).ceil() as i32;
    let rows = (world.size.y / minor).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * minor).min(world.size.x);
        let is_major = (x / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for i in 0..=rows {
        let y = (i as f32 * minor).min(world.size.y);
        let is_major = (y / major).fract().abs() < 0.01;
        let color = if is_major { major_color } else { minor_color };
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

pub(crate) fn draw_camera_frame(gizmos: &mut Gizmos, world: &ae::World, view: &CameraViewState) {
    let requested = ae::Aabb::new(view.target_world, view.requested_view * 0.5);
    let visible = ae::Aabb::new(view.center_world, view.visible_view * 0.5);
    draw_aabb(gizmos, world, visible, Color::srgba(0.20, 0.95, 1.00, 0.22));
    draw_aabb(
        gizmos,
        world,
        requested,
        Color::srgba(1.00, 0.95, 0.20, 0.22),
    );
}

pub(crate) fn draw_world_blocks(
    gizmos: &mut Gizmos,
    world: &ae::World,
    developer_tools: &DeveloperTools,
) {
    for block in &world.blocks {
        let color = match block.kind {
            ae::BlockKind::Solid => gray(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Soft,
            } => magenta(),
            ae::BlockKind::BlinkWall {
                tier: ae::BlinkWallTier::Hard,
            } => red(),
            ae::BlockKind::OneWay => blue(),
            ae::BlockKind::Hazard => red(),
            ae::BlockKind::PogoOrb => green(),
            ae::BlockKind::Rebound { .. } => orange(),
        };
        draw_aabb_styled(gizmos, world, block.aabb, color, developer_tools);
    }
}

/// Momentum-surface debug (demo plan S3b): draw every `SurfaceChain` — its
/// segments, and at each segment midpoint its TANGENT (green, along increasing
/// arc length) and its outward NORMAL (yellow, the `+normal` side a body rides).
/// Vertices get a small dot. Ships with the sanic_sandbox so the ride geometry
/// (slopes, the loop's interior winding) is legible without playing it.
pub(crate) fn draw_surface_chains(gizmos: &mut Gizmos, world: &ae::World) {
    let seg_color = Color::srgba(0.30, 0.90, 1.00, 0.85); // cyan — the surface line
    let normal_color = Color::srgba(1.00, 0.90, 0.20, 0.85); // yellow — ridden side
    let tangent_color = Color::srgba(0.40, 1.00, 0.55, 0.75); // green — arc direction
    let vertex_color = Color::srgba(1.00, 1.00, 1.00, 0.60);
    for chain in &world.chains {
        for &p in &chain.points {
            let c = w2(world, p);
            gizmos.line_2d(
                c + ae::Vec2::new(-3.0, 0.0),
                c + ae::Vec2::new(3.0, 0.0),
                vertex_color,
            );
            gizmos.line_2d(
                c + ae::Vec2::new(0.0, -3.0),
                c + ae::Vec2::new(0.0, 3.0),
                vertex_color,
            );
        }
        for i in 0..chain.segment_count() {
            let (a, b) = chain.segment(i);
            gizmos.line_2d(w2(world, a), w2(world, b), seg_color);
            let mid = (a + b) * 0.5;
            // Normal + tangent quills (world-space lengths; w2 handles the flip).
            let n = chain.normal(i);
            let t = chain.tangent(i);
            gizmos.line_2d(w2(world, mid), w2(world, mid + n * 22.0), normal_color);
            gizmos.line_2d(w2(world, mid), w2(world, mid + t * 14.0), tangent_color);
        }
    }
}

/// Lightweight coarse grid drawn straight through gizmos. Used when
/// `hide_sprites` strips the authored sprite grid so the player still
/// has a spatial reference. Spacing matches `ambition::engine_core::config::GRID_STEP`
/// (the same step the sprite grid spawned in `spawn_grid` uses).
pub(crate) fn draw_world_grid(gizmos: &mut Gizmos, world: &ae::World) {
    let step = ambition::engine_core::config::GRID_STEP;
    if step <= 0.0 {
        return;
    }
    let color = Color::srgba(0.45, 0.55, 0.70, 0.32);
    let cols = (world.size.x / step).ceil() as i32;
    let rows = (world.size.y / step).ceil() as i32;
    for i in 0..=cols {
        let x = (i as f32 * step).min(world.size.x);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(x, 0.0)),
            w2(world, ae::Vec2::new(x, world.size.y)),
            color,
        );
    }
    for j in 0..=rows {
        let y = (j as f32 * step).min(world.size.y);
        gizmos.line_2d(
            w2(world, ae::Vec2::new(0.0, y)),
            w2(world, ae::Vec2::new(world.size.x, y)),
            color,
        );
    }
}

pub(crate) fn draw_loading_zones(gizmos: &mut Gizmos, world: &ae::World, zones: &[LoadingZone]) {
    for zone in zones {
        let color = match zone.activation {
            LoadingZoneActivation::EdgeExit => cyan(),
            LoadingZoneActivation::Door => yellow(),
            // `Walk` zones — mid-room walk-through portals.
            // Distinct green so they don't read as either an edge
            // exit (cyan) or an interact door (yellow).
            LoadingZoneActivation::Walk => Color::srgba(0.40, 1.00, 0.55, 0.85),
        };
        draw_aabb(gizmos, world, zone.aabb, color);
    }
}

pub(crate) fn draw_ldtk_runtime_spine(
    gizmos: &mut Gizmos,
    world: &ae::World,
    spine_index: &ambition::actors::ldtk_world::LdtkRuntimeSpineIndex,
) {
    for entity in &spine_index.entities {
        let color = match entity.role {
            ambition::actors::ldtk_world::LdtkRuntimeRole::PlayerStart => green(),
            ambition::actors::ldtk_world::LdtkRuntimeRole::LoadingZone => {
                Color::srgba(1.0, 1.0, 1.0, 0.70)
            }
            ambition::actors::ldtk_world::LdtkRuntimeRole::DebugLabel => magenta(),
            ambition::actors::ldtk_world::LdtkRuntimeRole::CameraZone => blue(),
            // Solid runtime rects are drawn by the dedicated Solid index pass
            // so they can be color-keyed against the JSON-derived collision
            // blocks during the Step 2 raw-vs-runtime overlay work.
            ambition::actors::ldtk_world::LdtkRuntimeRole::Solid => continue,
            // OneWayPlatform / DamageVolume have their own dedicated runtime
            // indices and overlay passes; skip them in the generic spine
            // overlay so colors don't double-stamp.
            ambition::actors::ldtk_world::LdtkRuntimeRole::OneWayPlatform => continue,
            ambition::actors::ldtk_world::LdtkRuntimeRole::DamageVolume => continue,
            ambition::actors::ldtk_world::LdtkRuntimeRole::Other => continue,
        };
        draw_aabb(gizmos, world, entity.aabb(), color);
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    character_catalog: &ambition::characters::actor::character_catalog::CharacterCatalog,
    authored_attack_volumes: &ambition::actors::combat::authored_volumes::AuthoredAttackVolumeResolver,
    worn_character_id: &str,
    clusters: &ae::BodyClustersMut<'_>,
    // Dev-tool read: the overlay draws the policy's private internals (the
    // ledge anchor/climb-target, the live blink aim) straight off the model.
    motion_model: &ae::MotionModel,
    moving_platforms: &[ambition::actors::world::platforms::MovingPlatformState],
    attack: Option<&ambition::actors::MeleeSwing>,
    actions: Option<&ActionState<SandboxAction>>,
    gameplay_active: bool,
    developer_tools: &DeveloperTools,
    gravity_dir: ae::Vec2,
    labels: &mut DebugOverlayLabels,
) {
    let pos = clusters.kinematics.pos;
    let vel = clusters.kinematics.vel;
    let size = clusters.kinematics.size;
    let facing = clusters.kinematics.facing;
    let on_ground = clusters.ground.on_ground;
    let on_wall = clusters.wall.on_wall;
    let wall_normal_x = clusters.wall.wall_normal_x;
    // The overlay's maneuver reads via the same projection the sim publishes.
    let facts = ae::BodyMotionFacts::from_model(motion_model);
    // The player's body box through the SAME shared combat-geometry path the
    // damage resolution, enemies, and bosses use (`collision_aabb`), so the
    // overlay provably draws the gameplay hurtbox by construction rather than a
    // parallel computation that could drift. Identity under vertical gravity.
    let body = ambition::actors::features::collision_aabb(
        &ambition::actors::features::SimpleActorGeometry {
            pos: clusters.kinematics.pos,
            size: clusters.kinematics.size,
            facing: clusters.kinematics.facing,
            frame_down: gravity_dir,
        },
    );
    if developer_tools.show_player_hitbox {
        draw_aabb_styled(gizmos, world, body, cyan(), developer_tools);
        label_box(labels, body, "player", cyan(), LabelSpot::TopLeft);
    }

    let center = w2(world, pos);

    if developer_tools.show_player_vectors {
        let velocity_delta = engine_delta_to_bevy(vel * 0.18);
        draw_arrow(gizmos, center, center + velocity_delta, blue());

        let facing_end = center + BVec2::new(facing * 58.0, 0.0);
        draw_arrow(gizmos, center, facing_end, green());

        if on_ground {
            let feet = w2(world, ae::Vec2::new(pos.x, body.bottom()));
            draw_arrow(gizmos, feet, feet + BVec2::new(0.0, 44.0), green());
        }
        if on_wall {
            let side_x = if wall_normal_x < 0.0 {
                body.left()
            } else {
                body.right()
            };
            let side = w2(world, ae::Vec2::new(side_x, pos.y));
            draw_arrow(
                gizmos,
                side,
                side + BVec2::new(wall_normal_x * 48.0, 0.0),
                green(),
            );
        }
    }

    // Combat preview: an ACTIVE swing draws its real phase hitbox (startup =
    // yellow, active = red, recovery = gray). `controls` also feeds the blink-aim
    // debug below.
    let controls = actions.map(read_gameplay_control_frame).unwrap_or_default();
    if gameplay_active && developer_tools.show_combat_preview {
        let view = ambition::actors::combat::AttackView {
            pos,
            size,
            facing,
            on_ground,
            wall_clinging: facts.wall_clinging,
            dashing: facts.dashing,
            abilities_directional_primary: clusters.abilities.abilities.directional_primary,
        };
        if let Some(attack_state) = attack {
            // Draw the ACTUAL damage volume — the authored blade-arc poly (or the
            // hardcoded AABB fallback) the slash emits — not a separate preview
            // box, so the overlay matches what hits.
            let volume = ambition::actors::features::ecs::attack::player_attack_hitbox(
                character_catalog,
                authored_attack_volumes,
                Some(worn_character_id),
                &view,
                attack_state.spec.intent,
                gravity_dir,
            )
            .unwrap_or_else(|| {
                ambition::actors::combat::attack_hitbox_from_view(&view, attack_state.spec).into()
            });
            let color = match attack_state.phase() {
                Some(ambition::actors::combat::AttackPhase::Startup) => yellow(),
                Some(ambition::actors::combat::AttackPhase::Active) => red(),
                Some(ambition::actors::combat::AttackPhase::Recovery) => gray(),
                None => gray(),
            };
            draw_combat_volume(gizmos, world, &volume, color);
            label_box(labels, volume.bounds(), "atk", color, LabelSpot::TopRight);
        }
        // (The old yellow "where the swing WOULD land" preview that drew while
        // merely HOLDING attack between swings was removed — it dealt no damage
        // and read as a confusing stray box. The active swing above draws its real
        // gravity-correct hitbox, which is the only box that matters.)
    }

    // Ledge grab / climb debug (anchor + climb target are policy-private
    // internals — a dev overlay is allowed to look).
    if developer_tools.show_combat_preview {
        let axis_ledge = match motion_model {
            ae::MotionModel::AxisSwept(axis) => axis.state.ledge_grab.as_ref(),
            _ => None,
        };
        if let Some(ledge) = axis_ledge {
            let anchor_box = ae::Aabb::new(ledge.contact.anchor, ae::Vec2::splat(5.0));
            let target_box = ae::Aabb::new(ledge.contact.climb_target, size * 0.35);
            draw_aabb(gizmos, world, anchor_box, cyan());
            draw_aabb(
                gizmos,
                world,
                target_box,
                if ledge.climbing { green() } else { yellow() },
            );
            draw_arrow(
                gizmos,
                w2(world, ledge.contact.anchor),
                w2(world, ledge.contact.climb_target),
                if ledge.climbing { green() } else { yellow() },
            );
        }
    }

    // Blink aim preview.
    if gameplay_active
        && developer_tools.show_blink_preview
        && (controls.blink_held || facts.blink_aiming)
    {
        let blink_world = platforms::world_with_moving_platforms(world, moving_platforms);
        let (desired, target) = if facts.blink_aiming {
            let desired = pos + facts.blink_aim_offset;
            let target = ae::blink_destination_to_point_clusters(
                &blink_world,
                clusters.kinematics,
                clusters.abilities,
                desired,
            );
            (desired, target)
        } else {
            let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
                .normalize_or(ae::Vec2::new(facing, 0.0));
            let desired = pos + aim * ae::BLINK_DISTANCE;
            let target = ae::blink_destination_clusters(
                &blink_world,
                clusters.kinematics,
                clusters.abilities,
                aim,
                ae::BLINK_DISTANCE,
            );
            (desired, target)
        };
        let target_center = w2(world, target);
        draw_arrow(gizmos, center, target_center, magenta());
        draw_aabb(gizmos, world, ae::Aabb::new(target, size * 0.5), magenta());
        if (desired - target).length_squared() > 4.0 {
            draw_aabb(gizmos, world, ae::Aabb::new(desired, size * 0.35), red());
            gizmos.line_2d(w2(world, desired), target_center, red());
        }
    }

    // Small status ticks above the player: dash and air jump availability.
    let meter_y = body.top() - 18.0;
    let abilities = &clusters.abilities.abilities;
    let dash_slots = abilities.dash_charge_count().max(1) as usize;
    for i in 0..dash_slots {
        let x0 = pos.x - 28.0 + i as f32 * 12.0;
        let color = if i < clusters.dash.charges_available as usize {
            yellow()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 8.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
    let air_jump_slots = abilities.air_jump_count(ae::AIR_JUMPS).max(1) as usize;
    for i in 0..air_jump_slots {
        let x0 = pos.x + 6.0 + i as f32 * 11.0;
        let color = if i < clusters.jump.air_jumps_available as usize {
            cyan()
        } else {
            gray()
        };
        let a = w2(world, ae::Vec2::new(x0, meter_y));
        let b = w2(world, ae::Vec2::new(x0 + 7.0, meter_y));
        gizmos.line_2d(a, b, color);
    }
}

pub(crate) fn draw_moving_platform_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    moving_platforms: &[ambition::actors::world::platforms::MovingPlatformState],
) {
    for platform in moving_platforms {
        let aabb = platform.aabb();
        draw_aabb(gizmos, world, aabb, blue());
        let center = w2(world, aabb.center());
        draw_arrow(gizmos, center, center + BVec2::new(44.0, 0.0), blue());
    }
}

pub(crate) fn draw_health_bars(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_aabb: ae::Aabb,
    player_health: Option<&ambition::characters::actor::BodyHealth>,
) {
    let ratio = player_health.map_or(1.0, |h| h.health.ratio());
    draw_health_bar(gizmos, world, player_aabb, ratio, cyan());
    // Enemy / boss / breakable health bars are now drawn by
    // `sync_health_overlays` (the Bevy sprite overlay system), which reads
    // ECS actor disposition, boss cluster, and `BreakableFeature` components.
}

pub(crate) fn draw_health_bar(
    gizmos: &mut Gizmos,
    world: &ae::World,
    aabb: ae::Aabb,
    ratio: f32,
    fill: Color,
) {
    let width = (aabb.half_size().x * 2.0).max(28.0);
    let y = aabb.top() - 14.0;
    let left = aabb.center().x - width * 0.5;
    let right = aabb.center().x + width * 0.5;
    let fill_right = left + width * ratio.clamp(0.0, 1.0);
    gizmos.line_2d(
        w2(world, ae::Vec2::new(left, y)),
        w2(world, ae::Vec2::new(right, y)),
        gray(),
    );
    gizmos.line_2d(
        w2(world, ae::Vec2::new(left, y)),
        w2(world, ae::Vec2::new(fill_right, y)),
        fill,
    );
}

/// Draw debug rectangles for every gameplay feature (NPCs, enemies, bosses,
/// breakables, chests, hazards). Also overlays boss attack telegraph + active
/// volumes when an attack is firing. This is the "solid box" view the player
/// expects when `Hide Sprites` is also on — sprites disappear and the boxes
/// reveal exactly where each entity lives.
pub(crate) fn draw_feature_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    feature_q: &FeatureDebugQueries,
    // The primary player's (entity, box-center) — resolves the owner position of a
    // player-owned FollowOwner strike, which has no `CenteredAabb`. Passed in (not
    // queried here) because the player's `BodyKinematics` is borrowed `&mut` by the
    // overlay's cluster query, so a second read here would be a B0001 conflict.
    primary_player: Option<(bevy::prelude::Entity, ae::Vec2)>,
    developer_tools: &DeveloperTools,
    labels: &mut DebugOverlayLabels,
) {
    // Colors per role — strong enough to read against most backgrounds.
    let npc_color = Color::srgba(0.30, 1.00, 0.45, 0.85); // green
    let enemy_color = Color::srgba(1.00, 0.32, 0.32, 0.88); // red
    let boss_color = Color::srgba(1.00, 0.60, 0.10, 0.88); // orange
    let breakable_color = Color::srgba(0.55, 0.80, 1.00, 0.80); // light blue
    let chest_color = Color::srgba(1.00, 0.85, 0.25, 0.85); // gold
    let hazard_color = Color::srgba(1.00, 0.32, 0.92, 0.80); // magenta
    let telegraph_color = Color::srgba(1.00, 0.95, 0.20, 0.60); // yellow
    let active_color = Color::srgba(1.00, 0.12, 0.12, 0.95); // bright red

    // "fighting" (in a faction feud) is amber — distinct from "hostile" (after a
    // controlled character) red and "peaceful" green.
    let fighting_color = Color::srgba(1.00, 0.78, 0.20, 0.88);
    for (disposition, aggression, aabb, kin, attack, _surface) in feature_q.actors.iter() {
        // State is DERIVED, not a stored actor TYPE: an actor is "fighting" while it
        // has a combat target (the disposition stands down to peaceful the instant
        // the target is gone — a duel winner, an enemy that lost the player). The
        // label refines that: "hostile" when the target is a controlled character
        // (mode HostileToPlayer — debug-label convenience, true for any controlled
        // char incl. co-op), "fighting" when it's a faction-foe (HostileToFaction),
        // "peaceful" when it has no target. ("enemy"/"npc" was a misnomer — these are
        // states, not classes.)
        let fighting = disposition.is_hostile();
        let (actor_label, color) = if !fighting {
            ("peaceful", npc_color)
        } else if aggression.grudge.is_none() {
            // Fighting along faction lines only (a duel combatant / born enemy),
            // no personal grudge — distinct from a provoked actor hunting the
            // specific entity that struck it.
            ("fighting", fighting_color)
        } else {
            ("hostile", enemy_color)
        };
        // `CenteredAabb` is already oriented to the actor's surface (a clung
        // surface-walker swaps width<->height onto a wall — see
        // `update_ecs_actors`), so the drawn box matches the rotated sprite.
        draw_aabb_styled(gizmos, world, aabb.aabb(), color, developer_tools);
        label_box(labels, aabb.aabb(), actor_label, color, LabelSpot::TopLeft);
        // A FIGHTING actor (hostile to the player or in a faction feud) owns an
        // attack volume that becomes active during a swing — draw it whenever windup
        // or strike timer is live so the player can see exactly where the hit will
        // land. Telegraph wins when both are zero so a frame on the edge still reads
        // as "incoming".
        if fighting {
            if let (Some(kin), Some(attack)) = (kin, attack) {
                // Forward-swing hitbox geometry (matches
                // ActorMut::attack_aabb): offset by facing.
                let center = kin.pos
                    + ambition::engine_core::Vec2::new(
                        kin.facing * (kin.size.x * 0.55 + 24.0),
                        -4.0,
                    );
                let attack_box = ambition::engine_core::Aabb::new(
                    center,
                    ambition::engine_core::Vec2::new(34.0, 28.0),
                );
                if attack.is_active() {
                    draw_aabb_styled(gizmos, world, attack_box, active_color, developer_tools);
                    label_box(labels, attack_box, "atk", active_color, LabelSpot::Center);
                } else if attack.is_winding_up() {
                    draw_aabb_styled(gizmos, world, attack_box, telegraph_color, developer_tools);
                    label_box(
                        labels,
                        attack_box,
                        "atk",
                        telegraph_color,
                        LabelSpot::Center,
                    );
                }
            }
        }
    }
    // Boss debug colors — each color answers a distinct question
    // the player might ask while reading the overlay:
    //
    // - **orange** (`boss_color`, `boss.aabb()`): the combat-collision
    //   envelope. The boss uses this for kinematic step / world-bounds
    //   clamp. Does NOT, by itself, deal damage.
    // - **cyan** (`hurtbox_color`, `damageable_volumes`): where the
    //   *player's* attacks register hits on the boss. With the
    //   sprite-metadata-driven derivation, this can be one rect
    //   (single-piece boss) or many (multi-part body — head + body
    //   + arms).
    // - **magenta** (`body_contact_color`, `body_damage_aabb`): the
    //   boss's body-contact damage zone. Touching this when
    //   `BossBehaviorProfile::body_damage > 0` hurts the player
    //   (e.g. clockwork_warden has body_damage=1). Drawn separately
    //   so the player can answer "why did I get hit by just touching
    //   the boss?" without source-diving.
    // - **red** (`active_color`, `active_attack_volumes`): live
    //   strike volumes. These are also the source of `boss_attack_damage`.
    //
    // Special attack profiles (PitTrap, RotatingCross, HazardColumn,
    // MemorizedVolley, MinionCascade) route damage through World-
    // anchored `Hitbox` entities, drawn by the later
    // `feature_q.hitboxes` pass with faction colors.
    let hurtbox_color = cyan();
    let body_contact_color = Color::srgba(0.95, 0.30, 0.95, 0.85); // magenta
    for (bf, health, attack_state, animation_frame) in feature_q.bosses.iter() {
        let boss = bf.as_boss_ref();
        if !health.alive() {
            continue;
        }
        let ctx = ambition::actors::features::BossVolumeContext::from_ref(
            &feature_q.boss_catalog,
            bf.as_boss_ref(),
            attack_state,
        )
        .with_animation_frame(animation_frame);
        draw_aabb_styled(gizmos, world, boss.aabb(), boss_color, developer_tools);
        label_box(
            labels,
            boss.aabb(),
            "collision",
            boss_color,
            LabelSpot::BottomLeft,
        );
        // Body-contact damage zone — drawn ONLY when the boss
        // actually deals contact damage so a `body_damage = 0`
        // boss (like GNU-ton) doesn't show a misleading magenta
        // outline.
        if boss.config.behavior.body_damage > 0 {
            // Use `boss.aabb()` directly — that already factors in
            // `combat_offset` so the magenta box lines up with the
            // visible body (and matches the pogo zone, which uses
            // the same call).
            draw_aabb_styled(
                gizmos,
                world,
                boss.aabb(),
                body_contact_color,
                developer_tools,
            );
            label_box(
                labels,
                boss.aabb(),
                "contact",
                body_contact_color,
                LabelSpot::BottomRight,
            );
        }
        for hurtbox in ambition::actors::features::damageable_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, hurtbox, hurtbox_color, developer_tools);
            label_box(
                labels,
                hurtbox,
                "hurtbox",
                hurtbox_color,
                LabelSpot::TopLeft,
            );
        }
        for vol in ambition::actors::features::active_attack_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, vol, active_color, developer_tools);
            label_box(labels, vol, "active", active_color, LabelSpot::Center);
        }
    }
    for aabb in feature_q.breakables.iter() {
        draw_aabb_styled(gizmos, world, aabb.aabb(), breakable_color, developer_tools);
        label_box(
            labels,
            aabb.aabb(),
            "breakable",
            breakable_color,
            LabelSpot::TopLeft,
        );
    }
    for aabb in feature_q.chests.iter() {
        draw_aabb_styled(gizmos, world, aabb.aabb(), chest_color, developer_tools);
        label_box(
            labels,
            aabb.aabb(),
            "chest",
            chest_color,
            LabelSpot::TopLeft,
        );
    }
    for hf in feature_q.hazards.iter() {
        draw_aabb_styled(
            gizmos,
            world,
            hf.hazard.aabb(),
            hazard_color,
            developer_tools,
        );
        label_box(
            labels,
            hf.hazard.aabb(),
            "hazard",
            hazard_color,
            LabelSpot::TopLeft,
        );
    }

    // Live Hitbox entities — melee swings (FollowOwner) + World-
    // anchored boss specials (PitTrap pit, RotatingCross arms,
    // HazardColumn column). Without this pass, the World-anchored
    // hitboxes are invisible in debug mode even though they deal
    // damage. Faction-color-coded so you can read which side it
    // belongs to at a glance.
    let player_hitbox_color = Color::srgba(0.35, 0.85, 1.00, 0.90); // light blue
    let enemy_hitbox_color = Color::srgba(1.00, 0.18, 0.18, 0.90); // bright red
    let boss_hitbox_color = Color::srgba(1.00, 0.55, 0.10, 0.90); // bright orange
    let npc_hitbox_color = Color::srgba(0.60, 1.00, 0.45, 0.85); // light green
    for hitbox in feature_q.hitboxes.iter() {
        // Resolve the owner's box center: actors carry `CenteredAabb`, the player
        // carries `BodyKinematics` (same fallback the damage system uses). A
        // FollowOwner hitbox with NO resolvable owner pos is SKIPPED rather than
        // drawn at the world origin — drawing it at ZERO was the "stray hit:player
        // box in the top-left" smell (the player's strike, owner-pos unresolved).
        // World-anchored hitboxes carry their own center, so owner pos is moot.
        let owner_pos = if let Ok(aabb) = feature_q.hitbox_owners.get(hitbox.owner) {
            Some(aabb.center)
        } else if let Some((player_entity, player_pos)) = primary_player {
            (hitbox.owner == player_entity).then_some(player_pos)
        } else {
            None
        };
        let owner_pos = match (owner_pos, hitbox.anchor) {
            (Some(p), _) => p,
            // World-anchored: center is fixed at spawn, owner pos unused.
            (None, ambition::actors::features::HitboxAnchor::World { .. }) => ae::Vec2::ZERO,
            // FollowOwner with a dead/unknown owner: don't draw a ghost at origin.
            (None, ambition::actors::features::HitboxAnchor::FollowOwner { .. }) => continue,
        };
        // Draw the hitbox's TRUE damage volume — the authored convex blade / OBB /
        // circle the strike actually resolves against, not just its AABB. When a
        // hull is present the bounding box is drawn faint + vestigial; a bare-box
        // hitbox draws as the box itself. (The player's melee resolves the authored
        // per-clip poly through the moveset, so "hit:player" now shows that poly.)
        let volume = hitbox.world_volume(owner_pos);
        let (color, tag) = match hitbox.source {
            ambition::vfx::HitSide::Player => (player_hitbox_color, "hit:player"),
            ambition::vfx::HitSide::Enemy => (enemy_hitbox_color, "hit:enemy"),
            ambition::vfx::HitSide::Boss => (boss_hitbox_color, "hit:boss"),
            ambition::vfx::HitSide::Npc | ambition::vfx::HitSide::Neutral => {
                (npc_hitbox_color, "hit:npc")
            }
        };
        draw_hitbox_volume(gizmos, world, &volume, color, developer_tools);
        label_box(labels, volume.bounds(), tag, color, LabelSpot::TopRight);
    }
}

/// Draw in-flight player and enemy projectile AABBs so they remain
/// visible when `hide_sprites` strips the textured projectile ring.
/// Player projectiles use a warm orange (matches charge tint); enemy
/// projectiles use red so the faction is immediately readable.
pub(crate) fn draw_projectile_debug<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_bodies: impl IntoIterator<Item = &'a ambition::actors::actor::BodyKinematics>,
    enemy_bodies: impl IntoIterator<Item = &'a ambition::actors::actor::BodyKinematics>,
    developer_tools: &DeveloperTools,
) {
    let player_color = Color::srgba(1.00, 0.74, 0.30, 0.92);
    let enemy_color = Color::srgba(1.00, 0.32, 0.32, 0.92);
    for kin in player_bodies {
        draw_aabb_styled(gizmos, world, kin.aabb(), player_color, developer_tools);
    }
    for kin in enemy_bodies {
        draw_aabb_styled(gizmos, world, kin.aabb(), enemy_color, developer_tools);
    }
}

pub(crate) fn draw_rebound_vectors(gizmos: &mut Gizmos, world: &ae::World) {
    for block in &world.blocks {
        let ae::BlockKind::Rebound { impulse } = block.kind else {
            continue;
        };
        draw_aabb(gizmos, world, block.aabb, orange());
        let start = w2(world, block.aabb.center());
        let direction = impulse.normalize_or(ae::Vec2::new(0.0, -1.0));
        let end = start + engine_delta_to_bevy(direction * 70.0);
        draw_arrow(gizmos, start, end, orange());
    }
}
