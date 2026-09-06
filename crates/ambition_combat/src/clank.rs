//! TWO ATTACKS MEETING — hitbox-vs-hitbox arbitration, before either reaches a
//! victim.
//!
//! ⭐⭐ THE MISSING INTERACTION, and it is one a player sees constantly. Before
//! this, two fighters swinging into each other simply BOTH connected: each
//! attack found the other's body, both took damage, both were launched. Every
//! game in this genre resolves that meeting first — the attacks trade, both are
//! refused, and neither body is hit.
//!
//! ⭐ THE RULE IS RESEARCH, NOT A DECISION. Melee, Brawl, Smash 4 and Ultimate
//! all compare the two attacks' DAMAGE and all four use a threshold in the same
//! neighbourhood: close enough and both are cancelled; far enough apart and the
//! stronger one wins outright and continues untouched. That is the rule here,
//! with the threshold as a ruleset knob
//! ([`ResolvedCombatTuning::clank_damage_window`]) rather than one game's frame
//! data transcribed — where the games differ, this engine ships the knob.
//!
//! ⛔⛔ AND AN UNDECLARED WORLD DOES NOT CLANK. The knob defaults to `0.0`,
//! which refuses every pair, so Ambition's rooms behave exactly as they did.
//! Clanking is something a fighting game asks for.
//!
//! ## Why this is its own system rather than an arm of `apply_hitbox_damage`
//!
//! The question is about a PAIR OF ATTACKS and has no victim in it, while every
//! other rule in the damage sweep is about one attack and one victim. Folding it
//! in would mean asking a victim loop a question that does not mention the
//! victim, and — the deciding reason — the answer has to be known for BOTH
//! attacks before EITHER resolves. A sweep that arbitrated as it went would let
//! whichever hitbox the query yielded first land before the trade was known.

use bevy::prelude::*;

use crate::strike::Hitbox;
use ambition_platformer2d_core as ae;

/// Two attacks met and both were refused.
///
/// ⭐ THE PAIR, not a winner: a clank has no winner by definition, and the
/// message names both owners so a consumer can recoil both. The one-sided case
/// (a much stronger attack beating a weaker one) is NOT announced here — nothing
/// happened to the stronger attack, and a message saying "this attack continued"
/// would be a message on every ordinary frame.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttacksClanked {
    /// The two bodies whose ATTACKS traded, ascending.
    ///
    /// ⛔ THE OWNERS, NOT THE VOLUMES, and that is the fix rather than a
    /// simplification: an attack is what clanks. Naming volumes let a two-volume
    /// attack meeting a two-volume attack announce four trades and rebound the
    /// same two fighters four times.
    ///
    /// ⛔ ORDERED, and it is the SORT that makes it deterministic rather than
    /// the sweep: a clank has no first party, so the pair is canonicalised here
    /// instead of carrying whichever the loop happened to reach first.
    pub owners: (Entity, Entity),
}

/// How two opposed attacks' damage resolves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClankVerdict {
    /// Close enough: both attacks are refused.
    BothRefused,
    /// Far enough apart: the stronger one continues and the weaker is cancelled.
    StrongerWins,
}

/// The comparison, lifted out so it can be tested without a world.
///
/// ⛔ `window <= 0.0` REFUSES EVERY PAIR. That is the undeclared-world answer
/// and it is asked here rather than at the call site, so a ruleset that never
/// declared clanking cannot get it by accident through some other road.
pub fn clank_verdict(a_damage: i32, b_damage: i32, window: f32) -> Option<ClankVerdict> {
    if window <= 0.0 {
        return None;
    }
    let difference = (a_damage - b_damage).abs() as f32;
    Some(if difference > window {
        ClankVerdict::StrongerWins
    } else {
        ClankVerdict::BothRefused
    })
}

/// Arbitrate every pair of opposed, overlapping strike volumes, then cancel the
/// losers — before `apply_hitbox_damage` asks any of them about a victim.
///
/// ⭐ CANCELLING IS A DESPAWN, and deliberately not a new component. A strike
/// volume's life already ends by despawn ([`HitboxLifetime`]), the whole volume
/// entity family is already rollback-snapshotted, and a "spent" flag would be
/// new canonical state for a fact that is exactly "this volume is over".
pub fn arbitrate_attack_clanks(
    mut commands: Commands,
    // ⛔⛔ `StrikeVolume`, NOT `HitboxLifetime`. The first version filtered on the
    // lifetime component, and `advance_move_playback` spawns authored volumes
    // with a comment reading *"NO `HitboxLifetime` on purpose"* — the authored
    // Active window is their despawn authority. So every Smash jab, tilt, smash
    // and aerial was invisible to this system, and the tests that passed spawned
    // synthetic boxes carrying exactly the component production refuses.
    //
    // ⛔⛔ AND THE `SimId` IS `Option`, WHICH IS NOT A CONVENIENCE. A volume takes
    // its id from its OWNER, so a body outside the identified population spawns
    // volumes with none — and REQUIRING the component silently excluded them,
    // which is the same defect one layer in. Measured: a two-fighter fixture
    // produced two strike volumes and zero ids, and the sweep saw nothing.
    strikes: Query<(
        &Hitbox,
        &crate::moveset::StrikeVolume,
        Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
    )>,
    owner_pos: Query<&ae::BodyKinematics>,
    // ⭐⭐ ONLY GROUNDED ATTACKS CLANK, and that is research rather than tuning.
    // In this genre an aerial passes THROUGH an opposing attack — clanking is a
    // ground-game rule, and it is what keeps the air a place where committing
    // costs you. ⛔⛔ omitting it was measured, not argued: with aerials
    // clanking, two CPU fighters traded so constantly that
    // `every_live_fighter_stays_inside_the_frame` reported ZERO body-frames
    // outside the stage in a whole match — nobody was ever launched, because
    // nearly every exchange in the air ended in a refusal.
    factions: Query<&crate::components::ActorFaction>,
    teams: Query<&crate::targeting::MatchTeam>,
    mut playing: Query<&mut crate::moveset::MovePlayback>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
    mut clanked: MessageWriter<AttacksClanked>,
) {
    let rules = tuning.as_deref().copied().unwrap_or_default();
    if rules.clank_damage_window <= 0.0 {
        return;
    }
    // ⛔⛔ ORDERED BY `SimId`, NOT BY `Entity`. An `Entity` is an ALLOCATOR
    // identity: two peers whose archetypes filled differently hand out different
    // indices for the same volume, so a sweep ordered by it arbitrates the same
    // frame's pairs in different sequences and cancels different attacks.
    // `SimId::strike_volume` is derived from `(owner, move, window, volume)` and
    // is the same on every peer, which is what canonical gameplay ordering means.
    let mut live: Vec<(&Hitbox, Entity, &str, Entity)> = strikes
        .iter()
        .map(|(hitbox, volume, sim_id)| {
            (
                hitbox,
                volume.owner,
                sim_id.map(|id| id.as_str()).unwrap_or(""),
                volume.owner,
            )
        })
        .collect();
    // ⭐ `(SimId, owner)`, and the second key is only reached by volumes with no
    // id at all. Those belong to bodies outside the rollback-tracked population,
    // so they cannot desync a peer — and ordering them by owner keeps the sweep
    // total rather than leaving ties to the query's own order.
    live.sort_by(|a, b| a.2.cmp(b.2).then(a.3.cmp(&b.3)));

    // ⭐⭐ ONE RESOLUTION PER ATTACK PAIR. Arbitrating per VOLUME let a two-volume
    // attack meet a two-volume attack four times: four messages, and a rebound
    // applied four times to the same two fighters. The pair of OWNERS is the
    // contest — an attack is what clanks, not a rectangle.
    let mut resolved: std::collections::BTreeSet<(Entity, Entity)> =
        std::collections::BTreeSet::new();
    // Whose MOVE this sweep ended. Collected and applied after, so a body that
    // loses to two attackers on one tick is ended once.
    let mut ended: std::collections::BTreeSet<Entity> = std::collections::BTreeSet::new();

    for (index, (a, a_owner, _, _)) in live.iter().enumerate() {
        for (b, b_owner, _, _) in live.iter().skip(index + 1) {
            if a_owner == b_owner {
                continue;
            }
            let pair = if a_owner <= b_owner {
                (*a_owner, *b_owner)
            } else {
                (*b_owner, *a_owner)
            };
            // ⛔⛔ `resolved` IS THE DEDUP; `ended` IS NOT AN ELIGIBILITY GATE.
            // Skipping a pair because either owner was already ended let an
            // EARLIER pair's outcome decide whether a LATER pair was CONSIDERED
            // at all — measured with three equal attacks on one tick: A/B
            // resolves first by `SimId`, both end, A/C and B/C are skipped, and
            // C survives BECAUSE OF ID ORDER. Deterministic, and not
            // simultaneous.
            //
            // ⭐ `ended` is a COMMIT LEDGER, applied after the sweep, exactly as
            // its own comment below says: a body that loses to two attackers on
            // one tick is ended once. Reading it here made it a third thing.
            if resolved.contains(&pair) {
                continue;
            }
            if !opposed(*a_owner, *b_owner, &factions, &teams, rules) {
                continue;
            }
            // ⛔⛔ BOTH WERE GROUND ATTACKS WHEN THEY CAME OUT, asked of the
            // MOVE this system already holds. Asking `BodyGroundState` at
            // COLLISION time meant a ground attack stopped clanking the moment
            // its owner walked off a ledge mid-swing, and an aerial started
            // clanking when its owner landed. "Grounded attack" is a
            // CLASSIFICATION and it is settled when the swing comes out.
            //
            // ⭐ NO NEW CHANNEL: `MovePlayback::started_grounded` is the same
            // stance the SELECTOR used to choose this variant, and the loser's
            // playback is already in this system's hand to be cancelled.
            let swung_from_the_floor = |owner: Entity| {
                playing
                    .get(owner)
                    .map(|playback| playback.started_grounded)
                    .unwrap_or(true)
            };
            if !swung_from_the_floor(*a_owner) || !swung_from_the_floor(*b_owner) {
                continue;
            }
            let (Ok(a_kin), Ok(b_kin)) = (owner_pos.get(*a_owner), owner_pos.get(*b_owner)) else {
                continue;
            };
            if !a
                .world_volume(a_kin.pos)
                .intersects(&b.world_volume(b_kin.pos))
            {
                continue;
            }
            let Some(verdict) = clank_verdict(a.damage, b.damage, rules.clank_damage_window) else {
                continue;
            };
            resolved.insert(pair);
            match verdict {
                ClankVerdict::BothRefused => {
                    ended.insert(*a_owner);
                    ended.insert(*b_owner);
                    clanked.write(AttacksClanked { owners: pair });
                }
                ClankVerdict::StrongerWins => {
                    // ⛔ THE WEAKER ATTACK ENDS, NOT ITS RECTANGLE. Despawning
                    // one volume left the losing MOVE playing, so its sibling
                    // volumes and every later window carried on — a rectangle
                    // losing a contest the mechanic describes as an attack
                    // losing. ⭐ NOT announced: nothing happened to the winner,
                    // and the loser's owner learns it the way it learns any whiff.
                    ended.insert(if a.damage < b.damage {
                        *a_owner
                    } else {
                        *b_owner
                    });
                }
            }
        }
    }

    for owner in ended {
        if let Ok(mut playback) = playing.get_mut(owner) {
            crate::moveset::cancel_move_playback(&mut commands, owner, &mut playback, crate::moveset::MoveEnd::Interrupted);
        }
    }
}

/// May these two bodies' attacks meet at all?
///
/// The same question the damage sweep asks about an attack and a victim, asked
/// about two ATTACKERS — so a team-mate's swing passes through yours exactly as
/// their hit would.
fn opposed(
    a: Entity,
    b: Entity,
    factions: &Query<&crate::components::ActorFaction>,
    teams: &Query<&crate::targeting::MatchTeam>,
    rules: crate::rules::ResolvedCombatTuning,
) -> bool {
    // Teams outrank faction, exactly as they do for damage: two humans in a
    // match share a faction and are still opponents.
    if let (Ok(a_team), Ok(b_team)) = (teams.get(a), teams.get(b)) {
        return a_team != b_team;
    }
    let (Ok(a_faction), Ok(b_faction)) = (factions.get(a), factions.get(b)) else {
        return false;
    };
    crate::targeting::can_damage(*a_faction, *b_faction, rules.friendly_fire())
}

/// What a trade COSTS both fighters: their moves end, and both are thrown back.
///
/// ⭐⭐ WITHOUT THIS A CLANK IS NOT A MECHANIC. Cancelling the volumes alone
/// leaves both fighters standing where they were, playing an animation that can
/// no longer hit anything — which reads as the game having dropped two inputs.
/// The genre ends both attacks and pushes both bodies apart, so a trade RESETS
/// the exchange rather than freezing it.
///
/// ⛔ A HARD LOCK, not hitstun, and the same one the footstool flinch takes:
/// being traded with is not being hit. Nobody took damage, nobody is in
/// knockback, and what makes the moment matter is the frames neither fighter can
/// act in — which is the same read for both, because a clank has no winner.
///
/// ⭐ THE PUSH IS AWAY FROM THE OTHER BODY, resolved from the two positions
/// rather than from either one's facing: two fighters who traded are by
/// definition reaching toward each other, and facing is the thing a spinning or
/// mid-turn body is least reliable about.
pub fn rebound_from_clanks(
    mut clanked: MessageReader<AttacksClanked>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
        &mut ambition_characters::actor::BodyCombat,
    )>,
) {
    let rules = tuning.as_deref().copied().unwrap_or_default();
    let lock = feel
        .as_deref()
        .copied()
        .unwrap_or_default()
        .knockback_recoil_lock_time;
    // ⛔⛔ ONE RECOIL PER FIGHTER PER TICK, NOT ONE PER PAIR. Arbitration emits a
    // message for every qualifying PAIR, so three mutually-overlapping attacks
    // announce AB, AC and BC — and applying an impulse per message pushed the
    // middle fighter twice as hard as the other two for being in the middle. A
    // clash is one event to the body in it: the genre has a rebound, not a
    // rebound COUNT, and this repo already fixed the same shape once when a 2×2
    // volume overlap produced four rebounds for one conceptual clash.
    //
    // ⭐ THE DIRECTION IS THE SUM AND THE SPEED IS NOT. A fighter that clanked
    // two opponents is pushed away from BOTH — the sum says where — and then
    // pushed at exactly `clank_rebound_speed`, because being outnumbered is not
    // a reason to fly faster. Two opposite clanks sum to nothing and nobody is
    // pushed, which is the same answer coincident bodies already get.
    //
    // ⭐ ACCUMULATED IN A `Vec` IN MESSAGE ORDER rather than a map: message order
    // is write order and deterministic, and a hash map's iteration is neither.
    let mut recoil: Vec<(bevy::prelude::Entity, ae::Vec2)> = Vec::new();
    let mut push = |body: bevy::prelude::Entity, away: ae::Vec2| {
        match recoil.iter_mut().find(|(seen, _)| *seen == body) {
            Some((_, total)) => *total += away,
            None => recoil.push((body, away)),
        }
    };
    for clank in clanked.read() {
        let (a, b) = clank.owners;
        // The axis, once, from the pair. `get_many` is not used because the two
        // owners are looked up mutably one at a time below.
        let (Ok(a_pos), Ok(b_pos)) = (
            bodies.get(a).map(|(kin, _)| kin.pos),
            bodies.get(b).map(|(kin, _)| kin.pos),
        ) else {
            continue;
        };
        let apart = b_pos - a_pos;
        // Two bodies at the SAME point have no axis, and inventing one would
        // pick a direction out of floating-point noise. They keep their lock and
        // lose their moves; nobody is pushed.
        let axis = if apart.length_squared() > f32::EPSILON {
            apart.normalize()
        } else {
            ae::Vec2::ZERO
        };
        push(a, -axis);
        push(b, axis);
        // ⛔ THE MOVE IS ALREADY OVER — `arbitrate_attack_clanks` ends it, so
        // that the STRONGER-WINS case (which announces nothing) ends its
        // loser by the same road. Cancelling again here would be a second
        // authority on when an attack stops.
    }
    for (body, total) in recoil {
        let Ok((mut kin, mut combat)) = bodies.get_mut(body) else {
            continue;
        };
        // ⭐ THE LOCK IS OWED FOR HAVING CLANKED AT ALL, even when the pushes
        // cancelled: the fighter's attack ended and it does not get to act
        // through the beat just because the geometry was symmetrical.
        combat.recoil_lock_timer = combat.recoil_lock_timer.max(lock);
        if total.length_squared() > f32::EPSILON {
            kin.vel += total.normalize() * rules.clank_rebound_speed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐ THE COMPARISON ALONE, with no world in it.
    #[test]
    fn the_verdict_trades_close_attacks_and_lets_a_much_stronger_one_through() {
        assert_eq!(
            clank_verdict(10, 10, 9.0),
            Some(ClankVerdict::BothRefused),
            "two identical attacks did not trade"
        );
        assert_eq!(
            clank_verdict(4, 13, 9.0),
            Some(ClankVerdict::BothRefused),
            "a difference EXACTLY the window traded — the boundary is inclusive"
        );
        assert_eq!(
            clank_verdict(4, 14, 9.0),
            Some(ClankVerdict::StrongerWins),
            "a 10-damage gap still traded, so a heavy swing cannot beat a jab"
        );
        assert_eq!(clank_verdict(14, 4, 9.0), clank_verdict(4, 14, 9.0));
        // ⛔ AN UNDECLARED WORLD REFUSES EVERY PAIR — the answer for every
        // Ambition room, asked in the RULE so no other road reaches clanking by
        // accident.
        assert_eq!(clank_verdict(10, 10, 0.0), None);
        assert_eq!(clank_verdict(4, 99, 0.0), None);
    }

    /// ⛔⛔ THREE EQUAL ATTACKS MEETING ON ONE TICK: ALL THREE TRADE.
    ///
    /// The sweep skipped a pair when either owner was already in `ended`, so an
    /// EARLIER pair's outcome decided whether a LATER pair was CONSIDERED at
    /// all. With A/B/C overlapping and equal: A/B resolves first by `SimId`,
    /// both end, A/C and B/C are skipped — and **C survives because of `SimId`
    /// order**. Deterministic, and not simultaneous.
    ///
    /// ⭐ `resolved` IS THE DEDUP; `ended` IS A COMMIT LEDGER, applied after the
    /// sweep, exactly as its own comment says. Reading it as an eligibility gate
    /// was the bug.
    ///
    /// ⛔ NO TEST EXERCISED `arbitrate_attack_clanks` AT ALL before this — the
    /// only clank arm covered the pure `clank_verdict` — which is why a one-line
    /// change to shipped arbitration was written and reverted unverified on
    /// 2026-08-25 rather than shipped blind.
    /// One equal swing, long enough to still be playing after the sweep.
    fn a_swing(team: &str) -> ambition_entity_catalog::MoveSpec {
        ambition_entity_catalog::MoveSpec {
            display_name: None,
            id: format!("swing_{team}"),
            clip: ambition_entity_catalog::ClipBinding {
                clip: "attack".to_string(),
                fallbacks: vec![],
            },
            duration_s: 0.4,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            smash_charge: None,
            charge_gesture: ambition_entity_catalog::ChargeGesture::Smash,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
            equips: None,
            flow: None,
        }
    }

    #[test]
    fn three_attacks_meeting_at_once_all_trade() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<AttacksClanked>();
        app.insert_resource(crate::rules::ResolvedCombatTuning {
            clank_damage_window: 9.0,
            ..Default::default()
        });
        app.add_systems(Update, arbitrate_attack_clanks);

        // Three fighters standing on one spot, each mid-move, each on a
        // different side so every pair is opposed.
        let mut fighter = |team: &str| -> Entity {
            app.world_mut()
                .spawn((
                    ae::BodyKinematics {
                        pos: ae::Vec2::ZERO,
                        vel: ae::Vec2::ZERO,
                        size: ae::Vec2::new(16.0, 32.0),
                        facing: 1.0,
                    },
                    ae::BodyGroundState {
                        on_ground: true,
                        ..Default::default()
                    },
                    crate::targeting::MatchTeam::new(team.to_string()),
                    crate::moveset::MovePlayback::new(a_swing(team), 1.0),
                ))
                .id()
        };
        let a = fighter("a");
        let b = fighter("b");
        let c = fighter("c");

        // One equal strike volume each, all overlapping at the origin. The ids
        // are stated so the sweep's order is the fixture's, not the allocator's.
        for (owner, id) in [(a, "vol_a"), (b, "vol_b"), (c, "vol_c")] {
            app.world_mut().spawn((
                Hitbox {
                    strike_sfx: None,
                    owner,
                    source: crate::strike::HitSide::Enemy,
                    anchor: crate::strike::HitboxAnchor::FollowOwner {
                        local_offset: ae::Vec2::ZERO,
                    },
                    half_extent: ae::Vec2::new(20.0, 20.0),
                    shape: None,
                    facing: 1.0,
                    damage: 10,
                    knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
                    launch_dir: None,
                    frame_down: ae::Vec2::new(0.0, 1.0),
                    reaction: None,
                },
                crate::moveset::StrikeVolume { owner, window: 0 },
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
            ));
        }

        app.update();

        let still_swinging: Vec<Entity> = [a, b, c]
            .into_iter()
            .filter(|e| {
                app.world()
                    .get::<crate::moveset::MovePlayback>(*e)
                    .is_some()
            })
            .collect();
        assert!(
            still_swinging.is_empty(),
            "{} of three equal attacks survived a simultaneous clank — an \
             earlier pair's outcome is deciding whether a later pair is \
             CONSIDERED, so the last id standing wins by allocator-independent \
             luck rather than by the contest",
            still_swinging.len()
        );
    }

    /// ⛔⛔ A THREE-WAY CLASH IS ONE RECOIL EACH, NOT ONE PER PAIR.
    ///
    /// Arbitration announces every qualifying PAIR — AB, AC and BC for three
    /// mutually-overlapping attacks — and the rebound applied a full impulse per
    /// message, so the fighter in the MIDDLE was pushed twice as hard as the
    /// other two for being in the middle. A clash is one event to the body in
    /// it: the genre has a rebound, not a rebound count, and this repo already
    /// fixed the same shape once when a 2×2 volume overlap produced four
    /// rebounds for one conceptual clash.
    ///
    /// ⛔⛔ AND THE EXISTING THREE-WAY ARM COULD NOT SEE ANY OF IT, which is why
    /// this one exists beside it. It stands all three fighters at `Vec2::ZERO`,
    /// where the rebound axis is deliberately zero for coincident bodies — so it
    /// proves the attacks all end and says nothing whatever about recoil. These
    /// three are spread out.
    ///
    /// ⭐ THE MIDDLE FIGHTER IS THE MEASUREMENT. A and C are pushed apart; B,
    /// between them, is pushed by both and away from both, and the two pushes
    /// very nearly cancel. What must NOT happen is B leaving at twice anybody
    /// else's speed.
    #[test]
    fn a_three_way_clash_pushes_each_fighter_once() {
        use bevy::prelude::*;

        let mut app = App::new();
        app.add_message::<AttacksClanked>();
        let speed = 300.0;
        app.insert_resource(crate::rules::ResolvedCombatTuning {
            clank_rebound_speed: speed,
            ..Default::default()
        });
        app.add_systems(Update, rebound_from_clanks);

        // A — B — C in a line, so every pair has a real axis and B is between.
        let mut fighter = |x: f32| -> Entity {
            app.world_mut()
                .spawn((
                    ae::BodyKinematics {
                        pos: ae::Vec2::new(x, 0.0),
                        vel: ae::Vec2::ZERO,
                        size: ae::Vec2::new(16.0, 32.0),
                        facing: 1.0,
                    },
                    ambition_characters::actor::BodyCombat::default(),
                ))
                .id()
        };
        let a = fighter(-10.0);
        let b = fighter(0.0);
        let c = fighter(10.0);

        for owners in [(a, b), (a, c), (b, c)] {
            app.world_mut()
                .write_message(AttacksClanked { owners });
        }
        app.update();

        let speed_of = |e: Entity| {
            app.world()
                .get::<ae::BodyKinematics>(e)
                .expect("the fighter exists")
                .vel
                .length()
        };

        // The premise: the outer two really were pushed, so the assertion about
        // B is about ONE recoil rather than about a system that did nothing.
        assert!(
            (speed_of(a) - speed).abs() < 1e-3,
            "the outer fighter was not pushed at exactly one rebound speed: {}",
            speed_of(a)
        );
        assert!(
            (speed_of(c) - speed).abs() < 1e-3,
            "the other outer fighter was not pushed at exactly one rebound speed: {}",
            speed_of(c)
        );
        assert!(
            speed_of(b) <= speed + 1e-3,
            "the fighter in the MIDDLE of a three-way clash left at {} against a \
             rebound speed of {} — it is being pushed once per PAIR it is in, so \
             standing between two opponents launches you",
            speed_of(b),
            speed
        );
    }

    /// ⛔⛔ A GROUND SWING STAYS A GROUND SWING WHEN ITS OWNER LEAVES THE FLOOR.
    ///
    /// Eligibility asked `BodyGroundState::on_ground` AT COLLISION TIME, so a
    /// ground attack stopped clanking the moment its owner walked off a ledge
    /// mid-swing — and an aerial started clanking when its owner landed.
    /// "Grounded attack" is a CLASSIFICATION and it is settled when the swing
    /// comes out — `MovePlayback::started_grounded` is the stance the SELECTOR
    /// used, and the loser's playback is already in this system's hand.
    ///
    /// ⭐ THE ARMS STRADDLE THE LATCH with the FEET HELD WRONG in both: the
    /// clanking pair is airborne right now, and the refused pair is standing.
    #[test]
    fn the_clank_reads_the_swings_stance_not_the_owners_feet() {
        use bevy::prelude::*;

        let traded = |latched_grounded: bool, feet_on_floor: bool| -> bool {
            let mut app = App::new();
            app.add_message::<AttacksClanked>();
            app.insert_resource(crate::rules::ResolvedCombatTuning {
                clank_damage_window: 9.0,
                ..Default::default()
            });
            app.add_systems(Update, arbitrate_attack_clanks);

            let mut fighter = |team: &str| -> Entity {
                app.world_mut()
                    .spawn((
                        ae::BodyKinematics {
                            pos: ae::Vec2::ZERO,
                            vel: ae::Vec2::ZERO,
                            size: ae::Vec2::new(16.0, 32.0),
                            facing: 1.0,
                        },
                        ae::BodyGroundState {
                            on_ground: feet_on_floor,
                            ..Default::default()
                        },
                        crate::targeting::MatchTeam::new(team.to_string()),
                        crate::moveset::MovePlayback::new(a_swing(team), 1.0)
                            .started_in_stance(latched_grounded),
                    ))
                    .id()
            };
            let a = fighter("a");
            let b = fighter("b");
            for (owner, id) in [(a, "vol_a"), (b, "vol_b")] {
                app.world_mut().spawn((
                    Hitbox {
                        strike_sfx: None,
                        owner,
                        source: crate::strike::HitSide::Enemy,
                        anchor: crate::strike::HitboxAnchor::FollowOwner {
                            local_offset: ae::Vec2::ZERO,
                        },
                        half_extent: ae::Vec2::new(20.0, 20.0),
                        shape: None,
                        facing: 1.0,
                        damage: 10,
                        knockback: crate::strike::HitboxKnockback::FeelScale(0.0),
                        launch_dir: None,
                        frame_down: ae::Vec2::new(0.0, 1.0),
                        reaction: None,
                    },
                    crate::moveset::StrikeVolume { owner, window: 0 },
                    ambition_platformer2d_shared_tangle::sim_id::SimId::placement(id),
                ));
            }
            app.update();
            app.world().get::<crate::moveset::MovePlayback>(a).is_none()
        };

        assert!(
            traded(true, false),
            "two GROUND swings did not trade because their owners had left the \
             floor since — walking off a ledge mid-swing turned the attack into \
             something else"
        );
        assert!(
            !traded(false, true),
            "two AERIALS traded because their owners had landed since — the air \
             stopped being a place where committing costs you"
        );
    }
}
