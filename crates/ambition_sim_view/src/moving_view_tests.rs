//! Moving-body publication measurements through the production view systems.

use crate::combat_geometry_view::{rebuild_combat_geometry_view, CombatGeometryView};
use crate::view_index::{rebuild_feature_view_index, FeatureViewIndex};
use ambition_combat::components::{ActorDisposition, DamageableVolumes, FeatureId};
use ambition_platformer2d_core as ae;
use bevy::prelude::*;

#[derive(Component)]
struct Goal(ae::Vec2, f32);
#[derive(Resource, Default)]
struct Clock(u32);

fn advance(
    mut clock: ResMut<Clock>,
    mut bodies: Query<(
        &Goal,
        &mut ae::BodyKinematics,
        &mut ae::CenteredAabb,
        &mut ambition_characters::actor::BodyCombat,
        Option<&mut DamageableVolumes>,
    )>,
) {
    let t = clock.0 as f32 * 0.04;
    for (goal, mut kin, mut bounds, mut combat, volumes) in &mut bodies {
        kin.pos =
            goal.0 + ae::Vec2::new((t + goal.1).sin() * 90.0, (t * 0.7 + goal.1).cos() * 40.0);
        kin.vel = ae::Vec2::new((t + goal.1).cos() * 90.0, 0.0);
        bounds.center = kin.pos;
        combat.hit_flash = if clock.0 % 9 == 0 { 0.1 } else { 0.0 };
        if let Some(mut volumes) = volumes {
            if clock.0 % 11 == 0 {
                volumes.clear();
            } else {
                volumes.publish_body(vec![ae::CombatVolume::aabb(ae::Aabb::new(
                    ae::Vec2::ZERO,
                    ae::Vec2::new(8.0, 13.0),
                ))]);
            }
        }
    }
    clock.0 += 1;
}

#[test]
#[ignore = "measurement: moving view publication"]
fn measure_moving_view_publication() {
    println!("workload,bodies,sample,ns_per_update,view_hash");
    for workload in ["features", "geometry-fallback", "geometry-published"] {
        for count in [32, 128, 512, 1024] {
            for sample in 0..3 {
                let mut app = App::new();
                app.init_resource::<Clock>();
                app.init_resource::<FeatureViewIndex>();
                app.init_resource::<CombatGeometryView>();
                app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
                if workload == "features" {
                    app.add_systems(Update, (advance, rebuild_feature_view_index).chain());
                } else {
                    app.add_systems(Update, (advance, rebuild_combat_geometry_view).chain());
                }
                for index in 0..count {
                    let at = ae::Vec2::new((index % 32) as f32 * 80.0, (index / 32) as f32 * 90.0);
                    let mut body = app.world_mut().spawn((
                        Goal(at, index as f32 * 0.37),
                        FeatureId(format!("moving-{index:05}")),
                        ae::CenteredAabb::from_center_size(at, ae::Vec2::new(20.0, 28.0)),
                        ActorDisposition::Hostile,
                        ae::BodyKinematics {
                            pos: at,
                            canonical_size: ae::Vec2::new(20.0, 28.0),
                            ..Default::default()
                        },
                        ambition_characters::actor::BodyCombat::default(),
                        ambition_characters::actor::BodyHealth::new(
                            ambition_characters::actor::Health::new(10),
                        ),
                    ));
                    if workload == "geometry-published" {
                        body.insert(DamageableVolumes::body_box(ae::Vec2::new(8.0, 13.0)));
                    }
                }
                for _ in 0..10 {
                    app.update();
                }
                let start = std::time::Instant::now();
                for _ in 0..40 {
                    app.update();
                }
                let elapsed = start.elapsed().as_nanos() / 40;
                let text = if workload == "features" {
                    let mut rows: Vec<_> = app
                        .world()
                        .resource::<FeatureViewIndex>()
                        .iter()
                        .map(|(id, view)| (id.to_owned(), format!("{view:?}")))
                        .collect();
                    rows.sort_by(|a, b| a.0.cmp(&b.0));
                    format!("{rows:?}")
                } else {
                    format!("{:?}", app.world().resource::<CombatGeometryView>())
                };
                let hash = text
                    .bytes()
                    .fold(0_u64, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
                println!("{workload},{count},{sample},{elapsed},{hash}");
            }
        }
    }
}
