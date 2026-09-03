//! ⭐⭐ A WEAKER FORM ON THE FLOOR IS REFUSED, IN THE SHIPPED APP.
//!
//! `refuse_a_weaker_form_pickup` is the FLOOR half of Mary-O's monotonic form
//! ladder. The block half — `without_downgrading` — is guarded by unit tests
//! that call it directly. This one is the half no unit test can reach, because
//! the thing it guards against only exists as a consequence of TIMING in a
//! running world.
//!
//! ⛔ WHY THE FLOOR CASE IS REACHABLE AT ALL, which is the whole reason this
//! file exists. Every road a form item can arrive on runs through a block, and
//! `without_downgrading` already refuses to MINT a weaker one: a fire Mary-O
//! bonking 1-2's `AlwaysWand` brick gets coins, and no wand is ever spawned. So
//! it looks as though a fire Mary-O can never meet a wand — and that reading is
//! what makes the floor rule look redundant.
//!
//! It is wrong, and the wand's own motion is why. `wand_reward` gives it
//! `ItemMotionPlan::walker` — *"the wand WALKS and turns at walls, like the
//! mushroom"* — so a wand that pops out and is NOT caught keeps existing,
//! patrolling the room, for as long as the player leaves it there. Bonk two
//! ?-blocks while small and catch only one of the wands, and you are a tall
//! Mary-O with a wand still walking the floor. Nothing in the block payout has
//! any say about what happens when they meet.
//!
//! ⭐ THAT IS THE SCENARIO THIS TEST PLAYS, on the real `build_demo_app()`, out
//! of 1-1's own authored blocks. Nothing here writes a `WornEquipment`, spawns
//! an item, or calls the refusal directly.
//!
//! ⛔ AND THE FORM IS EARNED, never inserted — the rule `two_rooms.rs` sets and
//! says why: *"inserting a `WornEquipment` by hand would prove the transition
//! preserves something no player can obtain."* The same objection applies with
//! more force here, because the form's RANK is the input to the rule under test.
//! A hand-worn beacon would prove the refusal reads a component; earning it
//! proves the refusal reads the form a player can actually be in.
//!
//! ⚠ THE HELPERS ARE DUPLICATED FROM `two_rooms.rs` RATHER THAN SHARED, and the
//! duplication is deliberate rather than accidental. `autotests = false` means
//! both files are modules of the ONE `mary_o_it` binary, so sharing is a
//! `pub(crate)` away — but it would mean editing `two_rooms.rs`, which another
//! session is holding tonight. The debt is the cheaper of the two and it is
//! written down here so the next person collapses it on purpose.

use ambition_demo_mary_o::ldtk_vocabulary::{
    block_of, MaryOBlockContents, MaryOBlockLook, MaryOPickup,
};
use ambition_demo_mary_o::powerups::{SpentPowerBlocks, CINDER_BEACON_ID, STAR_WAND_ID};
use ambition_demo_mary_o_app::build_demo_app;
// ⛔ NOT `actors::items::…`. `actors` IS the actor monolith, and the touched
// collectible left it in D33 (2026-09-02) — importing it from there would name
// the kernel for something the kernel no longer owns.
use ambition_platformer2d::world_items::{WorldItem, WorldItemPayload};
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::input::ControlFrame;
use ambition_platformer2d::platformer::markers::PrimaryPlayer;
use bevy::prelude::*;

// ── harness ─────────────────────────────────────────────────────────────────

fn boot() -> App {
    let mut app = build_demo_app();
    ambition_platformer2d::scripted_input::drive_the_local_participant(&mut app);
    for _ in 0..90 {
        app.update();
    }
    app
}

fn step(app: &mut App, frame: ControlFrame) {
    app.world_mut()
        .resource_mut::<ambition_platformer2d::scripted_input::ScriptedControls>()
        .0 = frame;
    app.update();
}

/// Hold jump. `jump_pressed` rides along with `jump_held` because her jump is an
/// EDGE — a frame that only holds continues an ascent, it cannot start one.
fn with_jump(mut frame: ControlFrame) -> ControlFrame {
    frame.jump_pressed = true;
    frame.jump_held = true;
    frame
}

fn player_body(app: &mut App) -> (Vec2, Vec2) {
    let mut query = app
        .world_mut()
        .query_filtered::<&ae::BodyKinematics, With<PrimaryPlayer>>();
    let kin = query
        .iter(app.world())
        .next()
        .expect("gameplay has a primary player");
    (kin.pos, kin.size)
}

fn place_player(app: &mut App, pos: Vec2) {
    let mut query = app.world_mut().query_filtered::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    ), With<PrimaryPlayer>>();
    let world = app.world_mut();
    let (mut cluster_item, mut motion_model) = query
        .iter_mut(world)
        .next()
        .expect("gameplay has a primary player");
    let mut clusters = cluster_item.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut motion_model,
        &mut clusters,
        pos,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

/// The form, read off the BODY — the authority the ladder ranks.
fn wears(app: &mut App, row_id: &str) -> bool {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::equipment::WornEquipment, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .is_some_and(|worn| worn.wears(row_id))
}

fn wallet(app: &mut App) -> i32 {
    let mut query = app
        .world_mut()
        .query_filtered::<&ambition_platformer2d::characters::actor::BodyWallet, With<PrimaryPlayer>>();
    query
        .iter(app.world())
        .next()
        .expect("the player has a wallet")
        .balance
}

fn block_is_spent(app: &App, id: &ae::GeoId) -> bool {
    app.world()
        .get_resource::<SpentPowerBlocks>()
        .is_some_and(|spent| spent.is_spent(id))
}

/// Every uncollected form item in the world, with the row it would equip.
fn loose_items(app: &mut App) -> Vec<(Entity, String, Vec2)> {
    let mut query = app.world_mut().query::<(Entity, &WorldItem)>();
    query
        .iter(app.world())
        .map(|(entity, item)| {
            let WorldItemPayload::Equip(row) = &item.payload;
            (entity, row.id.clone(), item.pos)
        })
        .collect()
}

fn item_of(app: &mut App, entity: Entity) -> Option<Vec2> {
    loose_items(app)
        .into_iter()
        .find(|(e, _, _)| *e == entity)
        .map(|(_, _, pos)| pos)
}

// ── the level's own blocks ──────────────────────────────────────────────────

/// 1-1's laddered ?-blocks, left to right.
///
/// ⛔ Found by what the AUTHOR marked, never by a reconstructed id — the rule
/// `two_rooms.rs` and `power_loop`'s harness both follow. Filtered to
/// `Toward(Lantern)` on purpose: 1-1 also authors two `AlwaysQuasar` blocks that
/// LOOK identical (a quasar block is a `Question` whose contents say so), and a
/// quasar is not on the form ladder at all, so bonking one would pay a pocket
/// quasar and level her nowhere.
fn laddered_blocks() -> Vec<ae::world::Block> {
    let room = ambition_demo_mary_o::level_1_1();
    let mut blocks: Vec<ae::world::Block> = room
        .world
        .blocks
        .iter()
        .filter(|block| {
            block_of(&block.name).is_some_and(|authored| {
                authored.look == MaryOBlockLook::Question
                    && authored.contents == MaryOBlockContents::Toward(MaryOPickup::Lantern)
            })
        })
        .cloned()
        .collect();
    blocks.sort_by(|a, b| {
        a.aabb
            .center()
            .x
            .partial_cmp(&b.aabb.center().x)
            .expect("authored block positions are finite")
    });
    blocks
}

/// How far above the target a placement starts so the body LANDS on the surface
/// instead of being inserted at a guessed height.
const DROP_HEIGHT_PX: f32 = 100.0;

/// Strike one authored block from below, playing the jump.
///
/// The WALK to the block is skipped — the same concession `two_rooms.rs` makes,
/// for the same reason: 1-1 is 3328px of platforming and this is not a
/// playthrough. The strike is played.
fn bonk(app: &mut App, block: &ae::world::Block) {
    // `+y` is screen-down, so the block's `max.y` is the face she bonks.
    let underside = block.aabb.max.y;

    let centre = block.aabb.center();

    // ⛔⛔ THE PLACEMENT IS `two_rooms.rs`'s, AND IT MUST STAY A DROP. Its
    // comment — *"direct placement at the guessed resting height can leave this
    // fixture inert"* — is not a stylistic note, and I proved it the expensive
    // way: putting her at the height she was ALREADY standing at (which looks
    // like the safest possible value, since it is measured rather than guessed)
    // starts her overlapping the floor by the sliver collision had resolved
    // away, and the resolve sends her back to the level's spawn. The symptom is
    // a bonk that reports "she never struck it" from 130px away, in a test that
    // never mentions respawning. Start her ABOVE her footing and let her fall.
    let settle = |app: &mut App, y: f32| {
        place_player(app, Vec2::new(centre.x, y));
        for _ in 0..90 {
            step(app, ControlFrame::default());
        }
    };
    let from = player_body(app).0.y - DROP_HEIGHT_PX;
    settle(app, from);

    // ⚠ AND ONE RETRY, because the drop height is not right for every column.
    // The gap between the floor and a block's underside is ~80px here and the
    // drop is 100, so on a column with anything raised under it she lands ON
    // the block instead of beneath it — and a jump from up there can never
    // reach the face she is aiming at. Re-drop from just below the face when
    // that happens rather than letting the jump loop time out and blame the
    // strike.
    let (pos, size) = player_body(app);
    if pos.y - size.y * 0.5 <= underside {
        let retry_from = underside + size.y * 0.5 + 40.0;
        settle(app, retry_from);
    }

    // Say it plainly if the settle put her somewhere she cannot bonk FROM.
    let (pos, size) = player_body(app);
    assert!(
        pos.y - size.y * 0.5 > underside,
        "she settled at {pos:?} with her head above the block's underside \
         ({underside:.1}) — she is standing ON the block, not under it, so no \
         jump from here can ever strike it",
    );
    assert!(
        (pos.x - centre.x).abs() <= 48.0,
        "she settled at {pos:?}, {:.1}px from the block's column ({centre:?}) — \
         she did not stay where she was put, so the jump below would be aimed \
         at nothing. A fall out of the world and a respawn looks exactly like \
         this, and so does a placement that overlapped the floor.",
        (pos.x - centre.x).abs(),
    );

    // The variable jump, steered off GEOMETRY rather than a frame count: hold
    // while her head is still below the underside and release the instant it
    // arrives. A bare tap falls short of a 48px gap and a held jump sails over.
    let mut head_best = f32::MAX;
    for _ in 0..400 {
        let (pos, size) = player_body(app);
        let head = pos.y - size.y * 0.5;
        head_best = head_best.min(head);
        let frame = if head > underside {
            with_jump(ControlFrame::default())
        } else {
            ControlFrame::default()
        };
        step(app, frame);
        if block_is_spent(app, &block.id) {
            return;
        }
    }
    let (ended, _) = player_body(app);
    panic!(
        "she never struck the ?-block at {centre:?}, so it minted nothing. Best \
         head height reached: {head_best:.1} (needs <= {underside:.1}, lower \
         number is higher); she ended at {ended:?}. A head that CLEARED the \
         face without spending the block means she was not in its column when \
         she passed it.",
    );
}

/// Walk onto one specific item until the engine's touch-to-collect takes it.
///
/// Chased rather than waited for, because the wand TRAVELS. Placing her on it is
/// what collects it: `collect_world_items` claims it on overlap, exactly as a
/// player gets it.
fn collect(app: &mut App, entity: Entity, what: &str) {
    for _ in 0..400 {
        match item_of(app, entity) {
            Some(pos) => place_player(app, pos),
            // Gone from the world is what collected looks like.
            None => return,
        }
        step(app, ControlFrame::default());
    }
    panic!("she stood on the {what} for 400 frames and never picked it up");
}

/// The one item that popped out of the block just struck.
fn newly_popped(before: &[(Entity, String, Vec2)], after: &[(Entity, String, Vec2)]) -> Entity {
    let known: Vec<Entity> = before.iter().map(|(e, _, _)| *e).collect();
    let fresh: Vec<&(Entity, String, Vec2)> = after
        .iter()
        .filter(|(e, _, _)| !known.contains(e))
        .collect();
    assert_eq!(
        fresh.len(),
        1,
        "expected the strike to pop exactly one item; the world gained {:?}",
        fresh
            .iter()
            .map(|(_, id, _)| id.as_str())
            .collect::<Vec<_>>(),
    );
    fresh[0].0
}

// ── the proof ───────────────────────────────────────────────────────────────

/// A wand she already has an equal claim to, still walking the floor, is
/// refused — consumed, paid for, and it does not re-dress her.
///
/// ⭐⭐ THE EQUAL RUNG IS THE BOUNDARY, AND IT IS THE ARM THAT CATCHES THE BUG.
/// The rule refuses when `form_rank(row) <= worn_rank`, and the documented
/// table has both a strictly-weaker row (`fire + wand -> fire`) and an equal one
/// (`tall + wand -> tall`). Only the equal one is load-bearing for the
/// COMPARISON: a `<` written where `<=` belongs still refuses a wand offered to
/// a fire Mary-O — 1 is not greater than 2 either way — and lets a second wand
/// re-equip a tall one. So this arm fails on the off-by-one and the
/// strictly-weaker arm does not.
///
/// ⛔ THE FIRE ARM IS ABSENT FOR A MEASURED REASON, not an oversight. Reaching
/// fire costs TWO of 1-1's three `Toward(Lantern)` blocks (small→wand→tall,
/// tall→lantern→fire), so a wand left over needs the third — and the third,
/// at x=1920, STANDS OVER A PIT. Measured, not assumed: dropped in from above
/// she lands ON it (body centre 256, feet on its top face); dropped in from
/// below the face she falls to y≈969 in a 448-tall room, dies, and respawns at
/// the level start. No body can stand under it to bonk it, so the fire arm
/// needs a scripted pit-edge jump, which would be a test about jump tuning
/// wearing this rule's name. ⇒ If someone wants it, author a fourth ?-block
/// over floor, or build the scenario in 1-3 (which authors a `Question` at 256
/// AND an `AlwaysWand` at 1696).
///
/// ⭐ THE THREE CLAUSES ARE THE WHOLE RULE, and they are separable failures:
/// the wand is CONSUMED (a refused pickup must not lie there re-triggering
/// every frame), the coins ACKNOWLEDGE it (*"the block spends, flinches, wears
/// its used art and hands over coins — it does not go unresponsive"*, and the
/// floor owes the same trade), and the FORM IS UNCHANGED. A rule that only
/// despawned would pass clause one and rob the player; one that only paid would
/// pass clause two and leave a wand that pays forever.
#[test]
fn a_tall_mary_o_refuses_a_second_wand_still_walking_the_floor() {
    let mut app = boot();
    let blocks = laddered_blocks();
    assert!(
        blocks.len() >= 2,
        "1-1 authors {} laddered ?-blocks and this proof needs two struck while \
         she is small, so a SECOND wand is minted and left walking while she \
         wears the first. With fewer, the scenario is unreachable in the \
         shipped level.",
        blocks.len(),
    );
    // ⛔ THE TWO LEFTMOST, AND THE CHOICE IS MEASURED. 1-1's third laddered
    // block stands over a pit (see this test's own note above); these two stand
    // over floor, and a drop into either column settles her at y=400.
    let (keep_at, abandon_at) = (blocks[0].clone(), blocks[1].clone());

    // ── 1. small: pop the wand she WILL catch ──
    let before = loose_items(&mut app);
    bonk(&mut app, &keep_at);
    let ladder_wand = newly_popped(&before, &loose_items(&mut app));
    let popped_row = loose_items(&mut app)
        .into_iter()
        .find(|(e, _, _)| *e == ladder_wand)
        .map(|(_, id, _)| id)
        .expect("the item that just popped is in the world");
    assert_eq!(
        popped_row, STAR_WAND_ID,
        "a ?-block struck by a SMALL Mary-O should level her toward the wand; \
         it popped '{popped_row}' instead, so the rest of this proof would be \
         about the wrong item",
    );

    // ── 2. still small — the first wand is uncaught, so she has not levelled —
    //       pop the one she will LEAVE ──
    let before = loose_items(&mut app);
    bonk(&mut app, &abandon_at);
    let abandoned = newly_popped(&before, &loose_items(&mut app));

    // ── 3. catch the first: small + wand -> tall ──
    collect(&mut app, ladder_wand, "wand");
    assert!(
        wears(&mut app, STAR_WAND_ID),
        "she caught the wand and is not wearing it, so she is still small — and \
         a wand offered to a SMALL Mary-O is the next rung, not a redundant \
         pickup, so the refusal below would not be the rule under test",
    );

    // ⛔ THE PREMISE, CHECKED BEFORE IT IS USED. The wand is a `walker`; if it
    // strolled into her while she was levelling it was already consumed, and
    // every assertion below would pass on an empty encounter.
    let still_walking = item_of(&mut app, abandoned).unwrap_or_else(|| {
        panic!(
            "the second wand is gone before the encounter — she must have \
             brushed it while catching the first, so this run proves nothing \
             about a tall Mary-O meeting one"
        )
    });

    let purse_before = wallet(&mut app);

    // ── 4. the encounter: she walks onto it already wearing a wand ──
    place_player(&mut app, still_walking);
    for _ in 0..8 {
        step(&mut app, ControlFrame::default());
    }

    assert!(
        item_of(&mut app, abandoned).is_none(),
        "the wand is STILL LYING THERE after a tall Mary-O stood on it. A \
         refused pickup that is not consumed re-triggers every frame she \
         overlaps it — it pays out again and again, and the player can never \
         walk past the spot.",
    );
    assert_eq!(
        wallet(&mut app),
        purse_before + 1,
        "the wand vanished under a tall Mary-O and paid her nothing. A \
         redundant pickup is still ACKNOWLEDGED — the same trade the block \
         payout makes when it hands over coins instead of a form — otherwise \
         the game silently ate something the player earned.",
    );
    assert!(
        wears(&mut app, STAR_WAND_ID),
        "touching a second wand took the form she had OFF her. The exclusive \
         slot means a new row in it replaces the old one, which is correct and \
         general; refusing the redundant pickup is the only thing standing \
         between a player and a reward that undresses her.",
    );
    assert!(
        !wears(&mut app, CINDER_BEACON_ID),
        "a refused wand levelled her UP to the beacon. The floor rule consumes \
         a redundant form and pays coins; it is not a second road onto the \
         ladder, and a rung nobody earned is as wrong as a rung lost.",
    );
}

/// ⭐ THE CONTROL ARM: the refusal is not eating legitimate pickups.
///
/// Every assertion above passes on a rule that consumes EVERY form item it
/// touches — `form_rank(row) > rank` inverted, or a `<=` that lost its way — and
/// that rule would make the game uncompletable while looking correct from the
/// fire form's point of view. So the same road is walked by a small Mary-O, for
/// whom the wand is the next rung and must be KEPT.
#[test]
fn a_small_mary_o_still_collects_the_very_same_wand() {
    let mut app = boot();
    let blocks = laddered_blocks();
    let first = blocks.first().expect("1-1 authors a laddered ?-block").clone();

    assert!(
        !wears(&mut app, STAR_WAND_ID) && !wears(&mut app, CINDER_BEACON_ID),
        "she starts the demo already wearing a form, so 'small' is not the \
         state this arm is about",
    );

    let before = loose_items(&mut app);
    bonk(&mut app, &first);
    let wand = newly_popped(&before, &loose_items(&mut app));
    let purse_before = wallet(&mut app);

    collect(&mut app, wand, "wand");

    assert!(
        wears(&mut app, STAR_WAND_ID),
        "a SMALL Mary-O walked onto a wand and did not get it. The floor rule \
         refuses a form that is weaker than the one she is in — small is rank \
         0 and wears nothing, so there is no form for this pickup to be weaker \
         than.",
    );
    assert_eq!(
        wallet(&mut app),
        purse_before,
        "she got the wand AND the consolation coins, so the refusal fired on a \
         pickup the engine also equipped — the item paid twice on one touch",
    );
}
