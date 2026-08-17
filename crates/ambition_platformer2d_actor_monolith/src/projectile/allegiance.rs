//! **Which side a shot is on — a fact the SHOT carries.**

use bevy::prelude::Component;

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
/// * **Not the grudge.** A grudge is a live feud the firer holds *now*, not a
///   side the shot was launched on; a body that no longer exists is not feuding
///   with anyone. The stepper still reads it off a living owner.
/// * **Not the owner entity.** [`ProjectileOwner`](ambition_projectiles::ProjectileOwner)
///   already carries "who fired me" and is already remapped across a rewind. This
///   answers the other half — *which side was I on* — which an entity handle to a
///   despawned body cannot.
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
