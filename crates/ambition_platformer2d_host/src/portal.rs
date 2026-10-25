//! Ambition-specific host glue for the reusable portal mechanic and renderer.
//!
//! This adapter publishes host world/body/view facts into
//! `ambition_portal2d_presentation` without making that crate depend on Ambition
//! player or room types. Gameplay/content adapters remain in
//! `ambition_content::portal`.

mod host_adapter {
    use bevy::prelude::*;

    use ambition_platformer2d_core::cast::SolidWorldQuery;
    use ambition_platformer2d_core::BodyKinematics;
    use ambition_portal2d_presentation::{
        PortalAffordanceBody, PortalBodyView, PortalCameraContinuityCamera,
        PortalCameraContinuityConfig, PortalCameraContinuityFocus, PortalCameraContinuityHostView,
        PortalCameraContinuityState, PortalCameraTransitMode,
        PortalDebugOverlay, PortalFrames, PortalGunArt, PortalObservationSet, PortalSceneBody,
        PortalTransitView, PortalViewer, PortalViewers,
    };

    use ambition_platformer2d_core::RoomGeometry;
    use ambition_platformer2d_runtime::host_seams::DeveloperRuntimeState;
    use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;
    use ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual;
    use ambition_platformer2d_shared_tangle::markers::{
        ControlledSubject, PlayerEntity, PrimaryPlayer,
    };

    /// Bridge each local view's body and its room's collision world to the
    /// crate-owned [`PortalViewers`] seam, so each portal window is the wedge
    /// that body can see through the aperture.
    ///
    /// One eye for each live room that a local view frames. The eye of a view
    /// is the body the view looks at (`ResolvedViewSubject`), and for a view
    /// that names none, the CONTROLLED SUBJECT: the body holding
    /// `DrivingParticipant(PRIMARY)`, which is the possessed actor while
    /// possessing, else the home avatar. A session with no local view has the
    /// controlled subject as its one eye. The eye is in that body's own live
    /// room, and `occluders` is a snapshot of that room's solid blocks for
    /// the line-of-sight test.
    ///
    /// The views are taken in the order of their ids, and a room has the eye
    /// of the first view that frames it. So two players in two live rooms
    /// each have the windows of their own room, and two views of ONE room
    /// share the windows of the first.
    ///
    /// A body that is gone, or whose room cannot be told, has no eye. A live
    /// room with no eye has no window.
    pub fn sync_portal_viewer(
        rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomOf<RoomGeometry>,
        controlled: Res<ControlledSubject>,
        views: Query<
            (Entity, &ambition_sim_view::LocalViewId, &ambition_sim_view::ResolvedViewSubject),
            With<ambition_sim_view::LocalView>,
        >,
        bodies: Query<(
            &BodyKinematics,
            Option<&ambition_platformer2d_core::SweepSample>,
            Option<&ambition_portal2d::PortalBackside>,
            Option<&ambition_portal2d::PortalTransit>,
        )>,
        viewers: Option<ResMut<PortalViewers>>,
    ) {
        use ambition_platformer2d_core::AabbExt;
        let Some(mut viewers) = viewers else {
            return;
        };
        viewers.clear();
        let mut ordered: Vec<_> = views.iter().map(|(view, id, subject)| (*id, view, subject.0)).collect();
        ordered.sort();
        let mut eyes: Vec<(Option<Entity>, Option<Entity>)> = ordered
            .into_iter()
            .map(|(_, view, subject)| (Some(view), subject.or(controlled.0)))
            .collect();
        if eyes.is_empty() {
            eyes.push((None, controlled.0));
        }
        for (observer, body) in eyes {
            let Some(body) = body else {
                continue;
            };
            let (Ok((kin, last_step, backside, transit)), Some(room)) = (bodies.get(body), rooms.room_of(body)) else {
                continue;
            };
            let Some(world) = rooms.in_room(room) else {
                continue;
            };
            let mut occluders = Vec::new();
            world.0.for_each_solid_aabb(false, &mut |aabb| occluders.push(aabb));
            // A second view of a room that has an eye is refused there.
            // The way the body faces: its side, under the down of its last
            // step, by its facing.
            let down = last_step.map(|s| s.down).filter(|d| *d != Vec2::ZERO).unwrap_or(Vec2::new(0.0, 1.0));
            let forward = Vec2::new(down.y, -down.x) * kin.facing.signum();
            viewers.publish(PortalViewer {
                present: true,
                observer,
                eye: kin.pos,
                forward,
                // The box the body has: the line of sight is tested from its
                // four corners.
                half_size: kin.collision_box(last_step).half_size(),
                room: Some(room),
                on_back_of: backside.map(|back| back.channel),
                crossed_into: transit.filter(|t| t.crossed).map(|t| t.straddling),
                occluders,
            });
        }
    }

    /// Bridge the host's body visuals → the crate-owned [`PortalSceneBody`]
    /// seam: a body's own drawable goes through a portal pair while its body
    /// is in transit, and never by its own geometry (a giant whose arm's near
    /// line is over its back is not carried). And a part-drawn body's parts
    /// are drawn with their body (`PortalDrawnWith`). The presentation crate
    /// names none of these types.
    #[allow(clippy::type_complexity)]
    pub fn tag_portal_scene_bodies(
        mut commands: Commands,
        untagged: Query<
            Entity,
            (
                Or<(
                    With<PlayerVisual>,
                    With<ambition_render::rendering::FeatureVisual>,
                    With<ambition_render::rendering::projectile_visuals::VisualProjectile>,
                    With<ambition_render::rendering::GroundItemVisual>,
                    With<ambition_render::rendering::actors::rigged::LimbTwin>,
                )>,
                Without<PortalSceneBody>,
                Without<ambition_portal2d_presentation::PortalDrawnWith>,
            ),
        >,
        parts: Query<
            (Entity, &ambition_render::rendering::actors::rigged::RiggedPresentationOf),
            Without<ambition_portal2d_presentation::PortalDrawnWith>,
        >,
    ) {
        for entity in &untagged {
            commands.entity(entity).try_insert(PortalSceneBody);
        }
        for (owner, of) in &parts {
            commands.entity(owner).try_insert(ambition_portal2d_presentation::PortalDrawnWith(of.0));
        }
    }

    /// Bridge the CONTROLLED body → the crate-owned [`PortalAffordanceBody`]
    /// seam: whose hand the portal gun draws in, and who the disorientation
    /// indicator floats above.
    ///
    /// The authority is [`ControlledSubject`], not `PrimaryPlayer` — the same
    /// authority [`sync_portal_viewer`] uses for the eye of a view that names
    /// no body of its own. While possessing,
    /// the controlled body is the possessed actor, so the gun and the warp
    /// indicator now follow the body you are actually driving instead of
    /// staying on the home avatar.
    pub fn tag_portal_affordance_body(
        mut commands: Commands,
        controlled: Res<ControlledSubject>,
        tagged: Query<Entity, With<PortalAffordanceBody>>,
    ) {
        let subject = controlled.0;
        for entity in &tagged {
            if Some(entity) != subject {
                commands.entity(entity).remove::<PortalAffordanceBody>();
            }
        }
        // `try_insert`: a subject that despawned this frame must not resurrect
        // as a command-spawned shell.
        if let Some(entity) = subject {
            if tagged.get(entity).is_err() {
                commands.entity(entity).try_insert(PortalAffordanceBody);
            }
        }
    }

    /// Publish [`PortalBodyView`] — the pose facts portal presentation draws
    /// from — onto every entity the presentation crate must place a visual
    /// against.
    ///
    /// This is the read-model seam: presentation gets plain `Copy` pose data
    /// and never reads `BodyKinematics`, so the two sides never have to agree
    /// on which host type owns a pose.
    pub fn publish_portal_body_views(
        mut commands: Commands,
        bodies: Query<
            (
                Entity,
                &BodyKinematics,
                Option<&ambition_platformer2d_core::SweepSample>,
                Option<&ambition_portal2d::PortalTransit>,
                Has<PortalTransitView>,
            ),
            Or<(With<PortalSceneBody>, With<PortalAffordanceBody>)>,
        >,
    ) {
        for (entity, kin, last_step, transit, shown_in_transit) in &bodies {
            commands.entity(entity).try_insert(body_view(entity, kin, last_step));
            // Whether it straddles a portal, for the piece builder.
            match transit {
                Some(transit) => {
                    commands.entity(entity).try_insert(PortalTransitView { straddling: transit.straddling });
                }
                None if shown_in_transit => {
                    commands.entity(entity).remove::<PortalTransitView>();
                }
                None => {}
            }
        }
    }

    /// The pose facts of one body, for [`PortalBodyView`].
    fn body_view(body: Entity, kin: &BodyKinematics, last_step: Option<&ambition_platformer2d_core::SweepSample>) -> PortalBodyView {
        use ambition_platformer2d_core::AabbExt;
        PortalBodyView {
            pos: kin.pos,
            // The box the body has, turned as its last step turned it: the box
            // the portal core transits.
            size: kin.collision_box(last_step).half_size() * 2.0,
            facing: kin.facing,
            body,
        }
    }

    /// Each body that crosses a portal is cut at the seam, not the player
    /// only. A body that is not the player is drawn by another entity than
    /// its own (a `FeatureVisual` with its id), so the facts the piece
    /// builder cuts by are published onto that visual while its body
    /// straddles a portal: the scene-body tag, the pose, and the transit.
    ///
    /// The tag and the pose stay after the transit (the piece builder gives
    /// the whole sprite back on the frame after, and it visits tagged
    /// visuals only). The transit fact goes when the transit ends.
    ///
    /// A visual is matched to a body by id in its own live room: two live
    /// rooms can hold the same id.
    pub fn publish_transiting_feature_bodies(
        mut commands: Commands,
        live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        bodies: Query<(
            Entity,
            &ambition_combat::FeatureId,
            &BodyKinematics,
            Option<&ambition_platformer2d_core::SweepSample>,
            &ambition_portal2d::PortalTransit,
        )>,
        visuals: Query<
            (Entity, &ambition_render::rendering::FeatureVisual, Has<PortalTransitView>),
            Without<PlayerVisual>,
        >,
    ) {
        if bodies.is_empty() {
            // No body is in transit: take the fact off each visual that has it.
            for (visual, _, shown_in_transit) in &visuals {
                if shown_in_transit {
                    commands.entity(visual).remove::<PortalTransitView>();
                }
            }
            return;
        }
        for (visual, feature, shown_in_transit) in &visuals {
            let room = live.of(visual);
            let body = bodies
                .iter()
                .find(|(body, id, ..)| id.0 == feature.id && live.of(*body) == room);
            match body {
                Some((body, _, kin, last_step, transit)) => {
                    commands.entity(visual).try_insert((
                        PortalSceneBody,
                        body_view(body, kin, last_step),
                        PortalTransitView { straddling: transit.straddling },
                    ));
                }
                None if shown_in_transit => {
                    commands.entity(visual).remove::<PortalTransitView>();
                }
                None => {}
            }
        }
    }

    /// A limb in a portal pair carries its host's picture in: a giant's arm
    /// is drawn by the giant, and the limb that goes through is its fist
    /// (`docs/planning/engine/portals-a-body-in-two-places.md`).
    ///
    /// A host whose flipbook names the parts of the limb (`limb:<slot>`
    /// tracks) sends those parts only: while the limb is in a pair, the host
    /// draws the parts under the limb, the limb's twin (`rigged::LimbTwin`)
    /// draws the limb's parts and is cut by the pair as any body in it (all
    /// of the limb's picture past the plane goes through), and the host's
    /// over twin draws the parts over the limb, uncut. Cut as one picture,
    /// the band past the plane took the giant's back with its arm
    /// (2026-10-24).
    ///
    /// ⛔ A TWIN IS MADE BEFORE ITS LIMB GOES IN. A twin needs frames to be
    /// drawn (its animator, its parts, its composited image). Made on the
    /// frame its limb went in, the limb was drawn by nothing for those frames
    /// and the arm vanished (2026-10-25). So each limb of a host in a room
    /// that holds a portal has its twin, hidden while the limb is in no pair.
    ///
    /// A host that does not name its limbs' parts is cut as one picture by
    /// the pair of its deepest limb, as deep as the limb reaches
    /// ([`PortalHostReach`]), so the rest of the host stays. When no limb of
    /// it is in a pair, the facts go and its twins hide.
    ///
    /// [`PortalHostReach`]: ambition_portal2d_presentation::PortalHostReach
    #[allow(clippy::type_complexity)]
    pub fn publish_threaded_host_pictures(
        mut commands: Commands,
        live: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
        limbs: Query<(
            Entity,
            &ambition_characters::actor::Limb,
            &BodyKinematics,
            Option<&ambition_platformer2d_core::SweepSample>,
            Option<&ambition_portal2d::PortalTransit>,
        )>,
        hosts: Query<(&ambition_combat::FeatureId, &BodyKinematics, Option<&ambition_platformer2d_core::SweepSample>)>,
        hosts_in_transit: Query<(), With<ambition_portal2d::PortalTransit>>,
        portals: Query<(Entity, &ambition_portal2d::PlacedPortal)>,
        visuals: Query<
            (
                Entity,
                &ambition_render::rendering::FeatureVisual,
                Has<ambition_portal2d_presentation::PortalHostReach>,
                Option<&ambition_render::rendering::actors::rigged::RiggedPresentedBy>,
                Option<&ambition_render::rendering::actors::rigged::LimbTwins>,
                Has<ambition_render::rendering::actors::rigged::RiggedTrackMask>,
            ),
            Without<PlayerVisual>,
        >,
        presentations: Query<&ambition_render::rendering::actors::rigged::RiggedPresentation>,
        mut twins: Query<(
            &ambition_render::rendering::actors::rigged::LimbTwin,
            &mut Visibility,
            Has<PortalTransitView>,
            Has<ambition_portal2d_presentation::PortalHostReach>,
        )>,
    ) {
        use ambition_platformer2d_core::AabbExt;
        use ambition_render::rendering::actors::rigged::{limb_tracks_drawn, LimbTwin, RiggedTrackMask};
        let by_room = ambition_portal2d::PortalsByRoom::collect(&portals, &live);
        // Each limb of a host in a room that holds a portal: its host's id,
        // its room, its host, its slot, and the pair it is in and how deep.
        struct Carried {
            id: String,
            room: Option<ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance>,
            host: Entity,
            slot: String,
            within: Option<(ambition_portal2d::PortalChannel, f32)>,
        }
        let mut carried: Vec<Carried> = Vec::new();
        for (limb, of, kin, step, transit) in &limbs {
            let room = live.of(limb);
            let room_portals = by_room.in_room(room);
            if room_portals.is_empty() {
                continue;
            }
            let Ok((id, _, _)) = hosts.get(of.of) else {
                continue;
            };
            // A host that goes through a pair itself (all of it, as a giant
            // that falls into a portal) is cut as one body: its limbs are not
            // drawn apart from it.
            let within = transit.filter(|_| !hosts_in_transit.contains(of.of)).and_then(|transit| {
                let portal = ambition_portal2d::find_portal(room_portals, transit.straddling)?;
                let frame = portal.aperture().frame;
                let body = kin.collision_box(step);
                let corners = [body.min, body.max, Vec2::new(body.min.x, body.max.y), Vec2::new(body.max.x, body.min.y)];
                let deepest = corners.iter().map(|c| -(*c - frame.origin).dot(frame.normal)).fold(0.0_f32, f32::max);
                let thickness = body.half_size().x * frame.normal.x.abs() + body.half_size().y * frame.normal.y.abs();
                Some((transit.straddling, deepest + 2.0 * thickness))
            });
            carried.push(Carried { id: id.0.clone(), room, host: of.of, slot: of.slot.as_str().to_owned(), within });
        }
        for (visual, feature, has_reach, presented, limb_twins, masked) in &visuals {
            let room = live.of(visual);
            let mine: Vec<&Carried> = carried.iter().filter(|c| c.id == feature.id && c.room == room).collect();
            let presentation = presented.and_then(|presented| presentations.get(presented.owner()).ok());
            // The limbs whose own parts the host draws: each goes as its twin.
            let (by_twin, whole): (Vec<&Carried>, Vec<&Carried>) = mine
                .iter()
                .partition(|c| presentation.is_some_and(|presentation| limb_tracks_drawn(presentation, &c.slot)));
            // Each twin: made while its host is near a portal, shown and cut
            // while its limb is in a pair, else hidden.
            let made: Vec<(Entity, String)> = limb_twins
                .into_iter()
                .flat_map(|t| t.twins().iter().copied())
                .filter_map(|twin| twins.get(twin).ok().map(|(of, ..)| (twin, of.slot.clone())))
                .collect();
            let existing_twin = |slot: &str| made.iter().find(|(_, of)| of == slot).map(|(twin, _)| *twin);
            let over = match (by_twin.is_empty(), existing_twin("")) {
                (_, Some(over)) => Some(over),
                (false, None) => {
                    let over = LimbTwin::spawn_over(&mut commands, visual);
                    // The parts over the limb are the host's picture: they go
                    // through a pair only with the host.
                    commands.entity(over).insert(ambition_portal2d_presentation::PortalDrawnWith(visual));
                    Some(over)
                }
                (true, None) => None,
            };
            let mut spawned: Vec<&str> = Vec::new();
            let mut sent: Vec<String> = Vec::new();
            for c in &by_twin {
                let twin = match existing_twin(&c.slot) {
                    Some(twin) => twin,
                    None if !spawned.contains(&c.slot.as_str()) => {
                        spawned.push(c.slot.as_str());
                        LimbTwin::spawn(&mut commands, visual, &c.slot)
                    }
                    None => continue,
                };
                let Some((straddling, _)) = c.within else {
                    if let Ok((_, mut visibility, in_transit, _)) = twins.get_mut(twin) {
                        visibility.set_if_neq(Visibility::Hidden);
                        if in_transit {
                            commands.entity(twin).remove::<PortalTransitView>();
                        }
                    }
                    continue;
                };
                let Ok((_, kin, step)) = hosts.get(c.host) else {
                    continue;
                };
                sent.push(c.slot.clone());
                // Shown on the frame its limb goes in; while it is cut, the
                // portal hides and shows it (`resolve_portal_source_visibility`).
                match twins.get_mut(twin) {
                    Ok((_, mut visibility, false, _)) => *visibility = Visibility::Inherited,
                    Ok(_) => {}
                    Err(_) => {
                        commands.entity(twin).insert(Visibility::Inherited);
                    }
                }
                // The twin is the limb only: no depth bounds what goes through.
                if twins.get(twin).is_ok_and(|(.., reach)| reach) {
                    commands.entity(twin).remove::<ambition_portal2d_presentation::PortalHostReach>();
                }
                commands.entity(twin).try_insert((
                    PortalSceneBody,
                    body_view(c.host, kin, step),
                    PortalTransitView { straddling },
                ));
            }
            // A twin whose limb is gone, or whose host no longer draws the
            // limb's parts, hides.
            for twin in limb_twins.into_iter().flat_map(|t| t.twins().iter().copied()) {
                let Ok((of, mut visibility, in_transit, _)) = twins.get_mut(twin) else {
                    continue;
                };
                if of.is_over() || by_twin.iter().any(|c| c.slot == of.slot) {
                    continue;
                }
                visibility.set_if_neq(Visibility::Hidden);
                if in_transit {
                    commands.entity(twin).remove::<PortalTransitView>();
                }
            }
            // The host draws the parts under the limbs that go as twins, and
            // its over twin the parts over them.
            if let Some(over) = over {
                if let Ok((_, mut visibility, ..)) = twins.get_mut(over) {
                    visibility.set_if_neq(if sent.is_empty() { Visibility::Hidden } else { Visibility::Inherited });
                }
                if !sent.is_empty() {
                    commands.entity(over).try_insert(RiggedTrackMask::over(sent.clone()));
                }
            }
            match sent.is_empty() {
                false => {
                    commands.entity(visual).try_insert(RiggedTrackMask::under(sent));
                }
                true if masked => {
                    commands.entity(visual).remove::<RiggedTrackMask>();
                }
                true => {}
            }
            // One pair cuts the picture of a host whose limbs are not drawn
            // apart: the deepest limb's, then the lower channel, so the
            // picture does not follow the query's order. Two limbs in two
            // pairs at once are one cut: PORTAL-HOST-LIMBS.
            let deepest = whole
                .iter()
                .filter_map(|c| c.within.map(|(straddling, reach)| (c, straddling, reach)))
                .max_by(|a, b| a.2.total_cmp(&b.2).then(b.1.cmp(&a.1)));
            match deepest {
                Some((c, straddling, reach)) => {
                    let Ok((_, kin, step)) = hosts.get(c.host) else {
                        continue;
                    };
                    commands.entity(visual).try_insert((
                        PortalSceneBody,
                        body_view(c.host, kin, step),
                        PortalTransitView { straddling },
                        ambition_portal2d_presentation::PortalHostReach(reach),
                    ));
                }
                None if has_reach => {
                    commands
                        .entity(visual)
                        .remove::<(PortalTransitView, ambition_portal2d_presentation::PortalHostReach)>();
                }
                None => {}
            }
        }
    }

    /// Each item and each shot that straddles a portal is cut at the seam,
    /// as a body is: the facts the piece builder cuts by go onto the visual
    /// that draws it. An item's visual is linked to its item
    /// (`GroundItemVisual`, made again each frame from the item's row), and
    /// a shot's to its shot (`VisualProjectile`). The transit fact goes when
    /// the transit ends.
    #[allow(clippy::type_complexity)]
    pub fn publish_transiting_drawn_bodies(
        mut commands: Commands,
        transits: Query<&ambition_portal2d::PortalTransit>,
        items: Res<ambition_sim_view::GroundItemsView>,
        shots: Query<(&BodyKinematics, Option<&ambition_platformer2d_core::SweepSample>)>,
        item_visuals: Query<(Entity, &ambition_render::rendering::GroundItemVisual, Has<PortalTransitView>)>,
        shot_visuals: Query<(Entity, &ambition_render::rendering::projectile_visuals::VisualProjectile, Has<PortalTransitView>)>,
    ) {
        for (visual, link, shown_in_transit) in &item_visuals {
            let row = items.0.iter().find(|row| row.item == link.0);
            match (row, transits.get(link.0)) {
                (Some(row), Ok(transit)) => {
                    commands.entity(visual).try_insert((
                        PortalSceneBody,
                        PortalBodyView { pos: row.pos, size: row.half_extent * 2.0, facing: 1.0, body: link.0 },
                        PortalTransitView { straddling: transit.straddling },
                    ));
                }
                _ if shown_in_transit => {
                    commands.entity(visual).remove::<PortalTransitView>();
                }
                _ => {}
            }
        }
        for (visual, link, shown_in_transit) in &shot_visuals {
            match (shots.get(link.0), transits.get(link.0)) {
                (Ok((kin, last_step)), Ok(transit)) => {
                    commands.entity(visual).try_insert((
                        PortalSceneBody,
                        body_view(link.0, kin, last_step),
                        PortalTransitView { straddling: transit.straddling },
                    ));
                }
                _ if shown_in_transit => {
                    commands.entity(visual).remove::<PortalTransitView>();
                }
                _ => {}
            }
        }
    }

    /// Load the portal-gun mode sprites at startup into the crate-owned
    /// [`PortalGunArt`] seam — asset PATHS are Ambition content, so loading
    /// stays host-side.
    ///
    /// ⚠ THESE TWO REACH `Assets<Image>` WITH NO DEMAND STAMP, deliberately.
    /// The Hall's `[image-unrouted]` row names them (2026-09-02, alongside the
    /// LDtk editor-preview tileset), and routing them through
    /// `ambition_sprite_sheet::game_assets::load_sheet_image` would mean a new
    /// dependency edge from this crate to the asset ledger — for **0.0 MP** of
    /// icon art loaded once at startup. The stamp exists so residency can
    /// attribute megapixels to an owner; two icons that round to zero are not
    /// worth a crate edge to attribute. Revisit if this crate acquires that
    /// dependency for another reason, or if these ever become sheets.
    pub fn load_portal_gun_art(mut commands: Commands, assets: Res<AssetServer>) {
        commands.insert_resource(PortalGunArt {
            blue: assets.load("sprites/props/portal_gun_blue.png"),
            orange: assets.load("sprites/props/portal_gun_orange.png"),
        });
    }

    /// Drive the portal debug overlay's host-side F1 gate from the standard
    /// `DeveloperRuntimeState.debug` flag, so the portal gizmos stay quiet unless the
    /// global debug overlay is on.
    pub fn sync_portal_debug_overlay_to_f1(
        dev_state: Res<DeveloperRuntimeState>,
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

    /// The set [`tag_portal_camera_continuity_camera`] runs in.
    ///
    /// The debug overlay draws after the continuity camera is tagged, so its
    /// labels track a camera that already has its final identity this frame.
    ///
    /// ONE member — tagging the eligible camera is the whole step.
    #[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PortalContinuityCameraTagged;

    /// Tag the host's main gameplay camera as the one eligible for optional
    /// portal camera continuity. Capture cameras and UI cameras stay untagged.
    pub fn tag_portal_camera_continuity_camera(
        mut commands: Commands,
        cameras: Query<
            Entity,
            (
                With<ambition_platformer2d_shared_tangle::camera_layers::MainCamera>,
                Without<PortalCameraContinuityCamera>,
            ),
        >,
    ) {
        for entity in &cameras {
            commands.entity(entity).insert(PortalCameraContinuityCamera);
        }
    }

    /// Each body a portal pair carried this tick did not travel there: say so
    /// to the frame-clock smoothing, which takes a short hop (a thin-wall door
    /// moves a body 16 px) for a step and draws the body the whole hop ahead.
    pub fn declare_portal_pose_warps(
        mut transited: MessageReader<ambition_portal2d::PortalBodyTransited>,
        // Absent in a host that does not smooth poses on the frame clock.
        warps: Option<ResMut<ambition_sim_view::DeclaredPoseWarps>>,
    ) {
        let Some(mut warps) = warps else {
            transited.clear();
            return;
        };
        for transit in transited.read() {
            warps.declare(transit.body);
        }
    }

    /// Optional camera-continuity screen anchor for portal transits, for each
    /// local view.
    ///
    /// A view follows a body (its `ResolvedViewSubject`; a view that names
    /// none follows the [`PortalCameraContinuityFocus`]). When that body
    /// crosses a pair, the view's camera keeps it at that exact screen-space
    /// offset while it remains in the aperture; once it leaves, ordinary
    /// camera policy resumes immediately. Straight-through pairs are exact
    /// translation continuity by construction; quarter-turn pairs apply their
    /// roll immediately for the same aperture interval. Each view has its own
    /// anchor ([`PortalCameraContinuityState`] on the view) and its own camera
    /// sample ([`ambition_portal2d_presentation::PortalObserverViews`]), so two
    /// views whose bodies cross at once are each carried, and a crossing in
    /// one view does not move another.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_portal_camera_continuity(
        mut commands: Commands,
        transit: ambition_portal2d_presentation::PortalCameraTransit,
        config: Option<Res<PortalCameraContinuityConfig>>,
        (host_view, observer_views): (
            Option<Res<PortalCameraContinuityHostView>>,
            Option<Res<ambition_portal2d_presentation::PortalObserverViews>>,
        ),
        // The followed body's own live room: its size is the room the camera is in.
        frames: PortalFrames,
        focus_bodies: Query<Entity, With<PortalCameraContinuityFocus>>,
        // THE EASING STATE AND THE ANCHOR OF EVERY LOCAL VIEW. A portal maps
        // the world, so every observer of that world carries its own smoothed
        // target through the same map, and its own anchor.
        mut views: Query<
            (
                Entity,
                &mut CameraEaseState,
                Option<&ambition_sim_view::ResolvedViewSubject>,
                Option<&mut PortalCameraContinuityState>,
            ),
            With<ambition_sim_view::LocalView>,
        >,
        mut transited: MessageReader<ambition_portal2d::PortalBodyTransited>,
        gravity: Option<Res<ambition_platformer2d_shared_tangle::gravity::GravityField>>,
        body_kinematics: Query<&BodyKinematics>,
        body_transits: Query<&ambition_portal2d::PortalTransit>,
        portals: Query<&ambition_portal2d::PlacedPortal>,
        // The session's portal map convention, from the resource that owns it.
        tuning: Option<Res<ambition_portal2d::PortalTuning>>,
        (mut parallax_carry, time): (
            Option<ResMut<ambition_portal2d_presentation::PortalParallaxCarry>>,
            Option<Res<Time>>,
        ),
    ) {
        let convention = tuning
            .as_deref()
            .map(|tuning| tuning.convention.map_convention())
            .unwrap_or_default();
        let crossings: Vec<ambition_portal2d::PortalBodyTransited> = transited.read().cloned().collect();
        let Some(transit) = transit.mode() else {
            return;
        };
        let focus = focus_bodies.iter().next();
        let gravity_dir = ambition_platformer2d_shared_tangle::gravity::gravity_dir_or_default(
            gravity.as_deref(),
        );
        let portal_list: Vec<ambition_portal2d::PlacedPortal> = portals.iter().cloned().collect();

        for (view, mut camera_ease, subject, state) in &mut views {
            // A view is given its anchor the first frame it is seen.
            let Some(mut state) = state else {
                commands.entity(view).insert(PortalCameraContinuityState::default());
                continue;
            };
            // The body this view follows.
            let Some(followed) = subject.and_then(|s| s.0).or(focus) else {
                continue;
            };
            let Some(world_frame) = frames.of(followed).map(|placement| placement.frame) else {
                continue;
            };
            // The host camera policy records the last actually-rendered
            // gameplay camera center of each view. Treat that visible center
            // as the seam anchor for new transfers. This is the mathematical
            // continuity invariant:
            //
            //     screen_after = body_after - map(camera_before)
            //                  = map(body_before) - map(camera_before)
            //                  = body_before - camera_before
            //
            // for the translation-only portal pairs, and the same center
            // correction composes with temporary roll for quarter-turn pairs.
            let host_sample = observer_views
                .as_deref()
                .and_then(|views| views.of(Some(view)))
                .or(host_view.as_deref())
                .filter(|sample| sample.initialized);
            let previous_host_camera_world = host_sample
                .map(|sample| sample.current_center_world)
                .or(state.last_host_camera_world)
                .unwrap_or(world_frame.size * 0.5);
            let host_camera_world = host_sample
                .map(|sample| sample.ordinary_center_world)
                .unwrap_or(previous_host_camera_world);
            let host_sample_index = host_sample.map(|sample| sample.sample_index).unwrap_or(0);
            let host_target_world = host_sample.map(|sample| sample.target_world).unwrap_or(Vec2::ZERO);
            let host_visible_view = host_sample.map(|sample| sample.visible_view).unwrap_or(Vec2::ZERO);
            let host_active_camera_zones = host_sample.map(|sample| sample.active_camera_zones).unwrap_or(0);
            let host_active_camera_zone = host_sample
                .and_then(|sample| sample.active_camera_zone.as_deref())
                .unwrap_or("none");
            let host_sample_source = if host_sample.is_some() { "host_view" } else { "fallback_world_center" };

            let Some(config) = config.as_deref().filter(|_| transit != PortalCameraTransitMode::Pop) else {
                // Disabled: no stale transit is replayed when it is enabled
                // later. Keep the last visible camera anchor fresh even
                // while the effect is disabled.
                state.clear();
                state.last_host_camera_world = Some(previous_host_camera_world);
                continue;
            };

            for ev in crossings.iter().filter(|ev| ev.body == followed) {
                let Ok(body) = body_kinematics.get(ev.body) else {
                    continue;
                };
                let Ok(transit) = body_transits.get(ev.body) else {
                    continue;
                };
                let exit_channel = transit.straddling;
                let enter_channel = exit_channel.partner();
                let Some(exit_portal) = ambition_portal2d::find_portal(&portal_list, exit_channel) else {
                    continue;
                };
                let Some(enter_portal) = ambition_portal2d::find_portal(&portal_list, enter_channel) else {
                    continue;
                };
                let enter_frame = enter_portal.frame();
                let exit_frame = exit_portal.frame();

                // The exact-continuity correction is only meaningful when the
                // ENTRY aperture was on/near the previous visible view.
                // Repeated portal crossings intentionally chain from that
                // rendered view: body and camera are both mapped from the same
                // pre-transfer frame.
                let enter_portal_screen_before = enter_portal.pos - previous_host_camera_world;
                let entry_visible = enter_portal_screen_before.x.abs() <= config.max_entry_screen_offset.x
                    && enter_portal_screen_before.y.abs() <= config.max_entry_screen_offset.y;
                if !entry_visible {
                    if config.debug_log {
                        bevy::log::info!(
                            target: "ambition_platformer2d::portal::camera",
                            "portal camera continuity skip: offscreen entry view={:?} body={:?} enter={:?}@({:.1},{:.1}) exit={:?}@({:.1},{:.1}) entry_screen=({:.1},{:.1}) max_entry_screen=({:.1},{:.1}) prev_cam=({:.1},{:.1}) host_cam=({:.1},{:.1}) host_source={} host_sample={} host_target=({:.1},{:.1}) host_visible=({:.1},{:.1}) host_zone={} host_zones={} state_target={:?} active_weight={:.3}",
                            view,
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
                    // Drop any previous screen-anchor: the seam it was
                    // preserving is no longer the seam being crossed.
                    state.clear();
                    continue;
                }

                let desired_camera_world = ambition_portal2d::pieces::map_point(
                    previous_host_camera_world,
                    &enter_frame,
                    &exit_frame,
                    convention,
                );
                let correction = desired_camera_world - host_camera_world;

                // No pair is a special case: a door through a thin wall cuts
                // as a teleport does. The cut is by the same translation the
                // portal map is, and the view window shows the far side
                // already joined to the near side at the seam, so the picture
                // on screen is the same before and after it. With no cut, the
                // whole picture would jump by the wall's thickness when the
                // charts change over.
                let raw_roll = ambition_portal2d_presentation::camera_roll_for_portal_transit(
                    ev.enter_normal,
                    ev.exit_normal,
                    gravity_dir,
                    convention,
                );
                let roll = if raw_roll.abs() <= config.roll_epsilon_radians { 0.0 } else { raw_roll };

                // Debug the exact screen-space invariant.
                let body_after = body.pos;
                let body_before =
                    ambition_portal2d::pieces::map_point(body_after, &exit_frame, &enter_frame, convention);
                let screen_before = body_before - previous_host_camera_world;
                let screen_after = body_after - desired_camera_world;
                let screen_error = screen_after - screen_before;
                let body_screen_offset_world = screen_after;

                // Also log the aperture screen offsets. For pure translation
                // pairs, this is the user-visible seam invariant: the exit
                // aperture should occupy the same screen-space offset as the
                // entry aperture did.
                let exit_portal_screen_after = exit_portal.pos - desired_camera_world;
                let exit_portal_screen_host = exit_portal.pos - host_camera_world;
                let portal_screen_error = exit_portal_screen_after - enter_portal_screen_before;
                let previous_state_target = state.target_camera_world;
                let active_weight_before = state.active_weight();

                // Constraint diagnostics for portal-zone authoring: whether
                // the ordinary camera policy already agrees with the seamless
                // handoff center. Translation-only pairs should have zero
                // body/portal screen error here; if they still show a pop
                // after the screen anchor clears, the normal camera zone needs
                // enough padding/freedom around this aperture.
                let desired_minus_host = desired_camera_world - host_camera_world;
                let desired_minus_host_target = desired_camera_world - host_target_world;
                let host_center_minus_target = host_camera_world - host_target_world;
                let desired_room_padding =
                    camera_room_padding_needed(desired_camera_world, host_visible_view, world_frame.size);
                let host_room_padding =
                    camera_room_padding_needed(host_camera_world, host_visible_view, world_frame.size);
                let desired_outside_room = padding_any(desired_room_padding);
                let host_outside_room = padding_any(host_room_padding);
                let host_gap_max = desired_minus_host.x.abs().max(desired_minus_host.y.abs());
                let target_gap_max = desired_minus_host_target.x.abs().max(desired_minus_host_target.y.abs());
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
                        target: "ambition_platformer2d::portal::camera",
                        "portal camera continuity constraint: kind={} view={:?} body={:?} enter={:?} exit={:?} desired_minus_host=({:.1},{:.1}) desired_minus_host_target=({:.1},{:.1}) host_center_minus_target=({:.1},{:.1}) desired_room_padding_lrtb=({:.1},{:.1},{:.1},{:.1}) host_room_padding_lrtb=({:.1},{:.1},{:.1},{:.1}) desired_outside_room={} host_outside_room={} overlap_active={} active_weight={:.3} host_zone={} host_zones={} host_source={} host_sample={} note=if desired_minus_host remains large when the body leaves the aperture, add camera-zone/room padding around this portal so normal camera policy can take over without a pop",
                        constraint_kind,
                        view,
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

                // This view's camera moves through the pair; no other view's
                // does (review of 15d0cf99c, finding C).
                if camera_ease.target_initialized {
                    camera_ease.live_target_world = ambition_portal2d::pieces::map_point(
                        camera_ease.live_target_world,
                        &enter_frame,
                        &exit_frame,
                        convention,
                    );
                }
                state.start_screen_anchor(desired_camera_world, body_screen_offset_world, roll);
                // The camera moves through the pair on this frame; the
                // foreground does not jump with it.
                if let Some(carry) = parallax_carry.as_deref_mut() {
                    carry.carry(
                        Some(view),
                        desired_camera_world - previous_host_camera_world,
                        time.as_deref().map_or(0.0, Time::elapsed_secs),
                    );
                }
                if config.debug_log {
                    bevy::log::info!(
                        target: "ambition_platformer2d::portal::camera",
                        "portal camera continuity start: view={:?} body={:?} enter={:?}@({:.1},{:.1}) exit={:?}@({:.1},{:.1}) enter_n={:?} exit_n={:?} roll_deg={:.2} body_before=({:.1},{:.1}) body_after=({:.1},{:.1}) prev_cam=({:.1},{:.1}) host_cam=({:.1},{:.1}) desired_cam=({:.1},{:.1}) correction=({:.1},{:.1}) body_screen=({:.1},{:.1}) body_screen_error=({:.3},{:.3}) portal_before=({:.1},{:.1}) portal_after=({:.1},{:.1}) portal_host=({:.1},{:.1}) portal_screen_error=({:.3},{:.3}) host_source={} host_sample={} host_target=({:.1},{:.1}) host_visible=({:.1},{:.1}) host_zone={} host_zones={} previous_state_target={:?} active_weight_before={:.3} max_entry_screen=({:.1},{:.1})",
                        view,
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

            // While the followed body is in the aperture the anchor holds,
            // rolled as far through as it is; once it leaves, it clears.
            match (body_kinematics.get(followed), body_transits.get(followed)) {
                (Ok(body), Ok(transit)) => {
                    if let Some(roll) = portal_camera_roll_at_progress(
                        body,
                        transit,
                        &portal_list,
                        gravity_dir,
                        config.roll_epsilon_radians,
                        convention,
                    ) {
                        state.roll_radians = roll;
                    }
                }
                _ => state.clear_effect(),
            }
            state.last_host_camera_world = Some(previous_host_camera_world);
        }
    }

    fn portal_camera_roll_at_progress(
        body: &BodyKinematics,
        transit: &ambition_portal2d::PortalTransit,
        portals: &[ambition_portal2d::PlacedPortal],
        gravity_dir: Vec2,
        roll_epsilon_radians: f32,
        convention: ambition_portal2d::pieces::MapConvention,
    ) -> Option<f32> {
        let (enter_channel, exit_channel) = if transit.crossed {
            (transit.straddling.partner(), transit.straddling)
        } else {
            (transit.straddling, transit.straddling.partner())
        };
        let enter = ambition_portal2d::find_portal(portals, enter_channel)?;
        let exit = ambition_portal2d::find_portal(portals, exit_channel)?;
        let raw_roll = ambition_portal2d_presentation::camera_roll_for_portal_transit(
            enter.normal,
            exit.normal,
            gravity_dir,
            convention,
        );
        if raw_roll.abs() <= roll_epsilon_radians {
            return Some(0.0);
        }

        let frame = if transit.crossed {
            exit.frame()
        } else {
            enter.frame()
        };
        let normal_half_extent = (body.size * 0.5).dot(frame.normal.abs()).max(1.0);
        let front = ambition_portal2d::pieces::front_distance(body.pos, &frame);
        let progress = if transit.crossed {
            0.5 + front / (normal_half_extent * 2.0)
        } else {
            (normal_half_extent - front) / (normal_half_extent * 2.0)
        }
        .clamp(0.0, 1.0);

        Some(raw_roll * smoothstep01(progress))
    }

    fn smoothstep01(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
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
    /// Dev off-switch: the portal-gun developer action turns every gun on or
    /// off, so the always-on slice gun does not fire portals on every Attack
    /// while other sandbox mechanics are tested. (Visible build only.)
    ///
    /// The developer action is a host fact, so this reads it here and writes
    /// a host intent. `portal_guns_active_toggle_system` flips the guns inside
    /// the simulation, so a rewind does not undo the switch.
    pub fn portal_dev_toggle_system(
        mut actions: MessageReader<
            ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
        >,
        mut intents: ambition_platformer2d_runtime::host_intents::HostIntentWriter<
            ambition_portal2d::TogglePortalGunsActive,
        >,
    ) {
        let presses = actions
            .read()
            .filter(|action| {
                **action
                    == ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction::TogglePortalGun
            })
            .count();
        for _ in 0..presses {
            intents.write(ambition_portal2d::TogglePortalGunsActive);
        }
    }

    /// Registers the Ambition host-adapter glue in its OWN plugin (E4 slice
    /// 20 — render used to register these sim-side systems, the exact
    /// ownership inversion the observation boundary kills):
    ///
    /// - the presentation-seam publishers (`sync_portal_viewer`,
    ///   `sync_portal_camera_continuity_focus`,
    ///   `sync_portal_debug_overlay_to_f1`) in [`PortalObservationSet`];
    /// - `tag_portal_scene_bodies` too — the audit ruled its old
    ///   `.after(sync_visuals)` pin STALE: it tags SIM bodies
    ///   (`PlayerVisual` + `PortalSceneBody`), not render-spawned visuals;
    /// - the portal-gun developer toggle (semantic host action, no set);
    /// - `load_portal_gun_art` at startup (asset PATHS are Ambition content).
    pub struct PortalObservationPlugin;

    /// Run condition of the portal observation publishers: a portal stands or
    /// a body carries a transit latch, now or on the last frame. Without either
    /// they publish empty answers (no viewer, no transiting body), and the one
    /// run after the last portal goes publishes those, so the readers see them.
    fn portals_in_play_or_were(
        portals: Query<(), With<ambition_portal2d::PlacedPortal>>,
        transits: Query<(), With<ambition_portal2d::PortalTransit>>,
        mut were: Local<bool>,
    ) -> bool {
        let now = !portals.is_empty() || !transits.is_empty();
        let run = now || *were;
        *were = now;
        run
    }

    impl Plugin for PortalObservationPlugin {
        fn build(&self, app: &mut App) {
            app.add_message::<ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction>();
            // What the portal presentation reads. Asleep with it where no live
            // room uses portals, after one frame that publishes the empty
            // answers (no transiting body, no viewer).
            app.configure_sets(
                Update,
                PortalObservationSet.in_set(
                    ambition_platformer2d_shared_tangle::capability::CapabilityPresentationSet(
                        ambition_platformer2d_shared_tangle::capability::Capability::Portals,
                    ),
                ),
            );
            app.add_plugins(ambition_platformer2d_runtime::host_intents::HostIntentPlugin::<
                ambition_portal2d::TogglePortalGunsActive,
            >::default());
            app.add_systems(Startup, load_portal_gun_art).add_systems(
                Update,
                (
                    (
                        sync_portal_viewer,
                        // `.before(apply_portal_camera_continuity)` is enforced
                        // by the host's registration of the APPLY side, so no
                        // duplicate pin here.
                        sync_portal_camera_continuity_focus,
                        sync_portal_debug_overlay_to_f1,
                        // Chained: the publisher selects by the two marker
                        // seams, so the taggers' inserts must be applied
                        // first (Bevy inserts the sync point for an explicit
                        // ordering dependency). Without the pin the view
                        // would trail the tag by a frame on every retag.
                        (
                            tag_portal_scene_bodies,
                            tag_portal_affordance_body,
                            publish_portal_body_views,
                        )
                            .chain(),
                        // Each other body that crosses: its facts go onto the
                        // visual that draws it. A host whose limb is in a pair
                        // after: its facts stand.
                        (publish_transiting_feature_bodies, publish_threaded_host_pictures).chain(),
                    )
                        .in_set(PortalObservationSet)
                        .run_if(
                            ambition_platformer2d_shared_tangle::lifecycle::session_world_exists,
                        )
                        .run_if(portals_in_play_or_were),
                    portal_dev_toggle_system.run_if(ambition_platformer2d_core::a_message_waits::<
                        ambition_platformer2d_shared_tangle::developer_hotkeys::DeveloperAction,
                    >),
                ),
            );
            // A game may state its portal presentation for its own rooms. The
            // presentation crate cannot see rooms, so the active room's rules are
            // projected into its two rule resources before anything reads them.
            app.add_systems(
                Update,
                (
                    ambition_platformer2d_runtime::project_room_rule::<
                        PortalCameraTransitMode,
                        ambition_portal2d_presentation::PortalCameraTransitRule,
                    >,
                    ambition_platformer2d_runtime::project_room_rule::<
                        ambition_portal2d_presentation::PortalViewConeMode,
                        ambition_portal2d_presentation::PortalViewConeRule,
                    >,
                )
                    .in_set(PortalObservationSet)
                    .before(apply_portal_camera_continuity),
            );
            // After the tick that carried it and before the poses are resampled.
            app.add_systems(
                Update,
                declare_portal_pose_warps
                    .run_if(ambition_platformer2d_core::a_message_waits::<ambition_portal2d::PortalBodyTransited>)
                    .in_set(ambition_sim_view::PresentedPoseSet)
                    .before(ambition_sim_view::PresentedPoseStage::Resample),
            );
            // Items and shots that straddle a portal are cut at the seam:
            // their visuals are made this frame (the sprite authorities), and
            // cut this frame (the presentation follows observation).
            app.add_systems(
                Update,
                publish_transiting_drawn_bodies
                    .after(ambition_render::rendering::SpriteVisualSync)
                    .in_set(PortalObservationSet)
                    .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::host_adapter::{publish_portal_body_views, sync_portal_viewer, tag_portal_affordance_body};
    use ambition_platformer2d_core::{BodyKinematics, RoomGeometry};
    use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
    use ambition_portal2d_presentation::{PortalAffordanceBody, PortalBodyView, PortalViewers};
    use bevy::prelude::*;

    fn body(pos: Vec2) -> BodyKinematics {
        BodyKinematics {
            pos,
            vel: Vec2::ZERO,
            size: Vec2::new(24.0, 40.0),
            facing: 1.0,
        }
    }

    /// A host whose two limbs are in two pairs at once is cut by the deepest
    /// limb's pair, whichever limb the query meets first.
    #[test]
    fn a_host_with_limbs_in_two_pairs_is_cut_by_the_deepest_in_any_order() {
        use super::host_adapter::publish_threaded_host_pictures;
        use ambition_characters::actor::{Limb, LimbSlot};
        use ambition_portal2d::{PlacedPortal, PortalChannel, PortalChannelColor, PortalTransit};
        use ambition_portal2d_presentation::PortalTransitView;
        use ambition_render::rendering::FeatureVisual;
        let purple = PortalChannel::Authored(PortalChannelColor::Purple);
        let teal = PortalChannel::Authored(PortalChannelColor::Teal);
        let cut_by = |deep_first: bool| {
            let mut app = App::new();
            ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
                app.world_mut(),
                RoomGeometry(ambition_platformer2d_core::World::new("two pairs", Vec2::new(1000.0, 600.0), Vec2::ZERO, Vec::new())),
            );
            app.add_systems(Update, publish_threaded_host_pictures);
            // Two floor openings, their fronts up.
            for (channel, x) in [(purple, 0.0), (teal, 300.0)] {
                app.world_mut().spawn(PlacedPortal::fixed(channel, Vec2::new(x, 300.0), Vec2::new(0.0, -1.0), 40.0));
            }
            let host = app.world_mut().spawn((ambition_combat::FeatureId("giant".into()), body(Vec2::new(150.0, 200.0)))).id();
            // 30 into the purple opening, and 5 into the teal one.
            let deep = (purple, 0.0, 30.0, LimbSlot::HAND_LEFT);
            let shallow = (teal, 300.0, 5.0, LimbSlot::HAND_RIGHT);
            let limbs = if deep_first { [deep, shallow] } else { [shallow, deep] };
            for (channel, x, sunk, slot) in limbs {
                app.world_mut().spawn((
                    Limb { of: host, slot, home_offset: Vec2::ZERO },
                    body(Vec2::new(x, 300.0 - 20.0 + sunk)),
                    PortalTransit { straddling: channel, crossed: false },
                ));
            }
            let visual = app.world_mut().spawn(FeatureVisual { id: "giant".into() }).id();
            app.update();
            app.world().get::<PortalTransitView>(visual).map(|view| view.straddling)
        };
        assert_eq!(cut_by(true), Some(purple), "the deep limb met first");
        assert_eq!(cut_by(false), Some(purple), "the deep limb met second");
    }

    /// A body that is not the player (a dog) is drawn by a visual with its
    /// id. While the body straddles a portal the visual has the three facts
    /// the piece builder cuts it by; when the transit ends the transit fact
    /// goes and the tag stays, so the builder gives the whole sprite back. The
    /// control is a second body that does not transit: its visual gets none.
    #[test]
    fn a_body_that_is_not_the_player_is_cut_while_it_is_in_transit() {
        use super::host_adapter::publish_transiting_feature_bodies;
        use ambition_portal2d::{PortalChannel, PortalChannelColor, PortalTransit};
        use ambition_portal2d_presentation::{PortalSceneBody, PortalTransitView};
        use ambition_render::rendering::FeatureVisual;
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            RoomGeometry(ambition_platformer2d_core::World::new(
                "portal transit room",
                Vec2::new(1000.0, 600.0),
                Vec2::new(500.0, 300.0),
                Vec::new(),
            )),
        );
        app.add_systems(Update, publish_transiting_feature_bodies);
        let purple = PortalChannel::Authored(PortalChannelColor::Purple);
        let dog = app
            .world_mut()
            .spawn((
                ambition_combat::FeatureId("dog".into()),
                body(Vec2::new(500.0, 300.0)),
                PortalTransit { straddling: purple, crossed: false },
            ))
            .id();
        app.world_mut().spawn((ambition_combat::FeatureId("cat".into()), body(Vec2::new(100.0, 300.0))));
        let dog_visual = app.world_mut().spawn(FeatureVisual { id: "dog".into() }).id();
        let cat_visual = app.world_mut().spawn(FeatureVisual { id: "cat".into() }).id();
        app.update();

        let world = app.world();
        assert_eq!(world.get::<PortalTransitView>(dog_visual), Some(&PortalTransitView { straddling: purple }));
        assert!(world.get::<PortalSceneBody>(dog_visual).is_some());
        assert_eq!(world.get::<PortalBodyView>(dog_visual).map(|view| view.pos), Some(Vec2::new(500.0, 300.0)));
        assert!(world.get::<PortalSceneBody>(cat_visual).is_none(), "a body that does not transit was tagged");
        assert!(world.get::<PortalTransitView>(cat_visual).is_none());

        app.world_mut().entity_mut(dog).remove::<PortalTransit>();
        app.update();
        let world = app.world();
        assert!(world.get::<PortalTransitView>(dog_visual).is_none(), "the transit fact stayed after the transit");
        assert!(
            world.get::<PortalSceneBody>(dog_visual).is_some(),
            "the tag went with the transit, so the piece builder cannot give the sprite back"
        );
    }

    /// The player's visual has the facts the carry selects a body by while
    /// its body straddles a portal, through the host's own bridges: the
    /// scene-body tag, the pose and the transit. The carry draws a body's
    /// visual through a pair only while it has the transit fact.
    ///
    /// It lived in `ambition_content`, over a hand copy of these bridges (this
    /// crate is above that one). The copy published no `PortalTransitView`, so
    /// the carry saw no transit and drew nothing, and the test was red
    /// in the feature job only. Here it runs the shipped bridges.
    #[test]
    fn a_straddling_player_visual_has_the_facts_the_carry_selects_by() {
        use super::host_adapter::tag_portal_scene_bodies;
        use ambition_portal2d::{PlacedPortal, PortalChannel, PortalGunColor, PortalTransit};
        use ambition_portal2d_presentation::{PortalSceneBody, PortalTransitView};
        use ambition_render::rendering::PlayerVisual;
        let blue = PortalChannel::Gun(PortalGunColor::BLUE);
        let orange = PortalChannel::Gun(PortalGunColor::ORANGE);
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            RoomGeometry(ambition_platformer2d_core::World::new(
                "portal_test",
                Vec2::new(400.0, 400.0),
                Vec2::new(200.0, 360.0),
                Vec::new(),
            )),
        );
        app.add_systems(Update, (tag_portal_scene_bodies, publish_portal_body_views).chain());
        // A floor pair, so a body standing on the blue portal straddles its plane.
        let floor = Vec2::new(0.0, -1.0);
        app.world_mut()
            .spawn(PlacedPortal::fixed(blue, Vec2::new(200.0, 300.0), floor, ambition_portal2d::PORTAL_OPENING_HALF));
        app.world_mut()
            .spawn(PlacedPortal::fixed(orange, Vec2::new(300.0, 300.0), floor, ambition_portal2d::PORTAL_OPENING_HALF));
        // Its feet are below the plane (y 275..315, plane 300).
        let player = app
            .world_mut()
            .spawn((
                PlayerVisual,
                body(Vec2::new(200.0, 295.0)),
                ambition_platformer2d_core::BodyBaseSize { base_size: Vec2::new(24.0, 40.0) },
                Sprite::from_color(Color::WHITE, Vec2::new(24.0, 40.0)),
                Visibility::Inherited,
                PortalTransit { straddling: blue, crossed: false },
            ))
            .id();
        app.update();
        app.update();
        let world = app.world();
        assert!(world.get::<PortalSceneBody>(player).is_some(), "the player visual is not a scene body");
        assert_eq!(world.get::<PortalTransitView>(player), Some(&PortalTransitView { straddling: blue }));
        assert_eq!(world.get::<PortalBodyView>(player).map(|view| view.pos), Some(Vec2::new(200.0, 295.0)));
    }

    /// The portal affordances (held gun, disorientation indicator) follow the
    /// CONTROLLED body, so possessing an actor moves the gun into the possessed
    /// actor's hand — and takes it out of the home avatar's.
    ///
    /// The untag half is the one that regresses silently: a tagger that only
    /// ever inserts leaves two tagged bodies, and the presentation crate's
    /// `single()` query then silently draws NOTHING.
    #[test]
    fn portal_affordances_follow_the_controlled_body_through_possession() {
        let mut app = App::new();
        app.init_resource::<ControlledSubject>();
        app.add_systems(
            Update,
            (tag_portal_affordance_body, publish_portal_body_views).chain(),
        );

        let home = app.world_mut().spawn(body(Vec2::new(10.0, 20.0))).id();
        let possessed = app.world_mut().spawn(body(Vec2::new(300.0, 40.0))).id();

        // Driving the home avatar.
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(home);
        app.update();
        assert!(app.world().get::<PortalAffordanceBody>(home).is_some());
        assert!(app.world().get::<PortalAffordanceBody>(possessed).is_none());
        assert_eq!(
            app.world().get::<PortalBodyView>(home).unwrap().pos,
            Vec2::new(10.0, 20.0),
            "the published view carries the tagged body's pose",
        );

        // Possess the actor: the tag MOVES, it does not accumulate.
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(possessed);
        app.update();
        assert!(
            app.world().get::<PortalAffordanceBody>(home).is_none(),
            "the home avatar must lose the affordance tag, or `single()` finds two \
             carriers and the gun stops drawing entirely",
        );
        assert!(app.world().get::<PortalAffordanceBody>(possessed).is_some());
        assert_eq!(
            app.world().get::<PortalBodyView>(possessed).unwrap().pos,
            Vec2::new(300.0, 40.0),
        );

        // Exactly one carrier at all times.
        let carriers = app
            .world_mut()
            .query_filtered::<Entity, With<PortalAffordanceBody>>()
            .iter(app.world())
            .count();
        assert_eq!(carriers, 1, "exactly one portal carrier");
    }

    /// The view's `size` is the collision box of the body: the box the portal
    /// core transits and the presentation cuts into pieces. A body that lies
    /// along sideways gravity is 40 on x and 24 on y, and the view must say
    /// so, or the pieces are cut from a box the body does not have.
    #[test]
    fn the_published_size_is_the_box_the_body_has() {
        use ambition_platformer2d_core::SweepSample;
        use ambition_portal2d_presentation::PortalSceneBody;
        let mut app = App::new();
        app.add_systems(Update, publish_portal_body_views);
        let tall = body(Vec2::new(10.0, 20.0));
        let no_record = app.world_mut().spawn((tall, PortalSceneBody)).id();
        let level = app
            .world_mut()
            .spawn((tall, PortalSceneBody, SweepSample::at_rest(tall, Vec2::new(0.0, 1.0))))
            .id();
        let lying = app
            .world_mut()
            .spawn((tall, PortalSceneBody, SweepSample::at_rest(tall, Vec2::new(1.0, 0.0))))
            .id();
        app.update();
        let size = |entity| app.world().get::<PortalBodyView>(entity).unwrap().size;
        assert_eq!(size(no_record), Vec2::new(24.0, 40.0), "control: a body with no record is level");
        assert_eq!(size(level), Vec2::new(24.0, 40.0), "control: a body in normal gravity is level");
        assert_eq!(size(lying), Vec2::new(40.0, 24.0), "a body that lies along sideways gravity");
    }

    /// A3: one eye for each live room that a local view frames.
    ///
    /// Alice and Carl are in the first live room, and Bob lies along sideways
    /// gravity in the second. `views` is (the id of a view, the body it looks
    /// at); the views are spawned in the opposite order, so that the order of
    /// the eyes is the order of the ids and not of the spawns. Returns each
    /// eye: its room, its place, its half, and the id of its observer's view.
    fn eyes_of(views: &[(u8, Option<&str>)]) -> Vec<(usize, Vec2, Vec2, Option<u8>)> {
        use ambition_platformer2d_core::SweepSample;
        use ambition_platformer2d_shared_tangle::lifecycle::{
            insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
        };
        use ambition_sim_view::{LocalView, LocalViewId, ResolvedViewSubject};
        let room_size = Vec2::new(1000.0, 600.0);
        let geometry =
            |name: &str| RoomGeometry(ambition_platformer2d_core::World::new(name, room_size, room_size * 0.5, Vec::new()));
        let mut app = App::new();
        insert_live_room_component(app.world_mut(), geometry("the first room"));
        let rooms = [LiveRoomInstance::ACTIVATION, LiveRoomInstance::from_ordinal(1)];
        spawn_live_room(app.world_mut(), rooms[1], geometry("the second room"));
        app.init_resource::<ControlledSubject>();
        app.init_resource::<PortalViewers>();
        app.add_systems(Update, sync_portal_viewer);

        let tall = |x: f32| body(Vec2::new(x, 50.0));
        let alice = app.world_mut().spawn((tall(100.0), InRoomInstance(rooms[0]))).id();
        let carl = app.world_mut().spawn((tall(300.0), InRoomInstance(rooms[0]))).id();
        let bob = app
            .world_mut()
            .spawn((
                tall(700.0),
                InRoomInstance(rooms[1]),
                SweepSample::at_rest(tall(700.0), Vec2::new(1.0, 0.0)),
            ))
            .id();
        let named = |name: &str| match name {
            "alice" => alice,
            "carl" => carl,
            "bob" => bob,
            other => panic!("no body named {other}"),
        };
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(alice);
        let mut view_ids = Vec::new();
        for (id, subject) in views.iter().rev() {
            let view = app
                .world_mut()
                .spawn((LocalView, LocalViewId(*id), ResolvedViewSubject(subject.map(named))))
                .id();
            view_ids.push((view, *id));
        }
        app.update();
        app.world()
            .resource::<PortalViewers>()
            .iter()
            .map(|viewer| {
                (
                    rooms.iter().position(|room| Some(*room) == viewer.room).expect("an eye is in a live room"),
                    viewer.eye,
                    viewer.half_size,
                    viewer.observer.map(|observer| {
                        view_ids.iter().find(|(view, _)| *view == observer).expect("the observer is a view").1
                    }),
                )
            })
            .collect()
    }

    #[test]
    fn each_live_room_a_view_frames_has_the_eye_of_that_views_body() {
        let level = Vec2::new(12.0, 20.0);
        let lying = Vec2::new(20.0, 12.0);
        let alice = (0, Vec2::new(100.0, 50.0), level);
        let bob = (1, Vec2::new(700.0, 50.0), lying);

        // The control, the whole seam before: no local view, and the one eye
        // is the controlled subject. The second room has no eye.
        assert_eq!(eyes_of(&[]), vec![(alice.0, alice.1, alice.2, None)]);
        // One view that names no body frames the controlled subject.
        assert_eq!(eyes_of(&[(0, None)]), vec![(alice.0, alice.1, alice.2, Some(0))]);

        // Two views in two live rooms: an eye in each room. Bob's eye has the
        // half of the box he has, which lies along his gravity.
        assert_eq!(
            eyes_of(&[(0, Some("alice")), (1, Some("bob"))]),
            vec![(alice.0, alice.1, alice.2, Some(0)), (bob.0, bob.1, bob.2, Some(1))],
            "(the room, the eye, the half, the view of the observer)"
        );

        // Two views of ONE room: the room has the eye of the first view.
        assert_eq!(
            eyes_of(&[(0, Some("alice")), (1, Some("carl"))]),
            vec![(alice.0, alice.1, alice.2, Some(0))],
            "a room has one eye"
        );
        assert_eq!(
            eyes_of(&[(0, Some("carl")), (1, Some("alice"))]),
            vec![(0, Vec2::new(300.0, 50.0), level, Some(0))],
            "the first view by id, not by spawn"
        );
    }

    /// A despawned subject must not resurrect as a command-spawned shell.
    #[test]
    fn a_despawned_controlled_subject_does_not_resurrect() {
        let mut app = App::new();
        app.init_resource::<ControlledSubject>();
        app.add_systems(Update, tag_portal_affordance_body);

        let ghost = app.world_mut().spawn(body(Vec2::ZERO)).id();
        app.world_mut().resource_mut::<ControlledSubject>().0 = Some(ghost);
        app.world_mut().entity_mut(ghost).despawn();
        app.update();

        assert!(
            app.world().get_entity(ghost).is_err(),
            "no resurrected shell"
        );
    }

    /// The host projects the active room's portal rules into the two rule
    /// resources that the presentation crate reads, and it follows the player
    /// out of the room. The presentation crate cannot see rooms, so without the
    /// projection a game's rule reaches no reader.
    #[test]
    fn the_host_projects_the_active_rooms_portal_rules() {
        use ambition_combat::scoped_rules::{DeclareRulesExt as _, RulesScope};
        use ambition_platformer2d_world::rooms::{RoomMetadata, RoomSet, RoomSpec};
        use ambition_portal2d_presentation::{
            PortalCameraTransitMode, PortalCameraTransitRule, PortalViewConeMode,
            PortalViewConeRule,
        };

        let room = |id: &str, mode: Option<&str>| {
            let mut spec = RoomSpec::new(
                id,
                ambition_platformer2d_core::World::new(
                    id,
                    ambition_platformer2d_core::Vec2::splat(64.0),
                    ambition_platformer2d_core::Vec2::ZERO,
                    Vec::new(),
                ),
            );
            spec.metadata = RoomMetadata {
                mode: mode.map(str::to_string),
                ..Default::default()
            };
            spec
        };
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()));
        // The plugin loads the portal gun's art at startup.
        app.init_asset::<bevy::image::Image>();
        // The presentation plugin owns the two rule resources. Only its view
        // windows are on, and they wait for a render stack this app does not have.
        app.add_plugins(ambition_portal2d_presentation::PortalPresentationPlugin {
            portal_quads: false,
            body_pieces: false,
            far_side_compositing: false,
            gun_indicator: false,
            disorientation: false,
            view_cones: true,
        });
        app.add_plugins(super::PortalObservationPlugin);
        // What the plugin's other observers read. This test reads none of them.
        app.init_resource::<ControlledSubject>();
        app.init_resource::<ambition_platformer2d_runtime::host_seams::DeveloperRuntimeState>();
        app.add_message::<ambition_portal2d::PortalBodyTransited>();
        app.init_resource::<ambition_sim_view::GroundItemsView>();
        app.declare_rules(RulesScope::Mode("fight"), PortalCameraTransitMode::Pop);
        app.declare_rules(RulesScope::Mode("fight"), PortalViewConeMode::Static);
        ambition_platformer2d_world::rooms::insert_room_set(
            app.world_mut(),
            RoomSet::from_parts_or_panic(
                "home",
                vec![room("home", None), room("arena", Some("fight"))],
                Vec::new(),
            ),
        );
        let visit = |app: &mut App, id: &str| {
            ambition_platformer2d_world::rooms::seat_sole_live_room_by_id(app.world_mut(), id)
                .expect("the fixture holds the room");
            app.update();
            (
                *app.world().resource::<PortalCameraTransitRule>(),
                *app.world().resource::<PortalViewConeRule>(),
            )
        };

        assert_eq!(
            visit(&mut app, "arena"),
            (
                PortalCameraTransitRule(Some(PortalCameraTransitMode::Pop)),
                PortalViewConeRule(Some(PortalViewConeMode::Static)),
            ),
            "the arena's game stated its portal presentation and no reader got it"
        );
        assert_eq!(
            visit(&mut app, "home"),
            (PortalCameraTransitRule(None), PortalViewConeRule(None)),
            "the arena's portal presentation followed the player home"
        );
    }

    /// Each local view keeps its own portal anchor (finding C of the review
    /// of 15d0cf99c): two views whose bodies cross at once are each carried,
    /// each from its own camera, and a third view, whose body did not cross,
    /// is not.
    #[test]
    fn two_views_whose_bodies_cross_at_once_are_each_carried() {
        use ambition_platformer2d_shared_tangle::camera_ease::CameraEaseState;
        use ambition_portal2d::{PlacedPortal, PortalBodyTransited, PortalChannel, PortalChannelColor, PortalTransit};
        use ambition_portal2d_presentation::{
            PortalCameraContinuityConfig, PortalCameraContinuitySelection, PortalCameraContinuityState,
            PortalCameraTransitMode,
        };
        let mut app = App::new();
        ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component(
            app.world_mut(),
            RoomGeometry(ambition_platformer2d_core::World::new(
                "continuity room",
                Vec2::new(2000.0, 1000.0),
                Vec2::new(1000.0, 500.0),
                Vec::new(),
            )),
        );
        app.insert_resource(PortalCameraContinuitySelection { mode: PortalCameraTransitMode::Continuous });
        app.insert_resource(PortalCameraContinuityConfig {
            debug_log: false,
            max_entry_screen_offset: Vec2::splat(5000.0),
            ..Default::default()
        });
        app.add_message::<PortalBodyTransited>();
        app.add_systems(Update, super::host_adapter::apply_portal_camera_continuity);
        let (purple, yellow) = (
            PortalChannel::Authored(PortalChannelColor::Purple),
            PortalChannel::Authored(PortalChannelColor::Yellow),
        );
        // A wall pair: in at x = 100 facing +x, out at x = 1500 facing -x.
        app.world_mut().spawn(PlacedPortal::fixed(purple, Vec2::new(100.0, 500.0), Vec2::X, 46.0));
        app.world_mut().spawn(PlacedPortal::fixed(yellow, Vec2::new(1500.0, 500.0), -Vec2::X, 46.0));
        let crossed = |app: &mut App, y: f32| {
            app.world_mut()
                .spawn((body(Vec2::new(1495.0, y)), PortalTransit { straddling: yellow, crossed: true }))
                .id()
        };
        let (a, b) = (crossed(&mut app, 500.0), crossed(&mut app, 510.0));
        let c = app.world_mut().spawn(body(Vec2::new(700.0, 500.0))).id();
        let view = |app: &mut App, subject: Entity, camera_x: f32| {
            app.world_mut()
                .spawn((
                    ambition_sim_view::LocalView,
                    CameraEaseState::default(),
                    ambition_sim_view::ResolvedViewSubject(Some(subject)),
                    PortalCameraContinuityState {
                        last_host_camera_world: Some(Vec2::new(camera_x, 500.0)),
                        ..Default::default()
                    },
                ))
                .id()
        };
        let (view_a, view_b, view_c) = (view(&mut app, a, 300.0), view(&mut app, b, 250.0), view(&mut app, c, 700.0));
        for body in [a, b] {
            app.world_mut().write_message(PortalBodyTransited {
                body,
                enter_normal: Vec2::X,
                exit_normal: -Vec2::X,
                facing_flip: false,
                input_warp: false,
                exit_pos: Vec2::new(1495.0, 500.0),
            });
        }
        app.update();
        let state = |app: &App, view: Entity| *app.world().get::<PortalCameraContinuityState>(view).unwrap();
        // Each camera is carried from where it was, through the pair: 200
        // and 150 in front of the entry are 200 and 150 past the exit's
        // plane, where they are behind the body as before.
        assert_eq!(state(&app, view_a).target_camera_world, Some(Vec2::new(1700.0, 500.0)));
        assert_eq!(state(&app, view_b).target_camera_world, Some(Vec2::new(1650.0, 500.0)));
        assert_eq!(state(&app, view_c).active_weight(), 0.0, "a view whose body did not cross is not carried");
    }
}

pub use host_adapter::{
    apply_portal_camera_continuity, load_portal_gun_art, portal_dev_toggle_system,
    publish_portal_body_views, publish_transiting_feature_bodies, sync_portal_camera_continuity_focus,
    sync_portal_debug_overlay_to_f1, sync_portal_viewer,
    tag_portal_affordance_body, tag_portal_camera_continuity_camera, tag_portal_scene_bodies,
    PortalContinuityCameraTagged, PortalObservationPlugin,
};
