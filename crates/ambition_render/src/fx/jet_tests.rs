//! A jet throws every particle inside its cone, and a beam shoots from its
//! base along its direction.

use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::insert_live_room_component;

const AT: ae::Vec2 = ae::Vec2::new(300.0, 200.0);

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_message::<VfxInRoom>();
    app.add_systems(Update, (vfx_spawn_messages, update_particles, update_beams).chain());
    insert_live_room_component(
        app.world_mut(),
        ae::RoomGeometry(ae::World::new("jet room", ae::Vec2::new(800.0, 600.0), ae::Vec2::new(40.0, 40.0), Vec::new())),
    );
    app
}

#[test]
fn a_jet_throws_every_particle_inside_its_cone() {
    let mut app = app();
    let toward = ae::Vec2::new(-0.6, -0.8);
    let spread = 0.3;
    app.world_mut().write_message(VfxInRoom {
        room: None,
        vfx: VfxMessage::Jet {
            pos: AT,
            toward,
            spread,
            count: 24,
            speed: 1000.0,
            color: [1.0; 4],
            kind: ParticleKind::Streak,
        },
    });
    app.update();
    let mut particles = app.world_mut().query::<&ParticleVisual>();
    let angles: Vec<f32> = particles
        .iter(app.world())
        .map(|p| p.vel.normalize().dot(toward).clamp(-1.0, 1.0).acos())
        .collect();
    assert_eq!(angles.len(), 24, "one particle for each of the count");
    let widest = angles.iter().copied().fold(0.0, f32::max);
    // The wobble moves a particle by at most 0.3 * spread / count.
    assert!(widest <= spread * (1.0 + 0.3 / 24.0) + 1e-4, "a particle left at {widest} rad from the jet's axis");
    assert!(widest > spread * 0.9, "the jet fills its cone, not only its axis ({widest} rad)");
}

#[test]
fn a_beam_shoots_from_its_base_along_its_direction() {
    let mut app = app();
    app.world_mut().write_message(VfxInRoom {
        room: None,
        vfx: VfxMessage::Beam {
            pos: AT,
            toward: ae::Vec2::new(1.0, 0.0),
            length: 400.0,
            width: 60.0,
            color: [1.0; 4],
            seconds: 1.0,
        },
    });
    app.update();
    app.world_mut().resource_mut::<Time>().advance_by(std::time::Duration::from_millis(300));
    app.update();
    let mut beams = app.world_mut().query::<(&BeamVisual, &Sprite)>();
    let (beam, sprite) = beams.single(app.world()).expect("one beam");
    let size = sprite.custom_size.expect("a drawn size");
    let (length, width) = beam_shape(beam.length, beam.width, beam.age / beam.lifetime);
    assert!(beam.age > 0.0, "the beam's clock ran");
    assert_eq!(size, BVec2::new(length, width));
    assert!(length > 0.9 * 400.0, "it has shot out by 30% of its life ({length})");
}
