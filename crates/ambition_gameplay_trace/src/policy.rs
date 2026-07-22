//! Whether a trace dump is allowed to reach disk.
//!
//! The recorder has always been careful about WHEN it arms an automatic dump —
//! it disarms after the first one, suppresses a window around teleports, and
//! waits for enough lead-up frames to be worth reading. What it never had was a
//! way to say "not at all". On a machine that plays the game regularly that
//! adds up: every confirmed out-of-bounds writes a JSON blob plus a markdown
//! report into `debug_traces/`, nothing prunes them, and the directory grows
//! without anyone asking it to.
//!
//! So automatic dumps are now **opt-in**. A developer chasing an OOB turns them
//! on for that session; everyone else gets a recorder that still records — the
//! ring buffer, the events, the OOB detection, and the on-screen status are all
//! unchanged — and simply stops writing files nobody asked for.
//!
//! **Manual dumps are never gated.** Pressing F8 is a request, not a default,
//! and a switch that swallowed it would be a bug rather than a saving.

use bevy::ecs::resource::Resource;

/// Environment variable that opts into automatic trace dumps.
///
/// An env var rather than a build feature or a settings entry: the person who
/// wants this is mid-investigation and wants it for one run, without a rebuild
/// and without leaving a toggle switched on in a config file they will forget
/// about. Matches the existing `AMBITION_*` dev-toggle convention.
pub const AUTO_DUMP_ENV: &str = "AMBITION_TRACE_AUTO_DUMP";

/// Which trace dumps are permitted to write files.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceDumpPolicy {
    /// Whether OOB/teleport-triggered dumps write to disk. **Default false.**
    pub auto_dumps: bool,
}

impl Default for TraceDumpPolicy {
    /// Off. See the module docs: the automatic dumps are the ones that
    /// accumulate, and nothing prunes them.
    fn default() -> Self {
        Self { auto_dumps: false }
    }
}

impl TraceDumpPolicy {
    /// Read the policy from the environment.
    ///
    /// Anything other than an explicit opt-in leaves automatic dumps off,
    /// including a malformed value: a policy that silently enabled disk writes
    /// because someone typed `AMBITION_TRACE_AUTO_DUMP=ture` would be the same
    /// surprise this exists to remove.
    pub fn from_env() -> Self {
        let enabled = std::env::var(AUTO_DUMP_ENV)
            .map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(false);
        Self {
            auto_dumps: enabled,
        }
    }

    /// Whether a dump with this trigger may be written.
    ///
    /// `automatic` comes from the dump reason itself (`DumpReason::is_automatic`
    /// / `ActorDumpReason::is_automatic`) rather than from the call site, so a
    /// new automatic trigger cannot be added without inheriting the gate.
    pub const fn allows(&self, automatic: bool) -> bool {
        !automatic || self.auto_dumps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_dumps_are_off_unless_asked_for() {
        let policy = TraceDumpPolicy::default();
        assert!(!policy.auto_dumps);
        assert!(
            !policy.allows(true),
            "an automatic dump must not write by default"
        );
        assert!(
            policy.allows(false),
            "a manual dump is a request, and is never gated"
        );
    }

    #[test]
    fn an_enabled_policy_permits_both() {
        let policy = TraceDumpPolicy { auto_dumps: true };
        assert!(policy.allows(true));
        assert!(policy.allows(false));
    }
}
