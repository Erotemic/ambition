//! Super Mary-O's powerups — the M1 equipment chain, authored as A3 data.
//!
//! These are the M-track's proof that "powerups as equipment" is pure content on
//! the finished engine face (`docs/planning/demos/super-mary-o.md` §M1): a
//! mushroom-analog and a flower-analog authored entirely through the `ambition_platformer2d`
//! umbrella's re-exported A3 vocabulary, with **zero engine edits**. The engine's
//! `ambition_platformer2d::characters::equipment` module (A3) supplies the three mechanisms —
//! numeric modifiers, behavioral grants, on-hit armor — and this file just names
//! two rows that use them.
//!
//! Parody-original, like the rest of the demo (Q28): a "star wand" and a "spark
//! beacon", homage in role, not a copy.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ProjectileFlight, RangedActionSpec};
use ambition_platformer2d::characters::equipment::{
    EquipmentGrant, EquipmentRow, ModifierOp, ModifierScope, OnHit, ParamModifier, WornEquipment,
};

use ambition_platformer2d::actors::actor::{BodyBaseSize, PrimaryPlayer};
use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::items::{
    spawn_moving_world_item, ItemMotionPlan, WorldItem,
};
use ambition_platformer2d::actors::rooms::RoomLoaded;
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::collision_semantics::{ContactKind, ContactSource};
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;

use crate::provider::MARY_O_CHARACTER_ID;

/// The worn-character id of the GROWN form: a distinct SHEET
/// (`super_mary_o_tall`), not a scaled copy of the small sheet. Wearing it is how
/// the powerup grows Mary-O; reverting to [`MARY_O_CHARACTER_ID`] shrinks her.
const TALL_CHARACTER_ID: &str = "mary_o_tall";

/// The worn-character id of the FIRE form (the fire-flower analog). A distinct
/// SHEET (`super_mary_o_fire`) with its own fireball pose, tinted the classic
/// white-and-red fire palette — the SAME height as the grown form, so the spark
/// beacon changes her LOOK + spark loadout without a size flicker. Wearing the
/// [`CINDER_BEACON_ID`] row selects this; losing the spark reverts to
/// [`TALL_CHARACTER_ID`] (grown), then a second hit to [`MARY_O_CHARACTER_ID`]
/// (small). Before this she wore the plain tall sheet while spark-powered, so
/// there was no visible difference between grown and fire (Jon bug #10).
const SPARK_CHARACTER_ID: &str = "mary_o_fire";

/// Authored transform/reversion cue ids.  The packed SFX bank provides the
/// layered clips; Mary-O's provider registry supplies authorization and a tiny
/// procedural fallback while the bank is unavailable.
pub const SFX_SMALL_TO_BIG: &str = "mary_o.transform.small_to_big";
pub const SFX_BIG_TO_FIRE: &str = "mary_o.transform.big_to_fire";
pub const SFX_BIG_TO_SMALL: &str = "mary_o.revert.big_to_small";
pub const SFX_FIRE_TO_BIG: &str = "mary_o.revert.fire_to_big";
pub const SFX_FIRE_TO_SMALL: &str = "mary_o.revert.fire_to_small";

/// The star wand's half-extent — a small collectible box that pops out of a
/// bonked ?-block and grows Mary-O when she touches it.
const STAR_WAND_HALF: ae::Vec2 = ae::Vec2::new(12.0, 15.5);

/// The beacon's half-extent — the second collectible in the chain.
const CINDER_BEACON_HALF: ae::Vec2 = ae::Vec2::new(12.0, 18.0);

/// The presentation art id the wand `WorldItem` carries. The app binds it to the
/// generated `super_mary_o_star_wand` sprite in `WorldItemArt`; the render draws
/// that image, or the cream quad until it is regenerated. Shared here so the spawn
/// and the art binding name the exact same key.
pub const STAR_WAND_SPRITE: &str = "super_mary_o_star_wand";

/// The exclusive slot BOTH power rows occupy. Mary-O has exactly one power state
/// at a time, so collecting the beacon REPLACES the wand rather than stacking on
/// top of it. Stacking would silently invert the loss order — the older wand would
/// be found first by the armor spend, so a hit would shrink her while leaving the
/// spark, which is backwards. With one slot, the worn row IS her power state and
/// its `downgrade_to` is the authority on what losing it means.
pub const FORM_SLOT: &str = "mary_o_form";

/// Row id of the star wand (mushroom-analog).
pub const STAR_WAND_ID: &str = "star_wand";
/// Row id of the cinder beacon (flower-analog).
pub const CINDER_BEACON_ID: &str = "cinder_beacon";

/// The star wand: **one-hit armor**, the classic first powerup's take-a-hit half.
///
/// It is pure A3 [`OnHit::ConsumeAsArmor`] with `downgrade_to: None`: worn, it
/// absorbs one hit and is spent (removed); the very next read finds no wand and the
/// hit would reach HP — "big → small on hit", as data, no write-back.
///
/// The GROWN look and size are NOT a modifier here: "small and tall have different
/// sprites" (Jon), so growing swaps the worn identity to a distinct tall SHEET
/// ([`TALL_CHARACTER_ID`]) and bumps the body's collider — see [`sync_grown_form`],
/// which makes the tall form a pure view of *wearing this wand*. So the wand's whole
/// data effect is the armor; the size is a reactive consequence of possessing it.
pub fn star_wand() -> EquipmentRow {
    EquipmentRow {
        id: STAR_WAND_ID.to_string(),
        modifiers: Vec::new(),
        grants: Vec::new(),
        on_hit: Some(OnHit::ConsumeAsArmor { downgrade_to: None }),
        exclusive_slot: Some(FORM_SLOT.to_string()),
    }
}

/// The cinder beacon: **a ranged verb AND the outer layer of armor**.
///
/// It grants a bouncing spark ([`EquipmentGrant::Ranged`]) and scales that shot's
/// damage 1.5x at fire (a `Verb("ranged")`-scoped [`ranged_param::DAMAGE`]
/// modifier, folded in [`ambition_platformer2d::characters::equipment::resolved_ranged`] at
/// trigger-resolve).
///
/// Crucially it is ALSO armor, and its `downgrade_to` is the [`star_wand`]. That
/// single field is the whole power-state progression: worn over nothing it is the
/// spark-powered grown form; a hit spends it and splices the wand into its place,
/// so she loses the spark and stays tall; the next hit spends the wand and she
/// shrinks. Two hits, two distinct losses, expressed as data.
///
/// This used to be impossible. A grant-bearing armor row would leave a dangling
/// verb, because equip applied grants one-shot and the victim-side resolver could
/// not re-run them — so the beacon had to carry NO armor and be layered beside
/// the wand instead. Now that granted actions are RECONCILED from the worn set,
/// spending this row revokes its verb on the same path that granted it, and the
/// honest representation is available.
///
/// [`ranged_param::DAMAGE`]: ambition_platformer2d::characters::equipment::ranged_param::DAMAGE
pub fn cinder_beacon() -> EquipmentRow {
    use ambition_platformer2d::characters::equipment::ranged_param;
    EquipmentRow {
        id: CINDER_BEACON_ID.to_string(),
        modifiers: vec![ParamModifier {
            param: ranged_param::DAMAGE.to_string(),
            op: ModifierOp::Mul(1.5),
            scope: ModifierScope::Verb("ranged".to_string()),
        }],
        grants: vec![EquipmentGrant::Ranged(spark_shot())],
        on_hit: Some(OnHit::ConsumeAsArmor {
            downgrade_to: Some(Box::new(star_wand())),
        }),
        exclusive_slot: Some(FORM_SLOT.to_string()),
    }
}

/// The spark itself: a low, fast shot that falls and skips off floors, dying
/// after two bounces or a second and a half — whichever comes first.
///
/// Authored entirely as data on the ranged action. The shared projectile stepper
/// has no idea this shot exists: it reads gravity, a bounce budget, a lifetime and
/// a visual id off the spec, exactly as it does for every other projectile. That
/// is the point — a bouncing spark is a set of numbers, not a code path.
fn spark_shot() -> RangedActionSpec {
    RangedActionSpec::bolt(SPARK_SPEED, SPARK_DAMAGE)
        .with_flight(
            ProjectileFlight::arcing(SPARK_GRAVITY, SPARK_BOUNCES)
                .with_lifetime(SPARK_LIFETIME_S)
                .with_half_extent(ae::Vec2::new(7.0, 7.0)),
        )
        .with_visual(SPARK_VISUAL)
}

/// Launch speed of a spark (px/s).
const SPARK_SPEED: f32 = 300.0;
/// Base damage before the beacon's x1.5 fold — enough to end a snake.
const SPARK_DAMAGE: i32 = 4;
/// Downward pull, which is what turns a flat shot into a skipping arc.
const SPARK_GRAVITY: f32 = 900.0;
/// Floor skips before it burns out.
const SPARK_BOUNCES: u8 = 2;
/// Hard lifetime cap, so a spark that finds no floor still expires.
const SPARK_LIFETIME_S: f32 = 1.5;
/// The projectile-visual id Mary-O registers her spark look under.
pub const SPARK_VISUAL: &str = "mary_o_spark";

/// The presentation art id the beacon `WorldItem` carries, bound to a real
/// sprite by the provider through the shared `WorldItemArt` seam.
pub const CINDER_BEACON_SPRITE: &str = "super_mary_o_cinder_beacon";

/// Marker on a live Mary-O spark, so her two-at-a-time limit counts HER shots and
/// constrains nobody else's projectiles.
#[derive(Component, Debug)]
pub struct MaryOSpark;

/// Tag freshly spawned sparks by the visual identity her ranged action authored.
///
/// The shot itself is an ordinary shared projectile — it moves, collides, damages,
/// and despawns on the one shared path, and nothing here touches any of that. This
/// only stamps a content marker so her active-shot limit can count HER sparks
/// without the projectile domain learning what a spark is.
pub fn tag_mary_o_sparks(
    mut commands: Commands,
    fresh: Query<
        (Entity, &ambition_platformer2d::projectiles::ProjectileVisualId),
        (
            Added<ambition_platformer2d::projectiles::ProjectileVisualId>,
            Without<MaryOSpark>,
        ),
    >,
) {
    for (entity, visual) in &fresh {
        if visual.0 == SPARK_VISUAL {
            commands.entity(entity).try_insert(MaryOSpark);
        }
    }
}

// ---------------------------------------------------------------------------
// Runtime — the powerup wired onto the finished engine face.
//
// Three tiny content systems on two engine primitives (reactive blocks +
// `WorldItem`), zero engine edits beyond those primitives:
//   1. `bonk_power_blocks`  — a head-bonk on a ?-block pops a wand `WorldItem`.
//   2. (engine) `collect_world_items` equips `star_wand` when she touches it.
//   3. `sync_grown_form`    — the tall sheet + collider is a pure VIEW of
//                             wearing the wand; a hit spends the wand → she shrinks.
// ---------------------------------------------------------------------------

/// The ?-blocks already popped this level. `GeoId` keys, so a specific block pops
/// its wand exactly once; [`refill_power_blocks_on_room_loaded`] clears it on every
/// (re)load so a cyclic replay re-arms the blocks. Only `insert`/`contains`/`clear`
/// touch it — never iteration — so the banned std-hash-iteration order never bites.
#[derive(Resource, Default)]
pub struct SpentPowerBlocks(pub std::collections::HashSet<ae::GeoId>);

/// Small standing collider (30×48) and the grown collider (same width, 1.5× tall).
/// Width is held constant so growing never wedges her into a one-tile gap.
fn small_body_size() -> ae::Vec2 {
    ae::movement::default_player_body_size()
}
pub(crate) fn tall_body_size() -> ae::Vec2 {
    let s = small_body_size();
    ae::Vec2::new(s.x, s.y * 1.5)
}

/// **The ?-block bonk.** A head contact (`ContactKind::Head`) against a ?-block —
/// identified by the durable `GeoId` the engine now carries on
/// `ContactSource::Block`, NOT by point-matching — pops a `WorldItem` out on top
/// of that block, once per block per level.
///
/// WHICH item is a function of her current power state, read from the one
/// authority that state lives in (her worn equipment): small gets the wand that
/// grows her, grown gets the beacon, and a Mary-O who already has the beacon
/// gets nothing rather than a duplicate form row. There is no separate progress
/// flag to keep in sync — the equipment IS the progress.
pub fn bonk_power_blocks(
    mut commands: Commands,
    mut spent: ResMut<SpentPowerBlocks>,
    players: Query<(&PlayerBodyFrameOutput, Option<&WornEquipment>), With<PrimaryPlayer>>,
) {
    let Ok((frame, worn)) = players.single() else {
        return;
    };
    for contact in &frame.events.contacts {
        if contact.kind != ContactKind::Head {
            continue;
        }
        let ContactSource::Block { id, .. } = &contact.source else {
            continue;
        };
        // TWO block families, and which one was struck decides what comes out.
        // A quasar block yields the quasar to ANY form, because being briefly
        // untouchable is not a rung on the power ladder (Jon: "a quasar is not
        // part of the wand -> lantern item progression. Any form of maryo should
        // be able to get the quasar").
        let Some((i, reward_of)) = crate::power_block_index_for(id)
            .map(|i| (i, RewardSource::PowerLadder))
            .or_else(|| {
                crate::quasar_block_index_for(id).map(|i| (i, RewardSource::Quasar))
            })
        else {
            continue;
        };
        if spent.0.contains(id) {
            continue;
        }
        let Some(reward) = (match reward_of {
            RewardSource::Quasar => Some(quasar_reward()),
            RewardSource::PowerLadder => next_power_reward(worn),
        }) else {
            // No rung left to give. Unreachable while the ladder ends in the
            // star, and kept because `next_power_reward` is allowed to say no —
            // an un-spent block still has its reward waiting afterwards.
            continue;
        };
        spent.0.insert(id.clone());
        // The reward pops out resting on the block's top face (screen up = -y).
        let min = match reward_of {
            RewardSource::Quasar => crate::quasar_block_min(i),
            RewardSource::PowerLadder => crate::power_block_min(i),
        };
        // It starts INSIDE the block and rises out — the beat Jon asked for.
        let pos = ae::Vec2::new(min.x + crate::T * 0.5, min.y + crate::T * 0.5);
        spawn_moving_world_item(
            &mut commands,
            WorldItem::equipping(reward.row, pos, reward.half).with_sprite(reward.sprite),
            reward.motion,
        );
    }
}

/// Which family of ?-block was struck, and therefore what it owes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RewardSource {
    /// The wand -> lantern progression: what it gives depends on what she wears.
    PowerLadder,
    /// A quasar block: the same answer for every form.
    Quasar,
}

/// The quasar, which any Mary-O can collect at any power tier.
fn quasar_reward() -> PowerReward {
    PowerReward {
        row: crate::star::pocket_quasar(),
        half: crate::star::QUASAR_HALF,
        sprite: crate::star::QUASAR_SPRITE,
        // It BOUNDS (Jon: "the quasar doesn't bound like a mario superstar
        // does") — fast, and giving back most of every landing.
        motion: rises_from_a_block(
            ItemMotionPlan::bouncer(QUASAR_SPEED, QUASAR_RESTITUTION),
            crate::star::QUASAR_HALF.y,
        ),
    }
}

/// One rung up the ladder, chosen from what she is already wearing.
struct PowerReward {
    row: EquipmentRow,
    half: ae::Vec2,
    sprite: &'static str,
    /// **How it behaves once it is out of the block** — the engine steps this
    /// plan, so the demo only says which one it wants.
    motion: ItemMotionPlan,
}

/// How far and how long a reward takes to climb out of the block that produced
/// it. It spawns INSIDE and rises to rest on the top face, so the distance is
/// half a block plus its own half-height.
fn rises_from_a_block(plan: ItemMotionPlan, half_y: f32) -> ItemMotionPlan {
    plan.emerging(crate::T * 0.5 + half_y, 0.4)
}

/// Travel speed of the wand once it is out — slower than she walks, so a player
/// who wants it can always catch it.
const WAND_SPEED: f32 = 55.0;
/// The quasar is faster and gives back most of what it lands with: it is meant
/// to bound away from you like a superstar.
const QUASAR_SPEED: f32 = 105.0;
const QUASAR_RESTITUTION: f32 = 0.86;

/// `small -> wand`, `grown -> beacon`, `spark-powered -> nothing`.
///
/// Reading the worn set rather than a demo flag is what makes duplicates
/// unrepresentable: there is no state to drift out of sync with, because the
/// question "what does she have" has exactly one answer.
fn next_power_reward(worn: Option<&WornEquipment>) -> Option<PowerReward> {
    let wears = |id: &str| worn.is_some_and(|w| w.wears(id));
    if wears(CINDER_BEACON_ID) {
        // Fully powered: this ladder has nothing left to give. The quasar is
        // NOT its top rung — it is not a form at all, and gating it behind two
        // other powerups would mean a small Mary-O could never be invincible.
        // It has its own blocks; see `bonk_power_blocks`.
        None
    } else if wears(STAR_WAND_ID) {
        Some(PowerReward {
            row: cinder_beacon(),
            half: CINDER_BEACON_HALF,
            sprite: CINDER_BEACON_SPRITE,
            // The beacon waits on its block, like the classic flower.
            motion: rises_from_a_block(ItemMotionPlan::still(), CINDER_BEACON_HALF.y),
        })
    } else {
        Some(PowerReward {
            row: star_wand(),
            half: STAR_WAND_HALF,
            sprite: STAR_WAND_SPRITE,
            // The wand WALKS and turns at walls, like the mushroom.
            motion: rises_from_a_block(ItemMotionPlan::walker(WAND_SPEED), STAR_WAND_HALF.y),
        })
    }
}

/// **Grown = wearing the wand.** The tall sheet and the taller collider are a pure
/// VIEW of possessing [`star_wand`]: collecting the wand equips the wand (the engine's
/// `collect_world_items`) and she grows; a hit spends the wand (the engine's shared
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
            bevy::prelude::Entity,
            &mut WornCharacter,
            &mut BodyBaseSize,
            &mut ae::BodyKinematics,
            Option<&WornEquipment>,
        ),
        With<PrimaryPlayer>,
    >,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut commands: bevy::prelude::Commands,
) {
    let Ok((body, mut worn_char, mut base, mut kin, worn)) = players.single_mut() else {
        return;
    };
    // THREE forms, chosen from what she wears. The fire (beacon) and grown (wand)
    // forms are the SAME height — the beacon downgrades INTO the wand on a hit, so
    // across that transition she stays continuously tall and only her look + spark
    // loadout change; the size flickers on neither the grow nor the spark→grown
    // downgrade, only on the final grown→small hit.
    let (target_id, target_size) = if worn.is_some_and(|w| w.wears(CINDER_BEACON_ID)) {
        (SPARK_CHARACTER_ID, tall_body_size())
    } else if worn.is_some_and(|w| w.wears(STAR_WAND_ID)) {
        (TALL_CHARACTER_ID, tall_body_size())
    } else {
        (MARY_O_CHARACTER_ID, small_body_size())
    };
    if worn_char.0 == target_id {
        return;
    }
    // Feet stay planted across a size change (grow / shrink): shift the center up
    // by half the height gain (up = -y). The fire↔grown swap is same-size, so the
    // guard skips the shift there and only her sheet changes.
    if kin.size != target_size {
        kin.pos.y -= (target_size.y - kin.size.y) * 0.5;
        kin.size = target_size;
        base.base_size = target_size;
    }
    let previous_id = worn_char.0.clone();
    if let Some(cue_id) = power_transition_sfx(&previous_id, target_id) {
        // The named transition is content-authority: small→big, big→fire, and
        // each loss path have different layered sound design rather than sharing
        // one generic pitch slide.  The hit cue may still play on a downgrade;
        // this second voice says WHAT power state was lost.
        sfx.write_for(
            body,
            ambition_platformer2d::sfx::SfxMessage::Play {
                id: ambition_platformer2d::sfx::SfxId::new(cue_id),
                pos: kin.pos,
            },
        );
    }
    // The transformation MOMENT, in BOTH directions (Jon bugs #4 and #17: "the
    // growing", and "a similar transform animation down to the previous state
    // with non instant duration"). It used to fire only on a step UP, on the
    // reasoning that a reversion could use its hurt animation instead — but the
    // hurt read is the ordinary hitstun pose, and what a hit costs HER is a
    // FORM. The shrink clip is about the form, so it gets the same beat the
    // growth does. Same shape as the cue above: the transition picks it.
    commands.entity(body).try_insert((
        transform_beat_policy(target_id, power_tier(&previous_id), power_tier(target_id)),
        ambition_platformer2d::actors::features::transform_beat::TransformBeatRequested,
    ));
    worn_char.0 = target_id.to_string();
}

/// **The transformation numbers for ONE tier change**, authored per transition
/// rather than once per body, because which clip plays and how long the moment
/// lasts are facts about the CHANGE, not about Mary-O. The exact shape
/// [`power_transition_sfx`] already uses for her voice.
///
/// The duration is not a number: it is however long the arriving sheet's clip
/// takes to draw, asked of the art. A hand-authored 0.5s (what this was) cuts
/// the eight-frame fire transformation off before its reveal — which is exactly
/// the "transform into fire mode is nearly instant if it even exists" report —
/// and would go stale the first time the generator retimes a row.
///
/// The wall-clock conversion is the game's arithmetic, not the engine's: the
/// animator ticks on the WORLD clock, so a beat that asks for 0.6x time
/// stretches its own clip by 1/0.6 in wall seconds, and the beat's wall timer
/// has to cover that or the clip is cut short by exactly the dilation it asked
/// for.
fn transform_beat_policy(
    target_id: &str,
    from_tier: u8,
    to_tier: u8,
) -> ambition_platformer2d::actors::features::transform_beat::TransformBeatPolicy {
    let anim = transition_anim(from_tier, to_tier);
    // A step UP is a moment to savour, so it ASKS the regime to slow the world
    // (`ClockScaleRequest`; a regime with a second participant may refuse). A
    // reversion does not: she was just hit, and slowing the world on a hit takes
    // the recovery away from the player instead of giving it.
    let clock_scale = if to_tier > from_tier {
        STEP_UP_CLOCK_SCALE
    } else {
        1.0
    };
    ambition_platformer2d::actors::features::transform_beat::TransformBeatPolicy {
        duration: clip_seconds(target_id, anim) / clock_scale,
        anim,
        clock_scale,
        untouchable: true,
    }
}

/// How much the world slows while she steps up a tier. Mild and short — a beat
/// you notice rather than one you wait through — and BLIND until Jon plays it.
const STEP_UP_CLOCK_SCALE: f32 = 0.6;

/// Fallback beat length for a form whose sheet cannot be read at all.
const UNREADABLE_CLIP_SECS: f32 = 0.45;

/// **Which clip shows this tier change.** A transition clip is authored on the
/// sheet of the form ARRIVED AT, so this is always resolved against the target.
fn transition_anim(from_tier: u8, to_tier: u8) -> CharacterAnim {
    if to_tier > from_tier {
        // Arriving at fire is a same-size palette transformation; arriving at
        // grown is the silhouette flicker.
        if to_tier == FIRE_TIER {
            CharacterAnim::Transform
        } else {
            CharacterAnim::Grow
        }
    } else if from_tier - to_tier >= 2 {
        CharacterAnim::BigShrink
    } else {
        CharacterAnim::Shrink
    }
}

/// How long one pass of `anim` takes on the form's sheet, asked of the sheet.
///
/// Resolved through the sheet's anim set exactly as the drawing will be, so a
/// form that never drew the clip answers with the length of whatever it falls
/// back to — the beat then lasts as long as what the player actually sees.
fn clip_seconds(character_id: &str, anim: CharacterAnim) -> f32 {
    use ambition_platformer2d::sprite_sheet::character::{try_load_spec_for_target, SheetTuning};
    try_load_spec_for_target(sheet_target(character_id), &SheetTuning::default())
        .map(|spec| spec.clip_seconds(anim))
        .filter(|secs| *secs > 0.0)
        .unwrap_or(UNREADABLE_CLIP_SECS)
}

/// The sheet manifest target behind each form's catalog row.
///
/// `WornCharacter` carries the CATALOG id, which is not the sheet's name; the
/// demo authored both halves of this pairing in its character catalog (`lib.rs`,
/// `spritesheet:` / `manifest:`), so stating it here is reaching the same
/// authoring decision from the sim rather than inventing a second one.
fn sheet_target(character_id: &str) -> &'static str {
    match character_id {
        SPARK_CHARACTER_ID => "super_mary_o_fire",
        TALL_CHARACTER_ID => "super_mary_o_tall",
        _ => "super_mary_o",
    }
}

/// The top of the power ladder — arriving HERE is the same-size transformation
/// rather than a growth.
const FIRE_TIER: u8 = 2;

/// Power-tier of a worn-character id: small (0) < grown (1) < fire (2). The
/// direction and distance along this ladder pick both the transition clip and
/// whether the moment slows the world.
fn power_tier(character_id: &str) -> u8 {
    match character_id {
        SPARK_CHARACTER_ID => FIRE_TIER,
        TALL_CHARACTER_ID => 1,
        _ => 0,
    }
}

/// The exact authored sound for a form transition.
///
/// The direct fire→small edge is used by the large-damage path even though the
/// ordinary armor chain normally spends fire into big first.
fn power_transition_sfx(from: &str, to: &str) -> Option<&'static str> {
    match (from, to) {
        (MARY_O_CHARACTER_ID, TALL_CHARACTER_ID) => Some(SFX_SMALL_TO_BIG),
        (TALL_CHARACTER_ID, SPARK_CHARACTER_ID) => Some(SFX_BIG_TO_FIRE),
        (TALL_CHARACTER_ID, MARY_O_CHARACTER_ID) => Some(SFX_BIG_TO_SMALL),
        (SPARK_CHARACTER_ID, TALL_CHARACTER_ID) => Some(SFX_FIRE_TO_BIG),
        (SPARK_CHARACTER_ID, MARY_O_CHARACTER_ID) => Some(SFX_FIRE_TO_SMALL),
        _ => None,
    }
}

/// Re-arm every ?-block when level 1-1 (re)loads, so a cyclic replay pops fresh
/// wand. Mirrors the snake restage; the wand items themselves are room-scoped and
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
    use ambition_platformer2d::characters::equipment::{apply_equipment_grants, resolved_ranged, WornEquipment};

    /// The star wand absorbs one hit and is then spent — the A3 armor half of
    /// Mary-O's "big → small". (The tall LOOK/size is `sync_grown_form`'s pure
    /// view of *wearing* the wand; the wand's data is just this one-hit armor.)
    /// Proven through the umbrella's A3 API: if `ambition_platformer2d` didn't re-export
    /// `characters::equipment`, this demo would not compile (the E9 oracle).
    #[test]
    fn every_supported_form_transition_has_its_own_sfx() {
        assert_eq!(
            power_transition_sfx(MARY_O_CHARACTER_ID, TALL_CHARACTER_ID),
            Some(SFX_SMALL_TO_BIG)
        );
        assert_eq!(
            power_transition_sfx(TALL_CHARACTER_ID, SPARK_CHARACTER_ID),
            Some(SFX_BIG_TO_FIRE)
        );
        assert_eq!(
            power_transition_sfx(TALL_CHARACTER_ID, MARY_O_CHARACTER_ID),
            Some(SFX_BIG_TO_SMALL)
        );
        assert_eq!(
            power_transition_sfx(SPARK_CHARACTER_ID, TALL_CHARACTER_ID),
            Some(SFX_FIRE_TO_BIG)
        );
        assert_eq!(
            power_transition_sfx(SPARK_CHARACTER_ID, MARY_O_CHARACTER_ID),
            Some(SFX_FIRE_TO_SMALL)
        );
        assert_eq!(power_transition_sfx(MARY_O_CHARACTER_ID, SPARK_CHARACTER_ID), None);
    }

    #[test]
    fn the_star_wand_absorbs_one_hit_then_is_spent() {
        let mut worn = WornEquipment::new(vec![star_wand()]);
        assert!(worn.wears(STAR_WAND_ID), "worn, so she reads as grown");

        // A hit spends the wand...
        assert_eq!(worn.consume_armor().as_deref(), Some(STAR_WAND_ID));
        // ...and the wand is gone on the next read (no write-back), so she'll shrink.
        assert!(!worn.wears(STAR_WAND_ID), "losing the wand reverts to small");
        // The next hit finds no armor — it would reach HP.
        assert_eq!(worn.consume_armor(), None);
    }

    /// The cinder beacon grants a ranged verb and scales its shot's damage at fire.
    #[test]
    fn the_cinder_beacon_grants_a_scaled_bouncing_spark() {
        use ambition_platformer2d::characters::brain::action_set::ActionSet;
        use ambition_platformer2d::combat::moveset::{build_actor_moveset, RANGED_VERB};

        let worn = WornEquipment::new(vec![cinder_beacon()]);

        // The grant confers a ranged verb the moveset can fire.
        let mut actions = ActionSet::peaceful();
        assert!(actions.ranged.is_none());
        apply_equipment_grants(&mut actions, &worn);
        let moveset = build_actor_moveset(
            None,
            actions.melee.as_ref(),
            actions.ranged.as_ref(),
            actions.special.as_ref(),
        )
        .expect("the beacon's ranged verb yields a moveset");
        assert!(
            moveset.move_for_verb(RANGED_VERB).is_some(),
            "the cinder beacon grants a fireable ranged move"
        );

        // The spark leaves the barrel with folded (x1.5) damage.
        let base = actions.ranged.clone().expect("beacon set a ranged spec");
        let shot = resolved_ranged(base, &worn, "ranged", RANGED_VERB);
        assert_eq!(shot.damage(), 6, "x1.5 on the beacon's 4-damage spark");
        assert_eq!(shot.speed(), SPARK_SPEED, "speed is unmodified");
        // The equipment fold must not drop the authored flight/visual — that was a
        // real bug in the variant-by-variant rebuild this replaced.
        let flight = shot.flight.expect("the fold preserves authored flight");
        assert!(flight.gravity > 0.0, "the spark arcs");
        assert!(flight.bounce_on_world_contact, "and skips off floors");
        assert_eq!(flight.bounces, SPARK_BOUNCES);
        assert_eq!(shot.visual.as_deref(), Some(SPARK_VISUAL));
    }

    /// The spark expires by an authored policy — a bounce budget AND a lifetime,
    /// so a shot that never finds a floor still burns out.
    #[test]
    fn the_spark_expires_by_bounces_or_lifetime() {
        let shot = spark_shot();
        let flight = shot.flight.expect("the spark authors its flight");
        assert_eq!(flight.bounces, 2, "two floor skips, then it is spent");
        assert!(
            flight.max_lifetime > 0.0 && flight.max_lifetime < 3.0,
            "and a hard lifetime cap regardless of bounces"
        );
    }

    /// **The progression, as equipment.** small -> wand -> grown -> beacon ->
    /// spark-powered, with no rung repeatable and no parallel flag.
    #[test]
    fn the_power_block_reward_climbs_the_ladder_and_never_duplicates() {
        // Small: the wand.
        let bare = WornEquipment::default();
        assert_eq!(
            next_power_reward(None).map(|r| r.row.id),
            Some(STAR_WAND_ID.to_string()),
            "small Mary-O is offered the wand"
        );
        assert_eq!(
            next_power_reward(Some(&bare)).map(|r| r.row.id),
            Some(STAR_WAND_ID.to_string()),
            "an empty worn set reads as small too"
        );

        // Grown: the beacon.
        let grown = WornEquipment::new(vec![star_wand()]);
        assert_eq!(
            next_power_reward(Some(&grown)).map(|r| r.row.id),
            Some(CINDER_BEACON_ID.to_string()),
            "grown Mary-O is offered the beacon"
        );

        // Fully powered: the ladder is done. The quasar is NOT its top rung.
        let sparked = WornEquipment::new(vec![cinder_beacon()]);
        assert!(
            next_power_reward(Some(&sparked)).is_none(),
            "a fully powered Mary-O has nothing left to gain from a ?-block"
        );
    }

    /// **The quasar is not on the ladder at all** (Jon: "a quasar is not part of
    /// the wand -> lantern item progression. Any form of maryo should be able to
    /// get the quasar and be invincible").
    ///
    /// It briefly landed as the ladder's top rung, which read tidily and was
    /// wrong in the way that matters: it made being untouchable a reward for
    /// already being powerful, so the Mary-O who most needs it — small, one hit
    /// from death — was the one who could never have it.
    #[test]
    fn the_quasar_is_the_same_offer_to_every_form() {
        // It occupies no form slot, so taking it can never cost her a tier.
        assert!(
            quasar_reward().row.exclusive_slot.is_none(),
            "the quasar is not a form"
        );
        assert_eq!(quasar_reward().row.id, crate::star::POCKET_QUASAR_ID);

        // And its blocks are a family of their own: no ?-block id answers as a
        // quasar block, and no quasar block answers as a ?-block. That disjointness
        // is what lets ONE bonk rule serve both without either shadowing the other.
        for i in 0..2 {
            let quasar = crate::quasar_block_id(i);
            assert!(crate::quasar_block_index_for(&quasar).is_some());
            assert!(
                crate::power_block_index_for(&quasar).is_none(),
                "a quasar block must not read as a power block"
            );
        }
        for i in 0..3 {
            let power = crate::power_block_id(i);
            assert!(
                crate::quasar_block_index_for(&power).is_none(),
                "a power block must not read as a quasar block"
            );
        }
    }

    /// **Damage walks the ladder back down**, one rung per hit, through the
    /// ordinary armor spend. Spark-powered -> grown (loses the spark, stays tall)
    /// -> small.
    #[test]
    fn damage_downgrades_spark_to_grown_then_grown_to_small() {
        let mut worn = WornEquipment::new(vec![cinder_beacon()]);

        // Hit one: the beacon is spent and leaves the wand in its place.
        assert_eq!(worn.consume_armor().as_deref(), Some(CINDER_BEACON_ID));
        assert!(!worn.wears(CINDER_BEACON_ID), "the spark is gone");
        assert!(worn.wears(STAR_WAND_ID), "but she is still grown");

        // Hit two: the wand is spent and she is small.
        assert_eq!(worn.consume_armor().as_deref(), Some(STAR_WAND_ID));
        assert!(!worn.wears(STAR_WAND_ID), "now she is small");

        // Hit three: no armor left — the hit reaches HP, ordinary damage policy.
        assert_eq!(worn.consume_armor(), None);
    }

    /// Distinct ids so a body can wear both (the wand as armor, the beacon as
    /// capability) without one shadowing the other.
    #[test]
    fn the_two_powerups_are_distinct_rows() {
        assert_ne!(star_wand().id, cinder_beacon().id);
    }

    /// The reactive grow: wearing the wand swaps to the tall sheet + a taller
    /// collider, feet planted; losing it (a hit) reverts to small, feet planted.
    /// The tall form is a pure VIEW of possessing the wand — no manual revert.
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
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        // Feet (screen up = -y, so feet = max.y = pos.y + size.y/2).
        let feet = |app: &App| {
            let k = app.world().get::<ae::BodyKinematics>(body).unwrap();
            k.pos.y + k.size.y * 0.5
        };
        let feet0 = feet(&app);

        // Equip the wand -> she grows on the next tick.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![star_wand()]));
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().0,
            TALL_CHARACTER_ID,
            "wearing the wand grows her to the tall SHEET"
        );
        assert!(
            app.world().get::<ae::BodyKinematics>(body).unwrap().size.y > small.y,
            "the collider grew taller"
        );
        assert!(
            (feet(&app) - feet0).abs() < 1e-3,
            "feet stay planted on grow"
        );

        // Spend the wand (a hit) -> she shrinks on the next tick.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().0,
            MARY_O_CHARACTER_ID,
            "losing the wand shrinks her back to small"
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

    /// **Both power states are tall, and the downgrade between them does not
    /// flicker her size.** Losing the spark must change what she can DO, not how
    /// big she is; only the second hit shrinks her, feet planted throughout.
    #[test]
    fn spark_powered_is_tall_and_only_the_second_hit_shrinks_her() {
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
                WornEquipment::new(vec![cinder_beacon()]),
            ))
            .id();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        let feet = |app: &App| {
            let k = app.world().get::<ae::BodyKinematics>(body).unwrap();
            k.pos.y + k.size.y * 0.5
        };
        // "Tall" is a SIZE fact now, since the two power states share a height but
        // NOT a sheet: the fire form has its own distinct look.
        let is_tall =
            |app: &App| app.world().get::<ae::BodyKinematics>(body).unwrap().size.y > small.y;
        let form = |app: &App| app.world().get::<WornCharacter>(body).unwrap().0.clone();

        app.update();
        let feet_tall = feet(&app);
        assert!(is_tall(&app), "wearing the beacon alone reads as tall");
        assert_eq!(
            form(&app),
            SPARK_CHARACTER_ID,
            "the beacon shows the DISTINCT fire sheet, not the plain grown one"
        );

        // Hit one: spark -> grown. Same height, but the sheet reverts to grown.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert!(
            is_tall(&app),
            "losing the spark leaves her GROWN, not small"
        );
        assert_eq!(
            form(&app),
            TALL_CHARACTER_ID,
            "losing the spark drops the fire sheet back to the grown sheet"
        );
        assert!(
            (feet(&app) - feet_tall).abs() < 1e-3,
            "and her size never flickered, so her feet never moved"
        );

        // Hit two: grown -> small, feet still planted.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert!(!is_tall(&app), "the second hit finally shrinks her");
        assert_eq!(
            form(&app),
            MARY_O_CHARACTER_ID,
            "the second hit shrinks her back to the small sheet"
        );
        assert!(
            (feet(&app) - feet_tall).abs() < 1e-3,
            "feet stay planted through the shrink"
        );
    }

    /// **Every form change voices its OWN cue** (Jon bug #14). Not one generic
    /// chime with a direction test: five authored edges, because gaining fire and
    /// losing it are different events with different sound design, and the pair
    /// that share a direction (`small→big` and `big→fire`) are not the same
    /// sound either.
    ///
    /// A reversion is deliberately NOT silent. It used to be, on the reasoning
    /// that the hit already speaks — but the hit says "you were hit" and this
    /// says WHICH power state you just lost, which the hit cannot.
    #[test]
    fn each_form_transition_voices_its_own_cue() {
        use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxId, SfxMessage};

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
        app.add_message::<OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        // Every cue this frame voiced, in order.
        let voiced = |app: &mut App| -> Vec<SfxId> {
            app.world_mut()
                .resource_mut::<bevy::ecs::message::Messages<OwnedSfxMessage>>()
                .drain()
                .filter_map(|m| match m.request {
                    SfxMessage::Play { id, .. } => Some(id),
                    _ => None,
                })
                .collect()
        };

        // small -> grown.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![star_wand()]));
        app.update();
        assert_eq!(
            voiced(&mut app),
            vec![SfxId::new(SFX_SMALL_TO_BIG)],
            "growing voices the small->big cue, exactly once"
        );

        // grown -> fire: a DIFFERENT cue, not a repeat of the growth.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![cinder_beacon()]));
        app.update();
        assert_eq!(
            voiced(&mut app),
            vec![SfxId::new(SFX_BIG_TO_FIRE)],
            "gaining fire is its own sound, not the growth chime again"
        );

        // fire -> grown (a hit spends the beacon): the loss names what was lost.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            voiced(&mut app),
            vec![SfxId::new(SFX_FIRE_TO_BIG)],
            "losing the beacon voices the fire->big reversion"
        );

        // grown -> small: the last rung down.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            voiced(&mut app),
            vec![SfxId::new(SFX_BIG_TO_SMALL)],
            "and losing the wand voices the big->small reversion"
        );

        // Standing still is silent — the cue is the CHANGE, not the state.
        app.update();
        assert!(voiced(&mut app).is_empty(), "no change, no voice");
    }

    /// A head-bonk on a ?-block pops exactly one wand, matched by the block's
    /// durable `GeoId` on the contact — and a spent block never pops again.
    #[test]
    fn a_head_bonk_on_a_power_block_pops_one_wand_once() {
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
        let wand = |app: &mut App| {
            app.world_mut()
                .query::<&WorldItem>()
                .iter(app.world())
                .count()
        };
        assert_eq!(wand(&mut app), 1, "one bonk pops exactly one wand");
        // The same contact next frame must not re-pop: the block is spent.
        app.update();
        assert_eq!(wand(&mut app), 1, "a spent ?-block yields no more wand");
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
