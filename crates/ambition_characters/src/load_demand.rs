//! What characters a composition has asked to have realized.
//!
//! Room staging, match rosters, direct startup and worn-identity changes all
//! project into one [`CharacterLoadDemand`]; whoever materializes art drains
//! it. The demand knows tokens and nothing about tiers, sheets or budgets --
//! those are the drainer's.

use std::collections::BTreeSet;

use bevy::prelude::Resource;

/// Character tokens a session has staged and therefore needs art for.
///
/// A token is a character id. A placement's name or display name is not one.
/// Requests accumulate until the materializer drains them, so a submitter never
/// has to know whether the decode already happened.
#[derive(Resource, Default, Debug, Clone)]
pub struct CharacterLoadDemand {
    /// Tokens demanded and not yet taken. Every token is realized at the
    /// user's tier: nothing on a demand may ask for fewer pixels than the
    /// setting (Jon, 2026-09-02 — the room tier cap that carried a per-token
    /// floor here is gone, mechanism and all).
    pending: BTreeSet<String>,
}

impl CharacterLoadDemand {
    /// Ask for one character's art. Idempotent, and cheap enough to call every
    /// time a body's identity changes.
    pub fn request(&mut self, token: impl Into<String>) {
        let token = token.into();
        if !token.trim().is_empty() {
            self.pending.insert(token);
        }
    }

    /// Ask for many.
    pub fn request_all<I, S>(&mut self, tokens: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for token in tokens {
            self.request(token);
        }
    }

    /// Tokens demanded and not yet taken by the materializer.
    pub fn pending(&self) -> impl Iterator<Item = &str> {
        self.pending.iter().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// Drain everything. The materializer's road; a submitter never takes.
    pub fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending).into_iter().collect()
    }
}
