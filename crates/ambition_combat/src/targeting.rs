//! Per-frame `ActorTarget` selection for non-player actors.
//!
//! Runs at the top of the actor simulation chain so each enemy /
//! boss / NPC's downstream tick reads "who am I looking at right
//! now" from its `ActorTarget` component rather than from the global
//! primary-player query. Today's policy is "nearest alive player-
//! faction entity"; co-op / split-screen builds can later swap a
//! sticky-target or role-based selector here without touching any
//! actor update signatures (OVERNIGHT-TODO #17.8).

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use super::components::{
    ActorAggression, ActorFaction, ActorTarget, AggressionTarget, CenteredAabb,
};
use super::FeatureSimEntity;
use ambition_characters::actor::BodyHealth;
use ambition_platformer2d_core::BodyKinematics;
use ambition_platformer2d_shared_tangle::markers::PlayerEntity;
use ambition_platformer2d_shared_tangle::sim_id::SimId;

/// Number of [`ActorFaction`] variants (Player / Enemy / Npc / Boss / Neutral).
/// The relations matrix is indexed by `faction as usize`.
const FACTION_COUNT: usize = 5;

/// Who-fights-whom, as DATA rather than hard-coded actor types — the relational
/// targeting seam. `hostile[from][to] == true` means a `from`-faction actor
/// treats `to`-faction actors as a combat target this frame.
///
/// This is the seam future stealth / bounty / grudge / alliance systems write
/// to: revealing yourself flips the player's row, a bounty makes a faction
/// hostile to the player, an alliance clears two factions' mutual hostility — all
/// without touching the brains or the actor spawn path.
///
/// The default encodes the **combat baseline**: Player ↔ Enemy and Player ↔ Boss
/// are mutually hostile (the player and the hostile world fight), and nothing else
/// is — Npc / Neutral are peaceful, and same-faction actors don't fight. This is
/// the single source of truth the damage paths consult (melee + projectile),
/// so it reproduces today's player-vs-enemy combat with no behavior change while
/// making actor-vs-actor hostility expressible (a room sets, e.g.,
/// `set_mutual_hostile(Enemy, Boss, true)` for a spectator arena, and may *clear*
/// `Enemy → Player` so the combatants ignore the observing player).
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

/// Friendly-fire policy — the DAMAGE-side counterpart to [`FactionRelations`]
/// (which is the TARGETING side). Targeting decides whom a brain *aims at*;
/// this decides whether a hit that *lands* deals damage.
///
/// Damage is physical: a hit damages any body it overlaps that is NOT the
/// attacker (self is excluded at every call site by entity). The one default
/// exclusion is **same-faction allies** — friendly fire is OFF by default, so a
/// pirate's stray shot can't hurt another pirate. A different-faction bystander
/// (e.g. the player observing a duel) IS hit by strays; that's deliberate.
/// Set `enabled = true` to opt INTO friendly fire (free-for-all): same-faction
/// bodies then damage each other too. Per-entity grudges/charm overrides would
/// layer on top of this faction baseline later.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default)]
pub struct FriendlyFire {
    pub enabled: bool,
}

/// Register the relational-targeting resources combat OWNS (rule 5): the
/// default `FactionRelations` matrix + the `FriendlyFire` toggle. The
/// WorldPrep schedule calls this instead of init-ing combat's resources from
/// another module, so ownership travels with the types into `ambition_combat`.
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

/// Effective combat allegiance: a body a participant is currently driving (it
/// carries [`ambition_characters::brain::DrivingParticipant`]) fights as
/// [`ActorFaction::Player`] regardless of its AUTHORED faction. This is why
/// possession never overwrites `ActorFaction` (no flip, no restore bookkeeping):
/// every combat faction read — targeting, damage gates, hitbox stamps — resolves
/// through this, so a possessed body attacks its former allies and is targeted by
/// them, then reverts the instant control leaves (the authored faction was never
/// touched).
pub fn effective_faction(
    authored: ActorFaction,
    driver: Option<&ambition_characters::brain::DrivingParticipant>,
) -> ActorFaction {
    if driver.is_some() {
        ActorFaction::Player
    } else {
        authored
    }
}

/// A grudge is the DAMAGE-side counterpart to a [`FactionRelations`] entry: just as
/// relations make two FACTIONS hostile, a grudge makes one body hostile to one exact
/// ENTITY. So a grudge authorizes a hit even between SAME-faction bodies that
/// `can_damage` would otherwise spare — the mechanism behind two normal NPCs dueling
/// (both `Npc`, each grudging the other) without either being re-tagged a hostile
/// faction. Self-exclusion (`attacker_entity == victim_entity`) stays the caller's.
///
/// `attacker_grudge` is the firing body's [`ActorAggression::grudge`]; `None` (no grudge, or a
/// grudge-less attacker like the environment) falls straight back to the faction rule.
///
/// A team is a relation a RULESET declares, and it outranks faction for the one
/// question that matters here: may this hit land? Two bodies on different teams
/// damage each other; two on the same team do not.
///
/// It exists because faction cannot express it. `effective_faction` maps ANY
/// player-brained body to `ActorFaction::Player` — load-bearing for possession,
/// since a possessed enemy must stop being hittable by the player possessing it
/// — so two humans are always the same faction no matter what the roster says.
/// A versus stage had to switch on GLOBAL friendly fire to get around that,
/// which is right for a free-for-all and wrong the moment a 2v2 exists: it makes
/// teammates hittable too.
///
/// Only bodies that HAVE a team are judged by it. A body with no team is
/// unchanged in every respect, so nothing outside a match notices this exists.
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

/// **How two bodies stand to each other. ONE answer, for every consumer.**
///
/// **there were two, and they disagreed.** "May this damage land" read faction difference plus
/// a team override; "is this a target worth chasing" read the `FactionRelations` hostility
/// matrix and had never heard of a team. That is the hack this type exists to delete.
///
/// The three answers are the ones a combat rule actually needs:
///
/// * [`Self::Foe`] — go after it, and hit it. A different team, a hostile
///   faction relation, or a personal grudge.
/// * [`Self::Ally`] — same team, or same faction. Spared unless friendly fire.
/// * [`Self::Neutral`] — a different faction this one is not hostile TO. **Not
///   a target, but not protected either**, which is the distinction a single
///   boolean could never carry: damage is physical (a swing that reaches a
///   bystander hurts it) while targeting is relational (nobody goes hunting a
///   bystander). Collapsing the two is how a stray hit stopped landing, or a
///   town NPC became prey.
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

/// **THE combat-relationship policy.** Both "may I damage this" and "should I
/// chase this" resolve through here, so they cannot drift again.
///
/// Precedence, highest first:
///
/// 1. **A grudge** — a per-entity feud, deliberately stronger than any group
///    rule, so two teammates can settle something the ruleset did not
///    anticipate.
/// 2. **A team** — when BOTH bodies are in a match. Match rules outrank
///    authored world allegiance, which is the whole point of a crossover stage:
///    a Hall NPC and a boss can be seated as opponents without either
///    character's authored row being edited.
/// 3. **Authored faction**, through the [`FactionRelations`] matrix and then
///    plain sameness.
///
/// Allegiance is EFFECTIVE on both sides: a possessed body fights as its
/// driver's side without its authored faction being mutated.
#[allow(clippy::too_many_arguments)]
pub fn combat_relation(
    // `None` = **no matrix opinion**, which is what the DAMAGE side passes. Damage is physical:
    // a swing that reaches a different-faction body hurts it whether or not the two are
    // declared enemies, so the matrix arm is skipped and such a pair resolves
    // [`CombatRelation::Neutral`] — hittable, not huntable.
    relations: Option<&FactionRelations>,
    attacker_faction: ActorFaction,
    attacker_driver: Option<&ambition_characters::brain::DrivingParticipant>,
    attacker_team: Option<&MatchTeam>,
    attacker_grudge: Option<Entity>,
    candidate: Entity,
    candidate_faction: ActorFaction,
    candidate_driver: Option<&ambition_characters::brain::DrivingParticipant>,
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
    // **the damage side of [`combat_relation`], and it must not grow a second
    // opinion.** Callers pass ALREADY-EFFECTIVE factions here (the policy
    // re-resolves them, which is idempotent), so this stays a projection.
    //
    // **one behaviour deliberately changed**: friendly fire now also frees
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

/// Pick each non-player actor's `ActorTarget` for this frame.
///
/// When no player entities exist (pre-spawn, post-death-of-all-players,
/// headless probe with no player) every actor's `ActorTarget` is left
/// untouched so downstream ticks see the previous frame's target rather
/// than zeroing out.
///
/// Today's production game has exactly one player so this loop is
/// O(n) over actors. A many-player build can swap in a spatial
/// index here without changing the consumer side.
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
            Option<&ambition_characters::brain::DrivingParticipant>,
            // A match seat, when this body is in one. Selection has to see it or
            // it answers a different question from the damage side — see
            // [`combat_relation`].
            Option<&MatchTeam>,
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
            Option<&ambition_characters::brain::DrivingParticipant>,
            Option<&MatchTeam>,
        ),
        With<FeatureSimEntity>,
    >,
    // Stable semantic identity, used ONLY to put the candidate list in a
    // canonical order — never to decide who is a foe. See the sort below.
    sim_ids: Query<&SimId>,
) {
    let relations = relations.map(|r| r.clone()).unwrap_or_default();
    // ALIVE candidates only: a dead body (health drained to 0) is never a valid
    // target. So the instant a foe dies the actor goes target-less — it stops
    // swinging at the corpse and (downstream) stands down — instead of chasing a
    // dead entity until it despawns. Death zeroes `BodyHealth` on every body
    // (player + actor), so this is the one uniform liveness gate.
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
        .filter(|(_, _, hp, _, _)| hp.current() > 0)
        .map(|(e, kin, _, faction, team)| {
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
                .filter(|(_, _, _, hp, _, _)| hp.current() > 0)
                .map(|(e, aabb, faction, _, driver, team)| {
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
    if candidates.is_empty() {
        return;
    }
    for (self_entity, aabb, mut target, aggression, faction, driver, self_team) in actors.iter_mut()
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
        if policy == AggressionTarget::None {
            // Passive: no combat target. Point at self so a zero direction keeps
            // the actor's current facing.
            target.pos = actor_pos;
            target.entity = None;
            continue;
        }
        // One relational rule: a candidate is a FOE iff this actor's faction is
        // hostile to it (`FactionRelations`) OR this actor holds a grudge against
        // that exact entity (a provoked NPC chasing its attacker). The player is a
        // candidate like any other — it's hunted because the actor's faction opposes
        // Player (a born Enemy) or it's the grudge target (a provoked NPC), never
        // because it is "the player". Nearest foe wins.
        let mut best: Option<(Entity, ae::Vec2, f32)> = None;
        for (entity, pos, cand_faction, _, cand_team) in &candidates {
            if *entity == self_entity {
                continue;
            }
            // **THE one relationship policy**, the same call the damage side makes.
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
            // flip the target of a symmetric two-foe setup mid-resimulation
            // .
            let better = match best {
                None => true,
                Some((_, _, best_d)) => d < best_d,
            };
            if better {
                best = Some((*entity, *pos, d));
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
    }
}

/// Dissolve grudges that have SETTLED, so a feud resolves to peace on its own.
///
/// A grudge is a per-entity hostility (the duel mechanism); like any feud it should
/// END once it's decided. Two rules, both keyed off the one uniform liveness
/// authority ([`BodyHealth`]):
///
/// - **You forget a slain foe.** When a body's grudge target is no longer alive, the
///   grudge clears. The targeting filter already drops a dead foe so the holder stands
///   down ([`select_actor_targets`]); clearing the grudge too means it won't re-aggro
///   if that foe later revives — the duel survivor settles into a normal NPC for good.
/// - **A defeated body forgets its feud.** When a body itself is down (health 0,
///   awaiting respawn), its own grudge clears, so it **revives grudgeless** — a
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
