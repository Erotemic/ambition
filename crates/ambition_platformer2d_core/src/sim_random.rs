//! RANDOMNESS THAT SURVIVES A REWIND — and it survives by not existing.
//!
//! ⭐⭐ THERE IS NO STREAM, AND THAT IS THE DESIGN. A rollback simulation's
//! problem with a random generator is never the generator: it is that a STREAM
//! is state. Whoever owns it must rewind it, every consumer must draw the same
//! number of samples on a resimulated tick, and two systems sharing it depend on
//! the order the scheduler happened to run them in. Every one of those is a
//! desync waiting for the first person who forgets.
//!
//! So a draw here is a pure function of facts the simulation already rewinds:
//!
//! ```text
//! sim_random(domain, tick, salt)  ->  the same u64, on every peer, forever
//! ```
//!
//! ⇒ nothing to register, nothing to rewind, no consumption to keep in step, and
//! **schedule order cannot matter** — which is the trap `arbitrate_attack_clanks`
//! had to sort a query to avoid, dissolved rather than guarded.
//!
//! ⛔ THE FIGHTER BRAIN KEEPS ITS OWN STREAM, and should. Its noise must not
//! repeat within a tick and it carries per-body state that already rewinds
//! (`FighterState`), so a stream is the right shape THERE. This is for the other
//! kind of question — *"what does the world do this tick"* — where the tick is
//! the whole of the context.
//!
//! ## Domains
//!
//! `domain` separates consumers so two of them drawing on the same tick are not
//! handed correlated answers. ⛔ pick a NEW constant rather than reusing a
//! neighbour's: an item spawner and a stage picker sharing a domain would agree
//! with each other every time they drew together, which is the one failure this
//! is otherwise immune to.

/// What a draw is ABOUT. See the module docs — two consumers must not share one.
///
/// A plain `u64` rather than an enum so a game can mint its own without
/// upstreaming a variant; the constants below are the engine's.
pub type RandomDomain = u64;

/// Match item spawning.
pub const DOMAIN_ITEM_SPAWN: RandomDomain = 0x1734_0000_5A11_0001;

/// One uniformly-distributed `u64`, decided entirely by its inputs.
///
/// `salt` distinguishes draws a single consumer makes on ONE tick — which item
/// and which spawn point, say. ⛔ two draws with the same `(domain, tick, salt)`
/// are the same number, which is a feature (a resimulated tick reproduces) and a
/// footgun (a caller that forgets to vary the salt draws one value twice).
pub fn sim_random(domain: RandomDomain, tick: u64, salt: u64) -> u64 {
    // SplitMix64's finalizer over the three mixed inputs. The same mixing the
    // fighter brain's stream uses, applied to a POSITION instead of to a
    // running seed — which is exactly the difference between a stream and this.
    let mut z = domain
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(tick.wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add(salt.wrapping_mul(0x94D0_49BB_1331_11EB));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A draw in `0..len`, or `None` for an empty range.
pub fn sim_random_index(domain: RandomDomain, tick: u64, salt: u64, len: usize) -> Option<usize> {
    (len > 0).then(|| (sim_random(domain, tick, salt) % len as u64) as usize)
}

/// A weighted draw over `weights`, returning the chosen index.
///
/// `None` when the table is empty or every weight is zero — both mean "nothing
/// to choose", and a caller that treats them differently is inventing a
/// distinction the table does not make.
///
/// ⛔ ZERO-WEIGHT ROWS ARE UNREACHABLE, not rare. An author writing `0` is
/// switching a row off, which is the only reading that lets a rules screen turn
/// one item off without deleting its row.
pub fn sim_random_weighted(
    domain: RandomDomain,
    tick: u64,
    salt: u64,
    weights: &[u32],
) -> Option<usize> {
    let total: u64 = weights.iter().map(|w| u64::from(*w)).sum();
    if total == 0 {
        return None;
    }
    // The draw is over the total, then walked — so the distribution does not
    // depend on how many rows there are, only on their weights.
    let mut pick = sim_random(domain, tick, salt) % total;
    for (index, weight) in weights.iter().enumerate() {
        let weight = u64::from(*weight);
        if pick < weight {
            return Some(index);
        }
        pick -= weight;
    }
    // Unreachable: the walk consumes exactly `total`. Answering with the last
    // non-zero row rather than panicking, because a simulation that stops is
    // worse than one that picks.
    weights.iter().rposition(|w| *w > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ THE ONE PROPERTY THAT MATTERS: the same inputs give the same answer,
    /// and different ones do not.
    ///
    /// ⛔ THE SECOND HALF IS WHY THE DOMAIN EXISTS. Two consumers drawing on the
    /// same tick must not be handed correlated answers — an item spawner and a
    /// stage picker that agreed every time they drew together would look random
    /// in isolation and be obviously wrong in play.
    #[test]
    fn a_draw_is_decided_by_its_inputs_and_nothing_else() {
        let a = sim_random(DOMAIN_ITEM_SPAWN, 900, 0);
        assert_eq!(
            a,
            sim_random(DOMAIN_ITEM_SPAWN, 900, 0),
            "the same draw answered differently, so a resimulated tick does not \
             reproduce and this is a desync generator"
        );
        assert_ne!(
            a,
            sim_random(DOMAIN_ITEM_SPAWN, 901, 0),
            "the tick is inert"
        );
        assert_ne!(
            a,
            sim_random(DOMAIN_ITEM_SPAWN, 900, 1),
            "the salt is inert"
        );
        assert_ne!(
            a,
            sim_random(DOMAIN_ITEM_SPAWN + 1, 900, 0),
            "two DOMAINS drawing on the same tick agree, so every consumer that \
             shares a tick is correlated with every other"
        );
    }

    /// ⭐ A DRAW IS NOT A COUNTER. Successive ticks must not walk a pattern —
    /// the failure a naive `tick % n` has, which looks fine in a test and cycles
    /// visibly in play.
    #[test]
    fn successive_ticks_do_not_walk_a_pattern() {
        let picks: Vec<usize> = (0..64)
            .filter_map(|tick| sim_random_index(DOMAIN_ITEM_SPAWN, tick, 0, 4))
            .collect();
        assert_eq!(picks.len(), 64);
        // Every value reachable, and no value taking the whole run: a cycling
        // counter passes the first and fails nothing else, so both are asserted.
        for value in 0..4 {
            let hits = picks.iter().filter(|p| **p == value).count();
            assert!(
                (4..40).contains(&hits),
                "index {value} came up {hits} times in 64 draws over 4 — the \
                 draw is walking a pattern rather than spreading: {picks:?}"
            );
        }
    }

    /// ⭐⭐ A WEIGHTED TABLE HONOURS ITS WEIGHTS, and a ZERO is OFF.
    ///
    /// ⛔ the zero case is the one a rules screen depends on: turning an item off
    /// must not need its row deleted, so an unreachable row has to be genuinely
    /// unreachable rather than merely unlikely.
    #[test]
    fn a_weighted_draw_respects_its_weights_and_never_picks_a_zero() {
        let weights = [10u32, 0, 1];
        let mut counts = [0usize; 3];
        for tick in 0..2_000 {
            let pick = sim_random_weighted(DOMAIN_ITEM_SPAWN, tick, 0, &weights)
                .expect("a table with weight picks something");
            counts[pick] += 1;
        }
        assert_eq!(
            counts[1], 0,
            "a zero-weight row was picked {} times, so an author cannot switch \
             one off without deleting it",
            counts[1]
        );
        assert!(
            counts[0] > counts[2] * 4,
            "weight 10 came up {} times against weight 1's {} — the table is \
             being read as a uniform choice over its rows",
            counts[0],
            counts[2]
        );
        // …and nothing to choose is `None`, for both of the ways a table can
        // have nothing in it.
        assert_eq!(sim_random_weighted(DOMAIN_ITEM_SPAWN, 0, 0, &[]), None);
        assert_eq!(sim_random_weighted(DOMAIN_ITEM_SPAWN, 0, 0, &[0, 0]), None);
        assert_eq!(sim_random_index(DOMAIN_ITEM_SPAWN, 0, 0, 0), None);
    }
}
