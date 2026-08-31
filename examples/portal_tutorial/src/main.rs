//! A minimal host for `ambition_portal2d` and `ambition_portal2d_presentation`.
//!
//! The demo deliberately avoids the full Ambition game stack. It provides only
//! the four things a portal host must own:
//!
//! 1. a simulation clock,
//! 2. body motion,
//! 3. two linked portal entities,
//! 4. a sim-to-render observation bridge.

use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::time::SimDt;
use ambition_portal2d::{
    portal_half_extent, PlacedPortal, PortalBody, PortalChannelColor, PortalPlugin, PortalPolicy,
    PortalSet,
};
use ambition_portal2d_presentation::{
    PortalBodyView, PortalPresentationPlugin, PortalPresentationSet, PortalSceneBody,
    PortalWorldFrame,
};
use bevy::prelude::ButtonInput;
use bevy::prelude::KeyCode;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use KeyCode::*;

//use bevy::keyboard::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp, KeyA, KeyD, KeyS, KeyW};

const WORLD_SIZE: Vec2 = Vec2::new(960.0, 540.0);
const BODY_START: Vec2 = Vec2::new(600.0, 270.0);
const BODY_SPEED: f32 = 180.0;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TutorialSet {
    Clock,
    MoveBody,
    Observe,
}

#[derive(Component)]
struct TutorialBody;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Ambition portal tutorial".into(),
            resolution: WindowResolution::new(960, 540),
            resizable: false,
            ..default()
        }),
        ..default()
    }));

    // The real reusable portal crates. The tutorial disables gun-only and
    // expensive through-portal effects so the first example stays focused on
    // linked apertures, transit, and body decomposition.
    app.add_plugins(PortalPlugin);
    app.add_plugins(PortalPresentationPlugin {
        gun_indicator: false,
        disorientation: false,
        view_cones: false,
        ..default()
    });

    app.init_resource::<SimDt>();
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.035, 0.065)));

    // The host owns ordering around the portal systems. Motion must happen
    // before transit; presentation observes the post-transit body pose.
    app.configure_sets(
        Update,
        (
            TutorialSet::Clock,
            TutorialSet::MoveBody,
            PortalSet::Transit,
            TutorialSet::Observe,
            PortalPresentationSet,
        )
            .chain(),
    );

    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            mirror_sim_dt.in_set(TutorialSet::Clock),
            move_body.in_set(TutorialSet::MoveBody),
            observe_body.in_set(TutorialSet::Observe),
        ),
    );

    app.run();
}

fn setup(mut commands: Commands, mut frame: ResMut<PortalWorldFrame>) {
    frame.size = WORLD_SIZE;

    commands.spawn(Camera2d);

    // A little room dressing. These are ordinary Bevy sprites; the portals
    // themselves are drawn by `ambition_portal2d_presentation`.
    let wall_color = Color::srgb(0.14, 0.17, 0.24);
    spawn_rect(
        &mut commands,
        Vec2::new(100.0, WORLD_SIZE.y * 0.5),
        Vec2::new(32.0, 300.0),
        wall_color,
        0.0,
    );
    spawn_rect(
        &mut commands,
        Vec2::new(860.0, WORLD_SIZE.y * 0.5),
        Vec2::new(32.0, 300.0),
        wall_color,
        0.0,
    );

    let left_normal = Vec2::X;
    let right_normal = Vec2::NEG_X;
    commands.spawn((
        PlacedPortal::fixed(
            PortalChannelColor::Purple.channel(),
            Vec2::new(118.0, WORLD_SIZE.y * 0.5),
            left_normal,
            portal_half_extent(left_normal),
        ),
        Name::new("Purple portal"),
    ));
    commands.spawn((
        PlacedPortal::fixed(
            PortalChannelColor::Yellow.channel(),
            Vec2::new(842.0, WORLD_SIZE.y * 0.5),
            right_normal,
            portal_half_extent(right_normal),
        ),
        Name::new("Yellow portal"),
    ));

    let body_size = Vec2::new(42.0, 42.0);
    let body = BodyKinematics {
        pos: BODY_START,
        vel: Vec2::new(-BODY_SPEED, 0.0),
        size: body_size,
        facing: -1.0,
    };
    commands.spawn((
        TutorialBody,
        body,
        PortalBody,
        PortalPolicy {
            reorient: true,
            carry_velocity: true,
        },
        PortalSceneBody,
        PortalBodyView {
            pos: body.pos,
            size: body.size,
            facing: body.facing,
        },
        Sprite::from_color(Color::srgb(0.88, 0.93, 1.0), body_size),
        Transform::from_translation(world_to_render(body.pos, 20.0)),
        Name::new("Tutorial portal body"),
    ));

    commands.spawn((
        Text2d::new(
            "The square uses the real portal transit code.\n\
             No LDtk, gun, or full game host is involved.",
        ),
        TextFont {
            font_size: FontSize::Px(22.0),
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.86, 0.94)),
        Transform::from_translation(world_to_render(Vec2::new(480.0, 52.0), 30.0)),
    ));
}

fn mirror_sim_dt(time: Res<Time>, mut sim_dt: ResMut<SimDt>) {
    sim_dt.dt = time.delta_secs().min(1.0 / 20.0);
}

fn move_body(
    sim_dt: Res<SimDt>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut body: Single<&mut BodyKinematics, With<TutorialBody>>,
) {
    let mut direction = Vec2::ZERO;

    if keyboard.any_pressed([KeyA, ArrowLeft]) {
        direction.x -= 180.0;
    }
    if keyboard.any_pressed([KeyD, ArrowRight]) {
        direction.x += 180.0;
    }
    if keyboard.any_pressed([KeyW, ArrowUp]) {
        direction.y += 180.0;
    }
    if keyboard.any_pressed([KeyS, ArrowDown]) {
        direction.y -= 180.0;
    }

    let direction = direction.normalize_or_zero();

    //body.vel;
    //body.pos += vel * sim_dt.dt;
    body.pos += direction * BODY_SPEED * sim_dt.dt;
}

fn observe_body(
    frame: Res<PortalWorldFrame>,
    mut body: Single<(&BodyKinematics, &mut PortalBodyView, &mut Transform), With<TutorialBody>>,
) {
    let (kin, mut view, mut transform) = body.into_inner();
    *view = PortalBodyView {
        pos: kin.pos,
        size: kin.size,
        facing: kin.facing,
    };
    transform.translation = frame.to_render(kin.pos, 20.0);
}

fn spawn_rect(commands: &mut Commands, center: Vec2, size: Vec2, color: Color, z: f32) {
    commands.spawn((
        Sprite::from_color(color, size),
        Transform::from_translation(world_to_render(center, z)),
    ));
}

fn world_to_render(pos: Vec2, z: f32) -> Vec3 {
    ambition_platformer2d_core::config::world_size_to_bevy(WORLD_SIZE, pos, z)
}
