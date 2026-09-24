//! Leaving what is running: the row an experience contributes to the shell's
//! system menu (e.g. Smash's `Exit Match`), and the request it gets back when
//! the row is picked.
//!
//! The shell hosts the row but does not know what it means. The experience
//! supplies the words ([`ShellAbandonOffer`]) and the meaning (what it does on
//! reading [`ShellAbandonRequested`]). The shell only draws the row and
//! reports the press.
//!
//! This module is not gated on `basic_presentation`, so an experience can
//! state its offer in a composition that draws no menu.

use bevy::prelude::*;

/// A row the active experience contributes to the universal pause menu.
///
/// Present only while the experience has something to leave. The offerer must
/// remove it when that ends.
///
/// The shell does not act on the row: it does not retire the session, change
/// the route, or touch the sim. The experience does the leaving.
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
/// Written only while an offer stands. A reader should still check that the
/// thing to leave is still running.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShellAbandonRequested;
