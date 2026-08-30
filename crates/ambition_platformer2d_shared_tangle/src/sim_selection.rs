//! One rule for deciding a gameplay contest, so a rewind decides it the same way.
//!
//! ⛔⛔ BEVY QUERY ORDER IS NOT A GAMEPLAY RULE. It is archetype order, which
//! depends on the order entities were spawned into archetypes — a thing that
//! differs between a live tick and the resimulated one that replaces it, and
//! between two peers that reached the same state by different routes. A system
//! that resolves "who gets this" with `.iter().find(..)` or an unsorted `for`
//! has written down no rule at all, and the answer it gets is not wrong so much
//! as *unrepeatable*.
//!
//! ⭐ THE RULE IS ALWAYS THE SAME SHAPE: a gameplay metric first, then stable
//! [`SimId`]. The metric is what the designer means ("the nearest body gets the
//! ring"); the id is what makes the answer the same on every machine and every
//! replay of the same tick. Each of these had been written locally, differently,
//! or not at all — pickup collection and world-item assignment did not attempt
//! it, and the nearest-target searches that did had no final tie-break.
//!
//! ⚠ AN UNIDENTIFIED CANDIDATE CANNOT WIN A TIE, and this module will not
//! pretend otherwise. A candidate with no `SimId` has no stable identity to
//! break a tie WITH, so it loses to any identified candidate at an equal metric,
//! and a tie between two unidentified candidates falls back to encounter order —
//! which is exactly the non-determinism this module exists to remove. That is
//! reported by [`every_candidate_is_identified`] rather than hidden, so a caller
//! can assert it in a test instead of discovering it in a desync.

use crate::sim_id::SimId;

/// The deterministic winner of a contest: smallest `metric`, ties broken by
/// stable [`SimId`].
///
/// `identity` returns `None` for a candidate that carries no `SimId`; see the
/// module docs for why such a candidate loses ties.
///
/// Returns `None` only for an empty candidate set.
pub fn winner_by<T, I, M, D>(candidates: I, metric: M, identity: D) -> Option<T>
where
    I: IntoIterator<Item = T>,
    M: Fn(&T) -> f32,
    D: for<'a> Fn(&'a T) -> Option<&'a SimId>,
{
    let mut best: Option<T> = None;
    for candidate in candidates {
        let take = match &best {
            None => true,
            Some(current) => beats(&candidate, current, &metric, &identity),
        };
        if take {
            best = Some(candidate);
        }
    }
    best
}

/// `candidates`, in the order a rewind will also produce: ascending `metric`,
/// ties by stable [`SimId`].
///
/// ⭐ FOR THE OUTER LOOP, not just the inner pick. `collect_world_items` had the
/// same defect twice over: query order decided WHICH item a body received as
/// well as which body received it, because a system that collects at most one
/// item per body per frame lets the iteration order pick the survivor.
pub fn in_deterministic_order<T, I, M, D>(candidates: I, metric: M, identity: D) -> Vec<T>
where
    I: IntoIterator<Item = T>,
    M: Fn(&T) -> f32,
    D: for<'a> Fn(&'a T) -> Option<&'a SimId>,
{
    let mut ordered: Vec<T> = candidates.into_iter().collect();
    ordered.sort_by(|a, b| {
        metric(a)
            .total_cmp(&metric(b))
            .then_with(|| identity_order(identity(a), identity(b)))
    });
    ordered
}

/// Whether every candidate carries the identity the tie-break needs.
///
/// ⭐ A CALLER SHOULD ASSERT THIS IN A TEST rather than trust it. Sorting an
/// unidentified population produces a stable-LOOKING result that is still
/// encounter-ordered underneath, and a guard that cannot tell those apart is a
/// guard that cannot fail.
pub fn every_candidate_is_identified<T, I, D>(candidates: I, identity: D) -> bool
where
    I: IntoIterator<Item = T>,
    D: for<'a> Fn(&'a T) -> Option<&'a SimId>,
{
    candidates.into_iter().all(|c| identity(&c).is_some())
}

/// Whether the candidates' identities are all DISTINCT as well as present.
///
/// ⛔⛔ THE WEAKER CHECK IS NOT ENOUGH, and this is the second half of the same
/// hole. [`every_candidate_is_identified`] answers "does everyone have a name";
/// two candidates sharing one name still tie, and a tie still falls back to
/// encounter order. An id derived from a fact that is not unique per entity —
/// fixture geometry, a constant, a truncated key — passes the first check and
/// fails to order anything.
///
/// A caller that sorts a population by identity alone (a constant metric) owes
/// BOTH assertions.
pub fn no_two_candidates_share_an_identity<T, I, D>(candidates: I, identity: D) -> bool
where
    I: IntoIterator<Item = T>,
    D: for<'a> Fn(&'a T) -> Option<&'a SimId>,
{
    let mut seen: std::collections::BTreeSet<SimId> = std::collections::BTreeSet::new();
    for candidate in candidates {
        match identity(&candidate) {
            // An unidentified candidate is the OTHER check's business; it is not
            // a duplicate of anything, so this one does not fail on it.
            None => continue,
            Some(id) => {
                if !seen.insert(id.clone()) {
                    return false;
                }
            }
        }
    }
    true
}

fn beats<T, M, D>(candidate: &T, current: &T, metric: &M, identity: &D) -> bool
where
    M: Fn(&T) -> f32,
    D: for<'a> Fn(&'a T) -> Option<&'a SimId>,
{
    match metric(candidate).total_cmp(&metric(current)) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => identity_order(identity(candidate), identity(current)).is_lt(),
    }
}

/// `Some` before `None`, then by id. An unidentified candidate has nothing to
/// order BY, so it sorts after every identified one and ties with its own kind.
fn identity_order(a: Option<&SimId>, b: Option<&SimId>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Candidate {
        distance: f32,
        id: Option<SimId>,
    }

    fn c(distance: f32, id: &str) -> Candidate {
        Candidate {
            distance,
            id: Some(SimId::placement(id)),
        }
    }

    fn anonymous(distance: f32) -> Candidate {
        Candidate { distance, id: None }
    }

    fn winner(candidates: Vec<Candidate>) -> Option<Candidate> {
        winner_by(candidates, |c| c.distance, |c| c.id.as_ref())
    }

    #[test]
    fn the_metric_decides_when_it_can() {
        let won = winner(vec![c(9.0, "far"), c(1.0, "near"), c(4.0, "middle")]);
        assert_eq!(won.unwrap().id, Some(SimId::placement("near")));
    }

    /// ⭐ THE PROPERTY THE WHOLE MODULE IS FOR. Bevy query order is archetype
    /// order; a resimulated tick can present the same population in a different
    /// one. The winner may not depend on which order it arrived in.
    #[test]
    fn the_winner_does_not_depend_on_the_order_the_candidates_arrived_in() {
        let mut population = vec![c(3.0, "b"), c(3.0, "a"), c(3.0, "c")];
        let forward = winner(population.clone());
        population.reverse();
        let backward = winner(population.clone());
        population.swap(0, 2);
        let shuffled = winner(population);
        assert_eq!(forward, backward);
        assert_eq!(forward, shuffled);
        assert_eq!(forward.unwrap().id, Some(SimId::placement("a")));
    }

    #[test]
    fn an_identified_candidate_beats_an_unidentified_one_at_the_same_metric() {
        assert_eq!(
            winner(vec![anonymous(3.0), c(3.0, "identified")])
                .unwrap()
                .id,
            Some(SimId::placement("identified"))
        );
        // And the reverse arrival order agrees, which is the point.
        assert_eq!(
            winner(vec![c(3.0, "identified"), anonymous(3.0)])
                .unwrap()
                .id,
            Some(SimId::placement("identified"))
        );
    }

    #[test]
    fn a_closer_unidentified_candidate_still_wins_on_the_metric() {
        assert_eq!(
            winner(vec![c(3.0, "far"), anonymous(1.0)]).unwrap().id,
            None
        );
    }

    #[test]
    fn an_empty_contest_has_no_winner() {
        assert_eq!(winner(vec![]), None);
    }

    #[test]
    fn the_order_is_stable_under_a_reversed_input() {
        let population = vec![c(2.0, "b"), c(1.0, "z"), c(2.0, "a")];
        let ids =
            |v: Vec<Candidate>| -> Vec<Option<SimId>> { v.into_iter().map(|c| c.id).collect() };
        let forward = in_deterministic_order(population.clone(), |c| c.distance, |c| c.id.as_ref());
        let mut reversed = population;
        reversed.reverse();
        let backward = in_deterministic_order(reversed, |c| c.distance, |c| c.id.as_ref());
        assert_eq!(ids(forward.clone()), ids(backward));
        assert_eq!(
            ids(forward),
            vec![
                Some(SimId::placement("z")),
                Some(SimId::placement("a")),
                Some(SimId::placement("b")),
            ]
        );
    }

    /// ⚠ The honest half: sorting an unidentified population LOOKS stable and is
    /// not, so callers get a way to ask.
    #[test]
    fn an_unidentified_population_is_reported_rather_than_silently_ordered() {
        assert!(every_candidate_is_identified(
            vec![c(1.0, "a"), c(2.0, "b")],
            |c: &Candidate| c.id.as_ref()
        ));
        assert!(!every_candidate_is_identified(
            vec![c(1.0, "a"), anonymous(2.0)],
            |c: &Candidate| c.id.as_ref()
        ));
    }

    /// ⛔⛔ THE WEAKER CHECK PASSES A POPULATION THAT ORDERS NOTHING. Two
    /// candidates sharing one id are both "identified" and still tie, and a tie
    /// against a constant metric is encounter order.
    #[test]
    fn identified_is_not_the_same_as_distinctly_identified() {
        let duplicates = vec![c(0.0, "same"), c(0.0, "same")];
        assert!(
            every_candidate_is_identified(duplicates.clone(), |c| c.id.as_ref()),
            "both carry a name"
        );
        assert!(
            !no_two_candidates_share_an_identity(duplicates, |c| c.id.as_ref()),
            "and the name is the same one, so it orders nothing"
        );
        assert!(no_two_candidates_share_an_identity(
            vec![c(0.0, "a"), c(0.0, "b")],
            |c| c.id.as_ref()
        ));
    }

    /// An unidentified candidate is the OTHER check's business: it is not a
    /// duplicate of anything, so distinctness does not fail on it.
    #[test]
    fn distinctness_does_not_double_report_a_missing_identity() {
        assert!(no_two_candidates_share_an_identity(
            vec![anonymous(0.0), anonymous(1.0)],
            |c| c.id.as_ref()
        ));
    }
}
