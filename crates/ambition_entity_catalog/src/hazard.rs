//! **WHAT AN AUTHORED TECHNIQUE PUTS IN THE WORLD THAT CAN HURT SOMEBODY**, and
//! the laws by which it gets there.
//!
//! ⭐⭐ **ONE SUB-DOMAIN, CARVED OUT OF `lib.rs` 2026-09-21.** `ThreatTravel`,
//! `MoveHazard` and the `hazard_of` lowering are a closed family: a hazard's
//! travel law, what it does when it touches somebody, and the effect keys the
//! catalog has been taught. Nothing else in this crate is inside that family
//! and nothing in the family reaches outside it except to read the effect
//! params it lowers from.
//!
//! ⚠ **THE CUT IS THE BOUNDARY, NOT THE LINE COUNT.** `engine.module-size`
//! reported `lib.rs` at 5192 > 5000 and the gate's own rule is *"split only
//! when coherent boundaries exist ... not pressure to split coherent code
//! merely to satisfy a number"*. This family had grown its own type hierarchy,
//! its own two-question split (`travel_to` for aiming, `strikes_on_arrival`
//! for paying) and its own recorded debt over two days of work; it reads
//! better alone than as four hundred lines in the middle of the move-contract
//! file.
//!
//! ⛔ `hazard_of` stays `pub(crate)`: it is the lowering `MoveSpec::frame_data`
//! calls, not an API. A caller outside this crate holds a `MoveFrameData` and
//! reads [`MoveFrameData::hazard`], which is the joined answer.
//!
//! [`MoveFrameData::hazard`]: crate::MoveFrameData::hazard

use crate::EffectRef;

/// **What an UNJOINED reader gets for a move that pulls the owner's own
/// ranged trigger.**
///
/// ⚠ **THE BODY OWNS THE NUMBER, NOT THE MOVE.** [`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged)
/// fires whatever `RangedActionSpec` the BODY carries — its speed, its flight,
/// its lifetime — and a catalog derivation has no body to ask. So this states
/// the only thing true of every one of them: a shot crosses ground the swinger
/// cannot. It is wider than any stage this game ships (the smash platform is
/// 480px and its blast lines sit inside two widths), so to a reader that
/// cannot narrow it, a ranged move is admitted wherever the opponent is.
///
/// ⭐ A LAYER THAT CAN JOIN A MOVE TO ITS BODY'S ACTION NARROWS IT — the kit
/// builder is that layer, the same one that joins a grab to its capture params,
/// and [`MoveHazard::OwnersRangedAction`] is the request it answers. This is
/// what an UNJOINED reader gets, and a reader holding only a `MoveSpec` is
/// exactly the reader with no body to ask.
pub const RANGED_ACTION_REACH: f32 = 1_000.0;

/// **HOW A HAZARD COVERS THE GROUND BETWEEN LEAVING ITS OWNER AND TOUCHING
/// SOMEBODY** — the law, not a sample of it.
///
/// ⛔⛤ **THIS REPLACED `Spawned { reach, speed }`, AND THE REASON IS THE
/// SECOND REVIEW OF 2026-09-20:** a pair of scalars can only describe uniform
/// motion, and two of the four shapes the roster already ships are not
/// uniform. Flattening them cost a wrong answer each time, in the same units
/// as a right one:
///
/// - a BOOMERANG decelerates to a standstill at its turnaround, so its average
///   speed is right at maximum range and nowhere else. Projectile Polygon's
///   ponytail (`v0` 430px/s, turning at 0.34s) actually reaches 40px of centre
///   travel in **0.111s**; the average-speed model said 0.186s. At 200px/s of
///   closing speed that is 15px of excess lead, which is the size of the
///   tolerances this layer is being tuned against;
/// - a laid BOMB is not a stationary projectile whose blast is live when it
///   lands. It is an object on a FUSE — Projectile Polygon's is four seconds
///   — and `speed: 0.0` was documented as *"the whole reach is available the
///   moment it exists"*, which is the opposite of what the move's own
///   authoring says: *"laying a bomb is not a hit — the bomb is"*.
///
/// ⭐ **SO THE QUESTION THE TYPE ANSWERS IS `time_to(distance)`, NOT `speed`.**
/// Every consumer of the old pair was dividing a gap by a speed to get a
/// flight time; the shape that knows its own law can answer that directly, and
/// a shape that CANNOT reach a distance says so instead of returning a number.
///
/// ⚠ **ADD A VARIANT WITH THE CONTENT THAT NEEDS IT.** These four are the
/// shapes authored today. A fifth — a homing shot, a tether, a mine that arms
/// on proximity — is a new law and not a new scalar on an existing one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreatTravel {
    /// Constant velocity, out to `span` of centre travel.
    Straight {
        /// px/s, constant.
        speed: f32,
        /// How far the hazard's CENTRE travels before it expires.
        span: f32,
        /// Ground the hazard covers without flying: its spawn offset, its own
        /// half-extent, and any splash. Subtracted before the flight time is
        /// solved, because a shot touches somebody with its edge.
        free: f32,
    },
    /// A decelerating out-leg that stops at `out_s` and turns around:
    ///
    /// ```text
    /// x(t) = v0 t - v0 t^2 / (2 out_s),   x(out_s) = v0 out_s / 2
    /// ```
    ///
    /// The return leg is deliberately not modelled: a fighter deciding whether
    /// to THROW one is asking when it first connects, and that is the out-leg.
    Boomerang {
        /// Launch speed, px/s, before the deceleration.
        v0: f32,
        /// When it reaches the turnaround, seconds after release.
        out_s: f32,
        /// As [`Self::Straight::free`].
        free: f32,
    },
    /// An object PUT somewhere, which goes off by itself after
    /// `detonates_by_s` and can hurt somebody within `reach`. A bomb on a
    /// fuse.
    Placed {
        /// How far from the body the blast can touch somebody.
        ///
        /// ⚠ **A RADIUS, AND THE PLACEMENT IS A VECTOR — DEBT, REVIEWED
        /// 2026-09-20.** The polygon's bomb is authored at `offset (-16, +14)`
        /// with a 56px blast, deliberately BEHIND and below her, and this
        /// collapses that to `|offset.x| + blast_radius` = 72px. Two opponents
        /// 70px in front and 70px behind get the same answer though the bomb
        /// is 32px closer to one of them, and the `y` is discarded outright. A
        /// placed trap has a POSITION, a SHAPE and an activation law, not a
        /// reach — and the repair is to carry the region, not to add another
        /// scalar. Tracked on queue.md's BRAIN row; not done here because the
        /// front/back asymmetry is 32px on a 480px stage and the seam it
        /// belongs to is the resolved offer.
        reach: f32,
        /// Seconds after the object appears before it goes off BY ITSELF.
        ///
        /// ⛔⛤ **A DEADLINE, NOT AN EARLIEST — THIS FIELD WAS CALLED
        /// `earliest_s` FOR ONE COMMIT AND THE NAME WAS A LIE.** The shipped
        /// bomb detonates *"in four seconds OR on a sufficiently hard
        /// impact, whichever happens first"* (`DropBombParams::impact_speed`
        /// is the threshold), and the runtime implements exactly that. So
        /// four seconds is the LATEST it waits, not the soonest it can go, and
        /// a reader told otherwise would refuse a trap an opponent is about to
        /// run into. The impact road depends on what somebody else does to the
        /// object and is not modelled; the deadline is the part the thrower
        /// can count on.
        detonates_by_s: f32,
    },
}

impl ThreatTravel {
    /// The farthest this hazard can hurt somebody, measured from the body.
    pub fn reach(self) -> f32 {
        match self {
            Self::Straight { span, free, .. } => free + span.max(0.0),
            Self::Boomerang { v0, out_s, free } => {
                free + (v0.max(0.0) * out_s.max(0.0) / 2.0)
            }
            Self::Placed { reach, .. } => reach,
        }
    }

    /// When this hazard goes off by itself, seconds from release; `0.0` for
    /// anything that is dangerous from the moment it exists.
    ///
    /// ⚠ **A DEADLINE, NOT AN EARLIEST.** A bomb also detonates on a hard
    /// enough impact, which is sooner and depends on what somebody else does
    /// to it. See [`Self::Placed::detonates_by_s`].
    ///
    /// ⛔⛤ **A SEPARATE QUESTION FROM [`Self::travel_to`], AND MERGING THEM
    /// COST A FIGHTER HER BOMB — MEASURED 2026-09-20.** The first version
    /// answered both with one function, so a laid bomb's four-second fuse was
    /// fed to the aim lead and the brain carried the opponent forward four
    /// seconds of walking. Her bomb reaches 72px (`offset -16`, `blast_radius
    /// 56`), so at any walking speed at all the extrapolated opponent is
    /// outside it: on the 21-fighter grid Projectile Polygon went 144/228 to
    /// **131/183** and her repertoire from 17 distinct moves to 15.
    ///
    /// ⚠ **AND NOTHING PRICES THIS YET, WHICH IS WRITTEN DOWN RATHER THAN
    /// PATCHED.** *"Will they be within 72px in four seconds"* is not a
    /// question a velocity answers, and the attack-admission rule prices
    /// STRIKES. A trap's worth is a stage-control question — the same shape as
    /// the counters and buffs that are deliberately off the attack ranking
    /// until there is a defensive feature to price them with. Inventing one to
    /// keep a move on a list is the wrong order; spending the fuse in the
    /// lead, which is what merging these did, is worse.
    pub fn detonates_by_s(self) -> f32 {
        match self {
            Self::Straight { .. } | Self::Boomerang { .. } => 0.0,
            Self::Placed {
                detonates_by_s, ..
            } => detonates_by_s.max(0.0),
        }
    }

    /// Seconds between the hazard's release and the moment its dangerous
    /// region first COVERS something `distance` away — `None` when it never
    /// does.
    ///
    /// This is the AIMING question: where do I point this so that it lands on
    /// them. A placed object is placed where it is placed, so it answers
    /// `0.0` — there is nothing to aim, and its fuse is
    /// [`Self::detonates_by_s`].
    ///
    /// ⚠ THE `None` IS LOAD-BEARING. A consumer that fell back to a number
    /// here would be leading its aim at a shot that cannot land, which is the
    /// class of defect that put a 1000px placeholder on the attack menu.
    pub fn travel_to(self, distance: f32) -> Option<f32> {
        if distance > self.reach() {
            return None;
        }
        match self {
            Self::Straight { speed, free, .. } => {
                let fly = (distance - free).max(0.0);
                if fly <= 0.0 {
                    return Some(0.0);
                }
                (speed > 0.0).then(|| fly / speed)
            }
            Self::Boomerang { v0, out_s, free } => {
                let fly = (distance - free).max(0.0);
                if fly <= 0.0 {
                    return Some(0.0);
                }
                if v0 <= 0.0 || out_s <= 0.0 {
                    return None;
                }
                // The EARLIER root of `t^2 - 2 out_s t + 2 out_s fly / v0 = 0`,
                // which is the out-leg; the later root is the same distance on
                // the way back, which this does not model. The discriminant is
                // non-negative exactly while `fly <= v0 out_s / 2`, and the
                // guard above already established that.
                let disc = (out_s * out_s - 2.0 * out_s * fly / v0).max(0.0);
                Some(out_s - disc.sqrt())
            }
            Self::Placed { .. } => Some(0.0),
        }
    }
}

/// **WHAT A MOVE PUTS INTO THE WORLD THAT CAN HURT SOMEBODY** — and, for the
/// one shape the catalog cannot measure, a request for the layer that can.
///
/// ⛔⛤ **THIS REPLACED A BARE `hazard_reach: f32`, AND THE REASON IS THE
/// REVIEW FINDING OF 2026-09-20:** *"continuing to add exceptions for ranged
/// actions, bombs, summons, bolts, etc. will create a second approximate
/// combat model."* A single distance answered three different questions at
/// once — how far the hazard gets, whether there IS one, and whose number it
/// is — so each new road was a new special case folded into one `max`, and the
/// one road whose numbers live on the BODY had a placeholder folded in beside
/// real measurements with nothing marking it.
///
/// ⭐ **THE SPEED IS THE FIELD THAT WAS BEING THROWN AWAY.** The old fold
/// computed `speed × lifetime` and kept only the product, so a consumer
/// leading its aim could not ask when the hazard ARRIVES — only when it is
/// thrown. Measured on `director_train_of_thought`: the bolt crosses 671px at
/// 300px/s, so it lands up to two seconds after the throw, and the brain aimed
/// it at where the opponent was when it left.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveHazard {
    /// An authored hazard the catalog can measure whole, carrying the law it
    /// travels by rather than a sample of it.
    Spawned {
        /// How it covers the ground between leaving its owner and touching
        /// somebody.
        travel: ThreatTravel,
        /// **WHAT IT TAKES OFF THEM WHEN IT ARRIVES.**
        ///
        /// ⛔⛤ **A HAZARD MOVE WAS PRICED AT ZERO POWER UNTIL 2026-09-21.**
        /// [`MoveFrameData::max_damage`](crate::MoveFrameData::max_damage) folds ACTIVE VOLUMES and a shot is
        /// not one, so the one class of move whose whole job is to deal
        /// damage from across the stage reported dealing none. Measured on
        /// Projectile Polygon: her ponytail boomerang deals `7` and her
        /// charge shot `4`, and the option scorer could see neither — so
        /// once increment one gave the boomerang its true 83px of reach she
        /// fell through to the weaker shot at range, which is the loss that
        /// named this field.
        ///
        /// ⚠ **THE UNCHARGED SHOT, for the same reason the travel law is
        /// the uncharged one**: a `RangedCharge` multiplies damage as well
        /// as speed, and the press a brain is weighing is the one it is
        /// about to make rather than the one it might hold for.
        damage: i32,
    },
    /// The move pulls the owner's OWN ranged trigger
    /// ([`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged)), whose speed, flight and lifetime are the
    /// BODY's `RangedActionSpec` and not the move's.
    ///
    /// ⚠ **A REQUEST, NOT AN ANSWER.** [`Self::reach`] answers it with the
    /// standing fallback so an unjoined INTROSPECTING reader is no worse off
    /// than before this type existed. A reader that can see the body is
    /// expected to replace the variant outright — and, when the body turns out
    /// to carry no ranged action at all, to replace it with NOTHING.
    ///
    /// ⛔⛤ **IT MUST NEVER REACH A FIGHTER'S KIT, AND IT USED TO.** Reviewed
    /// 2026-09-20: `resolve_owners_ranged_action` returned early when neither
    /// an equipped weapon nor the body's standing action could answer, leaving
    /// the request in place — so the one layer that had just PROVEN the move
    /// fires nothing handed the brain a 1000px instantaneous threat. The join
    /// now clears the hazard on that road, and [`Self::travel_to`] refuses to
    /// answer for this variant so a consumer cannot quietly re-derive one.
    OwnersRangedAction,
}

impl MoveHazard {
    /// How far this hazard reaches, with [`RANGED_ACTION_REACH`] standing in
    /// for an unresolved ranged action.
    pub fn reach(self) -> f32 {
        match self {
            Self::Spawned { travel, .. } => travel.reach(),
            Self::OwnersRangedAction => RANGED_ACTION_REACH,
        }
    }

    /// Seconds from release until this hazard's dangerous region covers
    /// something `distance` away — `None` when it never does, and `None` for
    /// an UNRESOLVED ranged action, which has no law to answer with.
    pub fn travel_to(self, distance: f32) -> Option<f32> {
        match self {
            Self::Spawned { travel, .. } => travel.travel_to(distance),
            Self::OwnersRangedAction => None,
        }
    }

    /// Does this hazard's DAMAGE land at the time [`Self::travel_to`] answers?
    ///
    /// ⛔⛤ **`Placed` ANSWERS `Some(0.0)` AND THAT IS RIGHT FOR AIMING AND
    /// WRONG FOR PAYING — REVIEW 2026-09-21.** A laid object does not travel,
    /// so *"how long until it can touch somebody at this distance"* is
    /// genuinely zero; the option scorer then read that as *"the damage is
    /// available the moment it is dropped"* and spent the polygon's 12-damage
    /// bomb as an immediate punish, inside the 72px `Placed` reach. The bomb's
    /// activation is a four-second fuse OR a hard impact, whichever comes
    /// first — and this type's own doc already says nothing prices the fuse.
    ///
    /// ⇒ **THE SPLIT IS BETWEEN TWO QUESTIONS, NOT A NEW SCALAR.** `travel_to`
    /// stays the aiming answer. This says whether the arrival is also the
    /// moment the damage happens, which is true of a thing that FLIES and
    /// false of a thing that WAITS. A trap's worth is a stage-control
    /// question, and until the brain has a feature for that its direct strike
    /// payoff is honestly zero rather than optimistically immediate.
    ///
    /// ⚠ **`OwnersRangedAction` IS FALSE FOR A DIFFERENT REASON, AND THE
    /// DIFFERENCE IS WORTH KEEPING:** its numbers live on the BODY, so an
    /// unresolved request cannot answer either question. [`Self::damage`]
    /// already contributes nothing for it; this keeps the timing half
    /// consistent instead of letting an unresolvable request read as a
    /// same-instant hit.
    ///
    /// ⛔ A NEW TRAVEL LAW MUST ANSWER THIS. That is why it is a match on the
    /// variant rather than a field on `Spawned`: adding a variant is a compile
    /// error here, and a defaulted `true` is exactly the optimistic reading
    /// this removes.
    pub fn strikes_on_arrival(self) -> bool {
        match self {
            Self::Spawned { travel, .. } => match travel {
                ThreatTravel::Straight { .. } | ThreatTravel::Boomerang { .. } => true,
                ThreatTravel::Placed { .. } => false,
            },
            Self::OwnersRangedAction => false,
        }
    }

    /// When this hazard goes off by itself, seconds from release; `0.0` for
    /// an unresolved ranged action, which claims nothing.
    pub fn detonates_by_s(self) -> f32 {
        match self {
            Self::Spawned { travel, .. } => travel.detonates_by_s(),
            Self::OwnersRangedAction => 0.0,
        }
    }

    /// **WHAT THIS HAZARD TAKES OFF WHOEVER IT REACHES** — `0` for an
    /// UNRESOLVED ranged action, whose damage is on the BODY and not in
    /// anything the catalog can read.
    ///
    /// ⚠ **`0` HERE IS A REFUSAL AND NOT A MEASUREMENT**, the same way
    /// [`Self::travel_to`]'s `None` is: a reader that can see the body is
    /// expected to replace the variant outright. See
    /// [`MoveFrameData::strongest_hit`](crate::MoveFrameData::strongest_hit), which is where the two roads meet.
    pub fn damage(self) -> i32 {
        match self {
            Self::Spawned { damage, .. } => damage,
            Self::OwnersRangedAction => 0,
        }
    }
}

/// **WHAT AN AUTHORED TECHNIQUE PUTS IN THE WORLD THAT CAN HURT SOMEBODY**,
/// from where the move puts it. `None` for every key that puts no hazard
/// anywhere, which is most of them.
///
/// ⛔⛤ **`coverage: None` MEANT "THIS MOVE CANNOT MISS" AND FOR A LAUNCHER IT
/// MEANS THE OPPOSITE.** A move whose damage rides a projectile authors no
/// Active volume on its owner's body, so it folded to the same `None` as a
/// counter, a buff and a taunt — and an option scorer offered all four at every
/// range and priced all four at zero.
///
/// ⚠ **THE TABLE IS THE ROSTER'S, NOT A SURVEY OF THE VOCABULARY.** Measured
/// 2026-09-20 by `authored_movesets::offer_census`: of the moves that land no
/// volume and shove nobody, exactly two reach through something they spawn —
/// `director_train_of_thought` (a steered bolt) and `polygon_lay_bomb`. The
/// other hazard techniques this crate declares — `smash_mine::PLACE_MINE`,
/// `smash_mark::MARK_BODY`, `smash_tether::TETHER_PULL`,
/// `smash_homing::HOMING_DASH` — have no authored customer that needs an answer
/// here, and inventing one for zero callers is the generalisation nobody asked
/// for. Add the arm with the move.
///
/// ⚠ **A KEY THIS HAS NOT BEEN TAUGHT ANSWERS `None`, and that is a REFUSAL,
/// not a neutral default**: the admission rule reads the absence as *"this
/// move offers the opponent nothing"* and keeps it off the attack menu. Safe,
/// and loud enough to notice — a new projectile that is never thrown is the
/// symptom.
///
/// ⛔ AND THE BIGGEST PROJECTILE ROAD IS NOT A KEY AT ALL. Every ordinary
/// ranged move pulls the owner's own trigger through [`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged)
/// — `polygon_ponytail_boomerang` and `polygon_projectile_charge_shot` are
/// both that shape, and both author no Active volume, so a table of effect
/// keys alone would have taken the reference projectile fighter's whole game
/// off the menu. That arm is folded in beside this one and answers
/// [`MoveHazard::OwnersRangedAction`].
pub(crate) fn hazard_of(effect: &EffectRef) -> Option<MoveHazard> {
    let hazard = match effect.key.as_str() {
        // A bolt travels under its own power until its clock runs out. Its
        // speed is documented CONSTANT, so this is the whole flight — and an
        // UPPER bound, because a steered bolt that turns covers less ground
        // than one flown straight.
        crate::smash_bolt::STEERED_BOLT => effect
            .params
            .hydrate::<crate::smash_bolt::SteeredBoltParams>()
            .map(|p| {
                MoveHazard::Spawned {
                    travel: ThreatTravel::Straight {
                        speed: p.speed,
                        span: p.speed * p.lifetime_s,
                        free: p.offset.0.abs() + p.radius,
                    },
                    damage: p.damage,
                }
            })
            .ok(),
        // ⛔⛤ **A DROP BOMB IS DROPPED, NOT THROWN.** It appears at `offset`
        // and the blast is the only thing that travels, so its reach is where
        // it lands plus how far the blast carries. `impact_speed` is a
        // DETONATION THRESHOLD — *"minimum contact speed that detonates the
        // bomb"* — and reading it as a launch speed put the polygon's bomb at
        // 1096px, further than the bolt and further than the stage, off a
        // quantity that is not a distance per second of anything.
        crate::smash_bomb::DROP_BOMB => effect
            .params
            .hydrate::<crate::smash_bomb::DropBombParams>()
            .map(|p| {
                MoveHazard::Spawned {
                    travel: ThreatTravel::Placed {
                        reach: p.offset.0.abs() + p.blast_radius,
                        // ⛔⛤ **THE FUSE, AND IT USED TO BE ZERO.** This published
                        // `speed: 0.0`, documented as *"the whole reach is
                        // available the moment it exists"* — the opposite of what
                        // the move authors. `fuse_s` is *"seconds until it goes
                        // off by itself"*, four of them on the shipped polygon, so
                        // a brain pricing the drop as an immediate blast was
                        // pricing a trap as a strike.
                        //
                        // ⚠ IT IS THE LATEST, NOT THE ONLY, MOMENT: a bomb also
                        // detonates on a hard enough impact (`impact_speed`),
                        // which is SOONER and depends on what somebody else does
                        // to it. The deadline is the part the thrower can count
                        // on. (This comment said "earliest" for one commit, beside
                        // the field the rename had just corrected — which is the
                        // worse half of a rename, because prose is what a reader
                        // trusts when the name and the comment disagree.)
                        detonates_by_s: p.fuse_s,
                    },
                    // The blast at the CENTRE, which is what the move's own
                    // authoring calls its damage. A body caught at the edge
                    // of the radius takes the same number today; if the blast
                    // ever falls off with distance that is a law for
                    // `ThreatTravel::Placed` to carry, not a second scalar.
                    damage: p.damage,
                }
            })
            .ok(),
        _ => None,
    };
    // ⛔ A NON-POSITIVE REACH IS NO HAZARD, not a hazard of zero length: the
    // admission rule reads the ABSENCE as "this move offers the opponent
    // nothing", and a `Some` carrying 0.0 would be admitted at point blank.
    hazard.filter(|h| h.reach() > 0.0)
}
