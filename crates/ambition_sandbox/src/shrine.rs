//! Healing / save-point shrine.
//!
//! An interactable shrine that, on a single `Interact`, **heals the player to
//! full** (health + mana) and acts as a **save point** (decided: one Interact
//! does both). The save is a checkpoint write: touching `Res<SandboxSave>` marks
//! it changed, and the existing `autosave_sandbox_save` persists it (desktop;
//! no-op on wasm).
//!
//! Handoff / not-yet-built:
//! - placement is a single debug-spawned shrine; authored placement via an
//!   `Interactable` (so it routes through the affordance/prompt system) is the
//!   follow-up (see TODO "Healing / save-point shrine").
//! - a real shrine sprite (section B); a tinted pillar stands in for now.

use bevy::prelude::*;

use crate::engine_core::{self as ae, AabbExt};
use crate::input::ControlFrame;
use crate::player::{PlayerEntity, PlayerHealth, PlayerKinematics, PlayerMana, PrimaryPlayer};

/// A healing / save-point shrine the player can `Interact` with.
#[derive(Component, Clone, Copy, Debug)]
pub struct HealShrine {
    pub pos: Vec2,
    pub half_extent: Vec2,
}

/// Spawn one shrine near the player the first frame a player exists (debug
/// convenience until authored placement lands).
pub fn spawn_debug_shrine_once(
    mut commands: Commands,
    mut done: Local<bool>,
    players: Query<&PlayerKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    if *done {
        return;
    }
    let Ok(kin) = players.single() else {
        return;
    };
    *done = true;
    commands.spawn((
        HealShrine {
            pos: kin.pos + Vec2::new(-160.0, 0.0),
            half_extent: Vec2::new(22.0, 40.0),
        },
        Name::new("Heal/save shrine"),
    ));
}

/// `Interact` while overlapping a [`HealShrine`] heals the player to full
/// (health + mana) and writes a save checkpoint. `interact_pressed` is an edge,
/// so one press = one heal.
pub fn heal_save_shrine_system(
    control: Res<ControlFrame>,
    mut players: Query<
        (&PlayerKinematics, &mut PlayerHealth, &mut PlayerMana),
        (With<PlayerEntity>, With<PrimaryPlayer>),
    >,
    shrines: Query<&HealShrine>,
    mut save: ResMut<crate::persistence::save::SandboxSave>,
    mut sfx: MessageWriter<crate::audio::SfxMessage>,
) {
    if !control.interact_pressed {
        return;
    }
    let Ok((kin, mut health, mut mana)) = players.single_mut() else {
        return;
    };
    let player_aabb = ae::Aabb::new(kin.pos, kin.size * 0.5);
    let touching = shrines
        .iter()
        .any(|s| player_aabb.strict_intersects(ae::Aabb::new(s.pos, s.half_extent)));
    if !touching {
        return;
    }
    health.reset(); // health to full
    mana.meter.refill_full(); // mana to full
    // Save checkpoint: mark the live save changed so `autosave_sandbox_save`
    // persists the current state to disk.
    save.set_changed();
    sfx.write(crate::audio::SfxMessage::Play {
        id: ambition_sfx::ids::WORLD_HEALTH_COLLECT,
        pos: kin.pos,
    });
    bevy::log::info!(target: "ambition::shrine", "shrine: healed to full + saved");
}

// ---------------------------------------------------------------------------
// Presentation (visible build only).

/// Marks the shrine's visual.
#[derive(Component)]
pub struct ShrineVisual;

/// Draw each shrine as a glowing pillar so the player can find it.
pub fn sync_shrine_visual(
    mut commands: Commands,
    world: Res<crate::GameWorld>,
    visuals: Query<Entity, With<ShrineVisual>>,
    shrines: Query<&HealShrine>,
) {
    for entity in &visuals {
        commands.entity(entity).despawn();
    }
    for shrine in &shrines {
        let translation = crate::config::world_to_bevy(&world.0, shrine.pos, 8.0);
        commands.spawn((
            ShrineVisual,
            Sprite::from_color(Color::srgba(0.55, 0.95, 0.85, 0.85), shrine.half_extent * 2.0),
            Transform::from_translation(translation),
            Name::new("Shrine visual"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interacting_at_the_shrine_heals_to_full() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<crate::persistence::save::SandboxSave>();
        app.add_systems(Update, heal_save_shrine_system);

        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: Vec2::new(100.0, 100.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PlayerHealth::new(crate::actor::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
                PlayerMana::default(),
            ))
            .id();
        // Drain mana so we can see it refill.
        app.world_mut()
            .get_mut::<PlayerMana>(player)
            .unwrap()
            .meter
            .try_spend(40.0);
        app.world_mut().spawn(HealShrine {
            pos: Vec2::new(100.0, 100.0),
            half_extent: Vec2::new(22.0, 40.0),
        });

        // Interact while overlapping → heal to full.
        app.world_mut().resource_mut::<ControlFrame>().interact_pressed = true;
        app.update();

        let health = *app.world().get::<PlayerHealth>(player).unwrap();
        assert_eq!(health.current(), health.max(), "health should be full");
        let mana = app.world().get::<PlayerMana>(player).unwrap().meter;
        assert!(mana.is_full(), "mana should be refilled, got {}", mana.current);
    }

    #[test]
    fn no_heal_without_interact_or_when_not_touching() {
        let mut app = App::new();
        app.add_message::<crate::audio::SfxMessage>();
        app.insert_resource(ControlFrame::default());
        app.init_resource::<crate::persistence::save::SandboxSave>();
        app.add_systems(Update, heal_save_shrine_system);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                PlayerKinematics {
                    pos: Vec2::new(100.0, 100.0),
                    vel: Vec2::ZERO,
                    size: Vec2::new(24.0, 40.0),
                    base_size: Vec2::new(24.0, 40.0),
                    facing: 1.0,
                },
                PlayerHealth::new(crate::actor::Health {
                    current: 1,
                    max: 5,
                    invulnerable: false,
                }),
                PlayerMana::default(),
            ))
            .id();
        // A shrine far away.
        app.world_mut().spawn(HealShrine {
            pos: Vec2::new(900.0, 900.0),
            half_extent: Vec2::new(22.0, 40.0),
        });

        // Interact pressed but not touching → no heal.
        app.world_mut().resource_mut::<ControlFrame>().interact_pressed = true;
        app.update();
        assert_eq!(
            app.world().get::<PlayerHealth>(player).unwrap().current(),
            1,
            "no heal when not at the shrine"
        );
    }
}
