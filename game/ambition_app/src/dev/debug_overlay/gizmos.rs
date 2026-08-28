//! The GAME-SPECIFIC debug-overlay layers — player policy internals, feature
//! /projectile/portal debug draws, health bars, camera frame, LDtk spine.
//!
//! The engine-generic world layers (bounds/blocks/chains/grids/rebound/moving
//! platforms) moved to `ambition_platformer2d::render::rendering::debug_viz` — any game
//! opts into those via `DebugVizPlugin`; this overlay imports them back
//! through the parent module's re-export and composes them with the layers
//! here.

use super::*;

/// Draw each in-flight held-item shot (gun-sword bolt / Fireball): its solid
/// contact box (the box that registers a hit — `HeldProjectile::contact_aabb`)
/// and, for a Fireball, the fainter splash box it detonates with on contact.
#[cfg(feature = "input")]
pub(crate) fn draw_held_projectiles<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    projectiles: impl Iterator<
        Item = (
            &'a ambition_platformer2d::engine_core::BodyKinematics,
            &'a ambition_platformer2d::actors::items::pickup::HeldProjectile,
        ),
    >,
    developer_tools: &DeveloperTools,
) {
    use ambition_platformer2d::actors::items::pickup::HeldProjectile;
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
    portals: impl Iterator<Item = &'a ambition_platformer2d::portal::PlacedPortal>,
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
        let tangent = ambition_platformer2d::portal::pieces::portal_tangent(portal.normal);
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
            ambition_platformer2d::boss_encounter::BossClusterRef,
            &'static ambition_platformer2d::characters::actor::BodyHealth,
            &'static ambition_platformer2d::characters::brain::BossAttackState,
            Option<&'static ambition_platformer2d::boss_encounter::attack_geometry::BossAnimationFrameSample>,
        ),
        With<ambition_platformer2d::actor::FeatureSimEntity>,
    >,
    pub actors: Query<
        'w,
        's,
        (
            &'static ambition_platformer2d::combat::components::ActorDisposition,
            &'static ambition_platformer2d::combat::components::ActorAggression,
            &'static ambition_platformer2d::combat::components::CenteredAabb,
        ),
        With<ambition_platformer2d::actor::FeatureSimEntity>,
    >,
    pub breakables: Query<
        'w,
        's,
        &'static ambition_platformer2d::combat::components::CenteredAabb,
        (
            With<ambition_platformer2d::actor::FeatureSimEntity>,
            With<ambition_platformer2d::combat::components::BreakableFeature>,
        ),
    >,
    pub chests: Query<
        'w,
        's,
        &'static ambition_platformer2d::combat::components::CenteredAabb,
        (
            With<ambition_platformer2d::actor::FeatureSimEntity>,
            With<ambition_platformer2d::combat::components::ChestFeature>,
        ),
    >,
    pub hazards: Query<
        'w,
        's,
        &'static ambition_platformer2d::world::HazardFeature,
        With<ambition_platformer2d::actor::FeatureSimEntity>,
    >,
    /// Body-generic combat geometry read-model. Live hitboxes and effective
    /// hurtboxes are extracted once from simulation truth and shared by every
    /// host; this richer overlay consumes the same rows as standalone games.
    pub combat_geometry: Res<'w, ambition_platformer2d::sim_view::CombatGeometryView>,
    /// This frame's presentation translation for each body. Authoritative
    /// combat geometry is tick-clock, and the body beside it is resampled on the
    /// frame clock, so every row of that body has to take the same translation
    /// or the overlay reports attachment it does not have —
    /// `presentation_deltas` performs that join and the shared draw applies it.
    /// One component, every body: a boss answers here exactly as a player does.
    pub body_deltas: Query<'w, 's, &'static ambition_platformer2d::sim_view::PresentedPose>,
    /// In-flight held-item shots (gun-sword bolt / Fireball). Lives in this bundle (not a top-level
    /// param) to keep `draw_debug_overlay` under Bevy's 16-system-param ceiling.
    /// `Without<PlayerEntity>` keeps this read of `BodyKinematics` disjoint from the `&mut` player
    /// query (a held shot is never the player) — B0001.
    pub held_projectiles: Query<
        'w,
        's,
        (
            &'static ambition_platformer2d::engine_core::BodyKinematics,
            &'static ambition_platformer2d::actors::items::pickup::HeldProjectile,
        ),
        Without<ambition_platformer2d::platformer::markers::PlayerEntity>,
    >,
    /// The player's resolved gravity, so the player debug box can rotate to
    /// match its (now gravity-oriented) collision box + sprite. Lives in this
    /// bundle (not a top-level param) to keep `draw_debug_overlay` under Bevy's
    /// 16-system-param ceiling.
    pub gravity: Option<Res<'w, ambition_platformer2d::world::GravityField>>,
    /// App-local character authority and attack-volume bridge used by the combat
    /// preview. Keeping them in this bundle preserves the top-level system's
    /// parameter budget.
    pub character_catalog:
        Res<'w, ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog>,
    pub boss_catalog: Res<'w, ambition_platformer2d::boss_encounter::BossCatalog>,
    pub authored_attack_volumes:
        Res<'w, ambition_platformer2d::combat::authored_volumes::AuthoredAttackVolumeResolver>,
    /// Every in-flight projectile, with its frozen combat side when it has one.
    /// Bundled here so `draw_debug_overlay` stays under Bevy's parameter ceiling.
    /// Presentation vocabulary does not decide debug faction color.
    pub live_projectiles: Query<
        'w,
        's,
        (
            &'static ambition_platformer2d::engine_core::BodyKinematics,
            Option<&'static ambition_platformer2d::actors::projectile::ProjectileAllegiance>,
        ),
        (
            With<ambition_platformer2d::projectiles::LiveProjectile>,
            Without<ambition_platformer2d::platformer::markers::PlayerEntity>,
        ),
    >,
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
    spine_index: &ambition_platformer2d::ldtk_map::LdtkRuntimeSpineIndex,
) {
    for entity in &spine_index.entities {
        let color = match entity.role {
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::PlayerStart => green(),
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::LoadingZone => {
                Color::srgba(1.0, 1.0, 1.0, 0.70)
            }
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::DebugLabel => magenta(),
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::CameraZone => blue(),
            // Solid runtime rects are drawn by the dedicated Solid index pass
            // so they can be color-keyed against the JSON-derived collision
            // blocks during the Step 2 raw-vs-runtime overlay work.
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::Solid => continue,
            // OneWayPlatform / DamageVolume have their own dedicated runtime
            // indices and overlay passes; skip them in the generic spine
            // overlay so colors don't double-stamp.
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::OneWayPlatform => continue,
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::DamageVolume => continue,
            ambition_platformer2d::ldtk_map::LdtkRuntimeRole::Other => continue,
        };
        draw_aabb(gizmos, world, entity.aabb(), color);
    }
}

#[cfg(feature = "input")]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_player_debug(
    gizmos: &mut Gizmos,
    world: &ae::World,
    character_catalog: &ambition_platformer2d::characters::actor::character_catalog::CharacterCatalog,
    authored_attack_volumes: &ambition_platformer2d::combat::authored_volumes::AuthoredAttackVolumeResolver,
    worn_character_id: &str,
    clusters: &ae::BodyClustersMut<'_>,
    // Where the body is DRAWN this frame — the frame-clock presented
    // position, not `clusters.kinematics.pos`.
    //
    // The overlay is drawn through a camera that eases every RENDERED frame, while the cluster pose
    // advances only on SIM TICKS. Placing the box at the tick pose therefore makes it a step
    // function sampled by a smoothly moving observer: the box shakes at the tick rate while the
    // simulation behind it is perfectly regular, and while the SPRITE beside it — which does read
    // the presented pose — sits still.
    //
    // It costs no truthfulness. The box's size, shape, and relationship to the
    // art are unchanged — only its sub-tick sampling phase now matches its
    // viewer's. `PresentedPose::authoritative` is still there for an overlay that
    // deliberately wants to SHOW the extrapolation lead.
    draw_pos: ae::Vec2,
    // Dev-tool read: the overlay draws the policy's private internals (the
    // ledge anchor/climb-target, the live blink aim) straight off the model.
    motion_model: &ae::MotionModel,
    // The reticle could therefore point through a wall the blink stops at. The caller passes the
    // one collision read-API's answer now, so the preview and the action cannot disagree.
    blink_world: &ae::World,
    attack: Option<&ambition_platformer2d::combat::components::MeleeSwing>,
    actions: Option<&ActionState<Platformer2dInputActionMonolith>>,
    gameplay_active: bool,
    developer_tools: &DeveloperTools,
    gravity_dir: ae::Vec2,
    labels: &mut DebugOverlayLabels,
) {
    // Everything this function PLACES rides the frame clock; the facts it
    // reports (velocity, stance, contacts) still come from the clusters.
    let pos = draw_pos;
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
    let body = ambition_platformer2d::boss_encounter::attack_geometry::collision_aabb(
        &ambition_platformer2d::boss_encounter::attack_geometry::SimpleActorGeometry {
            // The presented centre, with the size/facing/frame the sim published:
            // the shared-geometry guarantee above is about the SHAPE, and moving
            // the centre onto the render clock leaves that untouched.
            pos,
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
        let view = ambition_platformer2d::combat::AttackView {
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
            let volume = ambition_platformer2d::combat::attack_support::player_attack_hitbox(
                character_catalog,
                authored_attack_volumes,
                Some(worn_character_id),
                &view,
                attack_state.spec.intent,
                gravity_dir,
            )
            .unwrap_or_else(|| {
                ambition_platformer2d::combat::attack_hitbox_from_view(&view, attack_state.spec)
                    .into()
            });
            let color = match attack_state.phase() {
                Some(ambition_platformer2d::combat::AttackPhase::Startup) => yellow(),
                Some(ambition_platformer2d::combat::AttackPhase::Active) => red(),
                Some(ambition_platformer2d::combat::AttackPhase::Recovery) => gray(),
                None => gray(),
            };
            draw_combat_volume(gizmos, world, &volume, color);
            label_box(labels, volume.bounds(), "atk", color, LabelSpot::TopRight);
        }
        // The active swing above draws its real gravity-correct hitbox, which is the only box
        // that matters.)
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
        let (desired, target) = if facts.blink_aiming {
            let desired = pos + facts.blink_aim_offset;
            let target = ae::blink_destination_to_point_clusters(
                blink_world,
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
                blink_world,
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

pub(crate) fn draw_health_bars(
    gizmos: &mut Gizmos,
    world: &ae::World,
    player_aabb: ae::Aabb,
    player_health: Option<&ambition_platformer2d::characters::actor::BodyHealth>,
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
    let active_color = Color::srgba(1.00, 0.12, 0.12, 0.95); // bright red

    // "fighting" (in a faction feud) is amber — distinct from "hostile" (after a
    // controlled character) red and "peaceful" green.
    let fighting_color = Color::srgba(1.00, 0.78, 0.20, 0.88);
    for (disposition, aggression, aabb) in feature_q.actors.iter() {
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
        // Actor strike geometry is deliberately NOT reconstructed here. Live
        // body-owned strikes are published by `CombatGeometryView` from the
        // authoritative `Hitbox` entities and drawn by the shared debug layer.
        // Keeping this pass to actor state/collision avoids a second, synthetic
        // "attack box" that can disagree with the geometry that actually hits.
    }
    // Boss debug colors — each color answers a distinct question
    // the player might ask while reading the overlay:
    //
    // - orange (`boss_color`, `boss.aabb()`): the combat-collision
    //   envelope. The boss uses this for kinematic step / world-bounds
    //   clamp. Does NOT, by itself, deal damage.
    // - cyan (`hurtbox_color`, `damageable_volumes`): where the
    //   *player's* attacks register hits on the boss. With the
    //   sprite-metadata-driven derivation, this can be one rect
    //   (single-piece boss) or many (multi-part body — head + body
    //   + arms).
    // - magenta (`body_contact_color`, `body_damage_aabb`): the
    //   boss's body-contact damage zone. Touching this when
    //   `BossBehaviorProfile::body_damage > 0` hurts the player
    //   (e.g. clockwork_warden has body_damage=1). Drawn separately
    //   so the player can answer "why did I get hit by just touching
    //   the boss?" without source-diving.
    // - red (`active_color`, `active_attack_volumes`): live
    //   strike volumes. These are also the source of `boss_attack_damage`.
    //
    // Special attack profiles (PitTrap, RotatingCross, HazardColumn,
    // MemorizedVolley, MinionCascade) route damage through World-anchored
    // `Hitbox` entities, drawn by the shared `CombatGeometryView` layer.
    let hurtbox_color = cyan();
    let body_contact_color = Color::srgba(0.95, 0.30, 0.95, 0.85); // magenta
    for (bf, health, attack_state, animation_frame) in feature_q.bosses.iter() {
        let boss = bf.as_boss_ref();
        if !health.alive() {
            continue;
        }
        let ctx =
            ambition_platformer2d::boss_encounter::attack_geometry::BossVolumeContext::from_ref(
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
        for hurtbox in
            ambition_platformer2d::boss_encounter::attack_geometry::damageable_volumes(&ctx)
        {
            // The published silhouette's REAL shape — a boss part may be an
            // authored hull, and drawing its bounding box here is how an
            // overlay tells you a lie that looks like a measurement.
            draw_hitbox_volume(gizmos, world, &hurtbox, hurtbox_color, developer_tools);
            label_box(
                labels,
                hurtbox.bounds(),
                "hurtbox",
                hurtbox_color,
                LabelSpot::TopLeft,
            );
        }
        for vol in
            ambition_platformer2d::boss_encounter::attack_geometry::active_attack_volumes(&ctx)
        {
            draw_hitbox_volume(gizmos, world, &vol, active_color, developer_tools);
            label_box(
                labels,
                vol.bounds(),
                "active",
                active_color,
                LabelSpot::Center,
            );
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

    // Live hitboxes and effective body hurtboxes are drawn by the shared
    // `CombatGeometryView` layer in the parent overlay. Keeping that truth in
    // the observation boundary means this app-specific pass no longer needs a
    // privileged-player owner fallback or its own strike-geometry resolver.
}

/// Draw every in-flight projectile AABB so shots remain visible when
/// `hide_sprites` strips textured art. Color comes from frozen combat allegiance,
/// never from whether the shot used named or open-visual presentation.
pub(crate) fn draw_projectile_debug<'a>(
    gizmos: &mut Gizmos,
    world: &ae::World,
    projectiles: impl IntoIterator<
        Item = (
            &'a ambition_platformer2d::engine_core::BodyKinematics,
            Option<&'a ambition_platformer2d::actors::projectile::ProjectileAllegiance>,
        ),
    >,
    developer_tools: &DeveloperTools,
) {
    let player_color = Color::srgba(1.00, 0.74, 0.30, 0.92);
    let hostile_color = Color::srgba(1.00, 0.32, 0.32, 0.92);
    for (kin, allegiance) in projectiles {
        let color = if allegiance.is_some_and(|side| {
            side.faction == ambition_platformer2d::characters::actor::ActorFaction::Player
        }) {
            player_color
        } else {
            hostile_color
        };
        draw_aabb_styled(gizmos, world, kin.aabb(), color, developer_tools);
    }
}
