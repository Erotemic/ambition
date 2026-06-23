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
            &'a ambition_gameplay_core::player::BodyKinematics,
            &'a ambition_gameplay_core::items::pickup::HeldProjectile,
        ),
    >,
    developer_tools: &DeveloperTools,
) {
    use ambition_gameplay_core::items::pickup::HeldProjectile;
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
    portals: impl Iterator<Item = &'a ambition_gameplay_core::portal::PlacedPortal>,
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
        let tangent = ambition_gameplay_core::portal::pieces::portal_tangent(portal.normal);
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
            ambition_gameplay_core::features::BossClusterRef,
            &'static ambition_gameplay_core::brain::BossAttackState,
            Option<&'static ambition_gameplay_core::features::BossAnimationFrameSample>,
        ),
        With<ambition_gameplay_core::features::FeatureSimEntity>,
    >,
    pub actors: Query<
        'w,
        's,
        (
            &'static ambition_gameplay_core::features::ActorDisposition,
            &'static ambition_gameplay_core::features::CenteredAabb,
            Option<&'static ambition_gameplay_core::features::BodyKinematics>,
            Option<&'static ambition_gameplay_core::features::ActorAttackState>,
            Option<&'static ambition_gameplay_core::features::ActorSurfaceState>,
        ),
        With<ambition_gameplay_core::features::FeatureSimEntity>,
    >,
    pub breakables: Query<
        'w,
        's,
        &'static ambition_gameplay_core::features::CenteredAabb,
        (
            With<ambition_gameplay_core::features::FeatureSimEntity>,
            With<ambition_gameplay_core::features::BreakableFeature>,
        ),
    >,
    pub chests: Query<
        'w,
        's,
        &'static ambition_gameplay_core::features::CenteredAabb,
        (
            With<ambition_gameplay_core::features::FeatureSimEntity>,
            With<ambition_gameplay_core::features::ChestFeature>,
        ),
    >,
    pub hazards: Query<
        'w,
        's,
        &'static ambition_gameplay_core::features::HazardFeature,
        With<ambition_gameplay_core::features::FeatureSimEntity>,
    >,
    /// All live `Hitbox` entities (melee swings, World-anchored
    /// hazards like the Gradient Sentinel's PitTrap pit /
    /// RotatingCross arms / HazardColumn column). Drawn so the debug
    /// view answers "what just hit me?" — without this pass the
    /// World-anchored boss specials are invisible even though they
    /// deal damage.
    pub hitboxes: Query<'w, 's, &'static ambition_gameplay_core::features::Hitbox>,
    /// CenteredAabb lookup for resolving `FollowOwner` hitboxes to
    /// their current world-space rectangle. World-anchored
    /// hitboxes don't need this — their AABB is fixed at spawn.
    pub hitbox_owners: Query<'w, 's, &'static ambition_gameplay_core::features::CenteredAabb>,
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
            &'static ambition_gameplay_core::player::BodyKinematics,
            &'static ambition_gameplay_core::items::pickup::HeldProjectile,
        ),
        Without<ambition_gameplay_core::player::PlayerEntity>,
    >,
    /// The player's resolved gravity, so the player debug box can rotate to
    /// match its (now gravity-oriented) collision box + sprite. Lives in this
    /// bundle (not a top-level param) to keep `draw_debug_overlay` under Bevy's
    /// 16-system-param ceiling.
    pub gravity: Option<Res<'w, ambition_gameplay_core::physics::GravityField>>,
    /// In-flight player projectiles (ECS entities). Bundled here (rather than a
    /// top-level param) so `draw_debug_overlay` has a slot free for the
    /// debug-label buffer while staying under the 16-param ceiling.
    /// `Without<PlayerEntity>` proves disjointness from the `&mut` player query.
    pub player_projectiles: Query<
        'w,
        's,
        &'static ambition_gameplay_core::player::BodyKinematics,
        (
            With<ambition_gameplay_core::projectile::PlayerProjectile>,
            Without<ambition_gameplay_core::player::PlayerEntity>,
        ),
    >,
    /// In-flight enemy projectiles (ECS entities); see `player_projectiles`.
    pub enemy_projectiles: Query<
        'w,
        's,
        &'static ambition_gameplay_core::player::BodyKinematics,
        (
            With<ambition_gameplay_core::enemy_projectile::EnemyProjectile>,
            Without<ambition_gameplay_core::player::PlayerEntity>,
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

/// Lightweight coarse grid drawn straight through gizmos. Used when
/// `hide_sprites` strips the authored sprite grid so the player still
/// has a spatial reference. Spacing matches `ambition_gameplay_core::config::GRID_STEP`
/// (the same step the sprite grid spawned in `spawn_grid` uses).
pub(crate) fn draw_world_grid(gizmos: &mut Gizmos, world: &ae::World) {
    let step = ambition_gameplay_core::config::GRID_STEP;
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
    spine_index: &ambition_gameplay_core::ldtk_world::LdtkRuntimeSpineIndex,
) {
    for entity in &spine_index.entities {
        let color = match entity.role {
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::PlayerStart => green(),
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::LoadingZone => {
                Color::srgba(1.0, 1.0, 1.0, 0.70)
            }
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::DebugLabel => magenta(),
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::CameraZone => blue(),
            // Solid runtime rects are drawn by the dedicated Solid index pass
            // so they can be color-keyed against the JSON-derived collision
            // blocks during the Step 2 raw-vs-runtime overlay work.
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::Solid => continue,
            // OneWayPlatform / DamageVolume have their own dedicated runtime
            // indices and overlay passes; skip them in the generic spine
            // overlay so colors don't double-stamp.
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::OneWayPlatform => continue,
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::DamageVolume => continue,
            ambition_gameplay_core::ldtk_world::LdtkRuntimeRole::Other => continue,
        };
        draw_aabb(gizmos, world, entity.aabb(), color);
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    clusters: &ae::PlayerClustersMut<'_>,
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
    attack: Option<&ambition_gameplay_core::PlayerAttackState>,
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
    // Oriented to the player's frame so the box matches the rotated sprite + the
    // (now gravity-oriented) collision box; identity under vertical gravity.
    let body = clusters.kinematics.aabb_oriented(gravity_dir);
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

    // Combat preview: active attacks show their real phase hitbox. When no
    // swing is active, holding the button previews the resolved directional
    // intent from the live input axes. Colors mirror the attack lifecycle:
    // startup = yellow, active = red, recovery = gray.
    let controls = actions.map(ControlFrame::read_gameplay).unwrap_or_default();
    let attack_held = actions
        .map(|actions| actions.pressed(&SandboxAction::Attack))
        .unwrap_or(false);
    let dedicated_pogo_held = actions
        .map(|actions| actions.pressed(&SandboxAction::Pogo))
        .unwrap_or(false);
    if gameplay_active && developer_tools.show_combat_preview {
        let view = ambition_gameplay_core::combat::AttackView {
            pos,
            size,
            facing,
            on_ground,
            wall_clinging: clusters.wall.wall_clinging,
            dash_timer: clusters.dash.timer,
            abilities_directional_primary: clusters.abilities.abilities.directional_primary,
        };
        if let Some(attack_state) = attack {
            let hitbox =
                ambition_gameplay_core::combat::attack_hitbox_from_view(&view, attack_state.spec);
            let color = match attack_state.phase() {
                Some(ambition_gameplay_core::combat::AttackPhase::Startup) => yellow(),
                Some(ambition_gameplay_core::combat::AttackPhase::Active) => red(),
                Some(ambition_gameplay_core::combat::AttackPhase::Recovery) => gray(),
                None => gray(),
            };
            draw_aabb(gizmos, world, hitbox, color);
            label_box(labels, hitbox, "atk", color, LabelSpot::TopRight);
        } else if attack_held || dedicated_pogo_held {
            let intent = ambition_gameplay_core::combat::resolve_attack_intent_from_view(
                &view,
                controls.axis_x,
                controls.axis_y,
                dedicated_pogo_held || controls.pogo_pressed,
            );
            let frame = ae::AccelerationFrame::new(gravity_dir);
            let spec = ambition_gameplay_core::combat::attack_spec_from_view(&view, intent)
                .into_world_frame(frame);
            let hitbox = ambition_gameplay_core::combat::attack_hitbox_from_view(&view, spec);
            draw_aabb(gizmos, world, hitbox, yellow());
        }
    }

    // Ledge grab / climb debug.
    if developer_tools.show_combat_preview {
        if let Some(ledge) = clusters.ledge.grab.as_ref() {
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
        && (controls.blink_held || clusters.blink.aiming)
    {
        let blink_world = platforms::world_with_moving_platforms(world, moving_platforms);
        let (desired, target) = if clusters.blink.aiming {
            let desired = pos + clusters.blink.aim_offset;
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
    moving_platforms: &[ambition_gameplay_core::world::platforms::MovingPlatformState],
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
    player_health: Option<&ambition_gameplay_core::player::PlayerHealth>,
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

    for (disposition, aabb, kin, attack, _surface) in feature_q.actors.iter() {
        // Color/label key off disposition now — "enemy" is hostile state, not a
        // type (a provoked NPC turns red automatically).
        let hostile = disposition.is_hostile();
        let color = if hostile { enemy_color } else { npc_color };
        // `CenteredAabb` is already oriented to the actor's surface (a clung
        // surface-walker swaps width<->height onto a wall — see
        // `update_ecs_actors`), so the drawn box matches the rotated sprite.
        draw_aabb_styled(gizmos, world, aabb.aabb(), color, developer_tools);
        let actor_label = if hostile { "enemy" } else { "npc" };
        label_box(labels, aabb.aabb(), actor_label, color, LabelSpot::TopLeft);
        // Hostile actors (and turned-hostile NPCs like the Kernel Guide)
        // own an attack volume that becomes active during a swing — draw
        // it whenever windup or strike timer is live so the player can
        // see exactly where the hit will land. Telegraph wins when both
        // are zero so a frame on the edge still reads as "incoming".
        if hostile {
            if let (Some(kin), Some(attack)) = (kin, attack) {
                // Forward-swing hitbox geometry (matches
                // ActorMut::attack_aabb): offset by facing.
                let center = kin.pos
                    + ambition_gameplay_core::engine_core::Vec2::new(
                        kin.facing * (kin.size.x * 0.55 + 24.0),
                        -4.0,
                    );
                let attack_box = ambition_gameplay_core::engine_core::Aabb::new(
                    center,
                    ambition_gameplay_core::engine_core::Vec2::new(34.0, 28.0),
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
    // - **yellow** (`telegraph_color`, `telegraph_volumes`):
    //   attack windup volumes (e.g. FloorSlam telegraph).
    // - **red** (`active_color`, `active_attack_volumes`): live
    //   strike volumes. These are also the source of `boss_attack_damage`.
    //
    // Special attack profiles (PitTrap, RotatingCross, HazardColumn,
    // MemorizedVolley, MinionCascade) route damage through World-
    // anchored `Hitbox` entities, drawn by the later
    // `feature_q.hitboxes` pass with faction colors.
    let hurtbox_color = cyan();
    let body_contact_color = Color::srgba(0.95, 0.30, 0.95, 0.85); // magenta
    for (bf, attack_state, animation_frame) in feature_q.bosses.iter() {
        let boss = bf.as_boss_ref();
        if !boss.status.alive {
            continue;
        }
        let ctx = ambition_gameplay_core::features::BossVolumeContext::from_ref(
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
        for hurtbox in ambition_gameplay_core::features::damageable_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, hurtbox, hurtbox_color, developer_tools);
            label_box(
                labels,
                hurtbox,
                "hurtbox",
                hurtbox_color,
                LabelSpot::TopLeft,
            );
        }
        for vol in ambition_gameplay_core::features::telegraph_volumes(&ctx) {
            draw_aabb_styled(gizmos, world, vol, telegraph_color, developer_tools);
            label_box(
                labels,
                vol,
                "telegraph",
                telegraph_color,
                LabelSpot::TopRight,
            );
        }
        for vol in ambition_gameplay_core::features::active_attack_volumes(&ctx) {
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
        let owner_pos = match feature_q.hitbox_owners.get(hitbox.owner) {
            Ok(aabb) => aabb.center,
            // Owner despawned or never had a CenteredAabb — for
            // World-anchored hitboxes this doesn't matter (the
            // anchor carries the center). For FollowOwner with a
            // dead owner the draw position is ambiguous; fall back
            // to ZERO and the rect will appear at the origin.
            Err(_) => ae::Vec2::ZERO,
        };
        let aabb = hitbox.world_aabb(owner_pos);
        let (color, tag) = match hitbox.source {
            ambition_gameplay_core::features::ActorFaction::Player => {
                (player_hitbox_color, "hit:player")
            }
            ambition_gameplay_core::features::ActorFaction::Enemy => {
                (enemy_hitbox_color, "hit:enemy")
            }
            ambition_gameplay_core::features::ActorFaction::Boss => (boss_hitbox_color, "hit:boss"),
            ambition_gameplay_core::features::ActorFaction::Npc
            | ambition_gameplay_core::features::ActorFaction::Neutral => {
                (npc_hitbox_color, "hit:npc")
            }
        };
        draw_aabb_styled(gizmos, world, aabb, color, developer_tools);
        label_box(labels, aabb, tag, color, LabelSpot::TopRight);
    }
}

/// Draw in-flight player and enemy projectile AABBs so they remain
/// visible when `hide_sprites` strips the textured projectile ring.
/// Player projectiles use a warm orange (matches charge tint); enemy
/// projectiles use red so the faction is immediately readable.
pub(crate) fn draw_projectile_debug<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_bodies: impl IntoIterator<Item = &'a ambition_gameplay_core::player::BodyKinematics>,
    enemy_bodies: impl IntoIterator<Item = &'a ambition_gameplay_core::player::BodyKinematics>,
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
