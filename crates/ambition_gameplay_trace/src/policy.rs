//! Policy controlling whether gameplay traces may write files.
//!
//! Automatic OOB/teleport dumps are opt-in; recording and in-memory diagnostics
//! remain active regardless. Manual dumps are always allowed because they are an
//! explicit developer request.

use bevy::ecs::resource::Resource;

/// Environment variable for opting into automatic trace dumps for a run.
pub const AUTO_DUMP_ENV: &str = "AMBITION_TRACE_AUTO_DUMP";

/// Which trace dumps are permitted to write files.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceDumpPolicy {
    /// Whether OOB/teleport-triggered dumps write to disk. Default false.
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
