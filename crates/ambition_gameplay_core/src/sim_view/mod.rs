//! The observation-boundary staging ground (E4): small sim-resolved view
//! resources presentation consumes INSTEAD of querying live sim components.
//!
//! Every resource here is a plain-data snapshot rebuilt once per tick in the
//! sim tail (`SandboxSet::FeatureViewSync`) by a function of sim state — no
//! caching across ticks, no `Entity`/`Handle` borrows — so any observer
//! (render, RL, netcode confirmation, the fighter brain) can read the same
//! facts. This module (with `view_index`/`anim_helpers`/`pose_view`/
//! `camera_snapshot`) is the seed of the `ambition_sim_view` crate; it moves
//! wholesale at the E4 mint.

use bevy::prelude::*;

use crate::abilities::traversal::possession::ControlledSubject;
use crate::actor::{BodyKinematics, BodyMana, PlayerEntity, PrimaryPlayer};
use ambition_characters::actor::{BodyHealth, BodyWallet};
use ambition_characters::brain::ActorControl;
use ambition_engine_core as ae;

/// The controlled body's HUD meters, resolved sim-side (E4 slices 5+6+16):
/// health / mana / wallet follow the [`ControlledSubject`] — while
/// possessing, the HUD shows THAT body's meters, never the vacated home
/// avatar's. `present == false` means no controlled body resolved this tick
/// (startup frames) and the HUD holds its last drawn state.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct PlayerHudFacts {
    pub present: bool,
    pub hp_current: i32,
    pub hp_max: i32,
    pub mana_current: f32,
    pub mana_fraction: f32,
    pub balance: i32,
}

pub fn rebuild_player_hud_facts(
    mut facts: ResMut<PlayerHudFacts>,
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<(&BodyHealth, &BodyMana, Option<&BodyWallet>)>,
    primary: Query<Entity, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    let subject = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .or_else(|| primary.single().ok());
    let Some((health, mana, wallet)) = subject.and_then(|e| bodies.get(e).ok()) else {
        facts.present = false;
        return;
    };
    *facts = PlayerHudFacts {
        present: true,
        hp_current: health.current(),
        hp_max: health.max(),
        mana_current: mana.meter.current,
        mana_fraction: mana.meter.fraction(),
        balance: wallet.map(|wallet| wallet.balance).unwrap_or(0),
    };
}

/// The controlled body's held item, resolved sim-side: the geometry facts
/// the hand-sprite needs plus the item identity and its brain-resolved aim
/// (so a possessed body's ranged item points where THAT body aims).
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldItemView(pub Option<HeldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HeldItemFact {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub item_id: String,
    pub ranged: bool,
    pub aim: ae::Vec2,
}

pub fn rebuild_held_item_view(
    mut view: ResMut<HeldItemView>,
    controlled: Option<Res<ControlledSubject>>,
    bodies: Query<(&BodyKinematics, &crate::features::HeldItem, &ActorControl)>,
) {
    view.0 = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .and_then(|e| bodies.get(e).ok())
        .map(|(kin, held, control)| HeldItemFact {
            pos: kin.pos,
            size: kin.size,
            facing: kin.facing,
            item_id: held.spec.id.clone(),
            ranged: held.spec.ranged.is_some(),
            aim: control.0.aim,
        });
}

/// Every ground item's visual facts (position, box, item id).
#[derive(Resource, Default, Clone, Debug)]
pub struct GroundItemsView(pub Vec<GroundItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct GroundItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    pub item_id: String,
}

pub fn rebuild_ground_items_view(
    mut view: ResMut<GroundItemsView>,
    grounds: Query<&crate::items::pickup::GroundItem>,
) {
    view.0.clear();
    view.0.extend(grounds.iter().map(|ground| GroundItemFact {
        pos: ground.pos,
        half_extent: ground.half_extent,
        item_id: ground.spec.id.clone(),
    }));
}

/// Every in-flight held shot (gun-sword laser / fireball).
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldShotsView(pub Vec<HeldShotFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldShotFact {
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    /// Radial fireball (draws the glowing sphere) vs a velocity-aligned
    /// spinning blade.
    pub fireball: bool,
}

pub fn rebuild_held_shots_view(
    mut view: ResMut<HeldShotsView>,
    projectiles: Query<(&BodyKinematics, &crate::items::pickup::HeldProjectile)>,
) {
    view.0.clear();
    view.0
        .extend(projectiles.iter().map(|(kin, proj)| HeldShotFact {
            pos: kin.pos,
            vel: kin.vel,
            fireball: proj.explode_half > 0.0,
        }));
}

/// Every player's dropped recall-mark position.
#[derive(Resource, Default, Clone, Debug)]
pub struct MarkBeaconsView(pub Vec<ae::Vec2>);

pub fn rebuild_mark_beacons_view(
    mut view: ResMut<MarkBeaconsView>,
    marks: Query<&crate::abilities::traversal::mark_recall::PlayerMark>,
) {
    view.0.clear();
    view.0.extend(marks.iter().filter_map(|mark| mark.pos));
}

/// Every gravity-flip switch's geometry.
#[derive(Resource, Default, Clone, Debug)]
pub struct GravitySwitchesView(pub Vec<GravitySwitchFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GravitySwitchFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
}

pub fn rebuild_gravity_switches_view(
    mut view: ResMut<GravitySwitchesView>,
    switches: Query<&crate::gravity::GravityFlipSwitch>,
) {
    view.0.clear();
    view.0.extend(switches.iter().map(|sw| GravitySwitchFact {
        pos: sw.pos,
        half_extent: sw.half_extent,
    }));
}

/// Every heal shrine's geometry.
#[derive(Resource, Default, Clone, Debug)]
pub struct ShrinesView(pub Vec<ShrineFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShrineFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
}

pub fn rebuild_shrines_view(
    mut view: ResMut<ShrinesView>,
    shrines: Query<&crate::shrine::HealShrine>,
) {
    view.0.clear();
    view.0.extend(shrines.iter().map(|shrine| ShrineFact {
        pos: shrine.pos,
        half_extent: shrine.half_extent,
    }));
}

/// Tick the shrine activation pulse SIM-side (it used to be decremented by
/// the render animator — a render-owned sim write, the exact back-edge E4
/// kills). Uses scaled time, so bullet-time slows the pulse with the world.
pub fn tick_shrine_activation_pulse(
    world_time: Res<ambition_time::WorldTime>,
    mut activation: ResMut<crate::shrine::ShrineActivationPulse>,
) {
    if activation.remaining > 0.0 {
        activation.remaining = (activation.remaining - world_time.scaled_dt).max(0.0);
    }
}

/// Every wielded gun-sword's presentation facts: where the hand is, where
/// it aims, and the wielder's height (the sprite scales off it). Resolved
/// sim-side from the rider actors + the primary player's position.
#[derive(Resource, Default, Clone, Debug)]
pub struct WieldedGunSwordsView(pub Vec<GunSwordFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GunSwordFact {
    pub hand_world: ae::Vec2,
    pub aim_world: ae::Vec2,
    pub rider_height: f32,
}

#[allow(clippy::type_complexity)]
pub fn rebuild_wielded_gun_swords_view(
    mut view: ResMut<WieldedGunSwordsView>,
    rider_actors: Query<(
        &crate::features::ActorDisposition,
        &crate::features::HeldItem,
        Option<&BodyKinematics>,
        Option<&BodyHealth>,
    )>,
    player_q: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    view.0.clear();
    let Ok(player) = player_q.single() else {
        return;
    };
    for (disposition, held_item, kin, health) in &rider_actors {
        if held_item.id() != "gun_sword" || disposition.is_peaceful() {
            continue;
        }
        let (Some(kin), Some(health)) = (kin, health) else {
            continue;
        };
        if !health.alive() {
            continue;
        }
        let rider_height = kin.size.y;
        view.0.push(GunSwordFact {
            hand_world: crate::features::rider_hand_world_pos(kin.pos, kin.facing, rider_height),
            aim_world: player.pos,
            rider_height,
        });
    }
}

/// Registers the observation-boundary view resources + their rebuilds in the
/// sim tail. Owned here (anti-god rule 5): the plugin that rebuilds a view
/// initializes it; presentation only reads.
pub struct SimViewPlugin;

impl Plugin for SimViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerHudFacts>()
            .init_resource::<HeldItemView>()
            .init_resource::<GroundItemsView>()
            .init_resource::<HeldShotsView>()
            .init_resource::<MarkBeaconsView>()
            .init_resource::<GravitySwitchesView>()
            .init_resource::<ShrinesView>()
            .init_resource::<WieldedGunSwordsView>();
        app.add_systems(
            Update,
            (
                rebuild_player_hud_facts,
                rebuild_held_item_view,
                rebuild_ground_items_view,
                rebuild_held_shots_view,
                rebuild_mark_beacons_view,
                rebuild_gravity_switches_view,
                rebuild_shrines_view,
                tick_shrine_activation_pulse,
                rebuild_wielded_gun_swords_view,
            )
                .in_set(crate::schedule::SandboxSet::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_facts_track_the_controlled_body() {
        use ambition_characters::actor::Health;
        let mut app = App::new();
        app.init_resource::<PlayerHudFacts>();
        app.add_systems(Update, rebuild_player_hud_facts);

        // Home avatar with a fat purse; a driven actor with its own economy.
        app.world_mut().spawn((
            PlayerEntity,
            PrimaryPlayer,
            BodyHealth::new(Health::new(20)),
            BodyMana::default(),
            BodyWallet { balance: 42 },
        ));
        let mut actor_hp = BodyHealth::new(Health::new(10));
        actor_hp.damage(7);
        let actor = app
            .world_mut()
            .spawn((actor_hp, BodyMana::default(), BodyWallet { balance: 7 }))
            .id();
        app.world_mut()
            .insert_resource(ControlledSubject(Some(actor)));
        app.update();

        let facts = *app.world().resource::<PlayerHudFacts>();
        assert!(facts.present);
        assert_eq!(
            (facts.hp_current, facts.hp_max),
            (3, 10),
            "HUD facts must snapshot the POSSESSED body's health"
        );
        assert_eq!(facts.balance, 7, "money is a body stat");
    }

    #[test]
    fn shrine_pulse_ticks_down_sim_side() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 0.1,
            scaled_dt: 0.1,
        });
        app.insert_resource(crate::shrine::ShrineActivationPulse { remaining: 0.25 });
        app.add_systems(Update, tick_shrine_activation_pulse);
        app.update();
        let remaining = app
            .world()
            .resource::<crate::shrine::ShrineActivationPulse>()
            .remaining;
        assert!((remaining - 0.15).abs() < 1e-6);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<crate::shrine::ShrineActivationPulse>()
                .remaining,
            0.0,
            "pulse clamps at zero"
        );
    }
}
