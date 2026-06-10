//! Player → static-chest open path on the ECS feature side.

use super::*;

/// Open ECS-owned static chests from the same interaction buffer used by doors
/// and legacy NPCs/switches.
pub fn open_ecs_chests(
    mut commands: Commands,
    mut banner: ResMut<GameplayBanner>,
    mut player: Query<
        (
            &crate::player::BodyKinematics,
            &mut crate::player::PlayerInteractionState,
            &mut crate::player::PlayerAnimState,
        ),
        With<crate::player::PlayerEntity>,
    >,
    chests: Query<
        (
            Entity,
            &FeatureId,
            &FeatureName,
            &FeatureAabb,
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
    for (player_kin, mut interaction, mut anim) in &mut player {
        if !interaction.buffered() {
            continue;
        }
        let player_aabb = player_kin.aabb();
        for (entity, id, name, aabb, opened, falling) in &chests {
            if falling.is_some() || opened.is_some() || !aabb.aabb().strict_intersects(player_aabb)
            {
                continue;
            }
            commands.entity(entity).insert(Opened);
            interaction.clear();
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
    use crate::player::{
        BodyKinematics, PlayerAnimState, PlayerBaseSize, PlayerEntity, PlayerInteractionState,
    };
    use bevy::prelude::{App, Entity, Update};

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(GameplayBanner::default());
        app.add_message::<SetFlagRequested>();
        app.add_message::<SfxMessage>();
        app.add_message::<VfxMessage>();
        app.add_systems(Update, open_ecs_chests);
        app
    }

    fn player(app: &mut App, pos: ae::Vec2, buffered: bool) -> Entity {
        app.world_mut()
            .spawn((
                PlayerEntity,
                BodyKinematics {
                    pos,
                    size: ae::Vec2::new(28.0, 46.0),
                    facing: 1.0,
                    ..Default::default()
                },
                PlayerBaseSize {
                    base_size: ae::Vec2::new(28.0, 46.0),
                },
                PlayerInteractionState {
                    interact_buffer_timer: if buffered { 0.5 } else { 0.0 },
                    ..Default::default()
                },
                PlayerAnimState::default(),
            ))
            .id()
    }

    fn chest(app: &mut App, id: &str, pos: ae::Vec2) -> Entity {
        app.world_mut()
            .spawn((
                FeatureSimEntity,
                FeatureId::new(id),
                FeatureName::new("Chest"),
                FeatureAabb::from_center_size(pos, ae::Vec2::new(24.0, 24.0)),
                ChestFeature::new(crate::interaction::Chest::new(id, None)),
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
