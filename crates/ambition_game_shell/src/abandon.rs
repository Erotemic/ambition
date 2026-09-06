//! LEAVING WHAT IS RUNNING — the row an experience contributes to the shell's
//! system menu, and the request it gets back when somebody picks it.
//!
//! ⭐⭐ THE SHELL HOSTS THE ROW AND DOES NOT KNOW WHAT IT MEANS. Jon, W8
//! playtest: *"During an active Smash match, the system/pause menu needs an
//! explicit `Exit Match`, which ends the match as No Contest."* A match is a
//! combat concept and this crate does not depend on combat — nor should it, or
//! every future experience with a mode to leave would add an arm to the menu.
//!
//! So the experience states its own WORDS ([`ShellAbandonOffer`]) and its own
//! MEANING (whatever it does on reading [`ShellAbandonRequested`]); the shell
//! only draws the row and reports the press.
//!
//! ⛔⛔ AND THIS MODULE IS NOT GATED ON `basic_presentation`, deliberately. The
//! menu that draws the row is; the CONTRACT is not, because an experience must
//! be able to state its offer in a composition that draws no menu at all —
//! otherwise every offering game needs the feature flag in its own manifest, and
//! the demo crates that compose the platformer without shell presentation would
//! not build.

use bevy::prelude::*;

/// A ROW THE ACTIVE EXPERIENCE CONTRIBUTES to the universal pause menu.
///
/// Present only while the experience has something to leave. ⛔ the offerer
/// RETRACTS it: a stale offer left behind by a finished match puts an `Exit
/// Match` row on the next screen's menu, pointing at nothing.
///
/// ⛔ the shell does NOT act on the row: it does not retire the session, change
/// the route, or touch the sim. Leaving is the experience's to do, because only
/// the experience knows what leaving costs — which is what makes this different
/// from `Quit to Title` sitting three rows below it.
#[derive(Resource, Clone, Debug, Default)]
pub struct ShellAbandonOffer {
    /// What the row says. The experience's own words — "Exit Match".
    pub label: String,
    /// The line under it. "End this match as a No Contest."
    pub detail: String,
}

/// The abandon row was picked. The offering experience reads this and does
/// whatever leaving means for it.
///
/// ⚠ WRITTEN ONLY WHEN AN OFFER STANDS, so it cannot arrive out of nowhere — but
/// a reader should still check that what it is about is still running, because a
/// message and a frame are not the same thing.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShellAbandonRequested;
