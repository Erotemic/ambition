//! Print-only diagnostic for bubble-shield placement.
//!
//! The probe compares shield view rows, the player's authoritative and presented pose,
//! `BubbleShieldVisual` entities, nearby marker-free sprites, and active camera transforms at
//! multiple player positions. It intentionally makes no gameplay assertion; its purpose is to
//! distinguish an incorrectly positioned shield visual from unrelated sprite art.
//!
//! Run with `cargo test -p ambition_app --test app_it -- shield_ring_probe --ignored --nocapture`.

use std::time::Duration;

use bevy::prelude::*;

use ambition_app::app::{
    build_visible_app_with, StartRoomMustResolve, StartRoomOverride, VisibleRenderMode,
};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::body_clusters::BodyShieldState;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::lifecycle::SessionRoot;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use ambition_platformer2d::render::rendering::bubble_shield::{
    BubbleShieldSprite, BubbleShieldVisual,
};
use ambition_platformer2d::sim_view::{PresentedPose, ShieldRingsView};

/// The room the reported capture was taken in.
const HALL: &str = "hall_of_characters";
/// Frames the boot may take before we give up waiting for a player body.
const BOOT_CAP: usize = 1200;

/// Step one frame, with a sliver of wall clock so the asset threads make
/// progress — the Hall stages ~130 bodies and a body whose sheet never
/// decodes reports a fallback size.
fn step(app: &mut App) {
    app.update();
    std::thread::sleep(Duration::from_millis(4));
}

fn player_body(app: &mut App) -> Option<(Entity, BodyKinematics)> {
    let mut query = app
        .world_mut()
        .query_filtered::<(Entity, &BodyKinematics), PrimaryPlayerOnly>();
    let world = app.world();
    query.iter(world).next().map(|(e, kin)| (e, *kin))
}

/// Boot the shipped visible composition straight into the Hall — the same
/// composition inputs `capture_scene <room> <character>` sets, through the same
/// `build_visible_app_with` hook it uses.
fn hall_app() -> App {
    let mut app = build_visible_app_with(VisibleRenderMode::NoWindow, false, |app| {
        app.insert_resource(StartRoomOverride(HALL.to_string()));
        // Loud rather than quiet: a capture that photographs the hub instead of
        // the room asked for is this tool's worst failure mode, and a probe that
        // measures the wrong room is the same mistake wearing a test's clothes.
        app.insert_resource(StartRoomMustResolve);
    });
    for _ in 0..BOOT_CAP {
        step(&mut app);
        if player_body(&mut app).is_some() {
            return app;
        }
    }
    panic!("no primary player body appeared within {BOOT_CAP} frames of booting `{HALL}`");
}

fn hold(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
}

/// The room's size, which is the whole of `world_to_bevy`'s input besides the
/// point itself (`Vec3::new(p.x - size.x * 0.5, size.y * 0.5 - p.y, z)`).
fn room_size(app: &mut App) -> Option<ae::Vec2> {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::RoomGeometry, With<SessionRoot>>();
    let world = app.world();
    query.iter(world).next().map(|geometry| geometry.0.size)
}

fn print_snapshot(app: &mut App, label: &str) {
    println!("\n================ {label} ================");

    // ── 3. every raised shield in the world ─────────────────────────────────
    let shielders: Vec<String> = {
        let mut query = app.world_mut().query::<(
            Entity,
            &BodyKinematics,
            &BodyShieldState,
            Option<&Name>,
            Option<&PresentedPose>,
        )>();
        let world = app.world();
        query
            .iter(world)
            .filter(|(_, _, shield, _, _)| shield.active)
            .map(|(entity, kin, shield, name, presented)| {
                format!(
                    "    {entity:?} name={:?} kin.pos=({:.2}, {:.2}) kin.size=({:.2}, {:.2}) \
                     presented={} parrying={}",
                    name.map(|n| n.as_str().to_string()),
                    kin.pos.x,
                    kin.pos.y,
                    kin.size.x,
                    kin.size.y,
                    presented.map_or("<none>".to_string(), |p| format!(
                        "({:.2}, {:.2})",
                        p.presented().x,
                        p.presented().y
                    )),
                    shield.parrying(),
                )
            })
            .collect()
    };

    // ── 2. the player ───────────────────────────────────────────────────────
    let player = {
        let mut query = app.world_mut().query_filtered::<(
            Entity,
            &BodyKinematics,
            Option<&PresentedPose>,
            Option<&BodyShieldState>,
        ), PrimaryPlayerOnly>();
        let world = app.world();
        query
            .iter(world)
            .next()
            .map(|(entity, kin, presented, shield)| {
                (
                    entity,
                    *kin,
                    presented.map(|p| p.presented()),
                    shield.map(|s| (s.active, s.parrying())),
                )
            })
    };
    match player {
        Some((entity, kin, presented, shield)) => println!(
            "player {entity:?}\n  kin.pos  = ({:.3}, {:.3})\n  kin.size = ({:.3}, {:.3})\n  \
             presented = {}\n  shield (active, parrying) = {shield:?}",
            kin.pos.x,
            kin.pos.y,
            kin.size.x,
            kin.size.y,
            presented.map_or("<no PresentedPose>".to_string(), |p| format!(
                "({:.3}, {:.3})",
                p.x, p.y
            )),
        ),
        None => println!("player: <none>"),
    }

    // ── 1. the read-model the renderer actually positions rings from ────────
    let rings: Vec<ambition_platformer2d::sim_view::ShieldRingFact> =
        app.world().resource::<ShieldRingsView>().0.clone();
    println!("ShieldRingsView.0.len() = {}", rings.len());
    for (i, ring) in rings.iter().enumerate() {
        println!(
            "  [{i}] pos=({:.3}, {:.3}) size=({:.3}, {:.3}) parrying={} \
             => custom_size would be ({:.3}, {:.3})",
            ring.pos.x,
            ring.pos.y,
            ring.size.x,
            ring.size.y,
            ring.parrying,
            ring.size.x * 1.55,
            ring.size.y * 1.25,
        );
    }

    println!(
        "bodies with BodyShieldState.active == true: {}",
        shielders.len()
    );
    for row in &shielders {
        println!("{row}");
    }

    // ── 4. the pooled ring sprites themselves ───────────────────────────────
    let visuals: Vec<String> = {
        let mut query = app.world_mut().query_filtered::<(
            Entity,
            &Transform,
            &GlobalTransform,
            &Sprite,
            &Visibility,
            Option<&InheritedVisibility>,
            Option<&bevy::sprite::Anchor>,
            Option<&Name>,
        ), With<BubbleShieldVisual>>();
        let world = app.world();
        query
            .iter(world)
            .map(
                |(entity, transform, global, sprite, vis, inherited, anchor, name)| {
                    format!(
                        "  {entity:?} name={:?}\n    Transform.translation = ({:.3}, {:.3}, {:.3})\n    \
                         GlobalTransform.translation = ({:.3}, {:.3}, {:.3})\n    \
                         Transform.scale = ({:.3}, {:.3})\n    \
                         Sprite.custom_size = {:?}\n    Sprite.color = {:?}\n    \
                         Sprite.anchor = {:?}\n    Visibility = {vis:?}  InheritedVisibility = {:?}",
                        name.map(|n| n.as_str().to_string()),
                        transform.translation.x,
                        transform.translation.y,
                        transform.translation.z,
                        global.translation().x,
                        global.translation().y,
                        global.translation().z,
                        transform.scale.x,
                        transform.scale.y,
                        sprite.custom_size,
                        sprite.color,
                        anchor,
                        inherited.map(|i| i.get()),
                    )
                },
            )
            .collect()
    };
    println!("entities With<BubbleShieldVisual>: {}", visuals.len());
    for row in &visuals {
        println!("{row}");
    }

    // ── 5. anybody ELSE drawing the shield texture ──────────────────────────
    let handle = app
        .world()
        .get_resource::<BubbleShieldSprite>()
        .map(|sprite| sprite.handle.clone());
    match handle {
        None => println!("BubbleShieldSprite resource: <absent>"),
        Some(handle) => {
            let others: Vec<String> = {
                let mut query = app.world_mut().query_filtered::<(
                    Entity,
                    &Sprite,
                    &Transform,
                    &Visibility,
                    Option<&Name>,
                ), Without<BubbleShieldVisual>>();
                let world = app.world();
                query
                    .iter(world)
                    .filter(|(_, sprite, _, _, _)| sprite.image == handle)
                    .map(|(entity, sprite, transform, vis, name)| {
                        format!(
                            "  {entity:?} name={:?} translation=({:.3}, {:.3}, {:.3}) \
                             custom_size={:?} color={:?} {vis:?}",
                            name.map(|n| n.as_str().to_string()),
                            transform.translation.x,
                            transform.translation.y,
                            transform.translation.z,
                            sprite.custom_size,
                            sprite.color,
                        )
                    })
                    .collect()
            };
            println!(
                "other sprites drawing the SAME shield texture (not BubbleShieldVisual): {}",
                others.len()
            );
            for row in &others {
                println!("{row}");
            }
        }
    }

    // ── 6. the frame the numbers above are expressed in ─────────────────────
    match (room_size(app), player) {
        (Some(size), Some((_, kin, presented, _))) => {
            let z = ae::config::WORLD_Z_PLAYER + 0.05;
            let from_sim = ae::config::world_size_to_bevy(size, kin.pos, z);
            println!(
                "room size = ({:.2}, {:.2})\n  world_to_bevy(kin.pos)   = ({:.3}, {:.3}, {:.3})",
                size.x, size.y, from_sim.x, from_sim.y, from_sim.z,
            );
            if let Some(presented) = presented {
                let from_presented = ae::config::world_size_to_bevy(size, presented, z);
                println!(
                    "  world_to_bevy(presented) = ({:.3}, {:.3}, {:.3})",
                    from_presented.x, from_presented.y, from_presented.z,
                );
            }
        }
        (size, _) => println!("room size = {size:?} (no player to convert)"),
    }

    // ── EVERY DRAWABLE STANDING NEAR THE PLAYER ─────────────────────────────
    //
    // the query that is not keyed to a marker, and that is the point.
    // Items 1–5 can only find things that are already known to be the shield;
    // a capture taken with the shield up shows TWO ring-shaped artefacts and
    // this crate owns exactly one of them, so the instrument has to be able to
    // see a drawable it was not told about. Anything within 150 bevy units of
    // the player is a short list, and the second ring is in it or it is not an
    // entity at all.
    if let Some((_, kin, _, _)) = player {
        if let Some(size) = room_size(app) {
            let anchor = ae::config::world_size_to_bevy(size, kin.pos, 0.0).truncate();
            let mut near: Vec<(f32, String)> = {
                let mut query = app.world_mut().query::<(
                    Entity,
                    &GlobalTransform,
                    &Sprite,
                    &Visibility,
                    Option<&InheritedVisibility>,
                    Option<&bevy::sprite::Anchor>,
                    Option<&Name>,
                )>();
                let world = app.world();
                query
                    .iter(world)
                    .filter_map(
                        |(entity, global, sprite, vis, inherited, sprite_anchor, name)| {
                            let at = global.translation();
                            let offset = bevy::math::Vec2::new(at.x - anchor.x, at.y - anchor.y);
                            (offset.length() <= 150.0).then(|| {
                                (
                                    offset.length(),
                                    format!(
                                    "  {entity:?} name={:?} offset_from_player=({:+.2}, {:+.2}) \
                                     z={:.2} custom_size={:?} anchor={:?} \
                                     => drawn-centre offset ({:+.2}, {:+.2}) \
                                     color={:?} {vis:?} inherited={:?}",
                                    name.map(|n| n.as_str().to_string()),
                                    offset.x,
                                    offset.y,
                                    at.z,
                                    sprite.custom_size,
                                    sprite_anchor.map(|a| a.0),
                                    // Bevy draws the quad centred at
                                    // `translation - anchor * custom_size`, so a
                                    // non-zero anchor is exactly a displacement
                                    // between where an entity IS and where its
                                    // picture LANDS.
                                    offset.x
                                        - sprite_anchor.map_or(0.0, |a| a.0.x)
                                            * sprite.custom_size.map_or(0.0, |s| s.x),
                                    offset.y
                                        - sprite_anchor.map_or(0.0, |a| a.0.y)
                                            * sprite.custom_size.map_or(0.0, |s| s.y),
                                    sprite.color,
                                    inherited.map(|i| i.get()),
                                ),
                                )
                            })
                        },
                    )
                    .collect()
            };
            near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            println!(
                "sprites within 150 bevy units of the player: {}",
                near.len()
            );
            for (_, row) in &near {
                println!("{row}");
            }
        }
    }

    let local_view = ambition_platformer2d::sim_view::the_only_view(app.world_mut());
    if let Some(camera) =
        app.world()
            .entity(local_view)
            .get::<ambition_platformer2d::sim_view::camera_snapshot::ResolvedCameraSnapshot>()
    {
        let snapshot = &camera.snapshot;
        let scale = snapshot.orthographic_scale;
        println!(
            "camera snapshot: orthographic_scale={scale:.4} visible_view=({:.1}, {:.1}) \
             zoom={:.3} center_world=({:.2}, {:.2}) follow_world=({:.2}, {:.2})",
            snapshot.visible_view.x,
            snapshot.visible_view.y,
            snapshot.zoom_multiplier,
            snapshot.center_world.x,
            snapshot.center_world.y,
            camera.follow_world.x,
            camera.follow_world.y,
        );
        if scale.abs() > f32::EPSILON {
            for (i, ring) in rings.iter().enumerate() {
                println!(
                    "  ring[{i}] on screen: quad {:.1} x {:.1} px; the drawn ring is \
                     ~0.92 of that ({:.1} x {:.1} px) because the texture's outer \
                     radius is 0.46 of its 64px extent",
                    ring.size.x * 1.55 / scale,
                    ring.size.y * 1.25 / scale,
                    ring.size.x * 1.55 / scale * 0.92,
                    ring.size.y * 1.25 / scale * 0.92,
                );
            }
        }
    }
    if let Some(viewport) =
        app.world()
            .entity(local_view)
            .get::<ambition_platformer2d::sim_view::camera_snapshot::CameraViewport>()
    {
        println!(
            "camera viewport = ({:.1}, {:.1}) px",
            viewport.px.x, viewport.px.y
        );
    }

    let cameras: Vec<String> = {
        let mut query = app
            .world_mut()
            .query::<(Entity, &Camera, &GlobalTransform, Option<&Name>)>();
        let world = app.world();
        query
            .iter(world)
            .filter(|(_, camera, _, _)| camera.is_active)
            .map(|(entity, camera, global, name)| {
                format!(
                    "  {entity:?} name={:?} order={} translation=({:.3}, {:.3}, {:.3})",
                    name.map(|n| n.as_str().to_string()),
                    camera.order,
                    global.translation().x,
                    global.translation().y,
                    global.translation().z,
                )
            })
            .collect()
    };
    println!("active cameras: {}", cameras.len());
    for row in &cameras {
        println!("{row}");
    }
}

/// The measurement. Print-only; it asserts nothing on purpose.
#[test]
#[ignore = "print-only probe for the misplaced bubble-shield ring (queue D55)"]
fn print_where_the_bubble_shield_ring_is_put() {
    let mut app = hall_app();
    // Let the Hall settle before anything is pressed, so the numbers are not a
    // half-loaded room's.
    for _ in 0..60 {
        step(&mut app);
    }
    print_snapshot(&mut app, "SHIELD DOWN (baseline)");

    // ── Raise the shield through the REAL input path ────────────────────────
    //
    // The same key the capture used: `quick_action` is E on the default preset
    // (`ambition_input::presets`) and `ambition_input::control` maps a pressed
    // QuickAction to `shield_held`, which `resolve_shield` turns into
    // `BodyShieldState.active`. `ButtonInput::press` is exactly what
    // `capture_scene`'s `hold:e` does, and Bevy's per-frame clear only drops
    // `just_pressed`, so one press is a held key.
    hold(&mut app, KeyCode::KeyE);
    for _ in 0..30 {
        step(&mut app);
    }
    print_snapshot(&mut app, "SHIELD HELD, STANDING STILL");

    // ── Walk right with the shield still up ─────────────────────────────────
    //
    // this is the half that separates "anchored to the body" from "anchored
    // to the camera": while the camera follows the player, BOTH read as
    // travelling with him on screen, and only the world-space numbers tell them
    // apart.
    hold(&mut app, KeyCode::ArrowRight);
    for _ in 0..90 {
        step(&mut app);
    }
    print_snapshot(&mut app, "SHIELD HELD, AFTER WALKING RIGHT");

    // ── Release, so a reader can see what the shield actually owns ──────────
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyE);
        keys.release(KeyCode::ArrowRight);
    }
    for _ in 0..60 {
        step(&mut app);
    }
    print_snapshot(&mut app, "SHIELD RELEASED");
}
