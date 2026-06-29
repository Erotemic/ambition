//! Portal mechanic facade + the Ambition host adapter for portal presentation.
//!
//! The portal **mechanic** (the portal gun place/replace/channel, the one
//! generic aperture transit over `PortalBody` + `PortalPolicy`, placement +
//! transit math, carve publishing, pieces geometry, lifecycle, the pure shot
//! helper over `SolidWorldQuery`, the portal events + `PortalPlugin`) lives in
//! the standalone, content-free crate
//! [`ambition_portal`](https://docs.rs/ambition_portal) (Stage 19; ADR 0019),
//! and the portal **visuals** (placed-portal quads + labels, the held / pickup
//! gun sprite, mid-transit body pieces, the disorientation indicator, the
//! through-portal view cones) live in its reusable default renderer
//! [`ambition_portal_presentation`]. This module is a thin FACADE that
//! re-exports both so every inbound `crate::portal::…` path keeps resolving
//! with zero churn, plus the Ambition-specific glue that is NOT reusable:
//!
//! - the **presentation host adapter** (render-gated): sync the crate-owned
//!   [`PortalWorldFrame`] from [`RoomGeometry`], tag [`PortalSceneBody`] on the
//!   player's visual entity, and load [`PortalGunArt`] from the Ambition asset
//!   paths. The presentation crate never names a host type; these three
//!   systems are the entire bridge.
//! - the `F7` dev off-switch (raw keyboard = host input concern).
//!
//! The Ambition adapters that bridge the MECHANIC's seams to game concepts
//! (input → fire intent, carve → collision overlay, room-reset → clear, sfx,
//! player input / ability shaping, identity → policy tagging) live in
//! [`crate::ambition_content::portal`]; the portal integration tests live in
//! `ambition_content::portal::tests`.

// The whole reusable mechanic, surfaced at the historic `crate::portal::…` paths.
pub use ambition_portal::*;

// The whole reusable default renderer, surfaced at the same historic paths
// (`crate::portal::sync_portal_visuals`, `crate::portal::PortalAimHint`, …).
// Render only — exclusively behind `portal_render`, so the portal *simulation*
// builds without any render-facing systems or components.
#[cfg(feature = "portal_render")]
pub use ambition_portal_presentation::*;

#[cfg(feature = "portal_render")]
mod host_adapter {
    use bevy::prelude::*;

    use ambition_platformer_primitives::world_query::SolidWorldQuery;
    use ambition_portal_presentation::{
        PortalCameraContinuityCamera, PortalCameraContinuityConfig, PortalCameraContinuityFocus,
        PortalCameraContinuityHostView, PortalCameraContinuitySelection,
        PortalCameraContinuityState, PortalCameraTransitMode, PortalDebugOverlay, PortalGunArt,
        PortalSceneBody, PortalViewer, PortalWorldFrame,
    };

    use crate::abilities::traversal::possession::PossessionState;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use crate::features::CenteredAabb;
    use crate::platformer_runtime::body::BodyKinematics;
    use crate::platformer_runtime::lifecycle::PlayerVisual;
    use crate::RoomGeometry;

    /// Bridge the controlled character + the collision world → the crate-owned
    /// [`PortalViewer`] seam, so each portal window is the wedge that character
    /// can actually see through the aperture. The eye is the possessed actor's
    /// body when possessing (so the view follows the body you're driving), else
    /// the primary player's; `occluders` is a snapshot of the world's solid
    /// blocks for the line-of-sight test. Absent player/possessed body ⇒
    /// `present = false`, and the renderer falls back to the static window.
    pub fn sync_portal_viewer(
        world: Res<RoomGeometry>,
        possession: Res<PossessionState>,
        feature_aabbs: Query<&CenteredAabb>,
        player: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
        viewer: Option<ResMut<PortalViewer>>,
    ) {
        let Some(mut viewer) = viewer else {
            return;
        };
        let body = possession
            .possessed
            .and_then(|e| feature_aabbs.get(e).ok().map(|a| (a.center, a.half_size)))
            .or_else(|| player.single().ok().map(|k| (k.pos, k.size * 0.5)));
        match body {
            Some((eye, half_size)) => {
                viewer.present = true;
                viewer.eye = eye;
                viewer.half_size = half_size;
                viewer.occluders.clear();
                world
                    .0
                    .for_each_solid_aabb(false, &mut |aabb| viewer.occluders.push(aabb));
            }
            None => viewer.present = false,
        }
    }

    /// Bridge [`RoomGeometry`] → the crate-owned [`PortalWorldFrame`] seam: the
    /// presentation crate only ever needs the world's size for its centered
    /// y-flip render transform, so the host copies that one field each frame
    /// (room transitions resize the world).
    pub fn sync_portal_world_frame(world: Res<RoomGeometry>, mut frame: ResMut<PortalWorldFrame>) {
        if frame.size != world.0.size {
            frame.size = world.0.size;
        }
    }

    /// Bridge the host's player-visual marker → the crate-owned
    /// [`PortalSceneBody`] seam, so the mid-transit body-piece decomposition
    /// draws the player's sprite without the presentation crate naming
    /// [`PlayerVisual`].
    pub fn tag_portal_scene_bodies(
        mut commands: Commands,
        untagged: Query<Entity, (With<PlayerVisual>, Without<PortalSceneBody>)>,
    ) {
        for entity in &untagged {
            commands.entity(entity).insert(PortalSceneBody);
        }
    }

    /// Load the portal-gun mode sprites at startup into the crate-owned
    /// [`PortalGunArt`] seam — asset PATHS are Ambition content, so loading
    /// stays host-side.
    pub fn load_portal_gun_art(mut commands: Commands, assets: Res<AssetServer>) {
        commands.insert_resource(PortalGunArt {
            blue: assets.load("sprites/props/portal_gun_blue.png"),
            orange: assets.load("sprites/props/portal_gun_orange.png"),
        });
    }

    /// Drive the portal debug overlay's host-side F1 gate from the standard
    /// `SandboxDevState.debug` flag, so the portal gizmos stay quiet unless the
    /// global debug overlay is on.
    pub fn sync_portal_debug_overlay_to_f1(
        dev_state: Res<crate::SandboxDevState>,
        debug: Option<ResMut<PortalDebugOverlay>>,
    ) {
        if let Some(mut debug) = debug {
            debug.enabled = dev_state.debug;
        }
    }

    /// Bridge Ambition's current primary controlled body to the portal
    /// presentation crate's actor-neutral camera-continuity focus seam. The
    /// portal presentation crate never names `PrimaryPlayer`; the host adapter
    /// chooses which body currently drives the main viewpoint effect.
    pub fn sync_portal_camera_continuity_focus(
        mut commands: Commands,
        untagged_primary: Query<
            Entity,
            (
                With<PlayerEntity>,
                With<PrimaryPlayer>,
                Without<PortalCameraContinuityFocus>,
            ),
        >,
        tagged: Query<(Entity, Has<PrimaryPlayer>), With<PortalCameraContinuityFocus>>,
    ) {
        for entity in &untagged_primary {
            commands.entity(entity).insert(PortalCameraContinuityFocus);
        }
        for (entity, still_primary) in &tagged {
            if !still_primary {
                commands
                    .entity(entity)
                    .remove::<PortalCameraContinuityFocus>();
            }
        }
    }

    /// Tag the host's main gameplay camera as the one eligible for optional
    /// portal camera continuity. Capture cameras and UI cameras stay untagged.
    pub fn tag_portal_camera_continuity_camera(
        mut commands: Commands,
        cameras: Query<
            Entity,
            (
                With<crate::session::camera_layers::MainCamera>,
                Without<PortalCameraContinuityCamera>,
            ),
        >,
    ) {
        for entity in &cameras {
            commands.entity(entity).insert(PortalCameraContinuityCamera);
        }
    }

    /// Optional camera-continuity screen anchor for portal transits.
    ///
    /// When the active continuity focus transits, map the previous visible
    /// camera center through the same portal BODY map that moved the focus.
    /// The host camera then keeps the focus at that exact screen-space offset
    /// only while the focus remains in the aperture; once it leaves, ordinary
    /// camera policy resumes immediately. Straight-through pairs are exact
    /// translation continuity by construction; quarter-turn pairs apply their
    /// roll immediately for the same aperture interval.
    pub fn apply_portal_camera_continuity(
        selection: Option<Res<PortalCameraContinuitySelection>>,
        config: Option<Res<PortalCameraContinuityConfig>>,
        host_view: Option<Res<PortalCameraContinuityHostView>>,
        world_frame: Option<Res<PortalWorldFrame>>,
        state: Option<ResMut<PortalCameraContinuityState>>,
        camera_state: Option<ResMut<crate::CameraEaseState>>,
        mut transited: MessageReader<ambition_portal::PortalBodyTransited>,
        gravity: Option<Res<ambition_platformer_primitives::gravity::GravityField>>,
        focus: Query<(), With<PortalCameraContinuityFocus>>,
        active_focus_transits: Query<
            (),
            (
                With<PortalCameraContinuityFocus>,
                With<ambition_portal::PortalTransit>,
            ),
        >,
        body_kinematics: Query<&BodyKinematics>,
        body_transits: Query<&ambition_portal::PortalTransit>,
        portals: Query<&ambition_portal::PlacedPortal>,
    ) {
        let Some(selection) = selection else {
            return;
        };
        let Some(mut state) = state else {
            return;
        };
        let Some(world_frame) = world_frame else {
            for _ in transited.read() {}
            return;
        };
        // The host camera policy records the last actually-rendered gameplay
        // camera center. Treat that visible center as the seam anchor for new
        // transfers. This is the mathematical continuity invariant:
        //
        //     screen_after = body_after - map(camera_before)
        //                  = map(body_before) - map(camera_before)
        //                  = body_before - camera_before
        //
        // for the translation-only portal pairs, and the same center correction
        // composes with temporary roll for quarter-turn pairs.
        let host_sample = host_view.as_deref().filter(|sample| sample.initialized);
        let previous_host_camera_world = host_sample
            .map(|sample| sample.current_center_world)
            .or(state.last_host_camera_world)
            .unwrap_or(world_frame.size * 0.5);
        let host_camera_world = host_sample
            .map(|sample| sample.ordinary_center_world)
            .unwrap_or(previous_host_camera_world);
        let host_sample_index = host_sample.map(|sample| sample.sample_index).unwrap_or(0);
        let host_target_world = host_sample
            .map(|sample| sample.target_world)
            .unwrap_or(Vec2::ZERO);
        let host_visible_view = host_sample
            .map(|sample| sample.visible_view)
            .unwrap_or(Vec2::ZERO);
        let host_active_camera_zones = host_sample
            .map(|sample| sample.active_camera_zones)
            .unwrap_or(0);
        let host_active_camera_zone = host_sample
            .and_then(|sample| sample.active_camera_zone.as_deref())
            .unwrap_or("none");
        let host_sample_source = if host_sample.is_some() {
            "host_view"
        } else {
            "fallback_world_center"
        };

        if selection.mode == PortalCameraTransitMode::Pop {
            // Drain while disabled so toggling Continuous later cannot replay a
            // stale transit from the disabled interval. Keep the last visible
            // camera anchor fresh even while the effect is disabled.
            for _ in transited.read() {}
            state.clear();
            state.last_host_camera_world = Some(previous_host_camera_world);
            return;
        }
        let Some(config) = config else {
            for _ in transited.read() {}
            state.clear();
            state.last_host_camera_world = Some(previous_host_camera_world);
            return;
        };

        let gravity_dir =
            ambition_platformer_primitives::gravity::gravity_dir_or_default(gravity.as_deref());
        let portal_list: Vec<ambition_portal::PlacedPortal> = portals.iter().copied().collect();
        let mut camera_state = camera_state;
        for ev in transited.read() {
            if focus.get(ev.body).is_err() {
                continue;
            }
            let Ok(body) = body_kinematics.get(ev.body) else {
                continue;
            };
            let Ok(transit) = body_transits.get(ev.body) else {
                continue;
            };
            let exit_channel = transit.straddling;
            let enter_channel = exit_channel.partner();
            let Some(exit_portal) = ambition_portal::find_portal(&portal_list, exit_channel) else {
                continue;
            };
            let Some(enter_portal) = ambition_portal::find_portal(&portal_list, enter_channel)
            else {
                continue;
            };

            let enter_frame = enter_portal.frame();
            let exit_frame = exit_portal.frame();

            // The exact-continuity correction is only meaningful when the ENTRY
            // aperture was on/near the previous visible view. Repeated portal
            // crossings intentionally chain from that rendered view: body and
            // camera are both mapped from the same pre-transfer frame.
            let enter_portal_screen_before = enter_portal.pos - previous_host_camera_world;
            let entry_visible = enter_portal_screen_before.x.abs()
                <= config.max_entry_screen_offset.x
                && enter_portal_screen_before.y.abs() <= config.max_entry_screen_offset.y;
            if !entry_visible {
                if config.debug_log {
                    bevy::log::info!(
                        target: "ambition::portal::camera",
                        "portal camera continuity skip: offscreen entry body={:?} enter={:?}@({:.1},{:.1}) exit={:?}@({:.1},{:.1}) entry_screen=({:.1},{:.1}) max_entry_screen=({:.1},{:.1}) prev_cam=({:.1},{:.1}) host_cam=({:.1},{:.1}) visible_cam=({:.1},{:.1}) host_source={} host_sample={} host_target=({:.1},{:.1}) host_visible=({:.1},{:.1}) host_zone={} host_zones={} state_target={:?} active_weight={:.3}",
                        ev.body,
                        enter_channel,
                        enter_portal.pos.x,
                        enter_portal.pos.y,
                        exit_channel,
                        exit_portal.pos.x,
                        exit_portal.pos.y,
                        enter_portal_screen_before.x,
                        enter_portal_screen_before.y,
                        config.max_entry_screen_offset.x,
                        config.max_entry_screen_offset.y,
                        previous_host_camera_world.x,
                        previous_host_camera_world.y,
                        host_camera_world.x,
                        host_camera_world.y,
                        previous_host_camera_world.x,
                        previous_host_camera_world.y,
                        host_sample_source,
                        host_sample_index,
                        host_target_world.x,
                        host_target_world.y,
                        host_visible_view.x,
                        host_visible_view.y,
                        host_active_camera_zone,
                        host_active_camera_zones,
                        state.target_camera_world,
                        state.active_weight(),
                    );
                }
                // Drop any previous screen-anchor: the seam it was preserving
                // is no longer the seam being crossed.
                state.clear();
                continue;
            }

            let desired_camera_world = ambition_portal::pieces::map_point(
                previous_host_camera_world,
                &enter_frame,
                &exit_frame,
            );
            let correction = desired_camera_world - host_camera_world;

            let raw_roll = ambition_portal_presentation::camera_roll_for_portal_transit(
                ev.enter_normal,
                ev.exit_normal,
                gravity_dir,
            );
            let roll = if raw_roll.abs() <= config.roll_epsilon_radians {
                0.0
            } else {
                raw_roll
            };

            // Debug the exact screen-space invariant. The body has already been
            // moved to the exit side; reverse-map it to estimate the pre-transfer
            // body center, then compare screen offsets around the mapped camera.
            let body_after = body.pos;
            let body_before =
                ambition_portal::pieces::map_point(body_after, &exit_frame, &enter_frame);
            let screen_before = body_before - previous_host_camera_world;
            let screen_after = body_after - desired_camera_world;
            let screen_error = screen_after - screen_before;
            let body_screen_offset_world = screen_after;

            // Also log the aperture screen offsets. For pure translation pairs,
            // this is the user-visible seam invariant: the exit aperture should
            // occupy the same screen-space offset as the entry aperture did.
            let exit_portal_screen_after = exit_portal.pos - desired_camera_world;
            let exit_portal_screen_host = exit_portal.pos - host_camera_world;
            let portal_screen_error = exit_portal_screen_after - enter_portal_screen_before;
            let previous_state_target = state.target_camera_world;
            let active_weight_before = state.active_weight();

            // Constraint diagnostics for portal-zone authoring: whether the
            // ordinary camera policy already agrees with the seamless handoff
            // center. Translation-only pairs should have zero body/portal
            // screen error here; if they still show a pop after the screen
            // anchor clears, the normal camera zone needs enough padding/freedom
            // around this aperture.
            let desired_minus_host = desired_camera_world - host_camera_world;
            let desired_minus_host_target = desired_camera_world - host_target_world;
            let host_center_minus_target = host_camera_world - host_target_world;
            let desired_room_padding = camera_room_padding_needed(
                desired_camera_world,
                host_visible_view,
                world_frame.size,
            );
            let host_room_padding =
                camera_room_padding_needed(host_camera_world, host_visible_view, world_frame.size);
            let desired_outside_room = padding_any(desired_room_padding);
            let host_outside_room = padding_any(host_room_padding);
            let host_gap_max = desired_minus_host.x.abs().max(desired_minus_host.y.abs());
            let target_gap_max = desired_minus_host_target
                .x
                .abs()
                .max(desired_minus_host_target.y.abs());
            let overlap_active = active_weight_before >= config.overlap_warn_weight;
            let room_padding_max = padding_max(desired_room_padding);
            let should_log_constraint = config.debug_log
                && (host_gap_max >= config.camera_constraint_warn_pixels
                    || target_gap_max >= config.camera_constraint_warn_pixels
                    || room_padding_max > 0.0
                    || overlap_active);
            if should_log_constraint {
                let constraint_kind = if room_padding_max > 0.0 {
                    "room_padding_or_camera_zone_required"
                } else if host_gap_max >= config.camera_constraint_warn_pixels {
                    "host_camera_recovery_gap"
                } else if overlap_active {
                    "overlapping_continuity_effect"
                } else {
                    "target_mismatch"
                };
                bevy::log::info!(
                    target: "ambition::portal::camera",
                    "portal camera continuity constraint: kind={} body={:?} enter={:?} exit={:?} desired_minus_host=({:.1},{:.1}) desired_minus_host_target=({:.1},{:.1}) host_center_minus_target=({:.1},{:.1}) desired_room_padding_lrtb=({:.1},{:.1},{:.1},{:.1}) host_room_padding_lrtb=({:.1},{:.1},{:.1},{:.1}) desired_outside_room={} host_outside_room={} overlap_active={} active_weight={:.3} host_zone={} host_zones={} host_source={} host_sample={} note=if desired_minus_host remains large when the body leaves the aperture, add camera-zone/room padding around this portal so normal camera policy can take over without a pop",
                    constraint_kind,
                    ev.body,
                    enter_channel,
                    exit_channel,
                    desired_minus_host.x,
                    desired_minus_host.y,
                    desired_minus_host_target.x,
                    desired_minus_host_target.y,
                    host_center_minus_target.x,
                    host_center_minus_target.y,
                    desired_room_padding.left,
                    desired_room_padding.right,
                    desired_room_padding.top,
                    desired_room_padding.bottom,
                    host_room_padding.left,
                    host_room_padding.right,
                    host_room_padding.top,
                    host_room_padding.bottom,
                    desired_outside_room,
                    host_outside_room,
                    overlap_active,
                    active_weight_before,
                    host_active_camera_zone,
                    host_active_camera_zones,
                    host_sample_source,
                    host_sample_index,
                );
            }

            if let Some(camera_state) = camera_state.as_deref_mut() {
                if camera_state.target_initialized {
                    camera_state.live_target_world = ambition_portal::pieces::map_point(
                        camera_state.live_target_world,
                        &enter_frame,
                        &exit_frame,
                    );
                }
            }
            state.start_screen_anchor(desired_camera_world, body_screen_offset_world, roll);
            if config.debug_log {
                bevy::log::info!(
                    target: "ambition::portal::camera",
                    "portal camera continuity start: body={:?} enter={:?}@({:.1},{:.1}) exit={:?}@({:.1},{:.1}) enter_n={:?} exit_n={:?} roll_deg={:.2} body_before=({:.1},{:.1}) body_after=({:.1},{:.1}) prev_cam=({:.1},{:.1}) host_cam=({:.1},{:.1}) visible_cam=({:.1},{:.1}) desired_cam=({:.1},{:.1}) correction=({:.1},{:.1}) body_screen=({:.1},{:.1}) body_screen_error=({:.3},{:.3}) portal_before=({:.1},{:.1}) portal_after=({:.1},{:.1}) portal_host=({:.1},{:.1}) portal_screen_error=({:.3},{:.3}) host_source={} host_sample={} host_target=({:.1},{:.1}) host_visible=({:.1},{:.1}) host_zone={} host_zones={} previous_state_target={:?} active_weight_before={:.3} max_entry_screen=({:.1},{:.1})",
                    ev.body,
                    enter_channel,
                    enter_portal.pos.x,
                    enter_portal.pos.y,
                    exit_channel,
                    exit_portal.pos.x,
                    exit_portal.pos.y,
                    ev.enter_normal,
                    ev.exit_normal,
                    roll.to_degrees(),
                    body_before.x,
                    body_before.y,
                    body_after.x,
                    body_after.y,
                    previous_host_camera_world.x,
                    previous_host_camera_world.y,
                    host_camera_world.x,
                    host_camera_world.y,
                    previous_host_camera_world.x,
                    previous_host_camera_world.y,
                    desired_camera_world.x,
                    desired_camera_world.y,
                    correction.x,
                    correction.y,
                    body_screen_offset_world.x,
                    body_screen_offset_world.y,
                    screen_error.x,
                    screen_error.y,
                    enter_portal_screen_before.x,
                    enter_portal_screen_before.y,
                    exit_portal_screen_after.x,
                    exit_portal_screen_after.y,
                    exit_portal_screen_host.x,
                    exit_portal_screen_host.y,
                    portal_screen_error.x,
                    portal_screen_error.y,
                    host_sample_source,
                    host_sample_index,
                    host_target_world.x,
                    host_target_world.y,
                    host_visible_view.x,
                    host_visible_view.y,
                    host_active_camera_zone,
                    host_active_camera_zones,
                    previous_state_target,
                    active_weight_before,
                    config.max_entry_screen_offset.x,
                    config.max_entry_screen_offset.y,
                );
            }
        }

        if state.active_weight() > 0.0 && active_focus_transits.is_empty() {
            state.clear_effect();
        }
        state.last_host_camera_world = Some(previous_host_camera_world);
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct CameraRoomPaddingNeeded {
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
    }

    fn camera_room_padding_needed(
        center_world: Vec2,
        visible_view: Vec2,
        world_size: Vec2,
    ) -> CameraRoomPaddingNeeded {
        if visible_view.x <= 0.0
            || visible_view.y <= 0.0
            || world_size.x <= 0.0
            || world_size.y <= 0.0
        {
            return CameraRoomPaddingNeeded::default();
        }
        let half = visible_view * 0.5;
        CameraRoomPaddingNeeded {
            left: (half.x - center_world.x).max(0.0),
            right: (center_world.x + half.x - world_size.x).max(0.0),
            top: (half.y - center_world.y).max(0.0),
            bottom: (center_world.y + half.y - world_size.y).max(0.0),
        }
    }

    fn padding_any(padding: CameraRoomPaddingNeeded) -> bool {
        padding.left > 0.0 || padding.right > 0.0 || padding.top > 0.0 || padding.bottom > 0.0
    }

    fn padding_max(padding: CameraRoomPaddingNeeded) -> f32 {
        padding
            .left
            .max(padding.right)
            .max(padding.top)
            .max(padding.bottom)
    }
    /// Dev off-switch: `F7` toggles the portal gun active/inactive so the
    /// always-on slice gun doesn't fire portals on every Attack while testing
    /// other sandbox mechanics. (Visible build only.) Final gating is via
    /// held-item equip; this is a developer convenience until then.
    ///
    /// This reads raw keyboard input (a host input / dev concern), so it lives
    /// host-side rather than in a portal crate — it just flips
    /// `PortalGun.active` the way the crate's message-driven toggle would.
    pub fn portal_dev_toggle_system(
        keys: Res<ButtonInput<KeyCode>>,
        mut guns: Query<&mut ambition_portal::PortalGun>,
    ) {
        if !keys.just_pressed(KeyCode::F7) {
            return;
        }
        for mut gun in &mut guns {
            gun.active = !gun.active;
            bevy::log::info!(target: "ambition::portal", "portal gun active = {}", gun.active);
        }
    }

    /// Dev: `F10` flips the game-wide portal map CONVENTION live, to A/B the
    /// feel — reflection (det −1, default: tangent preserved, facing/thin-wall
    /// pairs vertically flip) vs rotation (det +1: facing/thin-wall pairs are a
    /// clean straight-through "door", floor↔floor reverses horizontal). Affects
    /// transit, the view cones, the body copy, and collision pieces together.
    pub fn portal_convention_toggle_system(
        keys: Res<ButtonInput<KeyCode>>,
        tuning: Option<ResMut<ambition_portal::PortalTuning>>,
    ) {
        if !keys.just_pressed(KeyCode::F10) {
            return;
        }
        let next = !ambition_portal::portal_map_rotation();
        if let Some(mut tuning) = tuning {
            tuning.convention = ambition_portal::PortalConvention::from_rotation(next);
        }
        ambition_portal::set_portal_map_rotation(next);
        bevy::log::info!(
            target: "ambition::portal",
            "portal map convention = {}",
            if next { "rotation (det +1)" } else { "reflection (det -1)" }
        );
    }
}

#[cfg(feature = "portal_render")]
pub use host_adapter::{
    apply_portal_camera_continuity, load_portal_gun_art, portal_convention_toggle_system,
    portal_dev_toggle_system, sync_portal_camera_continuity_focus, sync_portal_debug_overlay_to_f1,
    sync_portal_viewer, sync_portal_world_frame, tag_portal_camera_continuity_camera,
    tag_portal_scene_bodies,
};
