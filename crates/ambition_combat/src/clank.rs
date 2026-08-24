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

use crate::strike::{Hitbox, HitboxLifetime};
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
    /// The two strike volumes that were cancelled, ascending.
    ///
    /// ⛔ ORDERED, and it is the SORT that makes it deterministic rather than
    /// the sweep: a clank has no first party, so the pair is canonicalised here
    /// instead of carrying whichever the loop happened to reach first.
    pub strikes: (Entity, Entity),
    /// The bodies that threw them, in the same order as `strikes`.
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
    strikes: Query<(Entity, &Hitbox), With<HitboxLifetime>>,
    owner_pos: Query<&ae::BodyKinematics>,
    factions: Query<&crate::components::ActorFaction>,
    teams: Query<&crate::targeting::MatchTeam>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
    mut clanked: MessageWriter<AttacksClanked>,
) {
    let rules = tuning.as_deref().copied().unwrap_or_default();
    if rules.clank_damage_window <= 0.0 {
        return;
    }
    // ⛔⛔ SORTED, AND THE SORT IS THE DETERMINISM. A Bevy query yields entities
    // in archetype order, which is not an order this simulation may depend on:
    // two peers whose archetypes filled differently would arbitrate the same
    // frame's pairs in different sequences and cancel different volumes.
    let mut live: Vec<(Entity, &Hitbox)> = strikes.iter().collect();
    live.sort_by_key(|(entity, _)| *entity);

    // Cancelled once, whatever else it meets this tick: a volume that already
    // lost has nothing left to trade.
    let mut cancelled: std::collections::BTreeSet<Entity> = std::collections::BTreeSet::new();

    for (index, (a_entity, a)) in live.iter().enumerate() {
        for (b_entity, b) in live.iter().skip(index + 1) {
            if cancelled.contains(a_entity) || cancelled.contains(b_entity) {
                continue;
            }
            // A body's own two volumes never trade — a multi-hit move overlaps
            // itself constantly — and neither do allies'.
            if a.owner == b.owner {
                continue;
            }
            if !opposed(a.owner, b.owner, &factions, &teams, rules) {
                continue;
            }
            let (Ok(a_owner), Ok(b_owner)) = (owner_pos.get(a.owner), owner_pos.get(b.owner))
            else {
                continue;
            };
            if !a
                .world_volume(a_owner.pos)
                .intersects(&b.world_volume(b_owner.pos))
            {
                continue;
            }
            let Some(verdict) = clank_verdict(a.damage, b.damage, rules.clank_damage_window) else {
                continue;
            };
            match verdict {
                ClankVerdict::BothRefused => {
                    cancelled.insert(*a_entity);
                    cancelled.insert(*b_entity);
                    // `live` is sorted ascending and `b` comes from the tail, so
                    // this pair is already canonical — stated rather than
                    // assumed, because the day the sweep changes shape is the
                    // day a consumer starts seeing the pair both ways round.
                    debug_assert!(a_entity < b_entity);
                    clanked.write(AttacksClanked {
                        strikes: (*a_entity, *b_entity),
                        owners: (a.owner, b.owner),
                    });
                }
                ClankVerdict::StrongerWins => {
                    // The weaker one alone. ⛔ NOT announced: nothing happened
                    // to the winner, and the loser's own owner learns about it
                    // the way it learns about any whiff — its move plays on.
                    cancelled.insert(if a.damage < b.damage {
                        *a_entity
                    } else {
                        *b_entity
                    });
                }
            }
        }
    }

    for strike in cancelled {
        commands.entity(strike).try_despawn();
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
    mut commands: Commands,
    mut clanked: MessageReader<AttacksClanked>,
    tuning: Option<Res<crate::rules::ResolvedCombatTuning>>,
    feel: Option<Res<crate::feel::Platformer2dFeelTuningMonolith>>,
    mut bodies: Query<(
        &mut ae::BodyKinematics,
        &mut ambition_characters::actor::BodyCombat,
    )>,
    mut playing: Query<&mut crate::moveset::MovePlayback>,
) {
    let rules = tuning.as_deref().copied().unwrap_or_default();
    let lock = feel
        .as_deref()
        .copied()
        .unwrap_or_default()
        .knockback_recoil_lock_time;
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
        for (body, away) in [(a, -axis), (b, axis)] {
            if let Ok((mut kin, mut combat)) = bodies.get_mut(body) {
                kin.vel += away * rules.clank_rebound_speed;
                combat.recoil_lock_timer = combat.recoil_lock_timer.max(lock);
            }
            // AND THE MOVE ENDS. Through the one teardown path, which despawns
            // whatever volumes the move still owns — a swing whose strike was
            // traded away must not keep spawning later windows.
            if let Ok(mut playback) = playing.get_mut(body) {
                crate::moveset::cancel_move_playback(&mut commands, body, &mut playback);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::ActorFaction;
    use crate::strike::{HitSide, HitboxAnchor, HitboxKnockback};

    const DOWN: ae::Vec2 = ae::Vec2::new(0.0, 1.0);

    /// ⭐ THE COMPARISON ALONE, with no world in it. Three claims, and the
    /// third is the one an engine that hosts more than a fighting game needs.
    #[test]
    fn the_verdict_trades_close_attacks_and_lets_a_much_stronger_one_through() {
        // Equal, and either side of the window.
        assert_eq!(
            clank_verdict(10, 10, 9.0),
            Some(ClankVerdict::BothRefused),
            "two identical attacks did not trade"
        );
        assert_eq!(
            clank_verdict(4, 13, 9.0),
            Some(ClankVerdict::BothRefused),
            "a difference EXACTLY the window traded — the boundary is inclusive, \
             so only a strictly greater gap wins outright"
        );
        assert_eq!(
            clank_verdict(4, 14, 9.0),
            Some(ClankVerdict::StrongerWins),
            "a 10-damage gap still traded, so a heavy swing cannot beat a jab"
        );
        // Symmetric: which one is named first cannot decide it.
        assert_eq!(clank_verdict(14, 4, 9.0), clank_verdict(4, 14, 9.0));

        // ⛔ AND AN UNDECLARED WORLD REFUSES EVERY PAIR. This is the answer for
        // every Ambition room, and it is asked in the rule rather than at the
        // call site so no other road can reach clanking by accident.
        assert_eq!(clank_verdict(10, 10, 0.0), None);
        assert_eq!(clank_verdict(4, 99, 0.0), None);
    }

    fn clank_app() -> App {
        let mut app = App::new();
        app.add_message::<AttacksClanked>();
        app.insert_resource(crate::rules::ResolvedCombatTuning {
            clank_damage_window: 9.0,
            ..Default::default()
        });
        app.add_systems(Update, arbitrate_attack_clanks);
        app
    }

    fn body(app: &mut App, x: f32, faction: ActorFaction) -> Entity {
        app.world_mut()
            .spawn((
                ae::BodyKinematics {
                    pos: ae::Vec2::new(x, 0.0),
                    vel: ae::Vec2::ZERO,
                    size: ae::Vec2::new(16.0, 32.0),
                    facing: 1.0,
                },
                faction,
            ))
            .id()
    }

    /// A swing reaching `reach` in front of `owner`, wide enough to meet one
    /// coming the other way.
    fn swing(app: &mut App, owner: Entity, reach: f32, damage: i32) -> Entity {
        app.world_mut()
            .spawn((
                Hitbox {
                    owner,
                    source: HitSide::Player,
                    anchor: HitboxAnchor::FollowOwner {
                        local_offset: ae::Vec2::new(reach, 0.0),
                    },
                    half_extent: ae::Vec2::new(20.0, 20.0),
                    shape: None,
                    facing: 1.0,
                    damage,
                    knockback: HitboxKnockback::LaunchSpeed {
                        base: 100.0,
                        growth: None,
                    },
                    launch_dir: None,
                    autolink: None,
                    frame_down: DOWN,
                    strike_sfx: None,
                },
                HitboxLifetime { remaining_s: 1.0 },
            ))
            .id()
    }

    fn alive(app: &App, strike: Entity) -> bool {
        app.world().get::<Hitbox>(strike).is_some()
    }

    /// ⭐⭐ TWO FIGHTERS SWINGING INTO EACH OTHER TRADE, and NEITHER attack
    /// survives to look for a victim.
    ///
    /// ⛔ THE ASSERTION IS ON BOTH VOLUMES, deliberately. Cancelling one is the
    /// failure mode this system's ordering exists to prevent: whichever the
    /// query yielded first would land on a body that was mid-swing itself, which
    /// is the interaction the genre replaces with a trade.
    #[test]
    fn two_attacks_that_meet_are_both_refused() {
        let mut app = clank_app();
        let left = body(&mut app, 0.0, ActorFaction::Player);
        let right = body(&mut app, 40.0, ActorFaction::Enemy);
        let a = swing(&mut app, left, 20.0, 10);
        let b = swing(&mut app, right, -20.0, 12);
        app.update();

        assert!(!alive(&app, a), "the first attack survived the trade");
        assert!(!alive(&app, b), "the second attack survived the trade");

        let messages = app
            .world()
            .resource::<bevy::ecs::message::Messages<AttacksClanked>>();
        let mut cursor = messages.get_cursor();
        let announced: Vec<_> = cursor.read(messages).collect();
        assert_eq!(announced.len(), 1, "a clank announced {:?}", announced);
        // As a SET: which body the pair names first is canonical ordering, not a
        // fact about the trade, and asserting the tuple would pin spawn order.
        let named =
            std::collections::BTreeSet::from([announced[0].owners.0, announced[0].owners.1]);
        assert_eq!(
            named,
            std::collections::BTreeSet::from([left, right]),
            "the clank named the wrong bodies, so nothing downstream can recoil \
             the two that traded"
        );
        assert!(
            announced[0].strikes.0 < announced[0].strikes.1,
            "the strike pair is not canonically ordered, so one consumer sees \
             (a, b) and another sees (b, a) for the same trade"
        );
    }

    /// ⭐ A MUCH STRONGER ATTACK WINS OUTRIGHT — and this is the half that makes
    /// clanking a mechanic rather than a stalemate generator. Without it a jab
    /// would cancel a fully charged smash.
    #[test]
    fn a_much_stronger_attack_beats_a_weak_one_and_keeps_going() {
        let mut app = clank_app();
        let left = body(&mut app, 0.0, ActorFaction::Player);
        let right = body(&mut app, 40.0, ActorFaction::Enemy);
        let jab = swing(&mut app, left, 20.0, 2);
        let smash = swing(&mut app, right, -20.0, 20);
        app.update();

        assert!(!alive(&app, jab), "the jab survived a much stronger attack");
        assert!(
            alive(&app, smash),
            "the stronger attack was cancelled too, so a heavy swing trades with \
             a jab instead of beating it"
        );
        let messages = app
            .world()
            .resource::<bevy::ecs::message::Messages<AttacksClanked>>();
        let mut cursor = messages.get_cursor();
        assert_eq!(
            cursor.read(messages).count(),
            0,
            "a one-sided win announced a clank, which has no winner by definition"
        );
    }

    /// ⛔⛔ AND THE THREE THAT MUST NOT TRADE. Each is a pair that overlaps and
    /// still has to pass through: a move's own volumes (a multi-hit overlaps
    /// itself constantly), two allies' swings, and any pair at all in a world
    /// that never declared clanking.
    #[test]
    fn a_moves_own_volumes_allies_and_an_undeclared_world_never_trade() {
        // One body's two volumes.
        let mut app = clank_app();
        let solo = body(&mut app, 0.0, ActorFaction::Player);
        let first = swing(&mut app, solo, 10.0, 10);
        let second = swing(&mut app, solo, 12.0, 10);
        app.update();
        assert!(
            alive(&app, first) && alive(&app, second),
            "a multi-hit move cancelled itself"
        );

        // Two bodies on the same side.
        let mut app = clank_app();
        let a_body = body(&mut app, 0.0, ActorFaction::Enemy);
        let b_body = body(&mut app, 40.0, ActorFaction::Enemy);
        let a = swing(&mut app, a_body, 20.0, 10);
        let b = swing(&mut app, b_body, -20.0, 10);
        app.update();
        assert!(
            alive(&app, a) && alive(&app, b),
            "two allies' swings cancelled each other"
        );

        // The same opposed pair, in a world that declared nothing.
        let mut app = clank_app();
        app.insert_resource(crate::rules::ResolvedCombatTuning::default());
        let left = body(&mut app, 0.0, ActorFaction::Player);
        let right = body(&mut app, 40.0, ActorFaction::Enemy);
        let a = swing(&mut app, left, 20.0, 10);
        let b = swing(&mut app, right, -20.0, 10);
        app.update();
        assert!(
            alive(&app, a) && alive(&app, b),
            "an undeclared world clanked — every Ambition room just changed to \
             buy a Smash feature"
        );
    }

    /// ⭐⭐ A TRADE COSTS BOTH FIGHTERS: their moves end and both are pushed
    /// apart. Driven by the REAL arbitration, not a hand-written message, so
    /// this fails if the two systems ever disagree about who traded.
    ///
    /// ⛔ THE DIRECTIONS ARE OPPOSITE AND THAT IS THE ASSERTION. A rebound that
    /// pushed both bodies the same way would be a shove, and would pass any test
    /// that only asked whether velocity changed.
    #[test]
    fn a_trade_ends_both_moves_and_throws_both_fighters_apart() {
        let mut app = clank_app();
        app.insert_resource(crate::rules::ResolvedCombatTuning {
            clank_damage_window: 9.0,
            clank_rebound_speed: 200.0,
            ..Default::default()
        });
        app.insert_resource(crate::feel::Platformer2dFeelTuningMonolith::default());
        app.add_systems(Update, rebound_from_clanks.after(arbitrate_attack_clanks));

        let left = body(&mut app, 0.0, ActorFaction::Player);
        let right = body(&mut app, 40.0, ActorFaction::Enemy);
        for owner in [left, right] {
            app.world_mut().entity_mut(owner).insert((
                ambition_characters::actor::BodyCombat::default(),
                crate::moveset::MovePlayback::new(swinging_move(), 1.0),
            ));
        }
        swing(&mut app, left, 20.0, 10);
        swing(&mut app, right, -20.0, 10);
        app.update();

        let vel = |app: &App, body: Entity| {
            app.world()
                .get::<ae::BodyKinematics>(body)
                .expect("still a body")
                .vel
                .x
        };
        assert!(
            vel(&app, left) < 0.0,
            "the left fighter was not thrown back: {}",
            vel(&app, left)
        );
        assert!(
            vel(&app, right) > 0.0,
            "the right fighter was not thrown back: {}",
            vel(&app, right)
        );

        for (owner, name) in [(left, "left"), (right, "right")] {
            assert!(
                app.world()
                    .get::<crate::moveset::MovePlayback>(owner)
                    .is_none(),
                "the {name} fighter's move survived the trade, so it plays on \
                 with no hitbox — which reads as the game dropping the input"
            );
            assert!(
                app.world()
                    .get::<ambition_characters::actor::BodyCombat>(owner)
                    .is_some_and(|c| c.recoil_lock_timer > 0.0),
                "the {name} fighter can act immediately, so a trade costs it \
                 nothing but its swing"
            );
        }
    }

    /// A move for the fighters above to be mid-way through. Only its existence
    /// and its teardown matter.
    fn swinging_move() -> ambition_entity_catalog::MoveSpec {
        ambition_entity_catalog::MoveSpec {
            display_name: None,
            id: "swing".to_string(),
            clip: ambition_entity_catalog::ClipBinding {
                clip: "swing".to_string(),
                fallbacks: vec![],
            },
            duration_s: 1.0,
            windows: vec![],
            events: vec![],
            gates: Default::default(),
            start_impulse: None,
            smash_charge_mult: 1.0,
            smash_charge: None,
            repeat: None,
            landing_lag_s: None,
            autocancel_after_s: None,
            sprite_spin_hz: None,
        }
    }

    /// ⭐ AND ATTACKS THAT DO NOT REACH EACH OTHER ARE NOT A TRADE.
    ///
    /// The non-vacuity for every case above: they all place two swings that
    /// genuinely overlap, and a system that cancelled on hostility alone would
    /// pass all of them.
    #[test]
    fn attacks_that_never_meet_are_left_alone() {
        let mut app = clank_app();
        let left = body(&mut app, 0.0, ActorFaction::Player);
        let right = body(&mut app, 400.0, ActorFaction::Enemy);
        let a = swing(&mut app, left, 20.0, 10);
        let b = swing(&mut app, right, -20.0, 10);
        app.update();
        assert!(
            alive(&app, a) && alive(&app, b),
            "two swings a stage-width apart traded"
        );
    }
}
