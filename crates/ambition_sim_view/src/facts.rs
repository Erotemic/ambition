//! The observation-boundary staging ground (E4): small sim-resolved view
//! resources presentation consumes INSTEAD of querying live sim components.
//!
//! Every resource here is a plain-data snapshot rebuilt once per tick in the
//! sim tail (`Platformer2dSimulationPhaseMonolith::FeatureViewSync`) by a function of sim state — no
//! caching across ticks, no `Entity`/`Handle` borrows — so any observer
//! (render, RL, netcode confirmation, the fighter brain) can read the same
//! facts. This module (with `view_index`/`anim_helpers`/`pose_view`/
//! `camera_snapshot`) is the seed of the `ambition_sim_view` crate; it moves
//! wholesale at the E4 mint.

use bevy::prelude::*;

use ambition_characters::actor::{BodyHealth, BodyWallet};
use ambition_characters::control::ActorControl;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::{BodyKinematics, BodyMana};
use ambition_platformer2d_shared_tangle::markers::ControlledSubject;
use ambition_platformer2d_shared_tangle::markers::{PlayerEntity, PrimaryPlayer};
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

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

/// EVERY body's held item, resolved sim-side: the geometry facts the hand-sprite
/// needs plus the item identity and its brain-resolved aim (so a possessed
/// body's ranged item points where THAT body aims).
///
/// ⛔ Not every holder — every holder [`drawn_in_the_hand`] claims. The
/// over-hand road takes the rest; see that function for why the line is there.
#[derive(Resource, Default, Clone, Debug)]
pub struct HeldItemView(pub Vec<HeldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HeldItemFact {
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub facing: f32,
    pub item_id: String,
    pub ranged: bool,
    pub aim: ae::Vec2,
}

/// WHICH OF THE TWO HELD-ITEM ROADS A HOLDER IS ON, stated once.
///
/// ⛔⛔ THE TWO ROADS MUST PARTITION THE HOLDERS — every holder on exactly one.
/// [`HeldItemView`] draws the item IN the hand from `HeldItemArtManifest`;
/// [`HostileWieldedItemsView`] draws it OVER the hand from
/// `WieldedItemVisualCatalog`, scaled to the wielder and gripped at an authored
/// point. Both registries carry `gun_sword`, so a holder on both roads is drawn
/// TWICE, and a holder on neither is drawn not at all — which is the defect this
/// function exists to make impossible to reintroduce in one road alone.
///
/// The line is the over-hand road's own admission test, hoisted out of it: a
/// living body with a hostile disposition. A match fighter carries no
/// `ActorDisposition` at all and a peaceful NPC's is `Peaceful`, so both fall to
/// the in-hand road, which is where their art is registered.
pub fn drawn_over_the_hand(
    disposition: Option<ambition_combat::components::ActorDisposition>,
    alive: Option<bool>,
) -> bool {
    disposition.is_some_and(|d| !d.is_peaceful()) && alive.unwrap_or(false)
}

/// The complement of [`drawn_over_the_hand`], for the road that reads it.
pub fn drawn_in_the_hand(
    disposition: Option<ambition_combat::components::ActorDisposition>,
    alive: Option<bool>,
) -> bool {
    !drawn_over_the_hand(disposition, alive)
}

#[allow(clippy::type_complexity)]
pub fn rebuild_held_item_view(
    mut view: ResMut<HeldItemView>,
    bodies: Query<(
        &BodyKinematics,
        &ambition_platformer2d_actor_monolith::features::HeldItem,
        &ActorControl,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        Option<&ambition_combat::components::ActorDisposition>,
        Option<&BodyHealth>,
    )>,
) {
    // ⛔⛔ THIS READ ONLY `ControlledSubject`, so exactly ONE held item was ever
    // drawn — in a couch match seat one's drawn weapon was invisible, and in a
    // CPU-versus-CPU match neither Admiral's gun-sword showed at all. The
    // argument against that is written twelve lines below for a sibling view:
    // *"it is plural: a couch-versus match has two driven bodies and neither is
    // more protected than the other, because a rule that privileges one
    // participant stops being a rule about bodies."* Same rule, same reason.
    //
    // ⛔⛔ AND `With<DrivingParticipant>` WAS STILL THE WRONG POPULATION, which
    // the plural fix did not go far enough to see: a weapon is visible because a
    // BODY holds it, not because a human drives that body. The Admiral's side-B
    // draws `admiral_gun_sword`, which is registered in the IN-HAND art manifest
    // and nowhere else, so a CPU Admiral's gun-sword was drawn by no road at all
    // — and a match fighter carries no `ActorDisposition`, so the over-hand road
    // could not pick it up either.
    //
    // ⇒ every holder now, minus the ones the over-hand road claims. See
    // [`drawn_over_the_hand`] for why that subtraction is a partition and not a
    // carve-out.
    //
    // ⛔ SORTED BY `SimId`, because this is a Vec built in query order and the
    // consumer spawns one visual per row: unsorted, two held items would swap
    // draw order between runs.
    let mut rows: Vec<_> = bodies
        .iter()
        .filter(|(_, _, _, _, disposition, health)| {
            drawn_in_the_hand(disposition.copied(), health.map(|health| health.alive()))
        })
        .map(|(kin, held, control, id, _, _)| {
            (
                id.clone(),
                HeldItemFact {
                    pos: kin.pos,
                    size: kin.size,
                    facing: kin.facing,
                    item_id: held.spec.id.clone(),
                    ranged: held.spec.ranged.is_some(),
                    aim: control.0.aim.vec(),
                },
            )
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    view.0.clear();
    view.0.extend(rows.into_iter().map(|(_, fact)| fact));
}

/// The box of every body a participant is DRIVING this tick.
///
/// Presentation needs this to know what it must not obscure — the world-label
/// placement pass dims a label that would be drawn across a driven body rather
/// than shoving the label aside (`ambition_render::rendering::label_layout`).
///
/// Derived from WHO DRIVES the body, not from a player marker, for two reasons.
/// First, possession is a SEAT REDIRECT, so a possessed enemy carries the seat
/// and the vacated home avatar does not — asking who holds the seat gets that
/// right for free. Second, it is plural: a couch-versus match has two driven
/// bodies and neither is more protected than the other, because a rule that
/// privileges one participant stops being a rule about bodies.
///
/// Note what this is NOT: it is not the nameplate index's `controlled` flag.
/// That flag lives on rows keyed by `FeatureId`, and the home avatar carries
/// no `FeatureId` at all — so the flag is only ever true while possessing a
/// feature actor. A label-occlusion rule built on it would have protected
/// every body EXCEPT the one you normally play.
#[derive(Resource, Default, Clone, Debug)]
pub struct ControlledBodiesView(pub Vec<ControlledBodyFact>);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlledBodyFact {
    pub center: ae::Vec2,
    pub size: ae::Vec2,
}

pub fn rebuild_controlled_bodies_view(
    mut view: ResMut<ControlledBodiesView>,
    bodies: Query<&BodyKinematics, With<ambition_characters::control::DrivingParticipant>>,
) {
    // AMBITION_REVIEW(determinism): query order is not stable, and this Vec is
    // built in it. Safe: the only consumer asks "does any of these boxes
    // overlap mine", which is order-independent, and this is derived
    // presentation state that never enters a sim trajectory.
    view.0.clear();
    view.0.extend(bodies.iter().map(|kin| ControlledBodyFact {
        center: kin.pos,
        size: kin.size,
    }));
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

/// only items that are IN THE WORLD. A picked-up item is no longer
/// destroyed — it keeps its entity and its identity and records that a body is
/// carrying it (`ItemCustody`) — so "there is a `GroundItem` component" stopped
/// meaning "there is an axe lying over there". The in-hand overlay is a separate
/// view (`HeldItemView`) drawn from the holder, and publishing a carried item
/// here would draw it twice: once in the hand and once on the floor where it was
/// grabbed.
pub fn rebuild_ground_items_view(
    mut view: ResMut<GroundItemsView>,
    grounds: Query<(
        &ambition_platformer2d_actor_monolith::items::pickup::GroundItem,
        &ambition_platformer2d_actor_monolith::items::pickup::ItemCustody,
    )>,
) {
    view.0.clear();
    view.0.extend(
        grounds
            .iter()
            .filter(|(_, custody)| custody.in_world())
            .map(|(ground, _)| GroundItemFact {
                pos: ground.pos,
                half_extent: ground.half_extent,
                item_id: ground.spec.id.clone(),
            }),
    );
}

/// Every walk-into world item's visual facts (position, box, the row it grants —
/// so the renderer can pick an icon/tint per pickup).
#[derive(Resource, Default, Clone, Debug)]
pub struct WorldItemsView(pub Vec<WorldItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct WorldItemFact {
    pub pos: ae::Vec2,
    pub half_extent: ae::Vec2,
    /// The equipment row id the item grants (e.g. `"grow_cap"`), used only to
    /// choose the visual. An empty string if the payload has no id.
    pub row_id: String,
    /// Optional presentation art id (e.g. `"super_mary_o_milk_carton"`) the render
    /// layer resolves to a real sprite; `None` draws the row-tinted quad.
    pub sprite: Option<String>,
    /// Still emerging from whatever produced it — draw it BEHIND the world.
    ///
    /// DERIVED from the motion, never mirrored from the item. `WorldItem` carried an
    /// `emerging: bool` that Mary-O set `true` at spawn and NOTHING ever set back to `false`,
    /// so a wand finished rising, began its ordinary arc, and stayed drawn behind the world for
    /// the rest of its life.
    ///
    /// the motion already knew — `ItemMotion::emerging()` compares elapsed rise
    /// against the authored one. A second mutable copy of a fact the simulation
    /// derives per frame can only ever go stale; this asks the one that cannot.
    pub emerging: bool,
}

pub fn rebuild_world_items_view(
    mut view: ResMut<WorldItemsView>,
    items: Query<(
        &ambition_platformer2d_actor_monolith::items::world_item::WorldItem,
        Option<&ambition_platformer2d_actor_monolith::items::item_motion::ItemMotion>,
    )>,
) {
    use ambition_platformer2d_actor_monolith::items::world_item::WorldItemPayload;
    view.0.clear();
    view.0
        .extend(items.iter().map(|(item, motion)| WorldItemFact {
            pos: item.pos,
            half_extent: item.half_extent,
            row_id: match &item.payload {
                WorldItemPayload::Equip(row) => row.id.clone(),
            },
            sprite: item.sprite.clone(),
            // An item with no motion is not rising: a dropped or authored item sits
            // where it is, and belongs in front of the world like any other pickup.
            emerging: motion.is_some_and(|motion| motion.emerging()),
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
    projectiles: Query<(
        &BodyKinematics,
        &ambition_platformer2d_actor_monolith::items::pickup::HeldProjectile,
    )>,
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
    marks: Query<
        &ambition_platformer2d_actor_monolith::abilities::traversal::mark_recall::PlayerMark,
    >,
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
    switches: Query<&ambition_platformer2d_actor_monolith::gravity::GravityFlipSwitch>,
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
    shrines: Query<&ambition_platformer2d_actor_monolith::shrine::HealShrine>,
) {
    view.0.clear();
    view.0.extend(shrines.iter().map(|shrine| ShrineFact {
        pos: shrine.pos,
        half_extent: shrine.half_extent,
    }));
}

pub fn tick_shrine_activation_pulse(
    world_time: Res<ambition_time::WorldTime>,
    mut activation: ResMut<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>,
) {
    if activation.remaining > 0.0 {
        activation.remaining = (activation.remaining - world_time.scaled_dt).max(0.0);
    }
}

/// Presentation facts for every living hostile actor wielding an item: the
/// authored item id, hand position, aim target, and wielder height. The sim
/// publishes the open identity; presentation catalogs decide which ids have a
/// visible over-hand prop and how that prop is drawn.
#[derive(Resource, Default, Clone, Debug)]
pub struct HostileWieldedItemsView(pub Vec<HostileWieldedItemFact>);

#[derive(Clone, Debug, PartialEq)]
pub struct HostileWieldedItemFact {
    pub item_id: String,
    pub hand_world: ae::Vec2,
    pub aim_world: ae::Vec2,
    pub wielder_height: f32,
}

/// A WIELDER AIMS AT WHAT IT IS FIGHTING, not at "the player".
///
/// This took `Query<&BodyKinematics, PrimaryPlayerOnly>`, `single()`d it, and
/// `return`ed without one — so in a match, where no session home avatar exists,
/// every hostile wielder's held item vanished from the view entirely. And when
/// there WAS a player the fact was still wrong for a match: two fighters both
/// aimed their weapons at a third body neither was fighting.
///
/// The controlled subject is the fallback for a wielder with no target — an exploration enemy
/// that has not acquired one still points its pistol at the person it is menacing — and a
/// wielder with neither is simply aimed where it faces, which is a fact rather than a hole.
#[allow(clippy::type_complexity)]
pub fn rebuild_hostile_wielded_items_view(
    mut view: ResMut<HostileWieldedItemsView>,
    wielders: Query<(
        &ambition_combat::components::ActorDisposition,
        &ambition_platformer2d_actor_monolith::features::HeldItem,
        Option<&BodyKinematics>,
        Option<&BodyHealth>,
        Option<&ambition_combat::components::ActorTarget>,
    )>,
    bodies: Query<&BodyKinematics>,
    controlled: Option<Res<ControlledSubject>>,
    player_q: Query<&BodyKinematics, (With<PlayerEntity>, With<PrimaryPlayer>)>,
) {
    view.0.clear();
    // The session's own subject, for a wielder that has acquired nothing. `None`
    // in a match with no local participant, which is legitimate rather than a
    // reason to publish nothing.
    let subject_pos = controlled
        .as_deref()
        .and_then(|subject| subject.0)
        .and_then(|entity| bodies.get(entity).ok())
        .or_else(|| player_q.single().ok())
        .map(|kin| kin.pos);
    for (disposition, held_item, kin, health, target) in &wielders {
        let Some(kin) = kin else {
            continue;
        };
        // ⛔ THE SAME LINE THE IN-HAND ROAD READS, so the two cannot both claim a
        // holder or both skip one.
        if !drawn_over_the_hand(Some(*disposition), health.map(|health| health.alive())) {
            continue;
        }
        let wielder_height = kin.size.y;
        // Its own target first; the session subject second; where it faces last.
        let aim_world = target
            .and_then(|target| target.entity)
            .and_then(|entity| bodies.get(entity).ok())
            .map(|kin| kin.pos)
            .or(subject_pos)
            .unwrap_or_else(|| kin.pos + ae::Vec2::new(kin.facing * wielder_height, 0.0));
        view.0.push(HostileWieldedItemFact {
            item_id: held_item.id().to_owned(),
            hand_world: ambition_mount::rider_hand_world_pos(kin.pos, kin.facing, wielder_height),
            aim_world,
            wielder_height,
        });
    }
}

/// Render queries ONLY this component — never the live `BodyKinematics` — and resolves `visual_id`
/// through the content-owned `ProjectileVisualCatalog`. Removed when a pooled projectile stops
/// being live.
#[derive(Component, Clone, Debug)]
pub struct ProjectileView {
    pub visual_id: String,
    pub pos: ae::Vec2,
    pub vel: ae::Vec2,
    pub size: ae::Vec2,
}

#[allow(clippy::type_complexity)]
pub fn rebuild_projectile_views(
    mut commands: Commands,
    mut live: Query<
        (
            Entity,
            &BodyKinematics,
            &ambition_projectiles::ProjectileVisualId,
            Option<&mut ProjectileView>,
        ),
        With<ambition_projectiles::LiveProjectile>,
    >,
    // Pooled projectiles: a reused entity that is no longer live must drop
    // its view so render despawns the visual instead of drawing a corpse.
    stale: Query<
        Entity,
        (
            With<ProjectileView>,
            Without<ambition_projectiles::LiveProjectile>,
        ),
    >,
) {
    for (entity, kin, visual_id, view) in &mut live {
        let next = ProjectileView {
            visual_id: visual_id.0.clone(),
            pos: kin.pos,
            vel: kin.vel,
            size: kin.size,
        };
        match view {
            Some(mut view) => *view = next,
            None => {
                commands.entity(entity).insert(next);
            }
        }
    }
    for entity in &stale {
        commands.entity(entity).remove::<ProjectileView>();
    }
}

#[derive(Clone, Debug)]
pub struct DynamicFeatureFact {
    pub id: String,
    /// Display label for the visual's debug `Name`.
    pub label: String,
    /// Family label ("Encounter mob" / "Staged actor" / "Post-boss NPC" /
    /// "Reward chest" / "Dropped pickup") — presentation naming only.
    pub family: &'static str,
    pub pos: ae::Vec2,
    pub size: ae::Vec2,
    pub visual_kind: ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind,
    pub fighting: bool,
    /// The placeholder entity-sprite the spawn resolves to (from the actor's
    /// brain / the NPC's interactable / the chest payload).
    pub sprite_key: Option<ambition_sprite_sheet::game_assets::EntitySprite>,
    /// An ANIMATED prop-sheet id to draw instead of the placeholder (a spinning
    /// ring, a pulsing gem) — the same `GameAssets.characters.props` key the
    /// room-load pass resolves for an authored pickup. `None`  the placeholder.
    pub prop_sheet: Option<String>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct DynamicFeatureViews(pub Vec<DynamicFeatureFact>);

#[allow(clippy::type_complexity)]
pub fn rebuild_dynamic_feature_views(
    mut view: ResMut<DynamicFeatureViews>,
    ecs_mobs: Query<
        (
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<&ambition_combat::actor_tuning::ActorConfig>,
        ),
        With<ambition_combat::components::EncounterMob>,
    >,
    staged_actors: Query<
        (
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<&ambition_combat::actor_tuning::ActorConfig>,
        ),
        With<ambition_combat::components::RuntimeStagedActor>,
    >,
    post_boss_npcs: Query<
        (
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::FeatureName,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ActorDisposition,
            Option<&ambition_combat::actor_tuning::ActorConfig>,
            Option<&ambition_combat::components::ActorInteraction>,
        ),
        With<ambition_combat::components::PostBossNpc>,
    >,
    ecs_reward_chests: Query<
        (
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::ChestFeature,
        ),
        bevy::prelude::Or<(
            With<ambition_combat::components::EncounterRewardChest>,
            With<ambition_combat::components::BossRewardChest>,
        )>,
    >,
    // Loot the running simulation MINTED — Sanic's scattered rings, and every
    // future drop. Selected by construction PROVENANCE (`SpawnOrigin::Dynamic`)
    // rather than a per-game marker: "this pickup was not in the room spec" is
    // exactly the condition under which the room-load visual pass could not have
    // seen it, so it is exactly the set that needs discovering here. An authored
    // pickup already has its visual and is filtered out below.
    dropped_pickups: Query<
        (
            &ambition_combat::components::FeatureId,
            &ambition_combat::components::FeatureName,
            &ambition_combat::components::CenteredAabb,
            &ambition_combat::components::PickupFeature,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
            Option<&ambition_platformer2d_actor_monolith::features::PickupArt>,
        ),
        Without<ambition_combat::components::Collected>,
    >,
) {
    use ambition_platformer2d_shared_tangle::feature_kind::FeatureVisualKind;
    use ambition_sprite_sheet::game_assets;
    view.0.clear();
    for (id, aabb, disposition, config) in &ecs_mobs {
        // ⛔⛔ "PEACEFUL" IS NOT "DOES NOT EXIST", AND THIS ARM USED TO SAY IT WAS.
        // It read *"Encounter mobs are hostile by construction; skip any peaceful
        // one"* and dropped them — so a runtime mob that was not fighting
        // published no `DynamicFeatureFact`, `spawn_dynamic_feature_visuals`
        // never made it a `FeatureVisual`, and it had no sprite no matter how
        // healthy its art was. That is how the pirate's summoned shark came out
        // INVISIBLE: it is deliberately nobody's enemy, the targeting stand-down
        // marks an unengaged hostile actor peaceful anyway, and presentation then
        // declined to draw a body that was standing right there. Jon saw a debug
        // box and nothing else; every measurement said the actor was fine,
        // because it was — the renderer was never asked.
        //
        // ⭐ THE FIELD FOR THIS ALREADY EXISTED. `fighting` is exactly the
        // distinction the skip was abusing existence to express, and the
        // post-boss arm below has always used it that way. Two arms disagreed
        // with a third in the same function.
        let Some(config) = config else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: config.name.clone(),
            family: "Encounter mob",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: !disposition.is_peaceful(),
            // ⚠ STILL THE BRAIN'S KEY for a peaceful one. Unlike a post-boss NPC
            // there is no dialogue interactable to resolve art from, and an
            // encounter mob's art is its own either way — a shark that stops
            // hunting is still a shark.
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
            prop_sheet: None,
        });
    }
    for (id, aabb, disposition, config) in &staged_actors {
        // The same correction as the arm above: a staged actor that is not
        // fighting is still a body somebody has to be able to see.
        let Some(config) = config else {
            continue;
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: config.name.clone(),
            family: "Staged actor",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting: !disposition.is_peaceful(),
            sprite_key: game_assets::entity_sprite_for_enemy(&config.brain),
            prop_sheet: None,
        });
    }
    for (id, name, aabb, disposition, config, interaction) in &post_boss_npcs {
        let fighting = !disposition.is_peaceful();
        // A peaceful post-boss NPC resolves its sprite from the dialogue
        // interactable; a hostile one (provoked) from its archetype brain.
        let sprite_key = if disposition.is_peaceful() {
            match interaction {
                Some(i) => game_assets::entity_sprite_for_runtime_interactable(&i.interactable),
                None => continue,
            }
        } else {
            match config {
                Some(c) => game_assets::entity_sprite_for_enemy(&c.brain),
                None => continue,
            }
        };
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: name.0.clone(),
            family: "Post-boss NPC",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Actor,
            fighting,
            sprite_key,
            prop_sheet: None,
        });
    }
    for (id, aabb, chest) in &ecs_reward_chests {
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: id.as_str().to_string(),
            family: "Reward chest",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Chest,
            fighting: false,
            sprite_key: game_assets::entity_sprite_for_runtime_chest(&chest.chest),
            prop_sheet: None,
        });
    }
    for (id, name, aabb, pickup, origin, art) in &dropped_pickups {
        if !matches!(
            origin,
            ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic { .. }
        ) {
            continue;
        }
        view.0.push(DynamicFeatureFact {
            id: id.as_str().to_string(),
            label: name.0.clone(),
            family: "Dropped pickup",
            pos: aabb.center,
            size: aabb.size(),
            visual_kind: FeatureVisualKind::Pickup,
            fighting: false,
            // The static per-kind fallback, used only when the drop names no animated sheet or
            // that sheet hasn't loaded.
            sprite_key: game_assets::entity_sprite_for_runtime_pickup(pickup.kind()),
            prop_sheet: art.map(|art| art.0.clone()),
        });
    }
}

/// Render draws the ember ring; it computes nothing.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct BlinkPreviewFact {
    /// Ring visible this tick (blink held / aiming, ability owned, gameplay
    /// allowed).
    pub active: bool,
    /// Predicted landing point.
    pub target: ae::Vec2,
    /// Precision (steered) aim vs quick-tap — picks the ember palette.
    pub precision: bool,
    /// The blinking body's smaller AABB extent — ring radius + ember size
    /// scale off it.
    pub body_min_extent: f32,
}

/// Rebuild [`BlinkPreviewFact`] each tick. Mirrors the destination
/// resolution used by the engine and the `show_blink_preview` debug overlay.
/// The blink button shares ground with menu input, so this honours the same
/// gameplay-only gate as `draw_player_debug` — paused / dialog states don't
/// light up the ring.
#[cfg(feature = "input")]
#[allow(clippy::type_complexity)]
pub fn rebuild_blink_preview_fact(
    mut fact: ResMut<BlinkPreviewFact>,
    // THE ONE COLLISION READ-API, because the preview was resolving
    // against a DIFFERENT WORLD than the blink. This took the room plus
    // `MovingPlatformSet` and composed `world_with_moving_platforms` itself,
    // under a comment claiming *"the moving-platform-aware temporary world is
    // what the actual blink resolves against"*. That was true when written and
    // is not: the body integrates against `world_with_sandbox_solids`, which
    // ALSO carries the ECS overlay (gate lock-walls, falling-sand pools,
    // broken-brick subtractions) and the portal carves.
    //
    //  the reticle could show a destination through a lock wall the blink
    // stops at, or stop at a portal aperture the blink passes through. A
    // preview that disagrees with the action is worse than none.
    collision: ambition_platformer2d_world::collision::CollisionWorld,
    mode: Res<bevy::prelude::State<ambition_platformer2d_shared_tangle::schedule::GameMode>>,
    action_query: Query<
        &leafwing_input_manager::prelude::ActionState<
            ambition_input::Platformer2dInputActionMonolith,
        >,
        (
            With<ambition_platformer2d_shared_tangle::lifecycle::PlayerVisual>,
            With<ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
    >,
    // The blink reticle previews from the CONTROLLED SUBJECT (the body
    // holding `DrivingParticipant(PRIMARY)`) — the body you are driving — so it
    // follows a possessed body instead of hovering at the vacated home
    // avatar. Both player and actor bodies carry these blink clusters.
    controlled: Res<ControlledSubject>,
    player_q: Query<(
        &BodyKinematics,
        &ambition_platformer2d_core::BodyAbilities,
        &ambition_platformer2d_core::BodyMotionFacts,
    )>,
) {
    use ambition_input::read_gameplay_control_frame;
    use ambition_platformer2d_core as ae;

    fact.active = false;
    let Ok((kin, abilities, motion_facts)) =
        controlled.0.and_then(|e| player_q.get(e).ok()).ok_or(())
    else {
        return;
    };
    let actions = if mode.get().allows_gameplay() {
        action_query.single().ok()
    } else {
        None
    };
    let controls = actions.map(read_gameplay_control_frame).unwrap_or_default();

    if !(abilities.abilities.blink && (controls.blink_held || motion_facts.blink_aiming)) {
        return;
    }

    // The SAME composition `step_motion` collides against — see the parameter.
    let Some(blink_world) = collision.solids() else {
        return;
    };
    let target = if motion_facts.blink_aiming {
        ae::blink_destination_to_point_clusters(
            &blink_world,
            kin,
            abilities,
            kin.pos + motion_facts.blink_aim_offset,
        )
    } else {
        let aim = ae::Vec2::new(controls.axis_x, controls.axis_y)
            .normalize_or(ae::Vec2::new(kin.facing, 0.0));
        ae::blink_destination_clusters(&blink_world, kin, abilities, aim, ae::BLINK_DISTANCE)
    };

    *fact = BlinkPreviewFact {
        active: true,
        target,
        precision: motion_facts.blink_aiming,
        body_min_extent: kin.size.min_element(),
    };
}

/// Registers the observation-boundary view resources + their rebuilds in the
/// sim tail. Owned here (anti-god rule 5): the plugin that rebuilds a view
/// initializes it; presentation only reads.
pub struct SimViewPlugin;

impl Plugin for SimViewPlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.init_resource::<PlayerHudFacts>()
            .init_resource::<HeldItemView>()
            .init_resource::<ControlledBodiesView>()
            .init_resource::<GroundItemsView>()
            .init_resource::<WorldItemsView>()
            .init_resource::<HeldShotsView>()
            .init_resource::<MarkBeaconsView>()
            .init_resource::<GravitySwitchesView>()
            .init_resource::<ShrinesView>()
            .init_resource::<HostileWieldedItemsView>()
            .init_resource::<DynamicFeatureViews>()
            .init_resource::<BlinkPreviewFact>();
        // The blink-preview resolve reads device actions, so it exists only
        // with the input layer; the FACT resource above is unconditional so
        // consumers read an inert default headless.
        #[cfg(feature = "input")]
        app.add_systems(
            sim,
            rebuild_blink_preview_fact
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
        app.add_systems(
            sim,
            (
                rebuild_player_hud_facts,
                rebuild_held_item_view,
                rebuild_controlled_bodies_view,
                rebuild_ground_items_view,
                rebuild_world_items_view,
                rebuild_held_shots_view,
                rebuild_mark_beacons_view,
                rebuild_gravity_switches_view,
                rebuild_shrines_view,
                tick_shrine_activation_pulse,
                rebuild_hostile_wielded_items_view,
                rebuild_projectile_views,
                rebuild_dynamic_feature_views,
            )
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ A PEACEFUL ENCOUNTER MOB IS STILL A BODY SOMEBODY HAS TO SEE.
    ///
    /// ⛔⛔ THIS ARM USED TO DROP THEM, under a comment asserting that *"encounter
    /// mobs are hostile by construction"*. They are not: the pirate's summoned
    /// shark is deliberately nobody's enemy, and the targeting stand-down marks
    /// any unengaged hostile actor peaceful anyway. A dropped fact means no
    /// `DynamicFeatureFact`, so `spawn_dynamic_feature_visuals` never builds a
    /// `FeatureVisual`, so the body has no sprite — however healthy its art is.
    /// Jon played it and saw a debug box hovering where a burning shark should
    /// be, and every measurement of the ACTOR came back correct, because the
    /// actor was correct. Presentation was never asked to draw it.
    ///
    /// ⭐ IT ASSERTS `fighting` TOO, not just presence. `fighting` is the field
    /// the skip was abusing existence to express, and a fix that published the
    /// mob while still calling it a fighter would trade one wrong answer for
    /// another.
    #[test]
    fn a_peaceful_encounter_mob_still_publishes_a_visual_fact() {
        use ambition_combat::actor_tuning::ActorConfig;
        use ambition_combat::components::{
            ActorDisposition, CenteredAabb, EncounterMob, FeatureId,
        };
        let mut app = App::new();
        app.init_resource::<DynamicFeatureViews>();
        app.add_systems(Update, rebuild_dynamic_feature_views);

        let config = ActorConfig {
            id: "smash_ride_shark".into(),
            name: "Burning Flying Shark".into(),
            tuning: Default::default(),
            brain_profile: Default::default(),
            brain: ambition_entity_catalog::placements::CharacterBrain::Custom(
                "burning_flying_shark".into(),
            ),
            sprite_override_npc_name: None,
            sprite_character_id: Some("npc_burning_flying_shark".into()),
            preserves_mirror_symmetry: false,
        };
        app.world_mut().spawn((
            EncounterMob {
                encounter_id: "smash".into(),
            },
            FeatureId("smash_ride_shark".to_string()),
            CenteredAabb::new(ae::Vec2::new(10.0, 20.0), ae::Vec2::new(48.0, 22.0)),
            ActorDisposition::Peaceful,
            config,
        ));
        app.update();

        let views = app.world().resource::<DynamicFeatureViews>();
        let shark = views
            .0
            .iter()
            .find(|fact| fact.id == "smash_ride_shark")
            .expect(
                "a peaceful encounter mob published no visual fact, so nothing \
                 downstream will ever give it a sprite",
            );
        assert!(
            !shark.fighting,
            "a peaceful mob was published as a fighter, which is the opposite \
             error from the one this test exists for"
        );
    }

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
        app.insert_resource(
            ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse { remaining: 0.25 },
        );
        app.add_systems(Update, tick_shrine_activation_pulse);
        app.update();
        let remaining = app
            .world()
            .resource::<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>()
            .remaining;
        assert!((remaining - 0.15).abs() < 1e-6);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .resource::<ambition_platformer2d_shared_tangle::shrine::ShrineActivationPulse>()
                .remaining,
            0.0,
            "pulse clamps at zero"
        );
    }
}

#[cfg(test)]
mod held_item_view_tests {
    use super::*;
    use ambition_characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    use bevy::prelude::*;

    fn holder(app: &mut App, sim: &str, x: f32, item: &str, slot: u8) {
        let spec = ambition_characters::brain::HeldItemSpec {
            id: item.to_owned(),
            melee: None,
            ranged: None,
            use_behavior: Default::default(),
        };
        app.world_mut().spawn((
            BodyKinematics {
                pos: ae::Vec2::new(x, 0.0),
                size: ae::Vec2::splat(32.0),
                ..Default::default()
            },
            ambition_platformer2d_actor_monolith::features::HeldItem::new(spec),
            ActorControl::default(),
            SimId::placement(sim),
            DrivingParticipant(PlayerSlot(slot)),
        ));
    }

    fn undriven_holder(
        app: &mut App,
        sim: &str,
        item: &str,
        standing: Option<(ambition_combat::components::ActorDisposition, bool)>,
    ) {
        let spec = ambition_characters::brain::HeldItemSpec {
            id: item.to_owned(),
            melee: None,
            ranged: None,
            use_behavior: Default::default(),
        };
        let mut body = app.world_mut().spawn((
            BodyKinematics {
                size: ae::Vec2::splat(32.0),
                ..Default::default()
            },
            ambition_platformer2d_actor_monolith::features::HeldItem::new(spec),
            ActorControl::default(),
            SimId::placement(sim),
        ));
        if let Some((disposition, alive)) = standing {
            let mut health = BodyHealth::new(ambition_characters::actor::Health::new(10));
            if !alive {
                health.damage(10);
            }
            body.insert((disposition, health));
        }
    }

    /// EVERY DRIVEN HOLDER DRAWS, NOT JUST THE ONE YOU ARE LOOKING THROUGH.
    ///
    /// ⛔⛔ THIS VIEW WAS AN `Option`, filled from `ControlledSubject`, so exactly
    /// one held item existed in the whole world: a couch match drew seat zero's
    /// weapon and nothing else, and a CPU-versus-CPU match drew neither Admiral's
    /// gun-sword. The argument against that was already written for a sibling view
    /// in this file — *"a couch-versus match has two driven bodies and neither is
    /// more protected than the other, because a rule that privileges one
    /// participant stops being a rule about bodies"*.
    ///
    /// ⛔ THE ORDER IS ASSERTED, not incidental. This is a `Vec` built from an
    /// unordered query and the renderer spawns one visual per row, so an unsorted
    /// build would swap two items' draw order between runs of the same match.
    #[test]
    fn two_driven_holders_publish_two_facts_in_sim_id_order() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        // Spawned in the REVERSE of the expected order, so a pass-through of
        // query order cannot agree with the assertion by luck.
        holder(&mut app, "seat_two", 200.0, "gun_sword", 1);
        holder(&mut app, "seat_one", 100.0, "axe", 0);
        app.update();

        let view = app.world().resource::<HeldItemView>();
        assert_eq!(
            view.0
                .iter()
                .map(|f| f.item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["axe", "gun_sword"],
            "both driven holders publish, ordered by `SimId` — one fact means the \
             view is still singular"
        );
    }

    /// ⛔⛔ A CPU FIGHTER'S WEAPON IS DRAWN, and the previous version of this test
    /// asserted the opposite.
    ///
    /// It read *"an autonomous holder drawing its item is a separate question
    /// about NPC presentation"* — but the Pirate Admiral's side-B draws
    /// `admiral_gun_sword`, an id registered in the IN-HAND art manifest and in no
    /// other, so in a CPU-versus-CPU match neither Admiral's gun-sword was drawn
    /// by any road at all. `DrivingParticipant` was never the question a renderer
    /// was asking; custody is.
    #[test]
    fn a_holder_nobody_drives_still_publishes() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(&mut app, "cpu_admiral", "admiral_gun_sword", None);
        app.update();
        assert_eq!(
            app.world()
                .resource::<HeldItemView>()
                .0
                .iter()
                .map(|f| f.item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["admiral_gun_sword"],
        );
    }

    /// ⛔ AND THE PARTITION HOLDS AT ITS OWN EDGE: a LIVING HOSTILE holder is the
    /// over-hand road's, so the in-hand road must not also claim it.
    ///
    /// Without this the widening above would draw a hostile pirate's `gun_sword`
    /// TWICE — once in the hand from `HeldItemArtManifest` and once over it from
    /// `WieldedItemVisualCatalog`, both of which register that id.
    #[test]
    fn a_living_hostile_holder_belongs_to_the_other_road() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(
            &mut app,
            "hostile_pirate",
            "gun_sword",
            Some((ambition_combat::components::ActorDisposition::Hostile, true)),
        );
        app.update();
        assert!(
            app.world().resource::<HeldItemView>().0.is_empty(),
            "a living hostile wielder is drawn over the hand, so publishing it \
             here too would draw its gun-sword twice"
        );
    }

    /// ⛔ AND A DEAD ONE COMES BACK, because the over-hand road drops it. Nobody
    /// may be on NEITHER road — that is how the Admiral vanished in the first
    /// place, and a corpse is the arm where the two conditions disagree.
    #[test]
    fn a_dead_hostile_holder_falls_back_to_the_in_hand_road() {
        let mut app = App::new();
        app.init_resource::<HeldItemView>()
            .add_systems(Update, rebuild_held_item_view);
        undriven_holder(
            &mut app,
            "fallen_pirate",
            "gun_sword",
            Some((
                ambition_combat::components::ActorDisposition::Hostile,
                false,
            )),
        );
        app.update();
        assert_eq!(app.world().resource::<HeldItemView>().0.len(), 1);
    }

    /// A PEACEFUL holder is nobody's over-hand business either.
    #[test]
    fn a_peaceful_holder_is_drawn_in_the_hand() {
        assert!(drawn_in_the_hand(
            Some(ambition_combat::components::ActorDisposition::Peaceful),
            Some(true)
        ));
        assert!(drawn_in_the_hand(None, Some(true)));
        assert!(!drawn_in_the_hand(
            Some(ambition_combat::components::ActorDisposition::Hostile),
            Some(true)
        ));
    }
}
