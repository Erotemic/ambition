//! **Which side a shot is on — a fact the SHOT carries.**

use bevy::prelude::{Commands, Component, Entity, Query, Without};

use ambition_characters::actor::ActorFaction;
use ambition_combat::targeting::MatchTeam;

/// **The firer's side of the fight, frozen onto the bolt.**
///
/// A projectile is an occurrence, not a limb: it leaves the body that made it and
/// keeps happening. So "whose attack is this" cannot be answered by looking the
/// firer up — that answer disappears the moment the firer does, and the shot is
/// still in the air.
///
/// ⛔ **it WAS looked up, and the code stated the consequence as if it were the
/// design**: an owner query that missed meant OWNERLESS, and ownerless meant
/// *indiscriminate* — "there is no one to be friendly to". Correct for an
/// environmental hazard's volley, catastrophic for a fighter's bolt. In a stocks
/// match `take_eliminated_fighters_out_of_play` DESPAWNS a fighter that has spent
/// their last stock (and says why in as many words), so a fighter who fired and
/// then lost the round had their shot turn on their own teammates one tick later.
///
/// ⭐ **the presentation half of the same bolt already knew this.**
/// [`inherit_projectile_presentation_sources`](crate::character_runtime::presentation::inherit_projectile_presentation_sources)
/// spells it out — *"the bolt is the emitter … it routinely outlives the body that
/// fired it. So the source is STAMPED at spawn rather than looked up at impact"* —
/// and the combat half was the one still counting who was left standing. This is
/// the same stamp for the other question.
///
/// # What is here, and what deliberately is not
///
/// * **Faction and team** — the two facts [`damage_lands_between`] asks of the
///   attacker, and the two that ARE the shot's side. Frozen, so a reflect can
///   REWRITE them deliberately (that is what a parry does) rather than having
///   them evaporate.
/// * **Not the grudge — AUDITED 2026-08-18 and kept.** A grudge is a live feud
///   the firer holds *now*, not a side the shot was launched on, and
///   [`dissolve_settled_grudges`](ambition_combat::targeting::dissolve_settled_grudges)
///   already ends it on a health rule that has nothing to do with residency. So
///   the stepper reads it off a living owner on purpose.
///   ⛔ **that was only defensible after the stepper stopped inverting it.**
///   While a missing owner meant INDISCRIMINATE, "the firer is gone, so there is
///   no feud" became "hit everyone, including the bodies the feud existed to
///   spare" — a narrowing turning into the broadest permission there is.
///   ⚠ the decision carries a condition: it holds while a grudge dies with its
///   holder. If one ever outlives a body, or a launch starts meaning *"I aimed
///   this AT you"*, the durable form belongs here — as the target's `SimId`,
///   never an `Entity` (N3.1 forbids entity handles in rollback blobs; see
///   `heal_projectile_owners` for what the healed-handle pattern costs).
/// * **Not the owner entity.** [`ProjectileOwner`](ambition_projectiles::ProjectileOwner)
///   already carries "who fired me" and is already remapped across a rewind. This
///   answers the other half — *which side was I on* — which an entity handle to a
///   despawned body cannot.
///
/// # The materialization boundary closes the stamp window
///
/// The projectile model cannot name combat vocabulary, so the request materializer
/// cannot construct this component itself. Instead the host chains this actor-domain
/// stamp immediately after **each** materializer and before any system can step or
/// settle the new shot. The two placements correspond to the two explicit
/// `ProjectileStart` timings; neither is a second spawn authority.
///
/// An ownerless/environmental request intentionally remains unstamped. A request
/// with a real owner whose body has no `ActorFaction` also remains unstamped, so
/// absence keeps its semantic meaning rather than being guessed from presentation.
///
/// ⚠ the faction is the firer's AUTHORED one, not `effective_faction`, because
/// that is exactly what the owner lookup read. Whether a possessed body's shot
/// should fight for its driver's side is a separate question, and changing it
/// here would hide a rule change inside a lifetime fix.
///
/// [`damage_lands_between`]: ambition_combat::targeting::damage_lands_between
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ProjectileAllegiance {
    /// The firing body's authored faction at the moment the shot took flight.
    pub faction: ActorFaction,
    /// The firing body's match team, when it had one. `None` outside a match —
    /// the faction rule then decides, exactly as it does for any unseated body.
    pub team: Option<MatchTeam>,
}

impl ProjectileAllegiance {
    /// The team, in the borrowed shape `damage_lands_between` takes.
    pub fn team(&self) -> Option<&MatchTeam> {
        self.team.as_ref()
    }
}

/// **Freeze a new shot's side BEFORE anything steps it.**
///
/// The stamp used to be taken lazily, on the projectile's first step inside
/// `step_projectiles`. That left exactly one window in which a bolt exists and
/// its side has not been frozen — and a firer who is eliminated inside that
/// window takes the answer with them, because the stepper's owner query wants a
/// non-optional `&ActorFaction` and there is no longer a body to read. The shot
/// then re-asks and re-fails every tick for the rest of its life.
///
/// It is installed TWICE, once after each materializer:
///
/// ```text
/// materialize_projectiles_for_this_tick    immediate shots materialize
/// stamp_new_projectile_allegiance           ← 1
/// step_projectiles
/// charge_projectile_input
/// materialize_projectiles_for_next_tick    delayed shots materialize
/// stamp_new_projectile_allegiance           ← 2
/// ```
///
/// ⛔⛔ **the second is not redundant, and the reasoning that says it is has a
/// specific hole.** It looks unnecessary because a delayed bolt materializes after
/// the step and so first ticks NEXT frame, by which time placement 1 has run.
/// That is true about STEPPING and false about the WINDOW: the window is bounded
/// by the firer's DESPAWN, not by the bolt's first step.
/// `take_eliminated_fighters_out_of_play` runs in `CombatSet::Settle`, and this
/// whole chain is in `CombatSet::Materialize` — which is EARLIER in the same
/// tick. So a fighter eliminated on the tick they fire loses the body after the
/// bolt exists and before placement 1 ever sees it.
///
/// ⛔ this cannot live in `ambition_projectiles` beside the two materializers,
/// the way the presentation source does: that crate depends on neither
/// `ambition_combat` nor `ambition_characters`, so it cannot name `ActorFaction`
/// or `MatchTeam` to stamp them.
///
/// ⚠ `Without<ProjectileAllegiance>` rather than `Added<ProjectileOwner>`, for
/// the reason `inherit_projectile_presentation_sources` gives: bevy_ggrs
/// destroys and recreates rollback entities, so an `Added` filter fires again on
/// every restored frame while the change-detection tick it reads is not the
/// sim's. Filtering on the component's ABSENCE is idempotent under any number of
/// loads, and it is what makes this safe to leave beside the stepper's own
/// first-sight stamp, which stays as the backstop for any path that mints a
/// projectile outside this chain (the parry re-own writes its own).
///
/// ⚠ a firer with no `ActorFaction` leaves the bolt unstamped, which is correct:
/// an environmental emitter has no side, and the stepper reads an unstamped bolt
/// with no NAMED owner as the indiscriminate volley it is.
pub fn stamp_new_projectile_allegiance(
    mut commands: Commands,
    unstamped: Query<
        (Entity, &ambition_projectiles::ProjectileOwner),
        Without<ProjectileAllegiance>,
    >,
    firers: Query<(&ActorFaction, Option<&MatchTeam>)>,
) {
    for (projectile, owner) in &unstamped {
        if let Ok((faction, team)) = firers.get(owner.0) {
            commands.entity(projectile).insert(ProjectileAllegiance {
                faction: *faction,
                team: team.cloned(),
            });
        }
    }
}
