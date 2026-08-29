//! George Booul's authored fighter repertoire.
//!
//! George is a heavy commitment fighter with three fast pokes and otherwise
//! slow, high-damage attacks; the startup gap between those groups is part of
//! his character contract and is guarded by comparative tests. The demo owns
//! George's table; stand-in robot fighters continue to use their provider-owned
//! repertoires.

use ambition_platformer2d::characters::moveset_authoring::Strike;
use ambition_platformer2d::characters::smash_repertoire::{
    DownSpecial, NeutralSpecial, SmashRepertoire, UpSpecial,
};
use ambition_platformer2d::entity_catalog::{CancelCondition, ImpulseMode, MovesetContract};

use crate::moveset::{feel, Feel};
use ambition_platformer2d::characters::moveset_authoring::{
    cancelable, committed_tail, impulse, on_hit, strike,
};

/// The rise George's Up-B commands, engine units per second against gravity.
///
/// authored as a SPEED and applied with `ImpulseMode::Set`, which is what
/// makes it a recovery: a body pressing this while falling at terminal velocity
/// gets exactly the same climb as one pressing it from a standstill. An additive
/// impulse would be strongest when George least needed it.
pub(crate) const ASCENT_SPEED: f32 = 1020.0;

/// When it arrives — the windup you can see coming, and the number the recovery
/// probe plans around.
pub(crate) const ASCENT_AT_S: f32 = 0.18;

/// And when the move lets go. The guard `the_ascent_is_a_save_and_not_a_flight` holds the
/// arithmetic.
pub(crate) const ASCENT_ENDS_S: f32 = 1.15;

/// The widest startup a POKE may have, and the narrowest a COMMITMENT may have.
///
/// these are the character, not tuning constants that happen to bracket the
/// numbers: the gap between them is the excluded middle, and the guard asserts
/// no move lands inside it. Retuning George means moving a move to one side or
/// the other, never into the band.
const POKE_MAX_STARTUP_S: f32 = 0.08;
const COMMIT_MIN_STARTUP_S: f32 = 0.15;

/// Where a smash freezes: FOUR FRAMES into its windup, and the same four frames
/// whatever the move's own startup is.
///
/// ⭐⭐ AUTHORED, not derived. `CHARGE_POSE_FRACTION` is the engine's fallback
/// for a move that says nothing, and it makes the pose a FRACTION of the
/// windup — so a slow smash would hold later in real time than a fast one, for
/// no reason a player could see. A charge pose is an ANIMATION fact: the swing
/// starts, and a few frames in it stops. Jon, 2026-08-23: *"it needs to hold on
/// the first frames of the smash animation, before letting the rest of the
/// animation, which actually has the hitboxes, play."*
///
/// ⛔ INSIDE THE LEADING STARTUP AND STRICTLY BEFORE THE FIRST ACTIVE WINDOW —
/// every windup in this roster is at least 0.22s, so four frames clears it with
/// room. `CatalogError::ChargeHoldOutsideWindup` refuses an authored pose that
/// does not, which is the check this authoring exists to satisfy rather than
/// to lean on.
const CHARGE_POSE_AT_S: f32 = 4.0 / 60.0;

/// See the module doc. Sixteen moves, the genre's standard verb map plus four
/// specials.
pub fn george_booul_moveset() -> MovesetContract {
    // ── the three pokes ──────────────────────────────────────────────────────
    //
    // Everything George owns that comes out quickly is also nearly harmless. He
    // is not paid for these; they exist so that "not committing" is a legal
    // move rather than standing still.
    let jab = strike(Strike {
        id: "jab",
        clip: "attack",
        startup_s: 0.05,
        active_s: 0.05,
        recover_s: 0.15,
        offset: (26.0, 0.0),
        half_extents: (18.0, 14.0),
        damage: 3,
        knockback: 50.0,
        knockback_growth: 1.05,
        launch_dir: None,
        on_hit: None,
    });
    // THE ONE ROUTE ACROSS THE GAP, AND IT ONLY OPENS ON CONTACT.
    //
    // George's whole problem is that he has three fast options worth nothing and
    // eight slow ones he can never start safely. This is the answer his own
    // character suggests: land the harmless thing and the committed thing
    // becomes free. `OnHit` is load-bearing — a whiffed jab cancels into
    // nothing, so the route is a REWARD for connecting rather than a way to
    // throw a smash with a jab's startup.
    //
    // The window opens with the active frames and runs through the recovery, so
    // it is the hit and its follow-through that buy the escape.
    //
    // ⭐ AND THE STRING GOES IN FRONT OF THE ROUTE. `jab2` is named first
    // because it is the answer to the UNDIRECTED follow-up — a neutral re-press
    // or a held button continues the jab — while the smash route is bought with
    // a DIRECTED press, which never reads as a string. George had neither link
    // until 2026-08-23: the chain shipped onto the shared table and he carries
    // his own, so a census of a George mirror counted zero `jab2` because he had
    // no such move, not because the brain would not choose it.
    //
    // ⛔ `Always` for the string and `OnHit` for the route, in ONE window,
    // which the cancel table cannot express — so the string gets its own window
    // and the route keeps the one it was authored with. A whiffed jab strings;
    // it still buys nothing.
    let jab = cancelable(jab, 0.05, 0.25, &["jab2"], CancelCondition::Always);
    let jab = cancelable(
        jab,
        0.05,
        0.25,
        &["smash", "special"],
        CancelCondition::OnHit,
    );
    let jab = feel(jab, Feel::Poke);

    let mut n_air = strike(Strike {
        id: "air_neutral",
        clip: "attack",
        startup_s: 0.06,
        active_s: 0.12,
        recover_s: 0.18,
        offset: (0.0, 0.0),
        half_extents: (26.0, 24.0),
        damage: 4,
        knockback: 65.0,
        knockback_growth: 1.20,
        launch_dir: None,
        on_hit: None,
    });
    n_air.landing_lag_s = Some(0.16);
    n_air.autocancel_after_s = Some(0.24);
    let n_air = feel(n_air, Feel::Poke);

    let mut u_air = strike(Strike {
        id: "air_up",
        clip: "attack",
        startup_s: 0.07,
        active_s: 0.09,
        recover_s: 0.19,
        offset: (2.0, -32.0),
        half_extents: (20.0, 24.0),
        damage: 4,
        knockback: 70.0,
        knockback_growth: 1.35,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    u_air.landing_lag_s = Some(0.16);
    u_air.autocancel_after_s = Some(0.26);
    let u_air = feel(u_air, Feel::Launcher);

    // ── the tilts, which for George are COMMITMENTS ──────────────────────────
    //
    // this is the single most character-defining pair in the table. A tilt is
    // the genre's safe middle option everywhere else — the poke you throw when
    // you do not want to decide. George does not have one. His up-tilt starts
    // more than twice as late as the shared table's and hits more than twice as
    // hard, which is the same trade every one of his slow moves makes.
    let up_tilt = strike(Strike {
        id: "tilt_up",
        clip: "attack",
        startup_s: 0.16,
        active_s: 0.09,
        recover_s: 0.26,
        offset: (10.0, -30.0),
        half_extents: (24.0, 28.0),
        damage: 11,
        knockback: 130.0,
        knockback_growth: 2.20,
        launch_dir: Some((0.1, -1.0)),
        on_hit: None,
    });
    let up_tilt = feel(up_tilt, Feel::Launcher);

    let down_tilt = strike(Strike {
        id: "tilt_down",
        clip: "attack",
        startup_s: 0.17,
        active_s: 0.08,
        recover_s: 0.28,
        offset: (30.0, 14.0),
        half_extents: (26.0, 11.0),
        damage: 11,
        knockback: 135.0,
        knockback_growth: 2.30,
        launch_dir: Some((1.0, -0.20)),
        on_hit: None,
    });
    let down_tilt = feel(down_tilt, Feel::Launcher);

    // ── the smashes ──────────────────────────────────────────────────────────
    //
    // the slowest and hardest in this composition, on a body that already
    // survives longest. That is deliberate and it is the risk: a heavyweight
    // who also lands the biggest hits is only fair because he can never throw
    // one without being seen doing it.
    let mut f_smash = strike(Strike {
        id: "smash_forward",
        clip: "attack",
        startup_s: 0.40,
        active_s: 0.08,
        recover_s: 0.46,
        offset: (46.0, -4.0),
        half_extents: (32.0, 24.0),
        damage: 21,
        knockback: 185.0,
        knockback_growth: 3.45,
        launch_dir: Some((1.0, -0.44)),
        on_hit: None,
    });
    f_smash.smash_charge_mult = 1.7;
    f_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
    // ⭐ THE TIP AND THE BASE. The volume above is the TIP — authored first, so
    // a body reached by both takes it. This is the base: the same commitment
    // landed at the wrong distance, which hurts and does not kill.
    //
    // George's forward smash is his longest commitment (0.40s of windup, 0.46s
    // of recovery), and until now every distance inside its reach paid the
    // same. Spacing it is a skill on the move now without the move becoming
    // two. ⛔ the ORDER in this list is the priority — writing the base first
    // would make every forward smash a base hit and nothing would warn you.
    for window in f_smash.windows.iter_mut().filter(|w| {
        matches!(
            w.tag,
            ambition_platformer2d::entity_catalog::WindowTag::Active
        )
    }) {
        let tip = window.volumes[0].clone();
        window
            .volumes
            .push(ambition_platformer2d::entity_catalog::HitVolume {
                shape: ambition_platformer2d::entity_catalog::VolumeShape::Rect {
                    // Inboard of the tip and overlapping it, so a body between
                    // the two is genuinely reached by both.
                    offset: (16.0, -4.0),
                    half_extents: (18.0, 24.0),
                },
                damage: 11,
                knockback: 82.0,
                knockback_growth: Some(82.0 * crate::SMASH_KNOCKBACK_GROWTH),
                // Flatter and weaker: a base hit puts them beside you, not away.
                launch_dir: Some((1.0, -0.16)),
                ..tip
            });
    }
    let f_smash = feel(f_smash, Feel::Heavy);

    let mut up_smash = strike(Strike {
        id: "smash_up",
        clip: "attack",
        startup_s: 0.36,
        active_s: 0.10,
        recover_s: 0.42,
        offset: (6.0, -38.0),
        half_extents: (26.0, 34.0),
        damage: 19,
        knockback: 178.0,
        knockback_growth: 3.30,
        launch_dir: Some((0.0, -1.0)),
        on_hit: None,
    });
    up_smash.smash_charge_mult = 1.7;
    up_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
    let up_smash = feel(up_smash, Feel::Heavy);

    let mut down_smash = strike(Strike {
        id: "smash_down",
        clip: "attack",
        startup_s: 0.34,
        active_s: 0.11,
        recover_s: 0.44,
        offset: (0.0, 16.0),
        half_extents: (44.0, 13.0),
        damage: 17,
        knockback: 165.0,
        knockback_growth: 3.05,
        launch_dir: Some((0.95, -0.45)),
        on_hit: None,
    });
    down_smash.smash_charge_mult = 1.7;
    down_smash.smash_charge = Some(ambition_platformer2d::entity_catalog::SmashChargeSpec {
        hold_at_s: CHARGE_POSE_AT_S,
        max_hold_s: ambition_platformer2d::entity_catalog::SmashChargeSpec::DEFAULT_MAX_HOLD_S,
        stores: false,
        roots: true,
        sustain: ambition_platformer2d::entity_catalog::ChargeSustain::WhileHeld,
    });
    let down_smash = feel(down_smash, Feel::Heavy);

    // ── the committed aerials ────────────────────────────────────────────────
    //
    // Three of his five aerials are on the slow side of the gap, with landing
    // lag to match. Jumping is not an escape for this body; it is another
    // decision.
    let mut f_air = strike(Strike {
        id: "air_forward",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.09,
        recover_s: 0.28,
        offset: (34.0, -2.0),
        half_extents: (26.0, 20.0),
        damage: 12,
        knockback: 140.0,
        knockback_growth: 2.35,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    f_air.landing_lag_s = Some(0.24);
    f_air.autocancel_after_s = Some(0.34);
    let f_air = feel(f_air, Feel::Poke);

    let mut b_air = strike(Strike {
        id: "air_back",
        clip: "attack",
        startup_s: 0.20,
        active_s: 0.08,
        recover_s: 0.30,
        offset: (-36.0, 0.0),
        half_extents: (26.0, 20.0),
        damage: 14,
        knockback: 155.0,
        knockback_growth: 2.75,
        launch_dir: Some((-1.0, -0.36)),
        on_hit: None,
    });
    b_air.landing_lag_s = Some(0.26);
    b_air.autocancel_after_s = Some(0.36);
    let b_air = feel(b_air, Feel::Heavy);

    // The heaviest landing lag on the grid. A missed spike over the stage is a
    // free smash for whoever is standing under it — which, for a fighter whose
    // whole table is commitments, is the correct punishment.
    let mut d_air = strike(Strike {
        id: "air_down",
        clip: "attack",
        startup_s: 0.22,
        active_s: 0.09,
        recover_s: 0.32,
        offset: (6.0, 32.0),
        half_extents: (22.0, 22.0),
        damage: 15,
        knockback: 150.0,
        knockback_growth: 2.55,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    d_air.landing_lag_s = Some(0.34);
    d_air.autocancel_after_s = Some(0.44);
    let d_air = feel(d_air, Feel::Dive);

    // ── the forward tilt, which was MISSING ──────────────────────────────────
    //
    // without it a grounded forward press fell down the chain to the jab, so
    // the most common input in the genre reached the weakest move in the table
    // and George's ground game was "poke, or spend forty frames". A stride into a
    // shoulder: the tilt that is still a commitment, because that is who he is.
    let mut f_tilt = strike(Strike {
        id: "tilt_forward",
        clip: "attack",
        startup_s: 0.18,
        active_s: 0.08,
        recover_s: 0.27,
        offset: (36.0, -2.0),
        half_extents: (28.0, 18.0),
        damage: 12,
        knockback: 140.0,
        knockback_growth: 2.80,
        launch_dir: Some((1.0, -0.28)),
        on_hit: None,
    });
    // A short stride, ADDITIVE: it contributes to whatever run George brought
    // into it, so the same move covers more ground out of a dash. This is
    // `start_impulse`'s meaning and the right one here — nothing about a shoulder
    // check should erase the momentum behind it.
    f_tilt.start_impulse = Some((190.0, 0.0));
    let f_tilt = feel(f_tilt, Feel::Heavy);

    // ── THE SPECIALS ─────────────────────────────────────────────────────────
    //
    // this table had none, and that was the hole. Four verbs the engine
    // has always resolved (`special` / `special_forward` / `special_up` /
    // `special_down` through the same directional chain as every attack) and
    // that no fighter in this demo had ever bound. None of what follows is a new
    // engine path; three of the four are ordinary strikes with authored numbers,
    // and the fourth needed one primitive that did not exist.

    // NEUTRAL — `bivalence`. Two active windows on one timeline: an early
    // weak pop and a late strong throw. Standing next to George while this
    // charges is a coin flip about which half you eat, and the answer is when you
    // chose to be there. The lingering second window is what a "commitment" is
    // supposed to buy.
    let mut bivalence = strike(Strike {
        id: "bivalence",
        clip: "special",
        startup_s: 0.30,
        active_s: 0.07,
        recover_s: 0.34,
        offset: (0.0, -6.0),
        half_extents: (36.0, 30.0),
        damage: 11,
        knockback: 160.0,
        knockback_growth: 2.00,
        launch_dir: Some((0.2, -1.0)),
        on_hit: None,
    });
    // ⛔ NO `smash_charge_mult` HERE, and it used to carry 1.6. A Special never
    // takes the smash gesture, so the multiplier could never be EARNED — and
    // until the charge payoff had one authority it was paid anyway, off the
    // move's own timeline, on every hit.
    //
    // ⇒ THE 1.6 IS BAKED INTO THE NUMBERS ABOVE AND BELOW, deliberately, so
    // this is the same George who has been fighting: damage 7→11 and 13→21,
    // knockback 100→160 and 170→272, which is exactly what the runtime was
    // computing (it scales damage and knockback base, never growth). Dropping
    // the multiplier without baking it took George out of every recovery
    // situation he had — `the_cpu_throws_its_authored_recovery_during_a_match`
    // went red over 1800 ticks with him otherwise fighting normally.
    //
    // ⚠ whether this SHOULD be his damage is a real question and it is recorded
    // in docs/planning/awaiting-maintainer-decision.md. What is not a question
    // is that the number a reader sees must be the number that lands.
    // The second half, authored as a window rather than a second move: same
    // press, same clock, harder answer.
    {
        let end = bivalence.duration_s;
        bivalence
            .windows
            .push(ambition_platformer2d::entity_catalog::MoveWindow {
                start_s: 0.42,
                end_s: 0.50,
                tag: ambition_platformer2d::entity_catalog::WindowTag::Active,
                volumes: vec![ambition_platformer2d::entity_catalog::HitVolume {
                    // An ordinary hit, not a gust.
                    shape: ambition_platformer2d::entity_catalog::VolumeShape::Circle {
                        offset: (0.0, -4.0),
                        radius: 46.0,
                    },
                    damage: 21,
                    knockback: 272.0,
                    knockback_growth: Some(3.40),
                    launch_dir: Some((0.85, -0.55)),
                    on_hit: None,
                    vfx: Some("slash_arc".to_string()),
                    hit_sfx: None,
                    reaction: None,
                }],
                motion_scale: 0.25,
                sustain_effect: None,
            });
        debug_assert!(end >= 0.50, "the second window must fit inside the move");
    }
    let bivalence = feel(bivalence, Feel::Special);

    // SIDE — `modus_ponens`. *If you are there, then you are here.* A
    // travelling body-check: the burst is `Set`, so it erases whatever George was
    // doing and replaces it with one committed direction, and the tail cannot be
    // steered out of. Thrown offstage it is a real horizontal recovery — and a
    // real way to die, because it also erases the fall you might have drifted
    // out of.
    let side_b = strike(Strike {
        id: "modus_ponens",
        clip: "special",
        startup_s: 0.20,
        active_s: 0.12,
        recover_s: 0.30,
        offset: (40.0, 0.0),
        half_extents: (30.0, 20.0),
        damage: 14,
        knockback: 160.0,
        knockback_growth: 3.20,
        launch_dir: Some((1.0, -0.30)),
        on_hit: None,
    });
    // so the zero stays as a CONTENT decision, not an engine one. George's
    // side special is a horizontal body-check and George's way home is his Up-B;
    // giving this move an arbitrary hop would be tuning it to please a reader.
    //
    // and the honest consequence is that this move is INVISIBLE to the
    // search, even though its own doc above calls it "a real horizontal
    // recovery". `lifting_candidates` filters on `lift_speed > 0`, and a purely
    // horizontal `Set` has none — so a George who could get home by charging
    // sideways is never offered the option. That gap is NAMED and left open on
    // purpose: closing it means the search proposing every displacing move, not
    // widening the lift derivation, and it is a decision about search cost rather
    // than about this table.
    //
    // the guard `the_ascent_commands_its_rise_and_advertises_it` still asserts
    // nothing else in George's table lifts. That assertion is now about GEORGE
    // (one fighter, one way home) rather than about the engine.
    let side_b = impulse(side_b, 0.20, (760.0, 0.0), ImpulseMode::Set);
    let side_b = committed_tail(side_b, 0.74, 0.0);
    let side_b = feel(side_b, Feel::Special);

    // UP — `excluded_middle`. THE RECOVERY.
    //
    // *"Either you are on the stage or you are not"* — and this is the move
    // that decides which. It is the reason `MoveEventKind::Impulse` exists: the
    // rise is COMMANDED (`Set`) at `ASCENT_AT_S`, after a windup, so a George
    // falling at terminal velocity gets exactly the climb a standing one does.
    // `start_impulse` could express neither half — it fires at the press and it
    // ADDS, which makes a recovery weakest precisely when it is needed.
    //
    // it is not flight, and the arithmetic is the reason rather than a
    // cooldown. No `Cancelable` window means the body cannot re-press until the
    // move ends, and the move outlasts its own arc (see `ASCENT_ENDS_S`), so
    // repeated use LOSES height. One press is a save; four presses is a slow
    // descent. That is a property of the authored numbers, held by a test, and
    // it costs no rollback state at all.
    //
    // The hit is deliberately weak: this is a way home that happens to be
    // dangerous to stand under, not a kill move with a rise attached.
    let mut up_b = strike(Strike {
        id: "excluded_middle",
        clip: "special",
        startup_s: ASCENT_AT_S,
        active_s: 0.14,
        recover_s: 0.16,
        offset: (2.0, -30.0),
        half_extents: (24.0, 34.0),
        damage: 6,
        knockback: 95.0,
        knockback_growth: 1.90,
        launch_dir: Some((0.05, -1.0)),
        on_hit: None,
    });
    // Landing out of the ascent costs — the other half of "a recovery is a
    // commitment". Onstage this makes it a bad panic button; offstage it is
    // irrelevant, which is exactly the right shape.
    up_b.landing_lag_s = Some(0.28);
    let up_b = impulse(up_b, ASCENT_AT_S, (0.0, -ASCENT_SPEED), ImpulseMode::Set);
    // The helpless tail. `0.15` leaves George able to nudge his landing and
    // nothing more, which is what makes an edgeguard against this move possible.
    let up_b = committed_tail(up_b, ASCENT_ENDS_S, 0.15);
    let up_b = feel(up_b, Feel::Recovery);

    // DOWN — `reductio`. Assume you are above me; derive a contradiction.
    // A commanded plunge with the pogo technique on contact: connect and George
    // is thrown back up by his own landing, which is the one thing in this table
    // that can happen twice in a row. Offstage it is a stock — for whoever is
    // wrong about who is above whom.
    let mut down_b = strike(Strike {
        id: "reductio",
        clip: "special",
        startup_s: 0.16,
        active_s: 0.24,
        recover_s: 0.20,
        offset: (4.0, 30.0),
        half_extents: (24.0, 26.0),
        damage: 16,
        knockback: 150.0,
        knockback_growth: 3.00,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    down_b.landing_lag_s = Some(0.36);
    let down_b = impulse(down_b, 0.16, (0.0, 1500.0), ImpulseMode::Set);
    let down_b = on_hit(
        down_b,
        ambition_platformer2d::characters::technique::POGO_BOUNCE_KEY,
    );
    let down_b = feel(down_b, Feel::Dive);

    // it also made a census lie. The kit report probed specials standing
    // on the ground, found `dspecial` resolving to the neutral-B, and recorded
    // George as missing a down-B he has had all along. The census asks both
    // postures now; this is the move it was asking for.
    //
    // DOWN, ON THE GROUND — `reductio_ad_absurdum`. Assume you are above me.
    // With his feet on the stage that assumption is false, so he MAKES it true
    // first: a short arc up, and then the same contradiction, derived on the way
    // down. the plunge impulse and the active window are `reductio`'s numbers
    // — this is the same argument with a premise added, not a second move.
    let ground_down_b = strike(Strike {
        id: "reductio_ad_absurdum",
        clip: "special",
        startup_s: 0.34,
        active_s: 0.24,
        recover_s: 0.22,
        offset: (4.0, 30.0),
        half_extents: (24.0, 26.0),
        damage: 16,
        knockback: 150.0,
        knockback_growth: 3.00,
        launch_dir: Some((0.0, 1.0)),
        on_hit: None,
    });
    // THE ARC IS AN `Add`, AND THE UP-B'S POISON IS WHY. `strike`'s
    // frame data derives `lift_speed` from `Set` impulses only — *"an `Add`
    // states no speed, so no static reader may claim one for it"* — and
    // `excluded_middle` is the one move in this table allowed to advertise a way
    // home. Written as a `Set`, this hop told the recovery policy that George's
    // DOWN-B is a recovery: offstage the CPU would press it and slam itself into
    // the blast zone. The test below caught it, which is what it is for.
    //
    // and an `Add` is honest here for the same reason it is wrong on the up-B:
    // this move is grounded-only, so he is standing still when it fires and there
    // is no momentum for it to compose with.
    let ground_down_b = impulse(ground_down_b, 0.10, (200.0, -620.0), ImpulseMode::Add);
    let ground_down_b = impulse(ground_down_b, 0.34, (0.0, 1500.0), ImpulseMode::Set);
    let ground_down_b = on_hit(
        ground_down_b,
        ambition_platformer2d::characters::technique::POGO_BOUNCE_KEY,
    );
    let ground_down_b = committed_tail(ground_down_b, 0.86, 0.10);
    let ground_down_b = feel(ground_down_b, Feel::Dive);

    // ── The hold ────────────────────────────────────────────────────────────
    //
    // the sixteen slots above stayed in Rust on purpose. They are built by
    // COMPOSING `strike` / `impulse` / `on_hit` / `committed_tail` / `feel`, and
    // the `debug_assert` below states a law about the shape of this whole table.
    // That composition is the design; flattening it into RON would trade authored
    // reasoning for a wall of numbers.
    let capture = crate::smash_pack::capture_kit(crate::SMASH_GEORGE_BOOUL);

    let repertoire = SmashRepertoire {
        taunt: ambition_platformer2d::characters::moveset_authoring::taunt(
            "george_booul_taunt",
            0.9,
        ),

        // GEORGE'S DASH ATTACK IS A COMMITMENT, and his own law decided
        // that. `no_move_lives_between_the_pokes_and_the_commitments` splits
        // his kit at `POKE_MAX_STARTUP_S`, and `the fast half must be the weak
        // half` — his pokes top out at 5 damage where his softest commitment is
        // 6. A 14-damage move cannot be fast HERE, so the genre's 0.05 startup
        // becomes `COMMIT_MIN_STARTUP_S`. that is not the law getting in the
        // way: George is the heavy, and a shoulder charge you can see coming is
        // what the heavy's dash attack should be.
        dash_attack: ambition_platformer2d::characters::moveset_authoring::dash_attack(
            "george_booul_dash_attack",
            ambition_platformer2d::characters::moveset_authoring::DashAttackShape {
                startup_s: COMMIT_MIN_STARTUP_S,
                ..ambition_platformer2d::characters::moveset_authoring::DashAttackShape::GENRE
            },
            14,
            175.0,
        ),
        jab,
        forward_tilt: f_tilt,
        up_tilt,
        down_tilt,
        forward_smash: f_smash,
        up_smash,
        down_smash,
        neutral_air: n_air,
        forward_air: f_air,
        back_air: b_air,
        up_air: u_air,
        down_air: d_air,
        neutral_special: NeutralSpecial::Authored(bivalence),
        side_special: side_b,
        up_special: UpSpecial::Standard(up_b),
        capture,
        down_special: DownSpecial::ByPosture {
            grounded: ground_down_b,
            airborne: down_b,
        },
    }
    .into_contract();

    // ⭐ THE STRING, FROM THE ONE PLACE IT IS AUTHORED. The verb map has a slot
    // for the jab and none for what follows it, because a chain is a cancel
    // table over ordinary moves rather than a verb — so the continuations join
    // the table directly. ⛔ NOT a second copy: George's own moveset is why the
    // chain reached nobody, and a copy would put the same trap one edit away.
    let mut repertoire = repertoire;
    repertoire
        .moves
        .extend(crate::moveset::jab_string_continuations());

    // the disjunction is checked WHERE IT IS AUTHORED, not only in the
    // test module. These two numbers are the character; a move edited into the
    // band between them stops being George's before anything else notices, and
    // this is the last place that still knows both halves at once.
    debug_assert!(
        repertoire.moves.iter().all(|m| {
            let startup = m
                .windows
                .iter()
                .find(|w| {
                    matches!(
                        w.tag,
                        ambition_platformer2d::entity_catalog::WindowTag::Active
                    )
                })
                .map_or(0.0, |w| w.start_s);
            startup <= POKE_MAX_STARTUP_S || startup >= COMMIT_MIN_STARTUP_S
        }),
        "a George move landed between the pokes and the commitments"
    );

    repertoire
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d::entity_catalog::{MoveSpec, WindowTag};

    fn find(set: &MovesetContract, id: &str) -> MoveSpec {
        set.moves
            .iter()
            .find(|m| m.id == id)
            .unwrap_or_else(|| panic!("{id} exists"))
            .clone()
    }

    /// The tell before a move becomes dangerous — `None` for a move that has no
    /// dangerous moment to lead into.
    ///
    /// it was an `.expect("a strike has an active window")` and the capture
    /// beats broke it, correctly. A pummel and a throw have NO Active window
    /// by design: they reach for nobody, because the target was selected when
    /// the capture was established. The law below is about the tell, and a move
    /// that cannot miss has none.
    fn startup(m: &MoveSpec) -> Option<f32> {
        m.windows
            .iter()
            .find(|w| matches!(w.tag, WindowTag::Active))
            .map(|w| w.start_s)
    }

    fn damage(m: &MoveSpec) -> i32 {
        m.windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .map(|v| v.damage)
            .max()
            .unwrap_or(0)
    }

    // Fourteen fighters each carried a copy of it: every bound verb names a move
    // this table defines, and the table binds the whole vocabulary. Both are now
    // unwritable defects rather than tested ones. `SmashRepertoire` owns the verb
    // strings, so there is no string in this file to misspell; it is a struct
    // with no `Default` and no private fields, so a missing or renamed slot is a
    // COMPILE error here. What the fourteen copies stood for — that every press
    // is answered, in every posture it is asked in — is checked once, by
    // `ambition_platformer2d::characters::smash_repertoire`, and by the host ratchet
    // `smash_roster_movesets::report_the_smash_kit_every_selectable_fighter_has`.

    /// THE EXCLUDED MIDDLE, AS AN ASSERTION.
    ///
    /// the claim the module doc makes and the one thing that cannot survive a
    /// careless retune: every move is a poke or a commitment, and the band
    /// between them is empty. A move that drifted into it would be a perfectly
    /// reasonable tilt and would quietly make George somebody else.
    #[test]
    fn no_move_lives_between_the_pokes_and_the_commitments() {
        let george = george_booul_moveset();

        // WHO IS EXEMPT, BY NAME. Six moves have no tell, and none of
        // them reaches for anybody: a pummel and FOUR throws, whose target was
        // already selected, and the TAUNT, whose whole content is that it buys
        // nothing. Pinning the list means a STRIKE that lost its Active window
        // fails here instead of quietly leaving the law it is supposed to obey.
        // the grab is NOT on it: a grab reaches, so a grab has a tell, and it
        // obeys the band like everything else (it was authored at `0.14` and
        // this caught it).
        let mut telless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_none())
            .map(|m| m.id.as_str())
            .collect();
        telless.sort_unstable();
        assert_eq!(
            telless,
            vec![
                "george_booul_taunt",
                "george_bthrow",
                "george_dthrow",
                "george_fthrow",
                "george_pummel",
                "george_uthrow",
            ],
            "the set of moves with no Active window changed"
        );

        for m in &george.moves {
            let Some(s) = startup(m) else { continue };
            assert!(
                s <= POKE_MAX_STARTUP_S || s >= COMMIT_MIN_STARTUP_S,
                "`{}` starts at {s}s, inside the band this fighter does not have \
                 ({POKE_MAX_STARTUP_S}..{COMMIT_MIN_STARTUP_S})",
                m.id
            );
        }

        // and the two halves are separated by PAYOFF, not only by timing —
        // otherwise "slow" would just mean "slow", and the disjunction would be
        // about the clock rather than about the decision.
        //
        // pinned by name for the same reason the tell exemption is: a SMASH
        // that lost its volumes would otherwise quietly become the softest
        // commitment and take the assertion down with it, or worse, satisfy it.
        let mut payless: Vec<&str> = george
            .moves
            .iter()
            .filter(|m| startup(m).is_some() && damage(m) == 0)
            .map(|m| m.id.as_str())
            .collect();
        payless.sort_unstable();
        assert_eq!(
            payless,
            // ⭐ the RUNNING grab is here for the same reason the standing one
            // is, and it is not authored: the capture kit derives it from this
            // fighter's own grab. ⚠ that also means the startup band above now
            // constrains a DERIVED move — if george's grab ever starts close
            // enough to `POKE_MAX_STARTUP_S`, the derived wind-up can push its
            // variant into the band this fighter says it does not have, and the
            // assertion above is what would say so.
            vec!["george_grab", "george_grab_dash"],
            "the set of moves that reach and deal no damage changed"
        );

        let (pokes, commits): (Vec<_>, Vec<_>) = george
            .moves
            .iter()
            // A move with no tell is neither a poke nor a commitment, and a move
            // with no damage payoff is not what this claim measures.
            .filter(|m| startup(m).is_some() && damage(m) > 0)
            .partition(|m| startup(m).unwrap_or_default() <= POKE_MAX_STARTUP_S);
        let hardest_poke = pokes.iter().map(|m| damage(m)).max().expect("pokes exist");
        let softest_commit = commits
            .iter()
            .map(|m| damage(m))
            .min()
            .expect("commitments exist");
        assert!(
            hardest_poke < softest_commit,
            "the fast half must be the weak half ({hardest_poke} vs {softest_commit})"
        );

        // the poison. The shared table has a real middle — its tilts sit
        // at 0.06–0.07 and its aerials climb through 0.09, 0.10, 0.12 — so if
        // this assertion ever passed for BOTH tables, the band would be
        // describing nothing.
        let shared = crate::moveset::fighter_moveset();
        assert!(
            shared.moves.iter().any(|m| {
                startup(m).is_some_and(|s| s > POKE_MAX_STARTUP_S && s < COMMIT_MIN_STARTUP_S)
            }),
            "the shared repertoire is supposed to HAVE a middle; if it does not, \
             this whole test is asserting a property of the threshold rather \
             than a property of George"
        );
    }

    /// comparative for the same reason the goblin's and the admiral's tests
    /// are: a table copied wholesale and renumbered would pass every other test
    /// in this file.
    #[test]
    fn george_commits_longer_and_hits_harder_than_the_shared_repertoire() {
        let george = george_booul_moveset();
        let shared = crate::moveset::fighter_moveset();
        for id in ["smash_forward", "smash_up", "smash_down"] {
            let (g, s) = (find(&george, id), find(&shared, id));
            // `expect`, not a filter: these three ARE strikes, and a smash
            // that lost its Active window is a defect rather than an exemption.
            let (gs, ss) = (
                startup(&g).expect("a smash has an active window"),
                startup(&s).expect("a smash has an active window"),
            );
            assert!(gs > ss, "`{id}`: the heavy commits longer ({gs} vs {ss})");
            assert!(
                damage(&g) > damage(&s),
                "`{id}`: and is paid for it ({} vs {})",
                damage(&g),
                damage(&s)
            );
        }

        // And nowhere is he FASTER. A heavyweight that also had the quicker
        // option somewhere would just be stronger.
        //
        // The count below is what stops the filter from quietly emptying the loop.
        let mut compared = 0;
        for m in &george.moves {
            let Some(s) = shared.moves.iter().find(|other| other.id == m.id) else {
                continue;
            };
            compared += 1;
            // Both sides are shared-table moves, so both are strikes; a `None`
            // here means one lost its Active window and should say so loudly.
            let (gs, ss) = (
                startup(m).expect("a shared-table move has an active window"),
                startup(s).expect("a shared-table move has an active window"),
            );
            assert!(
                gs >= ss,
                "`{}` is quicker than the shared table's ({gs} vs {ss})",
                m.id
            );
        }
        assert!(
            compared >= 11,
            "only {compared} moves were comparable; the two tables have stopped \
             overlapping and this test is asserting nothing"
        );
    }
    // ── the specials ─────────────────────────────────────────────────────────

    /// THE ASCENT IS A SAVE, NOT A FLIGHT — and the arithmetic is the reason.
    ///
    /// this is the guard that lets the Up-B exist with no cooldown, no
    /// per-airtime counter and no new rollback state. The body cannot re-press
    /// while the move is playing (no `Cancelable` window), so the only question
    /// is whether one full cycle gains height. It cannot: the move outlasts its
    /// own arc, so by the time George may press again he has fallen back through
    /// everything the burst bought and then some.
    #[test]
    fn the_ascent_is_a_save_and_not_a_flight() {
        let g = ambition_platformer2d::engine_core::DEFAULT_TUNING.gravity;
        let to_apex = ASCENT_SPEED / g;
        let tail = ASCENT_ENDS_S - ASCENT_AT_S;
        assert!(
            tail > 2.0 * to_apex,
            "the ascent climbs for {to_apex:.3}s and is handed back {tail:.3}s \
             after the burst; anything at or under {:.3}s returns George higher \
             than it found him, every press, which is flight",
            2.0 * to_apex
        );
        // And the windup is real: a recovery with no tell is a free escape.
        assert!(ASCENT_AT_S >= COMMIT_MIN_STARTUP_S);
        // Landing out of it costs, so it is a bad panic button ON the stage.
        let up_b = find(&george_booul_moveset(), "excluded_middle");
        assert!(up_b.landing_lag_s.unwrap_or(0.0) > 0.0);
    }

    /// THE RISE IS COMMANDED, NOT CONTRIBUTED.
    ///
    /// the whole difference between a recovery and a hop. Under
    /// `ImpulseMode::Add` a George falling at terminal velocity would climb at
    /// whatever was left over — the move would be weakest exactly when it is the
    /// only thing between him and the blast zone. `Set` makes the climb a
    /// property of the MOVE.
    ///
    /// and the same fact is what every policy layer reads: `lift_speed` is
    /// derived from `Set` impulses only, so this assertion is also the assertion
    /// that the brain and the recovery probe can see this move at all.
    #[test]
    fn the_ascent_commands_its_rise_and_advertises_it() {
        use ambition_platformer2d::entity_catalog::{ImpulseMode, MoveEventKind};
        let up_b = find(&george_booul_moveset(), "excluded_middle");
        let burst = up_b
            .events
            .iter()
            .find_map(|e| match &e.kind {
                MoveEventKind::Impulse { local, mode } => Some((e.at_s, *local, *mode)),
                _ => None,
            })
            .expect("the recovery special displaces its owner");
        assert_eq!(burst.2, ImpulseMode::Set);
        assert!(burst.1 .1 < 0.0, "the burst must point AGAINST gravity");
        assert_eq!(burst.0, ASCENT_AT_S);

        // The derived affordance, which is what the brain and the recovery probe
        // both consume. If this is zero the move is invisible to both of them and
        // the CPU goes back to drifting at a stage it cannot reach.
        let frames = up_b.frame_data();
        assert_eq!(frames.lift_speed, ASCENT_SPEED);
        assert_eq!(frames.lift_at_s, ASCENT_AT_S);

        // the poison: nothing ELSE in the table advertises a lift. A table
        // where every move looked like a recovery would satisfy the assertion
        // above and tell a policy layer nothing.
        let table = george_booul_moveset();
        let others: Vec<&str> = table
            .moves
            .iter()
            .filter(|m| m.id != "excluded_middle" && m.frame_data().lift_speed > 0.0)
            .map(|m| m.id.as_str())
            .collect();
        assert!(
            others.is_empty(),
            "these moves also claim to be ways home: {others:?}"
        );
    }

    /// FOUR SPECIALS, FOUR MECHANISMS.
    ///
    /// the brief this table exists to answer forbids *"rotated or mirrored
    /// clones of one base melee"*, and four specials built out of the same strike
    /// with different offsets would be exactly that. So the assertion is about
    /// MECHANISM: one commands a rise, one commands a plunge and rebounds off
    /// what it hits, one commands a horizontal charge it cannot steer out of, and
    /// one lands twice on one press. No two share a mechanism, and none of them
    /// is any other one rotated.
    #[test]
    fn the_four_specials_are_four_different_mechanisms() {
        use ambition_platformer2d::entity_catalog::{ImpulseMode, MoveEventKind, WindowTag};
        let set = george_booul_moveset();
        let commanded = |id: &str| -> Option<(f32, f32)> {
            find(&set, id).events.iter().find_map(|e| match &e.kind {
                MoveEventKind::Impulse {
                    local,
                    mode: ImpulseMode::Set,
                } => Some(*local),
                _ => None,
            })
        };
        // Up: a rise, and only a rise.
        let up = commanded("excluded_middle").expect("the Up-B displaces");
        assert!(up.1 < 0.0 && up.0 == 0.0);
        // Down: a plunge, and it rebounds off a body.
        let down = commanded("reductio").expect("the dive displaces");
        assert!(down.1 > 0.0);
        assert!(find(&set, "reductio")
            .windows
            .iter()
            .flat_map(|w| w.volumes.iter())
            .any(|v| v.on_hit.is_some()));
        // Side: a horizontal charge with a tail that cannot be steered.
        let side = commanded("modus_ponens").expect("the side special travels");
        assert!(side.0 > 0.0);
        assert!(
            find(&set, "modus_ponens")
                .windows
                .iter()
                .any(|w| matches!(w.tag, WindowTag::Recovery) && w.motion_scale == 0.0),
            "a charge you can steer out of is not a commitment"
        );
        // Neutral: no displacement at all — it lands TWICE instead.
        assert!(commanded("bivalence").is_none());
        assert_eq!(
            find(&set, "bivalence")
                .windows
                .iter()
                .filter(|w| matches!(w.tag, WindowTag::Active))
                .count(),
            2,
            "the neutral special's whole idea is the second window"
        );
    }

    /// EVERY PRESS A BODY CAN MAKE REACHES A MOVE, IN BOTH POSTURES.
    #[test]
    fn both_postures_reach_at_least_eight_distinct_moves() {
        use ambition_platformer2d::entity_catalog::AttackDir;
        let set = george_booul_moveset();
        let reachable = |grounded: bool| -> std::collections::BTreeSet<String> {
            let mut ids = std::collections::BTreeSet::new();
            for base in ["attack", "smash", "special"] {
                for dir in [
                    AttackDir::Neutral,
                    AttackDir::Forward,
                    AttackDir::Back,
                    AttackDir::Up,
                    AttackDir::Down,
                ] {
                    if let Some(m) = set.move_for_directional_verb(base, dir, grounded) {
                        ids.insert(m.id.clone());
                    }
                }
            }
            ids
        };
        let on_ground = reachable(true);
        let airborne = reachable(false);
        assert!(
            on_ground.len() >= 8,
            "a grounded George reaches only {:?}",
            on_ground
        );
        assert!(
            airborne.len() >= 8,
            "an airborne George reaches only {:?}",
            airborne
        );
        // The recovery is reachable from BOTH — a move you have to fall off the
        // stage to practise is a move nobody learns.
        assert!(on_ground.contains("excluded_middle"));
        assert!(airborne.contains("excluded_middle"));
        // and the forward press no longer falls through to the jab.
        assert_eq!(
            set.move_for_directional_verb("attack", AttackDir::Forward, true)
                .map(|m| m.id.as_str()),
            Some("tilt_forward")
        );
    }

    /// THE FEEDBACK IS DIFFERENTIATED, AND IT IS RESOLVABLE.
    ///
    /// Two claims in one test because they fail together: a table where every move sounds the
    /// same has no feedback, and a table naming an effect no shipped spritesheet carries has
    /// feedback that silently never plays.
    #[test]
    fn important_moves_sound_and_look_like_themselves() {
        use ambition_platformer2d::entity_catalog::MoveEventKind;
        let set = george_booul_moveset();
        let mut effects = std::collections::BTreeSet::new();
        let mut cues = std::collections::BTreeSet::new();
        for m in &set.moves {
            for problem in
                m.presentation_problems(ambition_platformer2d::sprite_sheet::fx::is_authored_effect)
            {
                panic!("{problem}");
            }
            for ev in &m.events {
                match &ev.kind {
                    MoveEventKind::Vfx { effect, .. } => {
                        effects.insert(effect.clone());
                    }
                    MoveEventKind::Sfx { cue } => {
                        cues.insert(cue.clone());
                    }
                    _ => {}
                }
            }
        }
        assert!(
            effects.len() >= 4,
            "a jab, a smash, a launcher, a special and a recovery cannot all \
             look the same: {effects:?}"
        );
        assert!(cues.len() >= 3, "{cues:?}");
        // The recovery ACTIVATING has its own burst — seeing one is how you know
        // a fighter is not dead yet.
        let up_b = find(&set, "excluded_middle");
        assert!(up_b.events.iter().any(|e| matches!(
            &e.kind,
            MoveEventKind::Vfx { effect, .. } if effect == "classic_burst"
        )));
        // And a heavy landing is heard apart from a poke landing.
        let heavy_hit = |id: &str| -> Option<String> {
            find(&set, id)
                .windows
                .iter()
                .flat_map(|w| w.volumes.iter())
                .find_map(|v| v.hit_sfx.clone())
        };
        assert_ne!(heavy_hit("smash_forward"), heavy_hit("jab"));
        assert!(heavy_hit("smash_forward").is_some());
        assert!(heavy_hit("jab").is_none(), "a jab does not clang");
    }

    /// THE JAB'S TWO CANCELS ARE DIFFERENT PROMISES, and the difference is the
    /// point: the STRING continues on a whiff because every game in this genre
    /// lets you jab at empty air three times, and the ROUTE across George's own
    /// gap is a REWARD FOR CONNECTING, so it stays `OnHit` and a whiff buys
    /// nothing.
    ///
    /// ⛔ read by what each window NAMES, never by which comes first: the
    /// version of this test that took the first `Cancelable` window it found
    /// started reading the string's the moment the string was authored, and
    /// would have gone on asserting about a window it was not written for.
    #[test]
    fn the_jab_strings_on_a_whiff_and_opens_the_commitments_only_when_it_lands() {
        use ambition_platformer2d::entity_catalog::{CancelCondition, WindowTag};
        let jab = find(&george_booul_moveset(), "jab");
        let cancels: Vec<(Vec<String>, CancelCondition)> = jab
            .windows
            .iter()
            .filter_map(|w| match &w.tag {
                WindowTag::Cancelable { into, condition } => Some((into.clone(), *condition)),
                _ => None,
            })
            .collect();
        let named = |target: &str| {
            cancels
                .iter()
                .find(|(into, _)| into.iter().any(|t| t == target))
                .unwrap_or_else(|| panic!("no cancel window names `{target}`"))
        };
        assert_eq!(
            named("jab2").1,
            CancelCondition::Always,
            "a whiffed jab must still string"
        );
        let route = named("smash");
        assert_eq!(
            route.1,
            CancelCondition::OnHit,
            "George's route across the gap is bought by connecting"
        );
        assert!(route.0.iter().any(|t| t == "special"));
        // ⭐ AND THE STRING IS NAMED FIRST. The chain takes the first successor
        // it can resolve BY MOVE ID, so an undirected follow-up has to reach
        // `jab2`; naming the route first would have made every held button
        // throw a smash.
        assert_eq!(
            cancels[0].0.first().map(String::as_str),
            Some("jab2"),
            "the string has to be the first thing the jab nominates"
        );
    }
}
