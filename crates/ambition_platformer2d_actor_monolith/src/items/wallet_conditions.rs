//! Authored WALLET conditions — "can the player pay for this?"
//!
//! ⛔⛔ THIS RETIRES A SECOND AUTHORITY, and the fork had more authored callers
//! than any condition the engine publishes. `can_afford(price)` was a bespoke
//! Yarn function closed over `YarnStateMirrorData::wallet_balance`, a per-frame
//! snapshot — and `kernel.yarn`'s shop menu calls it **ten** times, against
//! `inventory.holds`' seven and `boss.cleared`'s five.
//!
//! ⭐ THE MIGRATION WAS DECLARED FINISHED AND THIS SLIPPED THROUGH A REAL
//! ARGUMENT. `world-facts-observations-and-memory.md` ruled the mirror done by
//! enumerating its STRUCT FIELDS: `visit_counts` is dialogue bookkeeping and
//! `wallet_balance` is a NUMBER the boolean catalog cannot return. Both true.
//! But the fork lives in the FUNCTIONS bound over a field, and one field
//! carries two verbs of different shapes — `wallet_balance()` returns the
//! value and is genuinely exempt; `can_afford(price)` returns a BOOLEAN over
//! the same `i32` and is not. ⇒ Enumerate the authored surface, not the storage.
//!
//! ⚠ SO THE MIRROR'S `wallet_balance` FIELD STAYS, deliberately. An empty
//! mirror is not the goal; one authority per question is, and the numeric verb
//! still needs somewhere to read from.
//!
//! ⭐ A NUMBER PARAMETER, WITH A PRECEDENT: `body.fits 32` already takes a
//! `ParamKind::Number`, so no vocabulary is invented here — which was the
//! ruling's stated reason for believing the wallet inexpressible.

use ambition_characters::actor::BodyWallet;
use ambition_platformer2d_shared_tangle::authored_logic::{
    AuthoredArg, ConditionDescriptor, ConditionId, ConditionOutcome, ParamKind, ParamSpec, WhyNot,
};
use ambition_platformer2d_shared_tangle::markers::PrimaryPlayer;
use bevy::prelude::{With, World};

/// The domain segment every condition in this file is published under.
pub const DOMAIN: &str = "wallet";

const PRICE: ParamSpec = ParamSpec {
    name: "price",
    kind: ParamKind::Number,
    summary: "the cost in coins, as the authored shop line spells it",
};

/// `wallet.can_afford(price)` — does the player hold at least this much?
pub fn can_afford_descriptor() -> ConditionDescriptor {
    ConditionDescriptor {
        id: ConditionId::new(DOMAIN, "can_afford"),
        summary: "true while the player's wallet holds at least the named price",
        params: &[PRICE],
    }
}

/// `wallet.can_afford` — see [`can_afford_descriptor`].
///
/// ⭐ AT LEAST, NOT MORE THAN: a price the player can exactly meet is
/// affordable. The shop's own `buy` spends `balance -= price` and refuses only
/// when short, so `>=` is what the simulation already does; `>` here would let
/// a menu line grey out an item the purchase would have allowed.
///
/// ⚠ NO WALLET IS `Unanswerable`, NOT `false` — the same rule the save-layer
/// arms use. A composition with no wallet has not recorded that the player is
/// broke; it has no notion of money at all, and answering "cannot afford"
/// would be a confident claim about a world with no currency.
pub fn can_afford(world: &World, args: &[AuthoredArg]) -> ConditionOutcome {
    let Some(price) = args[0].as_number() else {
        return ConditionOutcome::unanswerable("`price` must be a number");
    };
    // ⛔ A NEGATIVE PRICE IS AN AUTHORING ERROR, not a free item. Reported
    // rather than silently satisfied, matching `body.fits`' rejection of a
    // non-positive opening: a shop line reading `can_afford(-5)` is a typo, and
    // answering `true` would hide it behind a door that always opens.
    if price < 0.0 {
        return ConditionOutcome::unanswerable(format!(
            "`{price}` is not a price; `wallet.can_afford` takes a non-negative cost in coins"
        ));
    }
    let Some(mut wallets) = world.try_query_filtered::<&BodyWallet, With<PrimaryPlayer>>() else {
        return ConditionOutcome::unanswerable(
            "no wallet is installed in this composition, so nothing has a price",
        );
    };
    let Some(balance) = wallets.iter(world).next().map(|wallet| wallet.balance) else {
        return ConditionOutcome::unanswerable(
            "no primary player carries a wallet, so there is nobody to charge",
        );
    };
    ConditionOutcome::from_bool(f64::from(balance) >= price, || {
        WhyNot::new(
            "wallet.can_afford",
            format!("{price}"),
            format!("the player's wallet holds {balance}"),
        )
    })
}

/// Publishes the wallet domain's conditions.
///
/// One plugin for one registration line, matching the inventory's and the
/// body's: composition adds it, and nothing else in the engine learns that the
/// purse can be asked about.
pub struct WalletConditionsPlugin;

impl bevy::prelude::Plugin for WalletConditionsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use ambition_platformer2d_shared_tangle::authored_logic::PublishCondition;
        app.publish_condition(can_afford_descriptor(), can_afford);
    }
}

#[cfg(test)]
mod tests;
