//! Per-frame `ActorTarget` selection for non-player actors.
//!
//! Runs at the top of the actor simulation chain so each enemy /
//! boss / NPC's downstream tick reads "who am I looking at right
//! now" from its `ActorTarget` component rather than from the global
//! primary-player query. Today's policy is "nearest alive player-
//! faction entity"; co-op / split-screen builds can later swap a
//! sticky-target or role-based selector here without touching any
//! actor update signatures (OVERNIGHT-TODO #17.8).

use ambition_engine_core as ae;
use bevy::prelude::*;

use super::components::{
    ActorAggression, ActorFaction, ActorTarget, AggressionTarget, CenteredAabb,
};
use super::FeatureSimEntity;
use ambition_characters::actor::BodyHealth;
use ambition_engine_core::BodyKinematics;
use ambition_platformer_primitives::markers::PlayerEntity;
use ambition_platformer_primitives::sim_id::SimId;

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

/// Effective combat allegiance: a body currently under player control (it carries
/// [`ambition_characters::brain::Brain::Player`]) fights as [`ActorFaction::Player`]
/// regardless of its AUTHORED faction. This is why possession never overwrites
/// `ActorFaction` (no flip, no restore bookkeeping): every combat faction read —
/// targeting, damage gates, hitbox stamps — resolves through this, so a possessed
/// body attacks its former allies and is targeted by them, then reverts the
/// instant control leaves (the authored faction was never touched).
pub fn effective_faction(
    authored: ActorFaction,
    brain: Option<&ambition_characters::brain::Brain>,
) -> ActorFaction {
    if brain.is_some_and(ambition_characters::brain::Brain::is_player) {
        ActorFaction::Player
    } else {
        authored
    }
}

/// Whether an `attacker`-faction body's landed hit DAMAGES a specific `victim`
/// body — the faction baseline ([`can_damage`]) PLUS the per-entity grudge override.
///
/// A grudge is the DAMAGE-side counterpart to a [`FactionRelations`] entry: just as
/// relations make two FACTIONS hostile, a grudge makes one body hostile to one exact
/// ENTITY. So a grudge authorizes a hit even between SAME-faction bodies that
/// `can_damage` would otherwise spare — the mechanism behind two normal NPCs dueling
/// (both `Npc`, each grudging the other) without either being re-tagged a hostile
/// faction. Self-exclusion (`attacker_entity == victim_entity`) stays the caller's.
///
/// `attacker_grudge` is the firing body's [`ActorAggression::grudge`]; `None` (no
/// grudge, or a grudge-less attacker like the environment) falls straight back to the
/// faction rule. This is a strict SUPERSET of `can_damage`, so it never spares a hit
/// the faction baseline would have landed.
pub fn damage_lands(
    attacker: ActorFaction,
    victim: ActorFaction,
    friendly_fire: FriendlyFire,
    attacker_grudge: Option<Entity>,
    victim_entity: Entity,
) -> bool {
    can_damage(attacker, victim, friendly_fire) || attacker_grudge == Some(victim_entity)
}

/// Pick each non-player actor's `ActorTarget` for this frame.
///
/// Selection is driven by each actor's [`ActorAggression`], not by its
/// [`ActorFaction`]: `ActorAggression::target_policy` says whether the
/// actor wants a target and which one. A non-passive actor
/// (`HostileToPlayer` / `RetaliatesWhenHit`) tracks the nearest alive
/// player-faction entity by straight-line distance — the same set of
/// actors the old `faction.needs_target()` shortcut targeted. A passive
/// actor takes no combat target and is pointed at itself so its facing
/// math keeps the current facing instead of snapping toward the origin.
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
    players: Query<(Entity, &BodyKinematics, &BodyHealth, &ActorFaction), With<PlayerEntity>>,
    // Non-player actors are candidate targets too (the relational, non-player-
    // centric part): an actor can target another actor whose faction it's hostile
    // to. Snapshotted, so this read-only borrow ends before the mutable pass.
    // `Option<&Brain>` on both candidate and acting queries: a possessed body
    // (carrying `Brain::Player`) is a Player-EFFECTIVE candidate/actor without
    // its authored `ActorFaction` being mutated — so former allies target it and
    // it targets them, purely through effective allegiance.
    others: Query<
        (
            Entity,
            &CenteredAabb,
            &ActorFaction,
            &BodyHealth,
            Option<&ambition_characters::brain::Brain>,
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
            Option<&ambition_characters::brain::Brain>,
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
    let mut candidates: Vec<(Entity, ae::Vec2, ActorFaction, Option<SimId>)> = players
        .iter()
        .filter(|(_, _, hp, _)| hp.current() > 0)
        .map(|(e, kin, _, faction)| (e, kin.pos, *faction, sim_ids.get(e).ok().cloned()))
        .chain(
            others
                .iter()
                .filter(|(_, _, _, hp, _)| hp.current() > 0)
                .map(|(e, aabb, faction, _, brain)| {
                    (
                        e,
                        aabb.center,
                        effective_faction(*faction, brain),
                        sim_ids.get(e).ok().cloned(),
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
    // Nothing to point at: leave every actor's target untouched so downstream
    // ticks keep last frame's value instead of zeroing (matches old behavior
    // when no candidates existed).
    if candidates.is_empty() {
        return;
    }
    for (self_entity, aabb, mut target, aggression, faction, brain) in actors.iter_mut() {
        let actor_pos = aabb.center;
        // The acting body's OWN effective allegiance (Player while possessed). A
        // body with neither an authored faction nor player control has no
        // faction-relational foes (only a personal grudge can point it) — same as
        // the old `faction.is_some()` gate.
        let player_controlled = brain.is_some_and(ambition_characters::brain::Brain::is_player);
        let has_allegiance = faction.is_some() || player_controlled;
        let self_faction = effective_faction(faction.copied().unwrap_or_default(), brain);
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
        for (entity, pos, cand_faction, _) in &candidates {
            if *entity == self_entity {
                continue;
            }
            let is_foe = (has_allegiance && relations.is_hostile(self_faction, *cand_faction))
                || aggression.grudge == Some(*entity);
            if !is_foe {
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
            // (fable review 2026-07-02 §B12; deep review 2026-07-19 §2.5).
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
