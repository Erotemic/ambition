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
/// A token is a catalog id or an authored display name — whatever content wrote.
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

    /// Take pending tokens until one frame's PIXEL ration is spent, leaving the
    /// rest for the next call. A token costs `cost` units -- the drainer's
    /// knowledge, since it depends on the tier the drainer decodes at; the
    /// first token is always taken, so a single Full character still starts
    /// this frame and the ration can never strand a token.
    pub fn take_within_budget(&mut self, budget_units: usize, cost: usize) -> Vec<String> {
        let mut taken = Vec::new();
        let mut spent = 0usize;
        while let Some(token) = self.pending.iter().next().cloned() {
            if !taken.is_empty() && spent + cost > budget_units {
                break;
            }
            self.pending.remove(&token);
            spent += cost;
            taken.push(token);
        }
        taken
    }

    /// Take at most `limit` pending tokens (all of them for `limit == 0`),
    /// leaving the rest pending for the next call.
    /// Drain at most `limit` tokens (`0` means all).
    pub fn take_bounded(&mut self, limit: usize) -> Vec<String> {
        if limit == 0 || self.pending.len() <= limit {
            return self.take();
        }
        let mut taken = Vec::with_capacity(limit);
        for _ in 0..limit {
            let Some(token) = self.pending.iter().next().cloned() else {
                break;
            };
            self.pending.remove(&token);
            taken.push(token);
        }
        taken
    }
}
