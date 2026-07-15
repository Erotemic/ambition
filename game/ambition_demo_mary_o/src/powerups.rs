//! Super Mary-O's powerups — the M1 equipment chain, authored as A3 data.
//!
//! These are the M-track's proof that "powerups as equipment" is pure content on
//! the finished engine face (`docs/planning/demos/super-mary-o.md` §M1): a
//! mushroom-analog and a flower-analog authored entirely through the `ambition`
//! umbrella's re-exported A3 vocabulary, with **zero engine edits**. The engine's
//! `ambition::characters::equipment` module (A3) supplies the three mechanisms —
//! numeric modifiers, behavioral grants, on-hit armor — and this file just names
//! two rows that use them.
//!
//! Parody-original, like the rest of the demo (Q28): a "grow cap" and a "spark
//! blossom", homage in role, not a copy.

use bevy::prelude::*;

use ambition::characters::brain::action_set::RangedActionSpec;
use ambition::characters::equipment::{
    EquipmentGrant, EquipmentRow, ModifierOp, ModifierScope, OnHit, ParamModifier, WornEquipment,
};

use ambition::actors::actor::{BodyBaseSize, PrimaryPlayer};
use ambition::actors::avatar::PlayerBodyFrameOutput;
use ambition::actors::items::{spawn_world_item, WorldItem};
use ambition::actors::rooms::RoomLoaded;
use ambition::characters::actor::WornCharacter;
use ambition::engine_core as ae;
use ambition::engine_core::collision_semantics::{ContactKind, ContactSource};

use crate::provider::MARY_O_CHARACTER_ID;

/// The worn-character id of the GROWN form: a distinct SHEET
/// (`super_mary_o_tall`), not a scaled copy of the small sheet. Wearing it is how
/// the powerup grows Mary-O; reverting to [`MARY_O_CHARACTER_ID`] shrinks her.
const TALL_CHARACTER_ID: &str = "mary_o_tall";

/// The milk carton's half-extent — a small collectible box that pops out of a
/// bonked ?-block and grows Mary-O when she touches it.
const MILK_HALF: ae::Vec2 = ae::Vec2::new(12.0, 14.0);

/// Row id of the grow-cap (mushroom-analog).
pub const GROW_CAP_ID: &str = "grow_cap";
/// Row id of the spark-blossom (flower-analog).
pub const SPARK_BLOSSOM_ID: &str = "spark_blossom";

/// The grow-cap: **one-hit armor**, the classic first powerup's take-a-hit half.
///
/// It is pure A3 [`OnHit::ConsumeAsArmor`] with `downgrade_to: None`: worn, it
/// absorbs one hit and is spent (removed); the very next read finds no cap and the
/// hit would reach HP — "big → small on hit", as data, no write-back.
///
/// The GROWN look and size are NOT a modifier here: "small and tall have different
/// sprites" (Jon), so growing swaps the worn identity to a distinct tall SHEET
/// ([`TALL_CHARACTER_ID`]) and bumps the body's collider — see [`sync_grown_form`],
/// which makes the tall form a pure view of *wearing this cap*. So the cap's whole
/// data effect is the armor; the size is a reactive consequence of possessing it.
pub fn grow_cap() -> EquipmentRow {
    EquipmentRow {
        id: GROW_CAP_ID.to_string(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
    }
}

/// The spark-blossom: **a ranged verb + a damage buff**, the fireball powerup.
///
/// It grants a ranged bolt ([`EquipmentGrant::Ranged`]) — from which the moveset
/// derives a real fireable move on equip — and scales that shot's damage 1.5× at
/// fire (a `Verb("ranged")`-scoped [`ranged_param::DAMAGE`] modifier, folded in
/// [`ambition::characters::equipment::resolved_ranged`] at trigger-resolve).
///
/// It deliberately carries NO armor: an on-hit downgrade cannot re-run grant
/// application (A3 v1 — see [`OnHit`]), so a grant-bearing armor row would leave a
/// dangling verb. Layering the blossom over the [`grow_cap`] gives the two-hit feel
/// (lose fire, then lose size) without that gap: the cap is the armor, the blossom
/// the capability.
///
/// [`ranged_param::DAMAGE`]: ambition::characters::equipment::ranged_param::DAMAGE
pub fn spark_blossom() -> EquipmentRow {
    use ambition::characters::equipment::ranged_param;
    EquipmentRow {
        id: SPARK_BLOSSOM_ID.to_string(),
        modifiers: vec![ParamModifier {
            param: ranged_param::DAMAGE.to_string(),
            op: ModifierOp::Mul(1.5),
            scope: ModifierScope::Verb("ranged".to_string()),
        }],
        grants: vec![EquipmentGrant::Ranged(RangedActionSpec::Bolt {
            speed: 420.0,
            damage: 6,
        })],
        on_hit: None,
    }
}

// ---------------------------------------------------------------------------
// Runtime — the powerup wired onto the finished engine face.
//
// Three tiny content systems on two engine primitives (reactive blocks +
// `WorldItem`), zero engine edits beyond those primitives:
//   1. `bonk_power_blocks`  — a head-bonk on a ?-block pops a milk `WorldItem`.
//   2. (engine) `collect_world_items` equips `grow_cap` when she touches it.
//   3. `sync_grown_form`    — the tall sheet + collider is a pure VIEW of
//                             wearing the cap; a hit spends the cap → she shrinks.
// ---------------------------------------------------------------------------

/// The ?-blocks already popped this level. `GeoId` keys, so a specific block pops
/// its milk exactly once; [`refill_power_blocks_on_room_loaded`] clears it on every
/// (re)load so a cyclic replay re-arms the blocks. Only `insert`/`contains`/`clear`
/// touch it — never iteration — so the banned std-hash-iteration order never bites.
#[derive(Resource, Default)]
pub struct SpentPowerBlocks(pub std::collections::HashSet<ae::GeoId>);

/// Small standing collider (30×48) and the grown collider (same width, 1.5× tall).
/// Width is held constant so growing never wedges her into a one-tile gap.
fn small_body_size() -> ae::Vec2 {
    ae::movement::default_player_body_size()
}
fn tall_body_size() -> ae::Vec2 {
    let s = small_body_size();
    ae::Vec2::new(s.x, s.y * 1.5)
}

/// **The ?-block bonk.** A head contact (`ContactKind::Head`) against a ?-block —
/// identified by the durable `GeoId` the engine now carries on
/// `ContactSource::Block`, NOT by point-matching — pops a milk `WorldItem` out on
/// top of that block, once per block per level.
pub fn bonk_power_blocks(
    mut commands: Commands,
    mut spent: ResMut<SpentPowerBlocks>,
    players: Query<&PlayerBodyFrameOutput, With<PrimaryPlayer>>,
) {
    let Ok(frame) = players.single() else {
        return;
    };
    for contact in &frame.events.contacts {
        if contact.kind != ContactKind::Head {
            continue;
        }
        let ContactSource::Block { id, .. } = &contact.source else {
            continue;
        };
        let Some(i) = crate::power_block_index_for(id) else {
            continue;
        };
        if spent.0.contains(id) {
            continue;
        }
        spent.0.insert(id.clone());
        // The milk pops out resting on the block's top face (screen up = -y).
        let min = crate::power_block_min(i);
        let pos = ae::Vec2::new(min.x + crate::T * 0.5, min.y - MILK_HALF.y);
        spawn_world_item(
            &mut commands,
            WorldItem::equipping(grow_cap(), pos, MILK_HALF),
        );
    }
}

/// **Grown = wearing the cap.** The tall sheet and the taller collider are a pure
/// VIEW of possessing [`grow_cap`]: collecting the milk equips the cap (the engine's
/// `collect_world_items`) and she grows; a hit spends the cap (the engine's shared
/// armor pass) and she shrinks — no manual "revert" wiring, the equipment state
/// drives both directions.
///
/// Growing is feet-anchored (she rises out of the ground, feet planted) to respect
/// the no-pushout rule; shrinking lowers her the same way. Swapping [`WornCharacter`]
/// re-derives her kit/sprite through the engine's `apply_worn_character_gameplay`;
/// the tall row's kit is byte-identical, so only her look and size change.
pub fn sync_grown_form(
    mut players: Query<
        (
            &mut WornCharacter,
            &mut BodyBaseSize,
            &mut ae::BodyKinematics,
            Option<&WornEquipment>,
        ),
        With<PrimaryPlayer>,
    >,
) {
    let Ok((mut worn_char, mut base, mut kin, worn)) = players.single_mut() else {
        return;
    };
    let wants_tall = worn.is_some_and(|w| w.wears(GROW_CAP_ID));
    let is_tall = worn_char.0 == TALL_CHARACTER_ID;
    if wants_tall == is_tall {
        return;
    }
    let (id, size) = if wants_tall {
        (TALL_CHARACTER_ID, tall_body_size())
    } else {
        (MARY_O_CHARACTER_ID, small_body_size())
    };
    // Feet stay planted: shift the center up by half the height gain (up = -y).
    kin.pos.y -= (size.y - kin.size.y) * 0.5;
    kin.size = size;
    base.base_size = size;
    worn_char.0 = id.to_string();
}

/// Re-arm every ?-block when level 1-1 (re)loads, so a cyclic replay pops fresh
/// milk. Mirrors the crony restage; the milk items themselves are room-scoped and
/// despawn with the room.
pub fn refill_power_blocks_on_room_loaded(
    mut rooms: MessageReader<RoomLoaded>,
    mut spent: ResMut<SpentPowerBlocks>,
) {
    for message in rooms.read() {
        if message.room_id == crate::LEVEL_1_1_ROOM_ID {
            spent.0.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition::characters::equipment::{apply_equipment_grants, resolved_ranged, WornEquipment};

    /// The grow-cap absorbs one hit and is then spent — the A3 armor half of
    /// Mary-O's "big → small". (The tall LOOK/size is `sync_grown_form`'s pure
    /// view of *wearing* the cap; the cap's data is just this one-hit armor.)
    /// Proven through the umbrella's A3 API: if `ambition` didn't re-export
    /// `characters::equipment`, this demo would not compile (the E9 oracle).
    #[test]
    fn grow_cap_absorbs_one_hit_then_is_spent() {
        let mut worn = WornEquipment::new(vec![grow_cap()]);
        assert!(worn.wears(GROW_CAP_ID), "worn, so she reads as grown");

        // A hit spends the cap...
        assert_eq!(worn.consume_armor().as_deref(), Some(GROW_CAP_ID));
        // ...and the cap is gone on the next read (no write-back), so she'll shrink.
        assert!(!worn.wears(GROW_CAP_ID), "losing the cap reverts to small");
        // The next hit finds no armor — it would reach HP.
        assert_eq!(worn.consume_armor(), None);
    }

    /// The spark-blossom grants a ranged verb and scales its shot's damage at fire.
    #[test]
    fn spark_blossom_grants_a_scaled_fireball() {
        use ambition::characters::brain::action_set::ActionSet;
        use ambition::combat::moveset::{build_actor_moveset, RANGED_VERB};

        let worn = WornEquipment::new(vec![spark_blossom()]);

        // The grant confers a ranged verb the moveset can fire.
        let mut actions = ActionSet::peaceful();
        assert!(actions.ranged.is_none());
        apply_equipment_grants(&mut actions, &worn);
        let moveset = build_actor_moveset(None, actions.melee.as_ref(), actions.ranged.as_ref())
            .expect("the blossom's ranged verb yields a moveset");
        assert!(
            moveset.move_for_verb(RANGED_VERB).is_some(),
            "the spark-blossom grants a fireable ranged move"
        );

        // The fireball leaves the barrel with folded (×1.5) damage.
        let base = actions.ranged.expect("blossom set a ranged spec");
        let shot = resolved_ranged(base, &worn, "ranged", RANGED_VERB);
        assert_eq!(shot.damage(), 9, "×1.5 on the blossom's 6-damage bolt");
        assert_eq!(shot.speed(), 420.0, "speed is unmodified");
    }

    /// Distinct ids so a body can wear both (the cap as armor, the blossom as
    /// capability) without one shadowing the other.
    #[test]
    fn the_two_powerups_are_distinct_rows() {
        assert_ne!(grow_cap().id, spark_blossom().id);
    }

    /// The reactive grow: wearing the cap swaps to the tall sheet + a taller
    /// collider, feet planted; losing it (a hit) reverts to small, feet planted.
    /// The tall form is a pure VIEW of possessing the cap — no manual revert.
    #[test]
    fn wearing_the_cap_grows_and_losing_it_shrinks_feet_planted() {
        let mut app = App::new();
        let small = small_body_size();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter(MARY_O_CHARACTER_ID.to_string()),
                BodyBaseSize { base_size: small },
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: small,
                    facing: 1.0,
                },
            ))
            .id();
        app.add_systems(Update, sync_grown_form);

        // Feet (screen up = -y, so feet = max.y = pos.y + size.y/2).
        let feet = |app: &App| {
            let k = app.world().get::<ae::BodyKinematics>(body).unwrap();
            k.pos.y + k.size.y * 0.5
        };
        let feet0 = feet(&app);

        // Equip the cap -> she grows on the next tick.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![grow_cap()]));
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().0,
            TALL_CHARACTER_ID,
            "wearing the cap grows her to the tall SHEET"
        );
        assert!(
            app.world().get::<ae::BodyKinematics>(body).unwrap().size.y > small.y,
            "the collider grew taller"
        );
        assert!(
            (feet(&app) - feet0).abs() < 1e-3,
            "feet stay planted on grow"
        );

        // Spend the cap (a hit) -> she shrinks on the next tick.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().0,
            MARY_O_CHARACTER_ID,
            "losing the cap shrinks her back to small"
        );
        assert_eq!(
            app.world().get::<ae::BodyKinematics>(body).unwrap().size,
            small,
            "the collider is small again"
        );
        assert!(
            (feet(&app) - feet0).abs() < 1e-3,
            "feet stay planted on shrink"
        );
    }

    /// A head-bonk on a ?-block pops exactly one milk, matched by the block's
    /// durable `GeoId` on the contact — and a spent block never pops again.
    #[test]
    fn a_head_bonk_on_a_power_block_pops_one_milk_once() {
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: crate::power_block_id(0),
                },
            });
        app.world_mut().spawn((PrimaryPlayer, frame));
        app.add_systems(Update, bonk_power_blocks);

        app.update();
        let milk = |app: &mut App| {
            app.world_mut()
                .query::<&WorldItem>()
                .iter(app.world())
                .count()
        };
        assert_eq!(milk(&mut app), 1, "one bonk pops exactly one milk");
        // The same contact next frame must not re-pop: the block is spent.
        app.update();
        assert_eq!(milk(&mut app), 1, "a spent ?-block yields no more milk");
    }

    /// A head-bonk on ANY OTHER block (not a ?-block) pops nothing — the GeoId
    /// match is specific, not "any block from below".
    #[test]
    fn a_head_bonk_on_a_plain_block_pops_nothing() {
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: ae::GeoId::anon(),
                },
            });
        app.world_mut().spawn((PrimaryPlayer, frame));
        app.add_systems(Update, bonk_power_blocks);
        app.update();
        let count = app
            .world_mut()
            .query::<&WorldItem>()
            .iter(app.world())
            .count();
        assert_eq!(count, 0, "a plain block is not a ?-block");
    }
}
