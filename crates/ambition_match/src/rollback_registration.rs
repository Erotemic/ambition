//! Rollback declarations for match identity, outcomes, and elapsed time.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;

const OWNER: &str = env!("CARGO_PKG_NAME");

pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    // ⛔ THE RECEIPT IS SNAPSHOTTED WHOLE AND CHECKSUMMED IN PART. TWO of its
    // four fields count something local: `session` counts this App's
    // activations, and `activated_on` its sim steps (menus included — see
    // `MatchInstance::activation_tick`). Both must survive a rewind and neither
    // is a fact two peers can agree on, so only the agreed seat count and the
    // peer match ordinal are compared. `ActiveMatch::peer_stable_checksum`
    // decides which half is which.
    registrar.rollback_resource_optional_canonical_checksum::<crate::ActiveMatch>(
        OWNER,
        "resource.active_match",
        "bevy_ggrs canonical codec snapshot + checksum over the agreed seat count and the peer match \
         ordinal, excluding the host-local session and activation-tick stamps",
        crate::ActiveMatch::peer_stable_checksum,
    );
    // Which match each stamped object belongs to, so the sweep never sees a
    // restored object without an identity. Clone-snapshotted: it is a stable
    // identity copied at spawn.
    registrar.rollback_component_clone_probed::<crate::MatchScoped>(
        OWNER,
        "match.scoped",
        crate::MatchScoped::localizer_probe,
    );
    // ⭐ THE ORDINAL MINT. It rewinds for the same reason any counter does: a
    // resimulated activation must draw the ordinal it drew the first time, or
    // the match re-rolls its item table on every rollback.
    //
    // ⛔⛤ THE COMMENT HERE USED TO SAY the `session` half "is compared only
    // against ITSELF" — over a `rollback_resource_canonical` registration, which
    // compares EVERY field. The sentence described `take`; the registrar decided
    // the checksum. A per-App session count was in the peer comparison.
    // `SessionMatchOrdinal::peer_stable_checksum` now owns the split and states
    // the one window that survives it.
    registrar.rollback_resource_canonical_checksum::<crate::seating::SessionMatchOrdinal>(
        OWNER,
        "resource.session_match_ordinal",
        "bevy_ggrs canonical codec snapshot + checksum over the count of matches this session has \
         activated only, excluding the host-local session stamp",
        crate::seating::SessionMatchOrdinal::peer_stable_checksum,
    );
    // The stocks ruleset's verdict is *the outcome for match X*, stamped with the
    // `MatchInstance` the receipt above publishes — so a rewind that restores one and not the
    // other would restore a verdict about a match that is not running. Registered together,
    // they rewind together.
    registrar.rollback_resource_canonical_checksum::<crate::StocksMatchSettled>(
        OWNER,
        "resource.stocks_match_settled",
        "bevy_ggrs canonical codec snapshot + checksum over the verdict and the peer match \
         ordinal, excluding the host-local session and activation-tick stamps",
        crate::StocksMatchSettled::peer_stable_checksum,
    );
    // …and whether it refused to be settled. Sudden death is entered by NOT
    // deciding, so this latch is the only thing standing between a level timeout
    // and re-entering the tie on every tick that follows.
    registrar.rollback_resource_canonical_checksum::<crate::SuddenDeathEntered>(
        OWNER,
        "resource.sudden_death_entered",
        "bevy_ggrs canonical codec snapshot + checksum over whether a match is latched and the \
         peer match ordinal, excluding the host-local session and activation-tick stamps",
        crate::SuddenDeathEntered::peer_stable_checksum,
    );
    // …and HOW LONG it has been fought. Counted, not derived: the timeout and
    // the item cadence both read it, and "how many ticks was this paused" is
    // written nowhere a rewind could recompute it from.
    registrar
        .rollback_resource_canonical_checksum::<crate::live_match_clock::LiveMatchTicks>(
            OWNER,
            "resource.live_match_ticks",
            "bevy_ggrs canonical codec snapshot + checksum over the elapsed micros, counted from \
             the match's own start, and the peer match ordinal, excluding the host-local session \
             and activation-tick stamps",
            crate::live_match_clock::LiveMatchTicks::peer_stable_checksum,
        );
    registrar.rollback_component_canonical::<crate::MatchSeat>(OWNER, "actor.match_seat");
}
