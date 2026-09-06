//! Per-frame combat relationship and `ActorTarget` selection.
//!
//! Autonomous actors consume their own `ActorTarget`; target selection must not depend on a global
//! primary-player entity.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use super::components::{
    ActiveCombatant, ActorAggression, ActorDisposition, ActorFaction, ActorTarget, AggressionTarget,
    CenteredAabb,
};
use super::FeatureSimEntity;
use ambition_characters::actor::BodyHealth;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Number of [`ActorFaction`] variants (Player / Enemy / Npc / Boss / Neutral).
/// The relations matrix is indexed by `faction as usize`.
const FACTION_COUNT: usize = 5;

/// Directed faction hostility used for autonomous target selection.
///
/// `hostile[from][to]` means `from` actors may select `to` actors as foes. The default declares
/// Player ↔ Enemy and Player ↔ Boss hostile; all other faction pairs are non-hostile.
#[derive(Resource, Clone, Debug)]
pub struct FactionRelations {
    hostile: [[bool; FACTION_COUNT]; FACTION_COUNT],
}

impl Default for FactionRelations {
    fn default() -> Self {
        let mut relations = Self {
            hostile: [[false; FACTION_COUNT]; FACTION_COUNT],
        };
        // The combat baseline: the player and the hostile world are at war.
        relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Enemy, true);
        relations.set_mutual_hostile(ActorFaction::Player, ActorFaction::Boss, true);
        relations
    }
}

impl FactionRelations {
    /// True iff `from`-faction actors currently treat `to`-faction actors as
    /// combat targets.
    pub fn is_hostile(&self, from: ActorFaction, to: ActorFaction) -> bool {
        self.hostile[from as usize][to as usize]
    }

    /// Set the one-directional stance `from → to`. Stealth/bounty/alliance
    /// systems call this; for mutual hostility call it both ways.
    pub fn set_hostile(&mut self, from: ActorFaction, to: ActorFaction, hostile: bool) {
        self.hostile[from as usize][to as usize] = hostile;
    }

    /// Set mutual hostility between two factions (both directions).
    pub fn set_mutual_hostile(&mut self, a: ActorFaction, b: ActorFaction, hostile: bool) {
        self.set_hostile(a, b, hostile);
        self.set_hostile(b, a, hostile);
    }
}

/// Whether same-faction allies may damage each other.
///
/// Different-faction overlaps remain physically damaging even when the factions are not hostile;
/// targeting and damage authorization are distinct questions.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct FriendlyFire {
    pub enabled: bool,
}

/// Register combat-owned relationship resources.
pub fn init_targeting_resources(app: &mut App) {
    app.init_resource::<FactionRelations>();
    app.init_resource::<FriendlyFire>();
}

/// Whether an `attacker`-faction hit may damage a `victim`-faction body. The
/// engine rule (see [`FriendlyFire`]): damage lands on any DIFFERENT faction;
/// same-faction is blocked unless friendly fire is enabled. Self-exclusion
/// (attacker entity == victim entity) is handled by the caller.
pub fn can_damage(
    attacker: ActorFaction,
    victim: ActorFaction,
    friendly_fire: FriendlyFire,
) -> bool {
    friendly_fire.enabled || attacker != victim
}

/// Resolve combat allegiance without mutating authored faction state.
///
/// A participant-driven body fights as [`ActorFaction::Player`]; otherwise its authored faction
/// applies. Possession therefore needs no faction overwrite/restore path.
pub fn effective_faction(
    authored: ActorFaction,
    driver: Option<&ambition_characters::control::DrivingParticipant>,
) -> ActorFaction {
    if driver.is_some() {
        ActorFaction::Player
    } else {
        authored
    }
}

/// Ruleset team identity for match participants.
///
/// When both bodies have teams, team relation outranks faction: same-team bodies are allies and
/// different-team bodies are foes. Bodies outside a team continue to use grudge/faction policy.
#[derive(bevy::prelude::Component, Clone, Debug, PartialEq, Eq)]
pub struct MatchTeam(pub String);

impl MatchTeam {
    pub fn new(team: impl Into<String>) -> Self {
        Self(team.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The team relation between two bodies, when both have one.
///
/// `None` means teams have nothing to say — at least one side is not in a match
/// — and the faction rule decides, exactly as before.
pub fn team_allows_damage(
    attacker: Option<&MatchTeam>,
    victim: Option<&MatchTeam>,
) -> Option<bool> {
    match (attacker, victim) {
        (Some(a), Some(v)) => Some(a != v),
        _ => None,
    }
}

/// Combat relationship shared by target selection and damage authorization.
///
/// `Foe` is targetable and damageable; `Ally` needs friendly fire to take damage; `Neutral` is not
/// targeted but can still be struck physically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRelation {
    Ally,
    Neutral,
    Foe,
}

impl CombatRelation {
    /// May a strike from the attacker damage this body?
    ///
    /// Physical: anything not an ally can be hit. An ally needs friendly fire.
    pub fn damage_lands(self, friendly_fire: FriendlyFire) -> bool {
        match self {
            CombatRelation::Foe | CombatRelation::Neutral => true,
            CombatRelation::Ally => friendly_fire.enabled,
        }
    }

    /// Is this a body an autonomous brain should go after?
    ///
    /// Relational: only a declared foe. A neutral bystander is left alone.
    pub fn is_target(self) -> bool {
        matches!(self, CombatRelation::Foe)
    }
}

/// Resolve the relationship used by both targeting and damage policy.
///
/// Precedence is grudge, then team when both bodies have one, then effective faction and the
/// optional [`FactionRelations`] matrix. Participant control affects effective faction without
/// mutating authored allegiance.
#[allow(clippy::too_many_arguments)]
pub fn combat_relation(
    // `None` = no matrix opinion, which is what the DAMAGE side passes. Damage is physical:
    // a swing that reaches a different-faction body hurts it whether or not the two are
    // declared enemies, so the matrix arm is skipped and such a pair resolves
    // [`CombatRelation::Neutral`] — hittable, not huntable.
    relations: Option<&FactionRelations>,
    attacker_faction: ActorFaction,
    attacker_driver: Option<&ambition_characters::control::DrivingParticipant>,
    attacker_team: Option<&MatchTeam>,
    attacker_grudge: Option<Entity>,
    candidate: Entity,
    candidate_faction: ActorFaction,
    candidate_driver: Option<&ambition_characters::control::DrivingParticipant>,
    candidate_team: Option<&MatchTeam>,
) -> CombatRelation {
    if attacker_grudge == Some(candidate) {
        return CombatRelation::Foe;
    }
    if let Some(different_team) = team_allows_damage(attacker_team, candidate_team) {
        return if different_team {
            CombatRelation::Foe
        } else {
            CombatRelation::Ally
        };
    }
    let attacker_faction = effective_faction(attacker_faction, attacker_driver);
    let candidate_faction = effective_faction(candidate_faction, candidate_driver);
    if relations.is_some_and(|r| r.is_hostile(attacker_faction, candidate_faction)) {
        CombatRelation::Foe
    } else if attacker_faction == candidate_faction {
        CombatRelation::Ally
    } else {
        CombatRelation::Neutral
    }
}

pub fn damage_lands(
    attacker: ActorFaction,
    victim: ActorFaction,
    friendly_fire: FriendlyFire,
    attacker_grudge: Option<Entity>,
    victim_entity: Entity,
) -> bool {
    can_damage(attacker, victim, friendly_fire) || attacker_grudge == Some(victim_entity)
}

/// [`damage_lands`], with a TEAM relation taking precedence when both bodies
/// have one.
///
/// A grudge still overrides: it is a per-entity feud and it is deliberately
/// stronger than any group rule, which is what lets two teammates settle
/// something the ruleset did not anticipate.
pub fn damage_lands_between(
    attacker: ActorFaction,
    victim: ActorFaction,
    attacker_team: Option<&MatchTeam>,
    victim_team: Option<&MatchTeam>,
    friendly_fire: FriendlyFire,
    attacker_grudge: Option<Entity>,
    victim_entity: Entity,
) -> bool {
    // the damage side of [`combat_relation`], and it must not grow a second
    // opinion. Callers pass ALREADY-EFFECTIVE factions here (the policy
    // re-resolves them, which is idempotent), so this stays a projection.
    //
    // one behaviour deliberately changed: friendly fire now also frees
    // same-TEAM damage. The old team arm returned a bare "different team?" and
    // ignored the toggle, so a teams match could never enable friendly fire —
    // which is a real platform-fighter setting, and the flag says what it means.
    combat_relation(
        None,
        attacker,
        None,
        attacker_team,
        attacker_grudge,
        victim_entity,
        victim,
        None,
        victim_team,
    )
    .damage_lands(friendly_fire)
}

/// Pick each non-player actor's `ActorTarget` and settle target-derived standing.
///
/// Players and non-player actors are one relational candidate population. A body
/// with no live foe is explicitly target-less; if it is socially hostile rather
/// than an [`ActiveCombatant`], this same authority stands it down to
/// [`ActorDisposition::Peaceful`]. Match participation is not inferred from
/// whether a target exists.
///
/// The current implementation is O(n²) in the all-actor case. A many-body build
/// can swap in a spatial index here without changing the consumer side.
pub fn select_actor_targets(
    relations: Option<Res<FactionRelations>>,
    // The player carries an `ActorFaction` (Player) like every body — read it so the
    // player is a RELATIONAL candidate (a foe only if this actor's faction opposes
    // Player, or it holds a grudge against this player), never an unconditional one.
    players: Query<
        (
            Entity,
            &BodyKinematics,
            &BodyHealth,
            &ActorFaction,
            Option<&MatchTeam>,
            // The world's hands are off it — see the candidate filter below.
            Has<crate::death_rules::OutOfPlay>,
        ),
        With<PlayerEntity>,
    >,
    // Non-player actors are candidate targets too (the relational, non-player-
    // centric part): an actor can target another actor whose faction it's hostile
    // to. Snapshotted, so this read-only borrow ends before the mutable pass.
    // `Option<&DrivingParticipant>` on both candidate and acting queries: a
    // possessed body (one a participant drives) is a Player-EFFECTIVE
    // candidate/actor without its authored `ActorFaction` being mutated — so
    // former allies target it and it targets them, purely through effective
    // allegiance.
    others: Query<
        (
            Entity,
            &CenteredAabb,
            &ActorFaction,
            &BodyHealth,
            Option<&ambition_characters::control::DrivingParticipant>,
            // A match seat, when this body is in one. Selection has to see it or
            // it answers a different question from the damage side — see
            // [`combat_relation`].
            Option<&MatchTeam>,
            // The world's hands are off it — see the candidate filter below.
            Has<crate::death_rules::OutOfPlay>,
        ),
        With<FeatureSimEntity>,
    >,
    mut actors: Query<
        (
            Entity,
            &CenteredAabb,
            &mut ActorTarget,
            &ActorAggression,
            Option<&ActorFaction>,
            Option<&ambition_characters::control::DrivingParticipant>,
            Option<&MatchTeam>,
            // Social hostility follows target ownership: when selection proves
            // there is no foe, this same authority may stand a non-match actor down.
            Option<&mut ActorDisposition>,
            Has<ActiveCombatant>,
        ),
        // ⛔ AND AN OUT-OF-PLAY ACTOR DOES NOT ACQUIRE. The world's hands are off
        // it, which has to mean its hands are off the world too: a fighter
        // waiting out its death beat was still refreshing its own `ActorTarget`
        // and came back holding a lock it picked while dead.
        (With<FeatureSimEntity>, Without<crate::death_rules::OutOfPlay>),
    >,
    // Stable semantic identity, used ONLY to put the candidate list in a
    // canonical order — never to decide who is a foe. See the sort below.
    sim_ids: Query<&SimId>,
) {
    let relations = relations.map(|r| r.clone()).unwrap_or_default();
    // REACHABLE candidates only: a body the world cannot touch is never a valid
    // target. So the instant a foe dies the actor goes target-less — it stops
    // swinging at the corpse and stands down in this same targeting phase —
    // instead of chasing it until it despawns.
    //
    // ⛔⛔ AND HEALTH IS NOT THE UNIFORM LIVENESS GATE ANY MORE, though this
    // comment said it was. D201's stock loss calls `health.reset()` the instant
    // the stock is spent — a fighter comes back FRESH — so a body waiting out its
    // death beat reads FULL HEALTH while lying untouchable at the blast line. A
    // surviving CPU went on selecting, chasing and aiming at it for the whole
    // respawn interval, and the hit filters that stop it HURTING that body do
    // nothing about where it walks. `body_is_untouchable` is the gate that knows
    // both facts.
    // ONE candidate set — the player is just another body, carrying faction Player.
    // No unconditional player special-case; nearest foe wins.
    let mut candidates: Vec<(
        Entity,
        ae::Vec2,
        ActorFaction,
        Option<SimId>,
        Option<MatchTeam>,
    )> = players
        .iter()
        .filter(|(_, _, hp, _, _, out_of_play)| {
            !crate::util::body_is_untouchable(Some(*hp), *out_of_play)
        })
        .map(|(e, kin, _, faction, team, _)| {
            (
                e,
                kin.pos,
                *faction,
                sim_ids.get(e).ok().cloned(),
                team.cloned(),
            )
        })
        .chain(
            others
                .iter()
                .filter(|(_, _, _, hp, _, _, out_of_play)| {
                    !crate::util::body_is_untouchable(Some(*hp), *out_of_play)
                })
                .map(|(e, aabb, faction, _, driver, team, _)| {
                    (
                        e,
                        aabb.center,
                        effective_faction(*faction, driver),
                        sim_ids.get(e).ok().cloned(),
                        team.cloned(),
                    )
                }),
        )
        .collect();
    // Canonical candidate order BEFORE any nearest-foe scan. Bevy's Query order
    // is not stable, and under GGRS rollback entity recreation the raw `Entity`
    // ids are not stable either — so neither can be allowed to decide anything.
    // Sorting by the stable `SimId` makes the exact-distance tie-break below
    // reproducible across a rewind. (`None` SimIds sort last among themselves by
    // `Entity`; a body without semantic identity is not snapshot-relevant.)
    candidates.sort_by(|a, b| match (&a.3, &b.3) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.0.cmp(&b.0),
    });
    for (
        self_entity,
        aabb,
        mut target,
        aggression,
        faction,
        driver,
        self_team,
        mut disposition,
        active_combatant,
    ) in actors.iter_mut()
    {
        let actor_pos = aabb.center;
        // The acting body's OWN effective allegiance (Player while possessed). A
        // body with neither an authored faction nor player control has no
        // faction-relational foes (only a personal grudge can point it) — same as
        // the old `faction.is_some()` gate.
        let player_controlled = driver.is_some();
        let has_allegiance = faction.is_some() || player_controlled;
        let self_faction = effective_faction(faction.copied().unwrap_or_default(), driver);
        let policy = aggression.target_policy();
        // One relational rule: a candidate is a FOE iff this actor's faction is
        // hostile to it (`FactionRelations`) OR this actor holds a grudge against
        // that exact entity (a provoked NPC chasing its attacker). The player is a
        // candidate like any other — it's hunted because the actor's faction opposes
        // Player (a born Enemy) or it's the grudge target (a provoked NPC), never
        // because it is "the player". Nearest foe wins.
        let mut best: Option<(Entity, ae::Vec2, f32)> = None;
        if policy != AggressionTarget::None {
            for (entity, pos, cand_faction, _, cand_team) in &candidates {
                if *entity == self_entity {
                    continue;
                }
                // THE one relationship policy, the same call the damage side makes.
                //
                // `has_allegiance` still gates the matrix arm: a body with neither an
                // authored faction nor player control has no faction-relational foes
                // and can only be pointed by a grudge. A TEAM, on the other hand,
                // speaks for itself — being seated in a match IS an allegiance.
                let relation = combat_relation(
                    has_allegiance.then_some(&relations),
                    self_faction,
                    None,
                    self_team,
                    aggression.grudge,
                    *entity,
                    *cand_faction,
                    None,
                    cand_team.as_ref(),
                );
                if !relation.is_target() {
                    continue;
                }
                let d = distance_squared(*pos, actor_pos);
                // Deterministic nearest-foe selection: strictly-nearer wins, and an
                // EXACT distance tie is decided by the canonical candidate ORDER
                // established above (first-seen wins), not by comparing raw `Entity`
                // ids. That distinction is load-bearing under GGRS: bevy_ggrs
                // destroys and recreates rollback entities, so `Entity` values are
                // NOT stable across a rewind and an id comparison could silently
                // flip the target of a symmetric two-foe setup mid-resimulation.
                let better = match best {
                    None => true,
                    Some((_, _, best_d)) => d < best_d,
                };
                if better {
                    best = Some((*entity, *pos, d));
                }
            }
        }
        if let Some((entity, pos, _)) = best {
            target.pos = pos;
            target.entity = Some(entity);
        } else {
            // No valid foe (faction-neutral with no grudge, or its foe is gone):
            // point at self so facing math reads a zero direction (hold facing).
            target.pos = actor_pos;
            target.entity = None;
        }
        // ⭐⭐ TARGET-DERIVED STANDING IS DERIVED IN BOTH DIRECTIONS, and until
        // now only one of them was written. Standing a target-less hostile down
        // to `Peaceful` while REACQUISITION reads `aggression.target_policy()`
        // meant a body could come back to a fight without coming back to being
        // hostile:
        //
        //     Hostile aggression + Hostile disposition
        //       -> last foe disappears -> disposition stands down to Peaceful
        //       -> a new faction foe arrives
        //       -> target reacquired by AGGRESSION, disposition still Peaceful
        //
        // and that body then attacks a foe while `Peaceful` tells the
        // interaction system it is talkable and `CombatStanding` calls it a
        // Bystander. Two authorities disagreeing about one fact - a latch set on
        // ABSENCE that nothing ever cleared.
        //
        // ⛔ THIS IS A TEMPORARY STANDING, NOT A PACIFY, and the distinction is
        // what keeps it composable. A deliberate pacify
        // (`brain_command`'s disposition authority) sets aggression to Passive
        // AND disposition to Peaceful together; because that drops
        // `target_policy` to `None`, the restore below cannot fire against it.
        // Permanent peace stays that authority's to declare.
        if let Some(disposition) = disposition.as_deref_mut() {
            let fighting = target.entity.is_some() && policy == AggressionTarget::Foe;
            if fighting {
                if !disposition.is_hostile() {
                    *disposition = ActorDisposition::Hostile;
                }
            } else if target.entity.is_none() && !active_combatant && disposition.is_hostile() {
                *disposition = ActorDisposition::Peaceful;
            }
        }
    }
}

/// Dissolve grudges that have SETTLED, so a feud resolves to peace on its own.
///
/// A grudge is a per-entity hostility (the duel mechanism); like any feud it should
/// END once it's decided. Two rules, both keyed off the one uniform liveness
/// authority ([`BodyHealth`]):
///
/// - You forget a slain foe. When a body's grudge target is no longer alive, the
///   grudge clears. The targeting filter already drops a dead foe so the holder stands
///   down ([`select_actor_targets`]); clearing the grudge too means it won't re-aggro
///   if that foe later revives — the duel survivor settles into a normal NPC for good.
/// - A defeated body forgets its feud. When a body itself is down (health 0,
///   awaiting respawn), its own grudge clears, so it revives grudgeless — a
///   defeated duel fighter comes back behaving like a normal NPC, exactly as a loser
///   should, rather than resuming the fight the instant it's back on its feet.
///
/// Together these make a duel between two grudge-feuding `Npc`s resolve to mutual
/// peace with no bespoke "end the duel" code. Runs just before
/// [`select_actor_targets`] so a cleared grudge takes effect the same frame. The
/// `&BodyHealth` read overlaps the mutable-aggression query only on the (immutable)
/// health component, so there is no access conflict.
pub fn dissolve_settled_grudges(
    mut actors: Query<(&BodyHealth, &mut ActorAggression)>,
    healths: Query<&BodyHealth>,
) {
    for (self_health, mut aggression) in &mut actors {
        let Some(foe) = aggression.grudge else {
            continue;
        };
        let self_down = self_health.current() == 0;
        // An absent foe entity (despawned) counts as gone, so a grudge never dangles.
        let foe_down = healths.get(foe).map(|h| h.current() == 0).unwrap_or(true);
        if self_down || foe_down {
            aggression.grudge = None;
        }
    }
}

fn distance_squared(a: ae::Vec2, b: ae::Vec2) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[cfg(test)]
mod tests;

/// Bend a commanded firing direction toward the nearest foe it was already
/// pointed at.
///
/// ⭐ THE COMMANDED DIRECTION IS STILL THE DECISION, and `max_angle_rad` is what
/// says so: a target outside that cone is not a target for THIS shot, however
/// close it is. Jon, on the pirate's gun-sword: *"angle the equipped gun and
/// shot so it fires in their direction IF THEY ARE IN THE HALF PLANE the side-b
/// was directed towards."*
///
/// ⛔⛔ THE TIE-BREAK IS THE STABLE `SimId`, NEVER THE `Entity`, and this file
/// already learned that once: `nearest_foe_tie_breaks_on_stable_identity_not_entity_id`
/// exists because bevy_ggrs destroys and recreates rollback entities, so a raw
/// id is not preserved across a rewind and a tie decided by one picks a
/// DIFFERENT target mid-resimulation than the confirmed timeline did. The first
/// version of this function compared `(distance, Entity)`.
///
/// A body with no `SimId` is not snapshot-relevant; those sort last among
/// themselves by `Entity`, which is the same rule `select_actor_targets` uses.
///
/// Returns `commanded` unchanged when nothing qualifies, which is the honest
/// answer: the shot goes where it was aimed.
pub fn assisted_fire_direction(
    from: ae::Vec2,
    commanded: ae::Vec2,
    assist: ambition_characters::brain::action_set::AimAssist,
    candidates: impl IntoIterator<Item = (Entity, Option<SimId>, ae::Vec2)>,
) -> ae::Vec2 {
    let commanded = commanded.normalize_or_zero();
    if commanded == ae::Vec2::ZERO {
        return commanded;
    }
    let cos_limit = assist.max_angle_rad.cos();
    let mut best: Option<(f32, Option<SimId>, Entity, ae::Vec2)> = None;
    for (entity, sim_id, at) in candidates {
        let offset = at - from;
        let distance = offset.length();
        if distance <= f32::EPSILON || distance > assist.max_range {
            continue;
        }
        let toward = offset / distance;
        if toward.dot(commanded) < cos_limit {
            continue;
        }
        let better = match &best {
            None => true,
            Some((best_distance, best_id, best_entity, _)) => {
                match distance.partial_cmp(best_distance) {
                    Some(std::cmp::Ordering::Less) => true,
                    Some(std::cmp::Ordering::Greater) | None => false,
                    // An exact tie: the stable identity decides, and only when
                    // neither body has one does `Entity` get a vote.
                    Some(std::cmp::Ordering::Equal) => match (&sim_id, best_id) {
                        (Some(mine), Some(theirs)) => mine < theirs,
                        (Some(_), None) => true,
                        (None, Some(_)) => false,
                        (None, None) => entity < *best_entity,
                    },
                }
            }
        };
        if better {
            best = Some((distance, sim_id, entity, toward));
        }
    }
    best.map_or(commanded, |(_, _, _, toward)| toward)
}
