//! What an authored technique puts in the world that can hurt somebody, and
//! the laws by which it gets there.
//!
//! `ThreatTravel`, `MoveHazard` and the `hazard_of` lowering are a closed
//! family: a hazard's travel law, what it does on contact, and the effect keys
//! the catalog knows. The family reads only the effect params it lowers from.
//!
//! `hazard_of` is `pub(crate)`: `MoveSpec::frame_data` calls it. Callers
//! outside this crate read [`MoveFrameData::hazard`], the joined answer.
//!
//! [`MoveFrameData::hazard`]: crate::MoveFrameData::hazard

use crate::EffectRef;

/// What an unjoined reader gets for a move that pulls the owner's own ranged
/// trigger.
///
/// The body owns the number, not the move:
/// [`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged) fires the body's
/// `RangedActionSpec` (speed, flight, lifetime), and a catalog derivation has
/// no body. So this states only what is true of all of them: a shot crosses
/// ground a swing cannot. It is wider than any shipped stage, so to a reader
/// that cannot narrow it, a ranged move is admitted wherever the opponent is.
///
/// The kit builder joins a move to its body's action and answers
/// [`MoveHazard::OwnersRangedAction`] with real numbers.
pub const RANGED_ACTION_REACH: f32 = 1_000.0;

/// How a hazard covers the ground between leaving its owner and touching
/// somebody: the law, not a sample of it.
///
/// A `(reach, speed)` pair describes only uniform motion, and some shipped
/// shapes are not uniform:
///
/// - A boomerang decelerates to a standstill at its turnaround, so its average
///   speed is correct only at maximum range. (Projectile Polygon's ponytail,
///   `v0` 430px/s turning at 0.34s, reaches 40px in 0.111s; the average-speed
///   model gives 0.186s.)
/// - A laid bomb is an object on a fuse, not a projectile whose blast is live
///   when it lands.
///
/// So the type answers `time_to(distance)`, not `speed`. A shape that cannot
/// reach a distance returns `None`.
///
/// Add a variant together with the content that needs it. A homing shot, a
/// tether or a proximity mine is a new law, not a new scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreatTravel {
    /// Constant velocity, out to `span` of centre travel.
    Straight {
        /// px/s, constant.
        speed: f32,
        /// How far the hazard's center travels before it expires.
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
    /// The return leg is not modeled: a fighter deciding whether to throw one
    /// asks when it first connects, which is on the out-leg.
    Boomerang {
        /// Launch speed, px/s, before the deceleration.
        v0: f32,
        /// When it reaches the turnaround, seconds after release.
        out_s: f32,
        /// As [`Self::Straight::free`].
        free: f32,
    },
    /// An object put somewhere, which goes off by itself after
    /// `detonates_by_s` and can hurt somebody within `reach`. A bomb on a fuse.
    Placed {
        /// How far from the body the blast can touch somebody.
        ///
        /// Known debt: this is a radius, but the placement is a vector. The
        /// polygon's bomb is authored at `offset (-16, +14)` (behind and
        /// below) with a 56px blast, and this collapses that to
        /// `|offset.x| + blast_radius` = 72px, discarding direction and `y`. The
        /// fix is to carry the region, not another scalar (tracked on
        /// queue.md's BRAIN row).
        reach: f32,
        /// Seconds after the object appears before it goes off by itself.
        ///
        /// A deadline, not an earliest time. The shipped bomb detonates after
        /// its fuse or on a hard enough impact (`DropBombParams::impact_speed`),
        /// whichever is first. The impact road depends on others and is not
        /// modeled; the deadline is what the thrower can count on.
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
    /// anything dangerous from the moment it exists.
    ///
    /// A deadline, not an earliest time (see [`Self::Placed::detonates_by_s`]).
    ///
    /// This is a separate question from [`Self::travel_to`]. Feeding a bomb's
    /// fuse into the aim lead would extrapolate the opponent seconds ahead,
    /// out of the blast, and the brain would stop using the bomb.
    ///
    /// Nothing prices the fuse yet. "Will they be within 72px in four
    /// seconds" is a stage-control question, and the attack-admission rule
    /// prices strikes. Like counters and buffs, traps stay off the attack
    /// ranking until a defensive feature can price them.
    pub fn detonates_by_s(self) -> f32 {
        match self {
            Self::Straight { .. } | Self::Boomerang { .. } => 0.0,
            Self::Placed {
                detonates_by_s, ..
            } => detonates_by_s.max(0.0),
        }
    }

    /// Seconds between the hazard's release and the moment its dangerous
    /// region first covers something `distance` away. `None` when it never
    /// does.
    ///
    /// This is the aiming question. A placed object answers `0.0`: there is
    /// nothing to aim, and its fuse is [`Self::detonates_by_s`].
    ///
    /// The `None` is required. A consumer that fell back to a number would
    /// lead its aim at a shot that cannot land.
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
                // The earlier root of `t^2 - 2 out_s t + 2 out_s fly / v0 = 0`,
                // which is the out-leg; the later root is the return leg, not
                // modeled. The discriminant is non-negative while
                // `fly <= v0 out_s / 2`, which the guard above ensures.
                let disc = (out_s * out_s - 2.0 * out_s * fly / v0).max(0.0);
                Some(out_s - disc.sqrt())
            }
            Self::Placed { .. } => Some(0.0),
        }
    }
}

/// What a move puts into the world that can hurt somebody. For the one shape
/// the catalog cannot measure, it is a request to the layer that can.
///
/// A single distance answered three questions at once: how far the hazard
/// gets, whether there is one, and whose number it is. This type separates
/// them, so each new hazard is not another special case folded into a `max`.
///
/// It keeps the travel law, so a consumer can ask when the hazard arrives,
/// not only when it is thrown. (A bolt that crosses 671px at 300px/s lands up
/// to two seconds after the throw.)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveHazard {
    /// An authored hazard the catalog can measure whole, carrying the law it
    /// travels by rather than a sample of it.
    Spawned {
        /// How it covers the ground between leaving its owner and touching
        /// somebody.
        travel: ThreatTravel,
        /// What it takes off the target when it arrives.
        ///
        /// [`MoveFrameData::max_damage`](crate::MoveFrameData::max_damage)
        /// folds only Active volumes, so without this a hazard move would be
        /// priced at zero power.
        ///
        /// The uncharged shot, like the travel law: a `RangedCharge` multiplies
        /// damage and speed, and the brain weighs the press it is about to
        /// make.
        damage: i32,
    },
    /// The move pulls the owner's own ranged trigger
    /// ([`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged)), whose speed,
    /// flight and lifetime are the body's `RangedActionSpec`, not the move's.
    ///
    /// A request, not an answer. [`Self::reach`] answers it with the standing
    /// fallback for an unjoined introspecting reader. A reader that can see the
    /// body replaces the variant; if the body has no ranged action, it
    /// replaces it with nothing.
    ///
    /// It must never reach a fighter's kit. `resolve_owners_ranged_action`
    /// clears the hazard when neither an equipped weapon nor the body's
    /// standing action answers, and [`Self::travel_to`] refuses to answer for
    /// this variant.
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
    /// something `distance` away. `None` when it never does, and `None` for an
    /// unresolved ranged action, which has no law.
    pub fn travel_to(self, distance: f32) -> Option<f32> {
        match self {
            Self::Spawned { travel, .. } => travel.travel_to(distance),
            Self::OwnersRangedAction => None,
        }
    }

    /// Does this hazard's damage land at the time [`Self::travel_to`] answers?
    ///
    /// `Placed` answers `Some(0.0)` to `travel_to`, which is right for aiming
    /// and wrong for paying: the bomb's damage comes after its fuse or on a
    /// hard impact, not when it is dropped. So `travel_to` stays the aiming
    /// answer, and this says whether arrival is also the damage moment: true
    /// for a thing that flies, false for a thing that waits. Until the brain
    /// can price stage control, a trap's direct strike payoff is zero.
    ///
    /// `OwnersRangedAction` is false for another reason: its numbers are on
    /// the body, so an unresolved request answers neither question.
    /// [`Self::damage`] also gives nothing for it.
    ///
    /// A match on the variant, not a field on `Spawned`, so a new travel law
    /// is a compile error here and must answer. A default of `true` would be
    /// the optimistic reading this avoids.
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

    /// What this hazard takes off whoever it reaches. `0` for an unresolved
    /// ranged action, whose damage is on the body.
    ///
    /// That `0` is a refusal, not a measurement, like [`Self::travel_to`]'s
    /// `None`: a reader that can see the body replaces the variant. See
    /// [`MoveFrameData::strongest_hit`](crate::MoveFrameData::strongest_hit),
    /// where the two roads meet.
    pub fn damage(self) -> i32 {
        match self {
            Self::Spawned { damage, .. } => damage,
            Self::OwnersRangedAction => 0,
        }
    }
}

/// What an authored technique puts in the world that can hurt somebody, from
/// where the move puts it. `None` for keys that put no hazard anywhere (most
/// of them).
///
/// A launcher authors no Active volume on its owner's body, so without this
/// its `coverage: None` would look like a counter, a buff or a taunt.
///
/// The table covers the roster, not the whole vocabulary. Other hazard
/// techniques (`smash_mine::PLACE_MINE`, `smash_mark::MARK_BODY`,
/// `smash_tether::TETHER_PULL`, `smash_homing::HOMING_DASH`) have no authored
/// customer yet. Add the arm with the move.
///
/// A key not listed here answers `None`. The admission rule reads that as
/// "this move offers the opponent nothing" and keeps it off the attack menu.
/// The symptom is a new projectile that is never thrown.
///
/// The main projectile road is not a key: an ordinary ranged move pulls the
/// owner's trigger through
/// [`MoveEventKind::Ranged`](crate::MoveEventKind::Ranged) and authors no
/// Active volume. That arm is handled beside this one and answers
/// [`MoveHazard::OwnersRangedAction`].
pub(crate) fn hazard_of(effect: &EffectRef) -> Option<MoveHazard> {
    let hazard = match effect.key.as_str() {
        // A bolt travels under its own power until its clock runs out. Its
        // speed is constant, so this is the whole flight. It is an upper
        // bound: a steered bolt that turns covers less ground.
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
        // A drop bomb is dropped, not thrown. It appears at `offset` and only
        // the blast travels, so its reach is where it lands plus the blast
        // radius. `impact_speed` is a detonation threshold, not a launch
        // speed.
        crate::smash_bomb::DROP_BOMB => effect
            .params
            .hydrate::<crate::smash_bomb::DropBombParams>()
            .map(|p| {
                MoveHazard::Spawned {
                    travel: ThreatTravel::Placed {
                        reach: p.offset.0.abs() + p.blast_radius,
                        // The fuse. It is the latest moment, not the only one:
                        // a bomb also detonates on a hard enough impact
                        // (`impact_speed`), which depends on others. The
                        // deadline is what the thrower can count on.
                        detonates_by_s: p.fuse_s,
                    },
                    // The blast at the center, which the move's authoring
                    // calls its damage. There is no falloff today; if one is
                    // added, it is a law for `ThreatTravel::Placed`, not a
                    // second scalar.
                    damage: p.damage,
                }
            })
            .ok(),
        _ => None,
    };
    // A non-positive reach is no hazard. The admission rule reads absence as
    // "offers nothing", and a `Some` of 0.0 would be admitted at point blank.
    hazard.filter(|h| h.reach() > 0.0)
}
