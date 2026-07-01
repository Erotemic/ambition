//! Player → static-chest open path on the ECS feature side.

use super::*;

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    controlled: Option<Res<crate::abilities::traversal::possession::ControlledSubject>>,
    // The local controller's buffered interact (published from the device onto its
    // slot); consumed for whatever body it currently drives.
    mut slot_gestures: ResMut<crate::player::SlotInteractionState>,
    // Interact-gesture pose + startup-frame fallback subject.
    mut input_surface: Query<
        (Entity, &mut crate::player::PlayerAnimState),
        (
            With<crate::actor::PlayerEntity>,
            With<crate::actor::PrimaryPlayer>,
        ),
    >,
    // The driven body's kinematics — reach is measured from the controlled subject.
    bodies: Query<&crate::actor::BodyKinematics>,
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &CenteredAabb,
            Option<&Opened>,
            Option<&FallingChest>,
        ),
        (With<FeatureSimEntity>, With<ChestFeature>),
    >,
    mut set_flag: MessageWriter<SetFlagRequested>,
    mut sfx: MessageWriter<SfxMessage>,
    mut vfx: MessageWriter<VfxMessage>,
) {
    // Iterate every player so each player's own buffered interact
    // can open a chest the player is overlapping. Per-player interact
    // state is independent (each player has their own
    // `PlayerInteractionState`); the chest is shared (a future co-op
    // build can still gate "first-come gets the open" by inserting
    // the `Opened` marker, which keeps subsequent attempts no-ops).
    // OVERNIGHT-TODO #17.6/#17.8 — preserve single-player behavior
    // because the iterator has one entity today.
    // Same hold time as the NPC / switch interact gesture. Kept in
    // sync with `interact_ecs_actors_and_switches::INTERACT_ANIM_HOLD_SECS`
    // so the player's reach-and-open animation feels uniform across
    // every interactable kind.
    const INTERACT_ANIM_HOLD_SECS: f32 = 0.28;
    let Ok((primary_entity, mut anim)) = input_surface.single_mut() else {
        return;
    };
    if !slot_gestures.primary().buffered() {
        return;
    }
    // Reach is measured from the controlled subject — a possessed actor opens the
    // chest IT is standing on, not one the vacated home avatar is next to.
    let subject = controlled
        .and_then(|subject| subject.0)
        .unwrap_or(primary_entity);
    let Ok(subject_kin) = bodies.get(subject) else {
        return;
    };
    let reach_aabb = subject_kin.aabb();
    {
        for (entity, id, name, aabb, opened, falling) in &chests {
            if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(reach_aabb) {
                continue;
            }
            commands.entity(entity).insert(Opened);
            slot_gestures.primary_mut().clear();
            anim.interact_anim_timer = INTERACT_ANIM_HOLD_SECS;
            banner.show(format!("opened {}", name.0.as_str()), 2.6);
            let pos = aabb.center;
            vfx.write(VfxMessage::Burst {
                pos,
                count: 16,
                speed: 230.0,
                color: [0.84, 0.95, 1.0, 0.82],
                kind: ParticleKind::Spark,
            });
            sfx.write(SfxMessage::Play {
                id: ambition_sfx::ids::WORLD_TREASURE_CHEST_OPEN,
                pos,
            });
            if let Some(encounter_id) = id.as_str().strip_prefix("encounter_chest_") {
                set_flag.write(SetFlagRequested {
                    id: format!("encounter_{encounter_id}_reward_dropped"),
                    on: true,
                });
            }
            break;
        }
    }
}

#[cfg(test)]
mod chest_tests {
    //! The player -> static-chest open path as a minimal-App harness:
    //! a buffered interact over an overlapping, unopened chest inserts
    //! `Opened`; an unbuffered player or a non-overlapping chest does not.
    use super::*;
    use crate::abilities::traversal::possession::ControlledSubject;
    use crate::actor::BodyBaseSize;
    use crate::actor::BodyKinematics;
    use crate::actor::{PlayerEntity, PrimaryPlayer};
    use crate::player::{PlayerAnimState, SlotInteractionState};
    use bevy::prelude::{App, Entity, Update};

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.init_resource::<SlotInteractionState>();
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_systems(Update, open_ecs_chests);
        app
    }

    fn player(app: &mut App, pos: ae::Vec2, buffered: bool) -> Entity {
        // The buffered interact is SLOT state now, not a per-body component.
        if buffered {
            app.world_mut()
                .resource_mut::<SlotInteractionState>()
                .primary_mut()
                .interact_buffer_timer = 0.5;
        }
        let entity = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
                },
                BodyBaseSize {
                    base_size: ae::Vec2::new(28.0, 46.0),
                },
                PlayerAnimState::default(),
            ))
            .id();
        app.world_mut()
            .insert_resource(ControlledSubject(Some(entity)));
        entity
    }

    fn chest(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new(id),
                FeatureName::new("Chest"),
                CenteredAabb::from_center_size(pos, ae::Vec2::new(24.0, 24.0)),
                ChestFeature::new(ambition_interaction::Chest::new(id, None)),
            ))
            .id()
    }

    #[test]
    fn buffered_interact_opens_an_overlapping_chest() {
        let mut app = app();
        let center = ae::Vec2::new(64.0, 64.0);
        player(&mut app, center, true);
        let c = chest(&mut app, "c1", center);
        app.update();
        assert!(
            app.world().get::<Opened>(c).is_some(),
            "buffered interact over the chest opens it"
        );
    }

    #[test]
    fn unbuffered_player_leaves_chest_closed() {
        let mut app = app();
        let center = ae::Vec2::new(64.0, 64.0);
        player(&mut app, center, false);
        let c = chest(&mut app, "c1", center);
        app.update();
        assert!(
            app.world().get::<Opened>(c).is_none(),
            "no buffered interact -> chest stays closed"
        );
    }

    #[test]
    fn distant_chest_is_not_opened() {
        let mut app = app();
        player(&mut app, ae::Vec2::new(64.0, 64.0), true);
        let c = chest(&mut app, "c1", ae::Vec2::new(2000.0, 2000.0));
        app.update();
        assert!(
            app.world().get::<Opened>(c).is_none(),
            "a non-overlapping chest stays closed even with a buffered interact"
        );
    }
}
