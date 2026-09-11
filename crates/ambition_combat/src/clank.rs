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

/// WHICH ATTACK a contender belongs to, and therefore what ending it means.
///
/// ⭐⭐ THE CONTEST IS BETWEEN ATTACKS, NOT BETWEEN RECTANGLES, and the two
/// families disagree about what an attack IS. A melee move is one attack however
/// many volumes it spawns — arbitrating per volume let a two-volume attack meet a
/// two-volume attack four times and rebound the same fighters four times. A
/// projectile is one attack PER SHOT: a fighter with three bolts in the air has
/// thrown three, and deduping those by owner would let the first bolt's outcome
/// decide whether the second was considered at all.
///
/// ⇒ The dedup key is this, not the owner. `Move` carries the owner because that
/// is the move's identity; `Shot` carries the projectile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClashAttack {
    /// The move a body is playing. Ending it cancels the playback.
    Move(Entity),
    /// One shot in flight. Ending it expires that shot and no other.
    Shot(Entity),
}

/// What kind of attack a contender is, for the pair rules that are not about
/// damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClashFamily {
    /// A melee swing that came out with its owner on the floor.
    GroundedMelee,
    /// A melee swing that came out in the air. ⭐ THE GENRE'S RULE: an aerial
    /// passes THROUGH an opposing swing, which is what keeps the air a place
    /// where committing costs you.
    AerialMelee,
    /// A projectile. ⛔ NOT subject to the grounded rule in either direction: a
    /// shot meets an aerial attack in this genre, and two shots meet each other
    /// wherever they are. The rule is about SWINGS.
    Shot,
}

/// The canonical order a contest is arbitrated in.
///
/// ⛔⛔ AN `Entity` IS AN ALLOCATOR IDENTITY. Two peers whose archetypes filled
/// differently hand out different indices for the same volume, so a sweep
/// ordered by it resolves the same frame's pairs in different sequences and
/// cancels different attacks. Both variants carry a key the simulation itself
/// minted: `SimId::strike_volume` is derived from `(owner, move, window, volume)`
/// and `ProjectileSeq` is a rollback-registered monotonic spawn id.
///
/// ⭐ The derived `Ord` puts every melee volume before every shot. That is
/// arbitrary and it is the point — it is the SAME arbitrary order on every peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClashOrder<'a> {
    /// A strike volume's `SimId`, or `""` for a volume outside the identified
    /// population (whose owner then breaks the tie).
    Melee(&'a str),
    /// A shot's `ProjectileSeq`.
    Shot(u64),
}

/// One attack entered into a clash contest, whatever family produced it.
pub struct ClashContender<'a> {
    pub order: ClashOrder<'a>,
    pub attack: ClashAttack,
    /// The body whose attack this is. ⛔ Read for PAIRING, never for the
    /// opposition test — a shot outlives its firer, so its side is the one
    /// stamped on it at launch and carried below, not whatever its owner's
    /// components say now (or no longer say).
    pub owner: Entity,
    /// Already placed in world space by the caller, because only the caller
    /// knows where its family's boxes live.
    pub volume: ae::CombatVolume,
    pub family: ClashFamily,
    pub damage: i32,
    /// This attack's side, frozen at the moment it came out.
    pub faction: crate::components::ActorFaction,
    /// Its match team, when it had one. `None` outside a match — the faction
    /// rule then decides, exactly as it does for any unseated body.
    pub team: Option<&'a crate::targeting::MatchTeam>,
}

/// What a contest decided.
pub struct ClashResolution {
    /// The attacks that lost and must end, in the order they were decided.
    pub defeated: Vec<ClashAttack>,
    /// The owner pairs that TRADED, each ascending and appearing once.
    pub clanked: Vec<(Entity, Entity)>,
}

/// Resolve every opposed, overlapping pair of attacks — the whole contest, with
/// no world in it.
///
/// ⭐⭐ ONE CONTEST FOR EVERY ATTACK FAMILY. Melee-vs-melee was arbitrated here
/// while projectile contact resolved in a stepper that ran EARLIER in the same
/// tick, so a shot could never meet a swing or another shot. Two arbitration
/// sites would have been two answers to "which attack wins"; this is the
/// decision, and each family's system contributes contenders to it and maps
/// [`ClashAttack`] back onto its own way of ending an attack.
///
/// ⭐ NO WORLD AND NO CLOSURE. Each contender carries the side it came out on,
/// so the whole contest — ordering, pairing, opposition, overlap and verdict —
/// is a function of its arguments. That is what lets a fixture put four attacks
/// on a table and assert the outcome without an `App`.
pub fn resolve_clashes(
    contenders: &mut [ClashContender<'_>],
    window: f32,
    rules: crate::rules::ResolvedCombatTuning,
) -> ClashResolution {
    let mut resolution = ClashResolution { defeated: Vec::new(), clanked: Vec::new() };
    if window <= 0.0 {
        return resolution;
    }
    // ⭐ `(order, owner)`: the second key is only reached by melee volumes with
    // no `SimId` at all. Those belong to bodies outside the rollback-tracked
    // population, so they cannot desync a peer — and ordering them by owner
    // keeps the sweep total rather than leaving ties to a query's own order.
    contenders.sort_by(|a, b| a.order.cmp(&b.order).then(a.owner.cmp(&b.owner)));

    // ⛔⛔ `resolved` IS THE DEDUP AND NOTHING ELSE. Skipping a pair because
    // either attack had already lost would let an EARLIER pair's outcome decide
    // whether a LATER pair was CONSIDERED — deterministic, and not simultaneous.
    let mut resolved: std::collections::BTreeSet<(ClashAttack, ClashAttack)> =
        std::collections::BTreeSet::new();
    let mut clanked: std::collections::BTreeSet<(Entity, Entity)> =
        std::collections::BTreeSet::new();
    let mut defeated: std::collections::BTreeSet<ClashAttack> =
        std::collections::BTreeSet::new();

    for (index, a) in contenders.iter().enumerate() {
        for b in contenders.iter().skip(index + 1) {
            if a.attack == b.attack || a.owner == b.owner {
                continue;
            }
            // ⛔ THE GROUNDED RULE IS A RULE ABOUT SWINGS MEETING SWINGS. An
            // aerial passes through an opposing swing; it does not pass through
            // a bolt, and a bolt does not pass through it.
            let both_melee = a.family != ClashFamily::Shot && b.family != ClashFamily::Shot;
            if both_melee
                && (a.family == ClashFamily::AerialMelee || b.family == ClashFamily::AerialMelee)
            {
                continue;
            }
            let pair = if a.attack <= b.attack {
                (a.attack, b.attack)
            } else {
                (b.attack, a.attack)
            };
            if resolved.contains(&pair) {
                continue;
            }
            if !sides_are_opposed(a, b, rules) {
                continue;
            }
            if !a.volume.intersects(&b.volume) {
                continue;
            }
            let Some(verdict) = clank_verdict(a.damage, b.damage, window) else {
                continue;
            };
            resolved.insert(pair);
            match verdict {
                ClankVerdict::BothRefused => {
                    defeated.insert(a.attack);
                    defeated.insert(b.attack);
                    clanked.insert(if a.owner <= b.owner {
                        (a.owner, b.owner)
                    } else {
                        (b.owner, a.owner)
                    });
                }
                ClankVerdict::StrongerWins => {
                    defeated.insert(if a.damage < b.damage { a.attack } else { b.attack });
                }
            }
        }
    }
    resolution.defeated = defeated.into_iter().collect();
    resolution.clanked = clanked.into_iter().collect();
    resolution
}

/// May these two ATTACKS meet at all?
///
/// The same question the damage sweep asks about an attack and a victim, asked
/// about two attacks — so a team-mate's swing passes through yours exactly as
/// their hit would, and so does their bolt.
fn sides_are_opposed(
    a: &ClashContender<'_>,
    b: &ClashContender<'_>,
    rules: crate::rules::ResolvedCombatTuning,
) -> bool {
    // Teams outrank faction, exactly as they do for damage: two humans in a
    // match share a faction and are still opponents.
    if let (Some(a_team), Some(b_team)) = (a.team, b.team) {
        return a_team != b_team;
    }
    crate::targeting::can_damage(a.faction, b.faction, rules.friendly_fire())
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
        // ⛔ THE MOVE IS ALREADY OVER — the clash arbiter ends it, so that the
        // STRONGER-WINS case (which announces nothing) ends its loser by the
        // same road. Cancelling again here would be a second authority on
        // when an attack stops.
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

}
