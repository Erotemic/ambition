//! Super Mary-O's powerups — the equipment chain, authored as A3 data.
//!
//! The engine's `ambition_platformer2d:characters:equipment` module (A3) supplies the three
//! mechanisms — numeric modifiers, behavioral grants, on-hit armor — and this file just names
//! two rows that use them.
//!
//! Parody-original, like the rest of the demo (Q28): a "star wand" and a "spark
//! beacon", homage in role, not a copy.

use bevy::prelude::*;

use ambition_platformer2d::characters::brain::action_set::{ProjectileFlight, RangedActionSpec};
use ambition_platformer2d::characters::equipment::{
    EquipmentGrant, EquipmentRow, ModifierOp, ModifierScope, OnHit, ParamModifier, WornEquipment,
};

use crate::ldtk_vocabulary::{MaryOBlockContents, MaryOPickup};
use ambition_platformer2d::actors::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d::actors::items::{spawn_moving_world_item, ItemMotionPlan, WorldItem};
use ambition_platformer2d::characters::actor::WornCharacter;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::collision_semantics::{ContactKind, ContactSource};
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use ambition_platformer2d::sprite_sheet::character::CharacterAnim;
use ambition_platformer2d::world::rooms::RoomLoaded;

use crate::provider::MARY_O_CHARACTER_ID;
use crate::T;

/// The worn-character id of the GROWN form: a distinct SHEET
/// (`mary_o_v2_tall`), not a scaled copy of the small sheet. Wearing it is how
/// the powerup grows Mary-O; reverting to [`MARY_O_CHARACTER_ID`] shrinks her.
const TALL_CHARACTER_ID: &str = "mary_o_tall";

/// The worn-character id of the FIRE form (the fire-flower analog). A distinct SHEET
/// (`mary_o_v2_fire`) with its own fireball pose, tinted the classic white-and-red fire palette
/// — the SAME height as the grown form, so the spark beacon changes her LOOK + spark loadout
/// without a size flicker. Wearing the [`CINDER_BEACON_ID`] row selects this; losing the spark
/// reverts to [`TALL_CHARACTER_ID`] (grown), then a second hit to [`MARY_O_CHARACTER_ID`]
/// (small).
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

/// The star wand: one-hit armor, the classic first powerup's take-a-hit half.
///
/// It is pure A3 [`OnHit::ConsumeAsArmor`] with `downgrade_to: None`: worn, it
/// absorbs one hit and is spent (removed); the very next read finds no wand and the
/// hit would reach HP — "big → small on hit", as data, no write-back.
///
/// The GROWN look and size are NOT a modifier here: "small and tall have different
/// sprites", so growing swaps the worn identity to a distinct tall SHEET
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

/// The cinder beacon: a ranged verb AND the outer layer of armor.
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
                .with_half_extent(ae::Vec2::new(SPARK_HALF_EXTENT, SPARK_HALF_EXTENT)),
        )
        .with_visual(SPARK_VISUAL)
}

/// Half-extent of a spark, in px — so the shot is 20 px across a 32 px tile.
///
/// This is a FEEL number and the obvious one to tune; what matters structurally is that it is the
/// ONE place the size is written. The render art asks for `ProjectileRenderSize::Body`, so the
/// drawn quad follows this half-extent and the sprite cannot disagree with the hitbox.
const SPARK_HALF_EXTENT: f32 = 10.0;

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
        (
            Entity,
            &ambition_platformer2d::projectiles::ProjectileVisualId,
        ),
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
///
/// `Clone` and a private set, both for the same reason. This is rollback
/// state — a block struck on a mispredicted frame must un-spend when that frame
/// is thrown away — and the registration needs the clone. The set is private so
/// the paragraph above stays TRUE rather than remaining a promise: `insert`,
/// `contains` and `clear` are the whole surface, and no caller can reach an
/// iteration whose order std does not define. ( of 5cc4337..47d7de3,
/// iterating at all is the stronger answer.)
///
/// its Sanic twin is a `Vec` and that asymmetry is CORRECT — do not
/// "fix" it. `ambition_demo_sanic::monitors::SpentMonitors` answers the same
/// question about the same kind of thing, and its own doc says why it is a Vec:
/// the overlay contribution ITERATES it every frame, and the determinism contract
/// bans std-hash iteration order. Nothing iterates this one. Two access patterns,
/// two shapes. and the dangerous direction of "making them consistent" is the
/// other one — turning `SpentMonitors` into a HashSet would reintroduce exactly
/// the order-dependence it was written to avoid.
#[derive(Resource, Default, Clone)]
pub struct SpentPowerBlocks {
    spent: std::collections::BTreeSet<ae::GeoId>,
    /// Hits taken by a multi-coin block that is not exhausted yet.
    ///
    /// An ordered container makes the question unaskable. a `Vec` would still be wrong — two peers'
    /// hashes would depend on strike ORDER.
    ///
    /// absent means untouched, not exhausted. The authority for "this
    /// block is done" stays `spent`: a partial entry is a block mid-payout, and
    /// the caller promotes it to `spent` when the count runs out. Two facts, one
    /// owner, and the older half keeps its exact meaning.
    partial: std::collections::BTreeMap<ae::GeoId, u8>,
}

impl SpentPowerBlocks {
    /// A checksum over WHICH blocks are spent, order-independent.
    ///
    /// XOR of per-id hashes is commutative, so the answer does not depend on
    /// the traversal — which was load-bearing while these were hash containers
    /// and is now belt-and-braces. Keep it: the property is free and it is what
    /// lets a future reader change the container without re-deriving safety.
    pub fn checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        // both halves, and the partial one folds its COUNT too — a block
        // that has paid two of five differs from one that has paid three, and a
        // checksum blind to that lets two peers disagree about how much a block
        // still owes while agreeing on the hash.
        let spent = self.spent.iter().fold(0u64, |acc, id| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut hasher);
            acc ^ hasher.finish()
        });
        self.partial.iter().fold(spent, |acc, (id, taken)| {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut hasher);
            taken.hash(&mut hasher);
            acc ^ hasher.finish()
        })
    }

    /// This block has already given up its pickup.
    pub fn is_spent(&self, id: &ae::GeoId) -> bool {
        self.spent.contains(id)
    }

    /// Record one payout from a multi-coin block and answer whether that was its
    /// LAST — in which case it is now spent.
    ///
    /// the promotion happens here rather than at the call site so "the counter
    /// reached zero" and "the block is spent" cannot disagree.
    pub fn take_one_coin(&mut self, id: &ae::GeoId, of: u8) -> bool {
        let taken = self.partial.entry(id.clone()).or_insert(0);
        *taken = taken.saturating_add(1);
        if *taken >= of.max(1) {
            self.partial.remove(id);
            self.spent.insert(id.clone());
            return true;
        }
        false
    }

    /// Coins this multi-coin block has already paid out.
    pub fn coins_taken(&self, id: &ae::GeoId) -> u8 {
        self.partial.get(id).copied().unwrap_or(0)
    }

    /// Record a block as spent. Idempotent.
    pub fn spend(&mut self, id: ae::GeoId) {
        self.partial.remove(&id);
        self.spent.insert(id);
    }

    /// Re-arm every block — a room (re)load, so a cyclic replay plays the same.
    pub fn rearm_all(&mut self) {
        self.spent.clear();
        self.partial.clear();
    }
}

// Mary-O uses sprite-authored body geometry. Each form supplies one world scale,
// and per-animation sheet body rectangles determine its collision and render
// footprint. Any desired gameplay inset belongs in the character generator,
// rather than in a second hand-authored collision box.

/// Standing height in world units. Pixel-to-world scale is derived from this
/// authored gameplay size so sprite regeneration cannot change level-scale body
/// dimensions.
pub(crate) const MARY_O_STANDING_HEIGHT: f32 = SMALL_FORM_HEIGHT;

/// Small-form standing height in the demo's world units.
pub const SMALL_FORM_HEIGHT: f32 = T;

/// Standing height shared by the grown and fire forms.
pub(crate) const GROWN_FORM_HEIGHT: f32 = 32.0;

/// Authored standing height for a Mary-O form. Fire shares the grown height so
/// changing loadout does not move the body or alter ceiling clearance.
pub(crate) fn form_height(target: &str) -> f32 {
    if target == SMALL_SHEET_TARGET {
        SMALL_FORM_HEIGHT
    } else {
        GROWN_FORM_HEIGHT
    }
}

/// World units per sheet pixel — ONE scale, shared by all three forms.
///
/// So a per-form scale reaching 32 has to draw the grown art 1.43x larger per pixel — and since the
/// sheets author ONE body width for all three forms, that widens her by the same 1.43x on the way
/// up. `her_forms_are_all_the_same_width` caught it immediately, and its reason is a gameplay rule
/// rather than a tidiness one: *"growing must not change her width"*, or a grow wedges her in a gap
/// she fit.
///
///  the 1:2 proportion is an ART question, not an arithmetic one. Reaching
/// grown = 32 at an unchanged width needs the grown sheet REDRAWN to twice the
/// small form's pixel height; scaling to it distorts her or fattens her. Until
/// then one shared scale keeps her proportions honest and grown lands where the
/// art puts it (22.3 at a 16-unit small form).
pub(crate) fn form_world_per_pixel(_target: &str) -> f32 {
    mary_o_world_per_pixel()
}

/// World units per sheet pixel, asked of the art rather than remembered.
///
/// Everything else — the grown form's height, all three widths, the sprite quad — follows from this
/// one number, so none of them can drift from each other either.
///
/// `posed_body_geometry` at a scale of 1.0 returns the bbox in PIXELS, which is
/// why there is no second registry lookup here: this asks the same function the
/// engine's per-tick sync asks, so the two cannot disagree about what the sheet
/// says.
pub(crate) fn mary_o_world_per_pixel() -> f32 {
    match small_form_pixel_height() {
        Some(pixels) => MARY_O_STANDING_HEIGHT / pixels,
        // No baked art (a headless fixture). Any scale is arbitrary here because
        // nothing will resolve a body from it; 1.0 keeps the arithmetic honest
        // instead of inventing a plausible-looking number.
        None => 1.0,
    }
}

/// How WIDE Mary-O's small form stands, in world units — the ruler every
/// other body in this demo is sized against.
///
/// Her HEIGHT is the authored number ([`MARY_O_STANDING_HEIGHT`]); her width
/// follows from the sheet, so this is a measurement rather than a second claim
/// and the two cannot drift apart.
///
/// it exists because "too big" is a comparison and needed a denominator.
/// attempt before this expressed the answer as a pixel scale — a unit in which
/// the comparison cannot be stated at all.
pub(crate) fn mary_o_body_width() -> Option<f32> {
    ambition_platformer2d::character_sprites::posed_body_geometry(
        SMALL_SHEET_TARGET,
        CharacterAnim::Idle,
        1.0,
    )
    .filter(|geometry| geometry.collision.x > 0.0 && geometry.collision.y > 0.0)
    .map(|geometry| geometry.collision.x * (MARY_O_STANDING_HEIGHT / geometry.collision.y))
}

/// The small form's body rectangle height in SHEET PIXELS, or `None` when no
/// record is baked. Separated so a test can ask whether the art resolved at all
/// — the scale above cannot report that, since its fallback is a real number.
pub(crate) fn small_form_pixel_height() -> Option<f32> {
    ambition_platformer2d::character_sprites::posed_body_geometry(
        SMALL_SHEET_TARGET,
        CharacterAnim::Idle,
        1.0,
    )
    .map(|geometry| geometry.collision.y)
    .filter(|pixels| *pixels > 0.0)
}

/// The sheet manifest targets her three forms resolve through. Named here
/// because both her definitions (which author the bodies) and the level
/// authoring (which asks how tall she gets) need the same strings.
pub(crate) const SMALL_SHEET_TARGET: &str = "mary_o_v2";
pub(crate) const TALL_SHEET_TARGET: &str = "mary_o_v2_tall";
pub(crate) const FIRE_SHEET_TARGET: &str = "mary_o_v2_fire";

/// The standing box one of her sheets authors, in world units.
///
/// The level asks this to size the clearances she has to fit through (pipe
/// mouths, vault exits). It is the same query the engine's per-tick sync makes,
/// so a gap authored from it and the body that walks into it cannot disagree.
///
/// Falls back to the engine default when the sheet registry has no record — a
/// headless fixture that baked no art. A clearance authored against the default
/// is wrong by a few pixels; refusing to author one at all would be a panic in a
/// test that never intended to draw anything.
pub(crate) fn form_body_size(target: &str) -> ae::Vec2 {
    ambition_platformer2d::character_sprites::posed_body_geometry(
        target,
        CharacterAnim::Idle,
        form_world_per_pixel(target),
    )
    .map(|geometry| geometry.collision)
    .unwrap_or_else(ae::movement::default_player_body_size)
}

/// How tall she gets — asked of the grown form's ART, not multiplied out of the
/// small one.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn tall_body_size() -> ae::Vec2 {
    form_body_size(TALL_SHEET_TARGET)
}

/// A weaker form on the FLOOR is refused too, not just one from a block.
///
/// Correct, and the overreach was in my prose rather than in the engine.)
///
/// so the rule moves to the COLLECTION, which is the one thing every road
/// passes through. This runs before the engine's collector and consumes a
/// redundant form item itself: she keeps the stronger form, the pickup still
/// disappears, and the coins are the acknowledgement — the same trade the block
/// payout makes, for the same reason.
///
/// the ranking stays Mary-O's. The engine's exclusive-slot replacement is
/// correct and general; *"a weaker form may not replace a stronger one"* is a
/// statement about this game's progression.
pub fn refuse_a_weaker_form_pickup(
    mut commands: Commands,
    items: Query<(Entity, &ambition_platformer2d::actors::items::WorldItem)>,
    mut players: Query<
        (
            &ae::BodyKinematics,
            Option<&WornEquipment>,
            Option<&mut ambition_platformer2d::characters::actor::BodyWallet>,
        ),
        With<PrimaryPlayer>,
    >,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
) {
    let Ok((body, worn, mut wallet)) = players.single_mut() else {
        return;
    };
    use ae::AabbExt as _;
    let body_aabb = ae::Aabb::new(body.pos, body.size * 0.5);
    let rank = worn_form_rank(worn);
    for (entity, item) in &items {
        if !body_aabb.strict_intersects(item.aabb()) {
            continue;
        }
        let ambition_platformer2d::actors::items::WorldItemPayload::Equip(row) = &item.payload;
        let is_form = row
            .exclusive_slot
            .as_deref()
            .is_some_and(|slot| slot == FORM_SLOT);
        if !is_form || form_rank(&row.id) > rank {
            continue;
        }
        if let Some(purse) = wallet.as_mut() {
            purse.add(COINS_PER_BLOCK);
        }
        sfx.write_from(
            crate::provider::MARY_O_EXPERIENCE,
            ambition_platformer2d::sfx::SfxMessage::Hit { pos: item.pos },
        );
        commands.entity(entity).despawn();
    }
}

/// Project discovered hidden blocks into the collision overlay.
///
/// `SpentPowerBlocks` is the shared authority for presentation and collision.
/// A spent `BonkOnly` block replaces its authored intangible form with a solid at
/// the same AABB; spent Question and Brick blocks remain authored because they
/// were already solid.
pub fn contribute_discovered_hidden_blocks_to_overlay(
    spent: Res<SpentPowerBlocks>,
    geometry: Option<
        ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
    >,
    mut overlay: ResMut<ambition_platformer2d::actors::features::FeatureEcsWorldOverlay>,
) {
    let Some(geometry) = geometry else {
        return;
    };
    for block in &geometry.0.blocks {
        if let Some(solid) = discovered_solid(&spent, block) {
            overlay.removed_block_names.push(block.name.clone());
            overlay.blocks.push(solid);
        }
    }
}

/// Return the solid replacement for a discovered hidden block, or `None` when
/// the authored block should remain unchanged.
pub fn discovered_solid(spent: &SpentPowerBlocks, block: &ae::Block) -> Option<ae::Block> {
    // HIDDEN only. A spent Question or Brick was always authored solid, and
    // re-adding it would put two blocks in one place.
    if block.kind != ae::BlockKind::BonkOnly || !spent.is_spent(&block.id) {
        return None;
    }
    Some(ae::Block {
        kind: ae::BlockKind::Solid,
        ..block.clone()
    })
}

pub fn bonk_power_blocks(
    mut commands: Commands,
    mut spent: ResMut<SpentPowerBlocks>,
    mut struck: bevy::prelude::MessageWriter<
        ambition_platformer2d::platformer::block_nudge::BlockStruck,
    >,
    // The coin a struck coin block visibly pays. Presentation only — the purse
    // is credited below whether or not anything is drawing.
    mut vfx: bevy::prelude::MessageWriter<ambition_platformer2d::vfx::VfxMessage>,
    // the WALLET rides the same query, because a coin block credits the body that struck it
    // rather than a global counter — the same component the vault's loose coins credit and the
    // same one the HUD's COINS readout is rebuilt from.
    mut players: Query<
        (
            &PlayerBodyFrameOutput,
            Option<&WornEquipment>,
            Option<&mut ambition_platformer2d::characters::actor::BodyWallet>,
        ),
        With<PrimaryPlayer>,
    >,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    // The room the contact happened in — a `GeoId` names a block, and only the
    // world can say WHICH block that is.
    geometry: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<ae::RoomGeometry>,
) {
    let Ok((frame, worn, mut wallet)) = players.single_mut() else {
        return;
    };
    for contact in &frame.events.contacts {
        if contact.kind != ContactKind::Head {
            continue;
        }
        let ContactSource::Block { id, .. } = &contact.source else {
            continue;
        };
        // The room answers now, and the answer is the KIND the author picked.
        let Some(block) = crate::authored_block_by_id(&geometry.0, id) else {
            continue;
        };
        // WHAT IT HOLDS, not what it looks like. This matched the block's
        // KIND — `Power` meant the ladder, `Quasar` meant a quasar — which made
        // unsayable, because appearance was the only thing carrying the answer.
        //
        // A brick that holds something now arrives here like any other reactive
        // block; a brick that holds nothing falls through to
        // `bricks::break_bricks`, which is what breaks it.
        let Some(authored) = crate::ldtk_vocabulary::block_of(&block.name) else {
            continue;
        };
        if authored.contents.is_empty() {
            continue;
        }
        let block_aabb = block.aabb;
        if spent.is_spent(id) {
            continue;
        }
        // A VALID BONK IS ALWAYS ACKNOWLEDGED. This used to read `let Some(reward) = … else
        // { continue }`, and `next_power_reward` answered `None` at the top of the ladder — so
        // a FIRE-form Mary-O hit a ?-block and got nothing at all: no flinch, no spent state,
        // no art change, no sound.
        let reward = reward_for(authored.contents, worn);
        // a multi-coin block is spent by its COUNTER, not by being hit.
        // Every other block owes one payout and retires on the strike; this one
        // owes N, so `take_one_coin` promotes it to spent on the last of them.
        // reset."* `rearm_all` clears both halves, so "until reset" is the
        // room reload that already re-arms every other block.
        match authored.contents {
            MaryOBlockContents::Coins(count) => {
                spent.take_one_coin(&id, count);
            }
            _ => spent.spend(id.clone()),
        }
        // Used blocks flinch in presentation only; moving collision geometry would lift bodies
        // standing on the block. This records which block was struck.
        struck.write(ambition_platformer2d::platformer::block_nudge::BlockStruck::new(id.clone()));
        // from the block's OWN centre, so a block the author dragged pops its reward where
        // it now sits.
        let pos = (block_aabb.min + block_aabb.max) * 0.5;
        let reward = match reward {
            None => continue,
            Some(BlockPayout::Coins(amount)) => {
                if let Some(purse) = wallet.as_mut() {
                    purse.add(amount);
                }
                // into it."* One coin per payout, launched from the block's top
                // face so it reads as coming OUT rather than through.
                vfx.write(ambition_platformer2d::vfx::VfxMessage::CoinPop {
                    pos: ae::Vec2::new(pos.x, block_aabb.min.y),
                });
                // this was the `Hit` cue — the MASONRY THUNK — and the
                // comment justifying it went stale. It read *"there is no
                // `Pickup` cue in the shared vocabulary yet"*, which was true
                // when written and is not now: the engine emits
                // `ids::WORLD_COIN_PICKUP` for every currency pickup, and
                // Mary-O's provider declares it (`COIN_PICKUP_SFX`).
                //
                // so a coin sounds like a coin whichever way she gets it.
                // Her loose coins are `currency:1` pickups voiced by the engine's
                // `collect_ecs_pickups`; a coin BLOCK never builds a pickup at
                // all (it credits the purse directly, three lines up), so it has
                // to name the cue itself. Same id, same declaration, one sound —
                // without this, popping a block for a coin played a brick
                // smashing.
                sfx.write_from(
                    crate::provider::MARY_O_EXPERIENCE,
                    ambition_platformer2d::sfx::SfxMessage::Play {
                        id: ambition_platformer2d::sfx::SfxId::from_static(
                            crate::provider::COIN_PICKUP_SFX,
                        ),
                        pos,
                    },
                );
                continue;
            }
            Some(BlockPayout::Item(reward)) => *reward,
        };
        let popped = spawn_moving_world_item(
            &mut commands,
            // it starts INSIDE the block and climbs out. Spawned at the block's own centre
            // rather than above it, so the first frame shows nothing and the pickup rises into view
            // through the block's top edge.
            //
            // being drawn BEHIND the world is not set here. It is derived from the motion's own
            // emergence window, so it ends exactly when the rise does.
            WorldItem::equipping(reward.row, pos, reward.half).with_sprite(reward.sprite),
            reward.motion,
        );
        // `spawn_moving_world_item` scopes to the ROOM, which answers *does this
        // survive leaving* and not *does this survive REPLAYING* —
        // `SpawnedThisAttempt`'s own doc says one scope cannot answer both. A wand
        // this attempt knocked out of a block is residue of an attempt that is
        // about to be un-played: the block un-spends on reset and will pop
        // another, so the old one has to go or the room accumulates them.
        //
        // marked HERE rather than inside the engine helper, because only the
        // caller knows an item was POPPED rather than authored into the level. If
        // a second caller ever needs the same thing, that is the signal to move it.
        commands
            .entity(popped)
            .insert(ambition_platformer2d::actors::features::ecs::SpawnedThisAttempt);
    }
}

/// What a block owes, given what it holds and what she wears.
///
/// Contents is an authored field now, and this is the one place that turns it into an item.
///
/// this is the difference the ONE-enum version could not hold. `PowerReward`
/// describes an item — a row, a half-extent, a sprite and a motion plan — and a
/// coin has none of those, so expressing "credit her instead" as a `PowerReward`
/// would have meant inventing a fake item and then teaching the spawner to skip
/// it. A block owes either a thing or a number.
enum BlockPayout {
    /// An item, spawned inside the block and rising out of it.
    Item(Box<PowerReward>),
    /// Coins, credited on the strike. No entity is spawned and nothing has to be
    /// caught: the block flinches, the cue plays, the counter moves.
    Coins(i32),
}

/// How much one coin block is worth. One, like the coins lying in the vault —
/// `currency:1` in the level file.
const COINS_PER_BLOCK: i32 = 1;

/// How strong a form this equipment row is. Small is 0 and wears nothing.
///
/// the ladder is a Mary-O rule and lives in Mary-O. The engine's exclusive-slot replacement
/// is correct and general — a new row in a slot replaces the old one — and "a weaker form may
/// not replace a stronger one" is a statement about THIS game's progression, not about
/// equipment.
fn form_rank(row_id: &str) -> u8 {
    match row_id {
        id if id == CINDER_BEACON_ID => 2,
        id if id == STAR_WAND_ID => 1,
        _ => 0,
    }
}

/// The rank of the form she is in now.
fn worn_form_rank(worn: Option<&WornEquipment>) -> u8 {
    let Some(worn) = worn else {
        return 0;
    };
    if worn.wears(CINDER_BEACON_ID) {
        2
    } else if worn.wears(STAR_WAND_ID) {
        1
    } else {
        0
    }
}

/// Is she SMALL? — the bottom rung, wearing no form row at all.
///
/// Only tall or fire should be able to."* That is a statement about her FORM, and the ladder that
/// knows what a form IS lives here — so [`crate::bricks`] asks this question rather than
/// re-deriving it from two equipment ids it would then have to keep in step with [`worn_form_rank`]
/// by hand.
///
/// "not small" means TALL OR FIRE, which is not the same as "wears the
/// wand". The beacon is worn ALONE at the top of the ladder — it downgrades
/// INTO the wand on a hit rather than stacking on top of it — so a caller that
/// asked `wears(STAR_WAND_ID)` would answer "small" for the strongest form in
/// the game. Reading the rank is what makes that mistake unsayable.
///
/// the quasar is not a form and does not count. `pocket_quasar` wears no
/// [`FORM_SLOT`], so a small Mary-O carrying one is still small here. That is
/// the classic behaviour and it falls out of the slot rather than out of a list
/// of ids this function would otherwise have to exclude.
pub(crate) fn is_small(worn: Option<&WornEquipment>) -> bool {
    worn_form_rank(worn) == 0
}

/// A pickup never makes her weaker.
///
/// 1-2 authors a `Brick` holding `AlwaysWand`, and fire Mary-O bonking it
/// became TALL. The wand and the beacon share one exclusive slot, so generic
/// replacement did exactly what it says on the tin and the form went down a
/// rung.
///
/// The rule, monotonic in both directions:
///
/// ```text
/// small + wand    -> tall      small + lantern -> fire
/// tall  + wand    -> tall      tall  + lantern -> fire
/// fire  + wand    -> fire      fire  + lantern -> fire
/// ```
///
/// a redundant pickup is still CONSUMED and still pays. The block spends,
/// flinches, wears its used art and hands over coins — it does not go
/// unresponsive, which is the failure the always-acknowledge rule above exists
/// to prevent.
///
/// this function guards the BLOCK PAYOUT only, and for a day its comment
/// claimed the whole rule. The floor is guarded by
/// [`refuse_a_weaker_form_pickup`], which runs before the engine's collector;
/// between them the invariant is true of every road a form item can arrive on.
fn without_downgrading(reward: PowerReward, worn: Option<&WornEquipment>) -> BlockPayout {
    let is_form = reward
        .row
        .exclusive_slot
        .as_deref()
        .is_some_and(|slot| slot == FORM_SLOT);
    if is_form && form_rank(&reward.row.id) <= worn_form_rank(worn) {
        return BlockPayout::Coins(COINS_PER_BLOCK);
    }
    BlockPayout::Item(Box::new(reward))
}

/// The payout a block with these contents owes her, or `None` when it owes
/// nothing at all.
fn reward_for(contents: MaryOBlockContents, worn: Option<&WornEquipment>) -> Option<BlockPayout> {
    Some(match contents {
        // A block holding nothing owes nothing. The caller filters these out
        // (a Brick with no contents BREAKS rather than paying), and saying so
        // here means a caller that forgets gets no reward instead of the best one.
        MaryOBlockContents::Empty => return None,
        // Now the reward builders say `None` for a coin and this is the single place that turns
        // that into coins. levelling TOWARD a coin is still just a coin: a coin is not on the
        // ladder for the same reason the quasar is not — it is not a form.
        MaryOBlockContents::Always(pickup) => match pickup_reward(pickup) {
            Some(reward) => without_downgrading(reward, worn),
            None => BlockPayout::Coins(COINS_PER_BLOCK),
        },
        MaryOBlockContents::Toward(pickup) => match next_rung_toward(pickup, worn) {
            Some(reward) => without_downgrading(reward, worn),
            None => BlockPayout::Coins(COINS_PER_BLOCK),
        },
        // one coin per HIT, whatever the authored count. The count says
        // how many hits the block has left, not how much a single hit is worth
        // the caller's, because only it knows which block was struck.
        MaryOBlockContents::Coins(_) => BlockPayout::Coins(COINS_PER_BLOCK),
    })
}

/// One pickup, built exactly as it is when it comes out of a block.
///
/// A coin genuinely has no ITEM to build, and a caller that forgot the routing above got the most
/// powerful reward in the game, silently.
fn pickup_reward(pickup: MaryOPickup) -> Option<PowerReward> {
    match pickup {
        MaryOPickup::Wand => Some(wand_reward()),
        MaryOPickup::Lantern => Some(beacon_reward()),
        MaryOPickup::Quasar => Some(quasar_reward()),
        // A coin is not an item. The caller turns this into `BlockPayout::Coins`.
        MaryOPickup::Coin => None,
    }
}

/// The next rung on the way to `target`, given the form she is in.
///
/// the ladder is small and explicit rather than a table, because the rungs are
/// not interchangeable: the quasar is not on it at all (any form takes one), and
/// the top rung REPEATS rather than answering nothing — see [`next_power_reward`]
/// for why that matters.
fn next_rung_toward(target: MaryOPickup, worn: Option<&WornEquipment>) -> Option<PowerReward> {
    let wears = |id: &str| worn.is_some_and(|w| w.wears(id));
    match target {
        // Levelling toward the quasar is levelling toward something off the
        // ladder, so it is just the quasar.
        MaryOPickup::Quasar => Some(quasar_reward()),
        // `None`, not a quasar. Same fix as `pickup_reward`: a coin is off
        // the ladder and has no rung to answer with, and answering the strongest
        // reward in the game for an impossible branch hides the caller's bug
        // instead of surfacing it.
        MaryOPickup::Coin => None,
        // Toward the wand: she gets the wand until she has it, then it repeats.
        MaryOPickup::Wand => Some(wand_reward()),
        // Toward the lantern: the full classic progression.
        MaryOPickup::Lantern => Some(if wears(STAR_WAND_ID) || wears(CINDER_BEACON_ID) {
            beacon_reward()
        } else {
            wand_reward()
        }),
    }
}

/// The quasar, which any Mary-O can collect at any power tier.
fn quasar_reward() -> PowerReward {
    PowerReward {
        row: crate::star::pocket_quasar(),
        half: crate::star::QUASAR_HALF,
        sprite: crate::star::QUASAR_SPRITE,
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
    /// How it behaves once it is out of the block — the engine steps this
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
fn wand_reward() -> PowerReward {
    PowerReward {
        row: star_wand(),
        half: STAR_WAND_HALF,
        sprite: STAR_WAND_SPRITE,
        // The wand WALKS and turns at walls, like the mushroom.
        motion: rises_from_a_block(ItemMotionPlan::walker(WAND_SPEED), STAR_WAND_HALF.y),
    }
}

/// The cinder beacon.
///
/// the top rung REPEATS; it does not answer NOTHING. Answering `None`
/// meant a fully-powered Mary-O bonked a block and the whole hit was swallowed —
/// no flinch, no spend, no art change. A player
/// cannot tell "you are already maxed" from "a block that does not work", and
/// only one of those is true.
///
/// another beacon is the placeholder the spec allows, not a design decision:
/// *"spawning another lantern is acceptable unless the existing design clearly
/// prefers score, a reserve item, or another reward."* Score or a reserve slot
/// is the classic answer and neither exists yet.
fn beacon_reward() -> PowerReward {
    PowerReward {
        row: cinder_beacon(),
        half: CINDER_BEACON_HALF,
        sprite: CINDER_BEACON_SPRITE,
        // The beacon waits on its block, like the classic flower.
        motion: rises_from_a_block(ItemMotionPlan::still(), CINDER_BEACON_HALF.y),
    }
}

/// Grown = wearing the wand. The tall sheet and the taller collider are a pure
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
            &ae::BodyKinematics,
            Option<&WornEquipment>,
        ),
        With<PrimaryPlayer>,
    >,
    mut sfx: ambition_platformer2d::sfx::BodySfxWriter,
    mut commands: bevy::prelude::Commands,
) {
    let Ok((body, mut worn_char, kin, worn)) = players.single_mut() else {
        return;
    };
    // THREE forms, chosen from what she wears. The fire (beacon) and grown (wand)
    // forms are the SAME height — the beacon downgrades INTO the wand on a hit, so
    // across that transition she stays continuously tall and only her look + spark
    // loadout change; the size flickers on neither the grow nor the spark→grown
    // downgrade, only on the final grown→small hit.
    let target_id = if worn.is_some_and(|w| w.wears(CINDER_BEACON_ID)) {
        SPARK_CHARACTER_ID
    } else if worn.is_some_and(|w| w.wears(STAR_WAND_ID)) {
        TALL_CHARACTER_ID
    } else {
        MARY_O_CHARACTER_ID
    };
    if worn_char.id() == target_id {
        return;
    }
    // No size is written here, and that is the fix.
    //
    // Her forms now author `SpriteAuthored` bodies (see [`MARY_O_WORLD_PER_PIXEL`]), so swapping
    // the identity below is the WHOLE change: the engine's per-tick sync reads the arriving sheet
    // and resizes her feet-planted through the one resize op that owns that rule. Growing is a
    // consequence of the art, not an instruction.
    let previous_id = worn_char.id().to_string();
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
    // The transformation MOMENT, in BOTH directions. The shrink clip is about the form, so it gets
    // the same beat the growth does. Same shape as the cue above: the transition picks it.
    commands.entity(body).try_insert((
        transform_beat_policy(target_id, power_tier(&previous_id), power_tier(target_id)),
        ambition_platformer2d::actors::features::transform_beat::TransformBeatRequested,
    ));
    worn_char.0 = target_id.into();
    commands
        .entity(body)
        .try_insert(ambition_platformer2d::characters::actor::RecharacterizeBody);
}

/// The transformation numbers for ONE tier change, authored per transition
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

const STEP_UP_CLOCK_SCALE: f32 = 0.6;

/// Fallback beat length for a form whose sheet cannot be read at all.
const UNREADABLE_CLIP_SECS: f32 = 0.45;

/// Which clip shows this tier change. A transition clip is authored on the
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
        SPARK_CHARACTER_ID => FIRE_SHEET_TARGET,
        TALL_CHARACTER_ID => TALL_SHEET_TARGET,
        _ => SMALL_SHEET_TARGET,
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
/// Dress her bonus blocks so a player can see which ones still hold something.
///
/// `BlockArt` is that seam: this attaches it to a LIVE block and removes it from a spent one, which
/// falls back to the kind's plain tile — exactly the block a used one becomes, so the used state
/// needs no art of its own.
///
/// it reads `SpentPowerBlocks` every frame rather than reacting to the bonk.
/// That set is ROLLBACK STATE: a block struck on a mispredicted frame un-spends
/// when the frame is thrown away, and art driven by the EVENT would keep the used
/// look through a rewind that undid the strike. Deriving from the state cannot
/// drift from it — the same reasoning `SpentPowerBlocks`' own doc gives for being
/// `Clone`.
pub fn dress_power_blocks(
    mut commands: Commands,
    spent: Res<SpentPowerBlocks>,
    blocks: Query<(
        Entity,
        &ambition_platformer2d::render::rendering::BlockVisual,
        Option<&ambition_platformer2d::render::rendering::BlockArt>,
    )>,
) {
    use ambition_platformer2d::actors::assets::game_assets::EntitySprite;
    use ambition_platformer2d::render::rendering::BlockArt;
    for (entity, visual, art) in &blocks {
        // the block's own NAME says what it is. This asked two index tables whether the id
        // matched a constant column; a block the author dragged answered neither, so it drew as
        // plain masonry however clearly it was marked a ?-block.
        use crate::ldtk_vocabulary::MaryOBlockLook;
        let look = crate::ldtk_vocabulary::block_look_of(&visual.block_name);
        let is_spent = spent.is_spent(&visual.geo_id);
        // a HIDDEN block wears nothing until it has paid. It is drawn
        // transparent at room build (`dress_authored_blocks`), and the only art
        // it ever gets is the spent tile — so striking one reveals it, which is
        // exactly the beat. A `Brick` is still not dressed at all: its look is
        // the room's, not this dresser's.
        let want = match look {
            Some(MaryOBlockLook::Question) => Some(BlockArt(if is_spent {
                EntitySprite::SpentBlockTile
            } else {
                EntitySprite::BonusBlockTile
            })),
            Some(MaryOBlockLook::Hidden) if is_spent => {
                Some(BlockArt(EntitySprite::SpentBlockTile))
            }
            // Unspent bricks keep room-authored art/color so they remain visually
            // indistinguishable from masonry. Once a brick pays out, give it the
            // spent-block texture; empty bricks shatter instead of reaching `spent`.
            Some(MaryOBlockLook::Brick) if is_spent => Some(BlockArt(EntitySprite::SpentBlockTile)),
            Some(MaryOBlockLook::Brick) => None,
            _ => None,
        };
        let Some(want) = want else {
            continue;
        };
        // A spent block keeps an explicit spent texture rather than falling back
        // to its block kind.
        if art != Some(&want) {
            commands.entity(entity).insert(want);
        }
    }
}

/// Re-arm every power block when its room loads or replays.
pub fn rearm_power_blocks_for_a_fresh_attempt(
    mut rooms: MessageReader<RoomLoaded>,
    mut replays: MessageReader<ambition_platformer2d::actors::session::reset::RoomReplayRequested>,
    mut spent: ResMut<SpentPowerBlocks>,
) {
    let reloaded = rooms.read().count() > 0;
    let replayed = replays.read().count() > 0;
    if reloaded || replayed {
        spent.rearm_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::characters::equipment::{
        apply_equipment_grants, resolved_ranged, WornEquipment,
    };

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
        assert_eq!(
            power_transition_sfx(MARY_O_CHARACTER_ID, SPARK_CHARACTER_ID),
            None
        );
    }

    #[test]
    fn the_star_wand_absorbs_one_hit_then_is_spent() {
        let mut worn = WornEquipment::new(vec![star_wand()]);
        assert!(worn.wears(STAR_WAND_ID), "worn, so she reads as grown");

        // A hit spends the wand...
        assert_eq!(worn.consume_armor().as_deref(), Some(STAR_WAND_ID));
        // ...and the wand is gone on the next read (no write-back), so she'll shrink.
        assert!(
            !worn.wears(STAR_WAND_ID),
            "losing the wand reverts to small"
        );
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

    /// The progression, as equipment. small -> wand -> grown -> beacon ->
    /// spark-powered, with no rung repeatable and no parallel flag.
    #[test]
    fn the_power_block_reward_climbs_the_ladder_and_never_duplicates() {
        // Small: the wand.
        let bare = WornEquipment::default();
        // the `Option` here is the COIN case and nothing else — a rung toward a
        // lantern always exists. `expect` rather than a match, so a `None` that
        // ever appears on this road is a failure with a name.
        let rung = |worn| {
            next_rung_toward(MaryOPickup::Lantern, worn)
                .expect("a lantern always has a next rung")
                .row
                .id
        };
        assert_eq!(rung(None), STAR_WAND_ID, "small Mary-O is offered the wand");
        assert_eq!(
            rung(Some(&bare)),
            STAR_WAND_ID,
            "an empty worn set reads as small too"
        );

        // Grown: the beacon.
        let grown = WornEquipment::new(vec![star_wand()]);
        assert_eq!(
            rung(Some(&grown)),
            CINDER_BEACON_ID,
            "grown Mary-O is offered the beacon"
        );

        // A player cannot tell "already maxed" from "this block is broken", and only one of those
        // is true. The quasar is NOT the top rung; the beacon repeats until there is a score or a
        // reserve slot to give instead.
        let sparked = WornEquipment::new(vec![cinder_beacon()]);
        assert_eq!(
            rung(Some(&sparked)),
            CINDER_BEACON_ID,
            "a fully powered Mary-O still gets an acknowledged hit"
        );
    }

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

        // And its blocks are a family of their own: no ?-block reads as a quasar
        // block and no quasar block reads as a ?-block. That disjointness is what
        // lets ONE bonk rule serve both without either shadowing the other.
        use crate::ldtk_vocabulary::{block_look_of, MaryOBlockLook};
        let room = crate::level_1_1();
        let of_kind = |want: MaryOBlockLook| {
            room.world
                .blocks
                .iter()
                .filter(|b| block_look_of(&b.name) == Some(want))
                .count()
        };
        assert!(
            of_kind(MaryOBlockLook::Question) > 0,
            "the level authors ?-blocks"
        );
        for block in &room.world.blocks {
            let kinds = [
                MaryOBlockLook::Question,
                MaryOBlockLook::Brick,
                MaryOBlockLook::Hidden,
            ]
            .into_iter()
            .filter(|k| block_look_of(&block.name) == Some(*k))
            .count();
            assert!(
                kinds <= 1,
                "`{}` reads as more than one kind of reactive block",
                block.name
            );
        }
    }

    /// Damage walks the ladder back down, one rung per hit, through the
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

    /// Her forms are as big as their ART says, and this is the whole of the
    /// content claim now that the sheets author her boxes.
    ///
    /// It reads the real baked manifests, so it fails if the generator re-crops
    /// her relative proportions — which is the point: a hand-authored `× 1.5`
    /// could not notice, and did not (the sheets' real ratio is 1.397).
    ///
    /// deliberately NOT asserting `small.y == 48`. That was true by
    /// construction the moment the scale started deriving FROM the height, so it
    /// was a check that could not fail. The scale resolving from real art at all
    /// is the precondition that can, and it is asserted separately below.
    #[test]
    fn her_forms_boxes_come_from_their_sheets() {
        let small = form_body_size(SMALL_SHEET_TARGET);
        let tall = form_body_size(TALL_SHEET_TARGET);
        let fire = form_body_size(FIRE_SHEET_TARGET);

        assert!(
            tall.y > small.y,
            "growing makes her taller because the tall sheet's art IS taller \
             (small {small:?}, tall {tall:?})"
        );
        assert!(
            (fire.y - tall.y).abs() < 1e-3,
            "the fire form stands the same HEIGHT as the grown one — the beacon \
             downgrades INTO the wand, and a height change on that swap would \
             move her feet or clip a ceiling (tall {tall:?}, fire {fire:?})"
        );
    }

    /// Her scale is derived from art that actually resolved.
    ///
    /// [`mary_o_world_per_pixel`] falls back to `1.0` when no sheet is baked, and a fallback that
    /// returns a real number cannot report its own absence — every size downstream would be
    /// plausible and wrong.
    #[test]
    fn her_scale_is_derived_from_baked_art_not_from_the_fallback() {
        let pixels = small_form_pixel_height()
            .expect("her small sheet must publish a body rectangle to scale against");
        assert!(
            (mary_o_world_per_pixel() - MARY_O_STANDING_HEIGHT / pixels).abs() < 1e-6,
            "the scale is the authored height over the MEASURED pixel height, so \
             a regeneration that re-crops her keeps her exactly as tall as the \
             level expects"
        );
    }

    /// Every form is exactly as WIDE as every other.
    ///
    /// Growing and catching fire change her height and her look, never how wide a gap she fits. The
    /// sheets now author one body width for all three, and this is the check that they still agree
    /// after a regeneration.
    #[test]
    fn her_forms_are_all_the_same_width() {
        let small = form_body_size(SMALL_SHEET_TARGET);
        let tall = form_body_size(TALL_SHEET_TARGET);
        let fire = form_body_size(FIRE_SHEET_TARGET);

        assert!(
            (tall.x - small.x).abs() < 1e-3,
            "growing must not change her width (small {small:?}, tall {tall:?})"
        );
        assert!(
            (fire.x - small.x).abs() < 1e-3,
            "the beacon must not widen her by the width of its flames \
             (small {small:?}, fire {fire:?})"
        );
    }

    /// HER GROWN CROUCH FITS WHERE HER SMALL FORM FITS.
    ///
    /// collision as her small form, but currently its a bit too tall, so she
    /// can't clear places she used to be able to)."*
    ///
    /// The SMB1 rule, and the reason it is a rule: a one-tile gap is authored
    /// for small Mary-O, and crouching is how the grown form is allowed through
    /// it. Miss by any amount and a route that worked before the mushroom stops
    /// working after it.
    ///
    /// Naming the gap she must fit is what makes the check able to fail — and it stays honest if
    /// her authored form heights are ever redrawn.
    #[test]
    fn her_grown_crouch_fits_where_her_small_form_fits() {
        use ae::BodyMode;

        let small = form_body_size(SMALL_SHEET_TARGET);
        let tall = form_body_size(TALL_SHEET_TARGET);
        let crouched = BodyMode::Crouching.shape(tall).size;

        // The premise: growing must actually make her taller, or a crouch that
        // "fits" proves nothing.
        assert!(
            tall.y > small.y + 1.0,
            "the grown form ({tall:?}) is not taller than the small one \
             ({small:?}), so this test is not about growing"
        );
        assert!(
            crouched.y <= small.y + 1e-3,
            "her grown crouch is {:.2} tall against a {:.2} small form, so she \
             misses every gap authored for the small form by {:.2} units. \
             Crouching is how a grown platformer body is allowed through a \
             one-tile gap; it has to reach the same height, not merely a \
             smaller one.",
            crouched.y,
            small.y,
            crouched.y - small.y,
        );
    }

    /// The reactive grow: wearing the wand swaps her IDENTITY to the tall sheet;
    /// losing it (a hit) reverts to small. The tall form is a pure VIEW of
    /// possessing the wand — no manual revert.
    ///
    /// Deliberately no size assertion. Asserting it here would mean re-implementing the projection
    /// in the fixture and then agreeing with it.
    #[test]
    fn wearing_the_cap_grows_and_losing_it_shrinks() {
        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter::new(MARY_O_CHARACTER_ID),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: form_body_size(SMALL_SHEET_TARGET),
                    facing: 1.0,
                },
            ))
            .id();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        // Equip the wand -> she grows on the next tick.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![star_wand()]));
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().id(),
            TALL_CHARACTER_ID,
            "wearing the wand grows her to the tall SHEET"
        );

        // Spend the wand (a hit) -> she shrinks on the next tick.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().id(),
            MARY_O_CHARACTER_ID,
            "losing the wand shrinks her back to small"
        );
    }

    /// A SMALL Mary-O who takes a lantern goes straight to fire.
    ///
    /// this became reachable the day blocks got authored CONTENTS. While
    /// every ?-block levelled toward the lantern, the ladder guaranteed she held
    /// the wand before a beacon could ever reach her — so "what happens if a
    /// small Mary-O collects a lantern" was a question about a state the game
    /// could not produce. `AlwaysLantern` produces it: an author puts a beacon
    /// in a block and the first player to bonk it is small.
    ///
    /// no intermediate tall step, and that is the claim. She does not grow
    /// and then ignite over two ticks; `sync_grown_form` reads the beacon FIRST,
    /// so one swap carries her from small to fire. A two-step version would show
    /// as a one-frame flicker of the wrong sheet.
    #[test]
    fn a_small_mary_o_taking_a_lantern_becomes_fire_without_passing_through_tall() {
        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter::new(MARY_O_CHARACTER_ID),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: form_body_size(SMALL_SHEET_TARGET),
                    facing: 1.0,
                },
            ))
            .id();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        // She is small, wearing nothing — the state an authored `AlwaysLantern`
        // block hands a lantern to.
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().id(),
            MARY_O_CHARACTER_ID,
            "she starts small"
        );

        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![cinder_beacon()]));
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().id(),
            SPARK_CHARACTER_ID,
            "ONE tick after taking the beacon she is the fire form — not tall, \
             and not tall-then-fire"
        );

        // and wearing BOTH is still fire, which is the case the check's
        // ORDER protects and the ordinary route to this form: the ladder gives
        // the wand first and the beacon second, so she holds both from then on.
        // A first probe at this test reordered the check and it stayed green —
        // because equipping only the beacon cannot tell the two orders apart.
        // This is the assertion that can.
        app.world_mut()
            .entity_mut(body)
            .insert(WornEquipment::new(vec![star_wand(), cinder_beacon()]));
        app.update();
        assert_eq!(
            app.world().get::<WornCharacter>(body).unwrap().id(),
            SPARK_CHARACTER_ID,
            "holding the wand AND the beacon is the fire form — the beacon is \
             the higher rung and the check has to read it first"
        );
    }

    /// Both power states are tall, and the downgrade between them does not
    /// resize her. Losing the spark must change what she can DO, not how big
    /// she is; only the second hit shrinks her.
    ///
    /// "Tall" is now an IDENTITY fact checked against the sheets' own sizes
    /// (above): the two power states share a height but not a sheet, so the
    /// claim is that the beacon and the wand select forms whose art agrees on
    /// height — which [`her_forms_boxes_come_from_their_sheets`] asserts — and
    /// that the downgrade lands on the grown form rather than the small one.
    #[test]
    fn spark_powered_is_tall_and_only_the_second_hit_shrinks_her() {
        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter::new(MARY_O_CHARACTER_ID),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: form_body_size(SMALL_SHEET_TARGET),
                    facing: 1.0,
                },
                WornEquipment::new(vec![cinder_beacon()]),
            ))
            .id();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        app.add_systems(Update, sync_grown_form);

        let form = |app: &App| {
            app.world()
                .get::<WornCharacter>(body)
                .unwrap()
                .id()
                .to_string()
        };

        app.update();
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
        assert_eq!(
            form(&app),
            TALL_CHARACTER_ID,
            "losing the spark drops the fire sheet back to the grown sheet, \
             not to the small one"
        );

        // Hit two: grown -> small.
        app.world_mut()
            .get_mut::<WornEquipment>(body)
            .unwrap()
            .consume_armor();
        app.update();
        assert_eq!(
            form(&app),
            MARY_O_CHARACTER_ID,
            "the second hit shrinks her back to the small sheet"
        );
    }

    /// Every form change voices its OWN cue. Not one generic
    /// chime with a direction test: five authored edges, because gaining fire and
    /// losing it are different events with different sound design, and the pair
    /// that share a direction (`small→big` and `big→fire`) are not the same
    /// sound either.
    ///
    /// A reversion is deliberately NOT silent.
    #[test]
    fn each_form_transition_voices_its_own_cue() {
        use ambition_platformer2d::sfx::{OwnedSfxMessage, SfxId, SfxMessage};

        let mut app = App::new();
        let body = app
            .world_mut()
            .spawn((
                PrimaryPlayer,
                WornCharacter::new(MARY_O_CHARACTER_ID),
                ae::BodyKinematics {
                    pos: ae::Vec2::new(0.0, 100.0),
                    vel: ae::Vec2::ZERO,
                    size: form_body_size(SMALL_SHEET_TARGET),
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
        use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

        let room = crate::level_1_1();
        let struck_id = room
            .world
            .blocks
            .iter()
            .find(|b| {
                crate::ldtk_vocabulary::block_look_of(&b.name)
                    == Some(crate::ldtk_vocabulary::MaryOBlockLook::Question)
            })
            .expect("the level authors a ?-block")
            .id
            .clone();

        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        let mut scope = ActiveSessionScope::default();
        let session = scope.begin();
        app.insert_resource(scope);
        app.world_mut().spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(session),
            ae::RoomGeometry(room.world.clone()),
        ));
        // The bonk announces the strike so the render layer can flinch the block;
        // an unregistered message fails parameter validation rather than being
        // ignored, so even a fixture that draws nothing has to declare it.
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                // A hand-built fixture contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: struck_id,
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

    /// A block that LOOKS like a brick but HOLDS a powerup pops it.
    ///
    /// like a brick but really has a powerup. We should also allow for bricks to
    /// have explicit items. E.g. always a wand, always a lantern, or a
    /// level-towards lantern powerup."* This is that, and it is the case that
    /// forced appearance and contents apart — with one enum answering both, a
    /// brick could only ever hold nothing.
    ///
    /// and it must NOT break. A brick that shattered would take the
    /// powerup with it, which is why `MaryOBlockContents::breaks_when_empty`
    /// derives breakability from the contents rather than making the author keep
    /// two fields consistent.
    #[test]
    fn a_brick_that_hides_a_lantern_pops_a_lantern_and_is_not_breakable() {
        use crate::ldtk_vocabulary::{
            block_of, reactive_block, MaryOBlock, MaryOBlockContents, MaryOBlockLook, MaryOPickup,
        };
        use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

        // Brick art, lantern inside — the two fields set independently.
        let hidden = MaryOBlock::new(
            MaryOBlockLook::Brick,
            MaryOBlockContents::Always(MaryOPickup::Lantern),
        );
        let block = reactive_block(
            hidden,
            "hidden_lantern_brick",
            ae::Vec2::new(64.0, 64.0),
            ae::Vec2::splat(32.0),
        );
        let struck_id = block.id.clone();
        assert_eq!(
            block_of(&block.name),
            Some(hidden),
            "the block carries BOTH fields through its name"
        );
        assert!(
            !hidden.contents.breaks_when_empty(),
            "a loaded brick is not breakable — the item would have nowhere to come from"
        );

        let world = ae::World::new(
            "hidden brick fixture",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(32.0, 400.0),
            vec![block],
        );
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        let mut scope = ActiveSessionScope::default();
        let session = scope.begin();
        app.insert_resource(scope);
        app.world_mut().spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(session),
            ae::RoomGeometry(world),
        ));
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                // A hand-built fixture contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: struck_id,
                },
            });
        // SMALL Mary-O, wearing nothing. `Always` means always: the whole
        // difference from `Toward` is that the ladder does not get a vote, and a
        // test run at full power could not tell the two apart.
        app.world_mut().spawn((PrimaryPlayer, frame));
        app.add_systems(Update, bonk_power_blocks);
        app.update();

        let popped: Vec<String> = app
            .world_mut()
            .query::<&WorldItem>()
            .iter(app.world())
            .map(|item| match &item.payload {
                ambition_platformer2d::actors::items::WorldItemPayload::Equip(row) => {
                    row.id.clone()
                }
            })
            .collect();
        assert_eq!(
            popped,
            vec![CINDER_BEACON_ID.to_string()],
            "a brick holding `AlwaysLantern` pops the lantern, to a form the \
             ladder would have given the wand"
        );
    }

    /// A coin block PAYS instead of POPPING.
    ///
    /// spawn as items, they just play an animation and your coin count goes
    /// up)."* Both halves are asserted, and the second is the one that matters:
    /// every other reward in this file spawns a `WorldItem` that has to be
    /// caught, so "no item" is the whole difference and a payout that also
    /// popped something would satisfy a balance-only test.
    ///
    /// Uses the `Hidden` look, whose default contents is a coin — so this also
    /// pins that an invisible block with nothing authored in it still pays.
    #[test]
    fn a_coin_block_credits_the_wallet_and_spawns_nothing() {
        use crate::ldtk_vocabulary::{
            reactive_block, MaryOBlock, MaryOBlockContents, MaryOBlockLook, MaryOPickup,
        };
        use ambition_platformer2d::platformer::lifecycle::ActiveSessionScope;

        let coin_block = MaryOBlock::plain(MaryOBlockLook::Hidden);
        assert_eq!(
            coin_block.contents,
            MaryOBlockContents::Always(MaryOPickup::Coin),
            "a hidden block defaults to the classic coin"
        );
        let block = reactive_block(
            coin_block,
            "hidden_coin",
            ae::Vec2::new(64.0, 64.0),
            ae::Vec2::splat(32.0),
        );
        let struck_id = block.id.clone();
        let world = ae::World::new(
            "coin block fixture",
            ae::Vec2::new(640.0, 480.0),
            ae::Vec2::new(32.0, 400.0),
            vec![block],
        );
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        let mut scope = ActiveSessionScope::default();
        let session = scope.begin();
        app.insert_resource(scope);
        app.world_mut().spawn((
            ambition_platformer2d::platformer::lifecycle::SessionRoot(session),
            ae::RoomGeometry(world),
        ));
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                // A hand-built fixture contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
                kind: ContactKind::Head,
                point: ae::Vec2::ZERO,
                normal: ae::Vec2::new(0.0, 1.0),
                toi: 0.0,
                surface_velocity: ae::Vec2::ZERO,
                source: ContactSource::Block {
                    kind: ae::BlockKind::Solid,
                    id: struck_id,
                },
            });
        app.world_mut().spawn((
            PrimaryPlayer,
            frame,
            ambition_platformer2d::characters::actor::BodyWallet { balance: 7 },
        ));
        app.add_systems(Update, bonk_power_blocks);
        app.update();

        let balance = app
            .world_mut()
            .query::<&ambition_platformer2d::characters::actor::BodyWallet>()
            .iter(app.world())
            .next()
            .expect("she has a wallet")
            .balance;
        assert_eq!(balance, 8, "the coin is credited on the strike");

        // visually pops out a coin when you jump up into it."* The wallet
        // assertion above passes with nothing drawn, which is exactly the state
        // this block was in before — the counter worked and the coin was
        // invisible.
        let popped: Vec<_> = app
            .world_mut()
            .resource_mut::<bevy::prelude::Messages<ambition_platformer2d::vfx::VfxMessage>>()
            .drain()
            .filter(|message| {
                matches!(
                    message,
                    ambition_platformer2d::vfx::VfxMessage::CoinPop { .. }
                )
            })
            .collect();
        assert_eq!(
            popped.len(),
            1,
            "one strike pays one coin and should draw exactly one; none means \
             the block credits an invisible coin, more than one means the payout \
             ran twice"
        );
        assert_eq!(
            app.world_mut()
                .query::<&WorldItem>()
                .iter(app.world())
                .count(),
            0,
            "a coin does not rise out of the block to be caught — that is the \
             entire difference between it and every other reward here"
        );

        // Coin blocks create no pickup, so this call site emits their collection SFX directly.
        let cues: Vec<ambition_platformer2d::sfx::SfxMessage> = app
            .world_mut()
            .resource_mut::<bevy::ecs::message::Messages<
                ambition_platformer2d::sfx::OwnedSfxMessage,
            >>()
            .drain()
            .map(|owned| owned.request)
            .collect();
        assert!(
            cues.iter().any(|cue| matches!(
                cue,
                ambition_platformer2d::sfx::SfxMessage::Play { id, .. }
                    if *id == ambition_platformer2d::sfx::ids::WORLD_COIN_PICKUP
            )),
            "a coin block must voice the SAME cue a loose coin does — the id the \
             provider declares and the engine emits. Got: {cues:?}"
        );
    }

    /// A head-bonk on ANY OTHER block (not a ?-block) pops nothing — the GeoId
    /// match is specific, not "any block from below".
    /// A head-bonk on ANY OTHER block (not a ?-block) pops nothing — the GeoId
    /// match is specific, not "any block from below".
    #[test]
    fn a_head_bonk_on_a_plain_block_pops_nothing() {
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        // The bonk announces the strike so the render layer can flinch the block;
        // an unregistered message fails parameter validation rather than being
        // ignored, so even a fixture that draws nothing has to declare it.
        app.add_message::<ambition_platformer2d::platformer::block_nudge::BlockStruck>();
        app.add_message::<ambition_platformer2d::vfx::VfxMessage>();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let mut frame = PlayerBodyFrameOutput::default();
        frame
            .events
            .contacts
            .push(ae::collision_semantics::Contact {
                // A hand-built fixture contact: nothing arrived at this surface.
                impact_speed: 0.0,
                involuntary: false,
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

    /// The whole downgrade table, both authoring moods.
    ///
    /// 1-2 authors `MaryOBlock { kind: Brick, contents: AlwaysWand }`, and a
    /// FIRE Mary-O who bonked it came out TALL. The wand and the beacon share
    /// one exclusive slot, so the engine's replacement rule did exactly what it
    /// promises and the form went down a rung.
    ///
    /// the redundant case still PAYS. `Coins` here is not "nothing" — the caller has
    /// already spent the block, flinched it and changed its art by the time it reads this, and
    /// coins are what it hands over instead.
    #[test]
    fn a_pickup_never_downgrades_the_form_she_is_already_in() {
        let small: Option<&WornEquipment> = None;
        let tall = WornEquipment::new(vec![star_wand()]);
        let fire = WornEquipment::new(vec![cinder_beacon()]);

        // Each row: the form she is in, the pickup authored, what it must pay.
        let wand = MaryOBlockContents::Always(MaryOPickup::Wand);
        let lantern = MaryOBlockContents::Always(MaryOPickup::Lantern);
        let coins = BlockPayout::Coins(COINS_PER_BLOCK);

        let paid = |contents, worn| match reward_for(contents, worn) {
            Some(BlockPayout::Item(reward)) => reward.row.id.clone(),
            Some(BlockPayout::Coins(amount)) => format!("coins:{amount}"),
            None => "nothing".to_string(),
        };
        let coins_label = match &coins {
            BlockPayout::Coins(amount) => format!("coins:{amount}"),
            BlockPayout::Item(_) => unreachable!(),
        };

        // Climbing is untouched: a weaker form takes what it is given.
        assert_eq!(paid(wand, small), STAR_WAND_ID, "small + wand -> tall");
        assert_eq!(
            paid(lantern, small),
            CINDER_BEACON_ID,
            "small + lantern -> fire"
        );
        assert_eq!(
            paid(lantern, Some(&tall)),
            CINDER_BEACON_ID,
            "tall + lantern -> fire"
        );

        // Sideways and downward pay coins instead of undressing her.
        assert_eq!(
            paid(wand, Some(&tall)),
            coins_label,
            "tall + wand stays tall"
        );
        assert_eq!(
            paid(wand, Some(&fire)),
            coins_label,
            "⛔ fire + wand STAYS FIRE"
        );
        assert_eq!(
            paid(lantern, Some(&fire)),
            coins_label,
            "fire + lantern stays fire"
        );

        // the quasar is not on the ladder, so it is never a downgrade: any
        // form takes one. It wears no exclusive form slot, which is what the
        // rule keys on rather than a list of ids it would have to be kept in
        // step with.
        let quasar = MaryOBlockContents::Always(MaryOPickup::Quasar);
        assert_eq!(
            paid(quasar, Some(&fire)),
            crate::star::POCKET_QUASAR_ID,
            "fire + quasar -> quasar"
        );

        // And a coin block is coins from every form, by the routing above
        // rather than by three call sites each remembering to check.
        let coin = MaryOBlockContents::Always(MaryOPickup::Coin);
        assert_eq!(paid(coin, Some(&fire)), coins_label);
        assert_eq!(paid(coin, small), coins_label);

        // An empty block owes NOTHING, which is a different answer from coins.
        assert_eq!(paid(MaryOBlockContents::Empty, small), "nothing");
    }
}

#[cfg(test)]
mod loose_form_tests {
    use super::*;
    use ambition_platformer2d::actors::items::WorldItem;
    use ambition_platformer2d::characters::equipment::WornEquipment;

    /// A wand lying on the floor cannot demote fire Mary-O.
    #[test]
    fn a_loose_wand_is_consumed_rather_than_demoting_her() {
        let mut app = App::new();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let at = ae::Vec2::new(100.0, 100.0);
        let item = app
            .world_mut()
            .spawn(WorldItem::equipping(star_wand(), at, ae::Vec2::splat(8.0)))
            .id();
        app.world_mut().spawn((
            PrimaryPlayer,
            ae::BodyKinematics {
                pos: at,
                size: ae::Vec2::new(30.0, 48.0),
                ..Default::default()
            },
            WornEquipment::new(vec![cinder_beacon()]),
        ));
        app.add_systems(Update, refuse_a_weaker_form_pickup);
        app.update();

        assert!(
            app.world().get_entity(item).is_err(),
            "the redundant wand was left on the floor for the engine's collector \
             to equip, which is the demotion this rule exists to stop"
        );

        // …and the control: SMALL Mary-O keeps it, so the rule is not "refuse
        // every wand".
        let mut app = App::new();
        app.add_message::<ambition_platformer2d::sfx::OwnedSfxMessage>();
        let item = app
            .world_mut()
            .spawn(WorldItem::equipping(star_wand(), at, ae::Vec2::splat(8.0)))
            .id();
        app.world_mut().spawn((
            PrimaryPlayer,
            ae::BodyKinematics {
                pos: at,
                size: ae::Vec2::new(30.0, 48.0),
                ..Default::default()
            },
        ));
        app.add_systems(Update, refuse_a_weaker_form_pickup);
        app.update();
        assert!(
            app.world().get_entity(item).is_ok(),
            "small Mary-O's wand was eaten; the rule must only refuse a WEAKER \
             form, not every form"
        );
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;

    fn block(kind: ae::BlockKind, id: ae::GeoId, name: &str) -> ae::Block {
        ae::Block {
            id,
            name: name.to_string(),
            aabb: ae::Aabb::new(ae::Vec2::new(48.0, 96.0), ae::Vec2::splat(8.0)),
            kind,
            art_color: None,
            velocity: ae::Vec2::ZERO,
        }
    }

    /// Discovery turns an invisible block into a real solid — both halves.
    ///
    /// the BEFORE case is asserted too, and it is the half a careless guard
    /// would drop. Solidifying every hidden block on sight passes "it is solid
    /// after" while deleting the mechanic: you would stand on blocks you have
    /// never found.
    #[test]
    fn a_hidden_block_is_air_until_it_is_struck_and_solid_after() {
        let id = ae::GeoId::anon();
        let hidden = block(ae::BlockKind::BonkOnly, id.clone(), "hidden_coin_1");

        assert!(
            discovered_solid(&SpentPowerBlocks::default(), &hidden).is_none(),
            "an unstruck hidden block was solidified, which deletes the mechanic: \
             you would stand on blocks you have never found"
        );

        let mut spent = SpentPowerBlocks::default();
        spent.spend(id);
        let solid =
            discovered_solid(&spent, &hidden).expect("a struck hidden block becomes something");
        assert_eq!(
            solid.kind,
            ae::BlockKind::Solid,
            "a discovered block must be an ORDINARY solid — supports Mary-O, \
             supports enemies, blocks sideways"
        );
        assert_eq!(
            solid.aabb, hidden.aabb,
            "the solid must sit exactly where the authored block did"
        );
        assert_eq!(
            solid.name, hidden.name,
            "it keeps its name, because the overlay removes the authored block \
             BY name and a rename would leave both in the room"
        );
    }

    /// A spent QUESTION or BRICK is already authored solid; re-adding it would
    /// put two blocks in one place.
    #[test]
    fn only_hidden_blocks_are_upgraded() {
        let id = ae::GeoId::anon();
        let question = block(ae::BlockKind::Solid, id.clone(), "question_1");
        let mut spent = SpentPowerBlocks::default();
        spent.spend(id);
        assert!(
            discovered_solid(&spent, &question).is_none(),
            "a spent Question was re-added, doubling an already-solid block"
        );
    }
}

#[cfg(test)]
mod multi_coin_counter_tests {
    use super::*;

    fn id(name: &str) -> ae::GeoId {
        // `GeoId::new` does not exist — the crate's `new` at that line belongs
        // to `PlacementId`. A block id is a placement or a tile-layer slot.
        ae::GeoId::tile_layer(name, 0)
    }

    /// The counter lives beside the spent set rather than replacing it, so the
    /// older half keeps its exact meaning: a partial entry is a block mid-payout
    /// and `is_spent` stays the single authority for "this block is done".
    #[test]
    fn a_three_coin_block_pays_three_times_then_retires() {
        let mut spent = SpentPowerBlocks::default();
        let block = id("coin_block");

        for hit in 1..=2 {
            assert!(
                !spent.take_one_coin(&block, 3),
                "hit {hit} of 3 must not exhaust the block"
            );
            assert!(
                !spent.is_spent(&block),
                "hit {hit} of 3 left the block spent, so it cannot be struck again"
            );
            assert_eq!(spent.coins_taken(&block), hit);
        }

        assert!(spent.take_one_coin(&block, 3), "the third hit exhausts it");
        assert!(spent.is_spent(&block), "an exhausted block is spent");
        // and the partial entry is GONE, not left at 3 — two records of the
        // same fact would be a second thing to keep in step.
        assert_eq!(spent.coins_taken(&block), 0);
    }

    /// POISON: a one-coin block behaves exactly as every block did before.
    /// `Coins(1)` is the default instance, so if it took two hits the whole
    /// existing cast of ?-blocks would have changed behaviour.
    #[test]
    fn a_one_coin_block_retires_on_the_first_hit() {
        let mut spent = SpentPowerBlocks::default();
        let block = id("ordinary");
        assert!(spent.take_one_coin(&block, 1));
        assert!(spent.is_spent(&block));
    }

    /// A reset re-arms BOTH halves.
    #[test]
    fn a_reset_rearms_a_partly_paid_block() {
        let mut spent = SpentPowerBlocks::default();
        let block = id("coin_block");
        spent.take_one_coin(&block, 5);
        spent.take_one_coin(&block, 5);
        assert_eq!(spent.coins_taken(&block), 2);

        spent.rearm_all();
        assert_eq!(spent.coins_taken(&block), 0, "the count came back full");
        assert!(!spent.is_spent(&block));
    }

    /// the checksum sees the COUNT. Two peers whose blocks have paid a
    /// different number of coins must not agree on the hash — otherwise the
    /// divergence rides silently until one of them runs out first.
    #[test]
    fn the_checksum_distinguishes_two_from_three_coins_paid() {
        let block = id("coin_block");
        let mut two = SpentPowerBlocks::default();
        two.take_one_coin(&block, 9);
        two.take_one_coin(&block, 9);
        let mut three = SpentPowerBlocks::default();
        three.take_one_coin(&block, 9);
        three.take_one_coin(&block, 9);
        three.take_one_coin(&block, 9);
        assert_ne!(
            two.checksum(),
            three.checksum(),
            "⛔ the hash is blind to how much a block still owes"
        );
    }
}

#[cfg(test)]
mod block_dressing_tests {
    //! WHAT EACH LOOK WEARS, BEFORE AND AFTER IT HAS PAID.
    //!
    //! blocks (or question mark blocks which currently work correctly) need to change their tile
    //! sprite to spent blocks.
    //!
    //! the unspent BRICK row is the load-bearing one. It is the reason the
    //! fix is guarded on `is_spent` rather than unconditional: a brick that has
    //! not paid must still name NO art, because naming art strips the level's
    //! authored paint and a camouflaged block that announces itself before it
    //! pays is the secret given away.

    use super::*;
    use crate::ldtk_vocabulary::{MaryOBlock, MaryOBlockContents, MaryOBlockLook, MaryOPickup};
    use ambition_platformer2d::actors::assets::game_assets::EntitySprite;
    use ambition_platformer2d::render::rendering::{BlockArt, BlockVisual};
    use bevy::prelude::{App, Entity, Update};

    /// Spawn one render-side block the way the room build does — the ENCODED
    /// name, so the dresser decodes the same vocabulary the converter writes.
    fn block(app: &mut App, iid: &str, block: MaryOBlock) -> Entity {
        let geo_id = ae::GeoId::placement(ae::PlacementId::new(iid.to_string()), 0);
        app.world_mut()
            .spawn(BlockVisual {
                block_name: crate::ldtk_vocabulary::encoded_name(block, iid),
                geo_id,
            })
            .id()
    }

    fn art(app: &App, entity: Entity) -> Option<EntitySprite> {
        app.world().get::<BlockArt>(entity).map(|art| art.0)
    }

    #[test]
    fn every_look_wears_the_spent_plate_once_it_has_paid_and_a_brick_hides_until_then() {
        let mut app = App::new();
        app.init_resource::<SpentPowerBlocks>();
        app.add_systems(Update, dress_power_blocks);

        let quasar = MaryOBlockContents::Always(MaryOPickup::Quasar);
        let question_unspent = block(
            &mut app,
            "q_fresh",
            MaryOBlock::new(MaryOBlockLook::Question, quasar),
        );
        let question_spent = block(
            &mut app,
            "q_used",
            MaryOBlock::new(MaryOBlockLook::Question, quasar),
        );
        let hidden_unspent = block(
            &mut app,
            "h_fresh",
            MaryOBlock::new(MaryOBlockLook::Hidden, quasar),
        );
        let hidden_spent = block(
            &mut app,
            "h_used",
            MaryOBlock::new(MaryOBlockLook::Hidden, quasar),
        );
        // hiding a quasar, in 1-1.
        let brick_unspent = block(
            &mut app,
            "b_fresh",
            MaryOBlock::new(MaryOBlockLook::Brick, quasar),
        );
        let brick_spent = block(
            &mut app,
            "b_used",
            MaryOBlock::new(MaryOBlockLook::Brick, quasar),
        );

        {
            let mut spent = app.world_mut().resource_mut::<SpentPowerBlocks>();
            for iid in ["q_used", "h_used", "b_used"] {
                spent.spend(ae::GeoId::placement(
                    ae::PlacementId::new(iid.to_string()),
                    0,
                ));
            }
        }
        app.update();

        // the NON-VACUITY floor: if the dresser ran over nothing, every row
        // below would be `None` and three of them expect exactly that.
        assert_eq!(
            art(&app, question_unspent),
            Some(EntitySprite::BonusBlockTile),
            "an unpaid ?-block is not wearing the bonus tile, so the dresser did not run"
        );

        assert_eq!(
            art(&app, question_spent),
            Some(EntitySprite::SpentBlockTile),
            "a paid ?-block must wear the used plate (Jon: this one already worked)"
        );
        assert_eq!(
            art(&app, hidden_unspent),
            None,
            "an undiscovered hidden block named art, so it is visible before it is found"
        );
        assert_eq!(
            art(&app, hidden_spent),
            Some(EntitySprite::SpentBlockTile),
            "a struck hidden block must reveal itself as a used block"
        );
        assert_eq!(
            art(&app, brick_unspent),
            None,
            "a brick that has NOT paid named art, which strips the level's authored \
             paint and announces the secret it exists to keep"
        );
        assert_eq!(
            art(&app, brick_spent),
            Some(EntitySprite::SpentBlockTile),
            "a brick that gave up its quasar still wears masonry, so the player cannot \
             tell which bricks they have already emptied (Jon, 2026-08-19)"
        );
    }
}
