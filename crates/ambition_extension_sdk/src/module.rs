//! Module and entry descriptors. Admission reads these; it never calls an
//! entry to find out what it does.

use crate::digest::Digest;
use crate::invoke::{Fault, Invocation};
use crate::port::PortKey;
use crate::schema::{SchemaKey, StateSchema};
use crate::{ApiVersion, Name};

/// The namespaced identity of a module.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleKey {
    pub provider: Name,
    pub key: Name,
}

impl ModuleKey {
    pub const fn new(provider: &'static str, key: &'static str) -> Self {
        Self {
            provider: Name::Borrowed(provider),
            key: Name::Borrowed(key),
        }
    }
}

impl std::fmt::Display for ModuleKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.provider, self.key)
    }
}

/// Where the executable code of a module comes from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CodeIdentity {
    /// Rust code linked into the host executable. Its identity is the host
    /// build. A change to it rebuilds the host, so this is NOT the no-relink
    /// procedural loop; it is the native semantic reference (I4).
    StaticNative { crate_name: Name, version: Name },
}

/// A public semantic phase, for example `technique_execution`. A phase is a
/// fact the host guarantees (inputs ready, consume barrier not passed), not
/// the name of a private system.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Phase(pub Name);

impl Phase {
    pub const fn new(name: &'static str) -> Self {
        Self(Name::Borrowed(name))
    }
}

/// The public phases a host can offer. Each states what is TRUE when an
/// entry in it runs, not which private system runs it.
pub mod phases {
    use super::Phase;

    /// A body's move or brain has asked for a keyed technique this tick.
    /// Guarantees: the asking body's kinematics and target for this tick are
    /// settled; requests submitted here are consumed THIS tick, before the
    /// projectile and effect executors run. In the Ambition runtime this is
    /// `CombatSet::ContentSpecials`.
    pub const TECHNIQUE_EXECUTION: Phase = Phase::new("technique_execution");
}

/// The trigger port that starts an entry, and the selector inside that port
/// (for example the special-action key `echo_fan`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TriggerBinding {
    pub port: PortKey,
    pub selector: Name,
}

/// Deterministic limits for one invocation of an entry. A module that goes
/// past a limit faults; machine load never decides the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Limits {
    pub max_requests: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self { max_requests: 64 }
    }
}

/// The function an entry runs. The in-process Rust binding calls it directly.
pub type EntryFn = fn(&mut Invocation<'_>) -> Result<(), Fault>;

/// One entry point of a module.
#[derive(Clone, Debug)]
pub struct EntryDescriptor {
    /// Stable inside the module.
    pub key: Name,
    pub phase: Phase,
    pub trigger: TriggerBinding,
    /// Observation ports the entry reads.
    pub reads: Vec<PortKey>,
    /// Schemas whose records the entry may read and write.
    pub writes: Vec<SchemaKey>,
    /// Request ports the entry submits to.
    pub requests: Vec<PortKey>,
    /// Entries (`provider::module/entry`) that must run before this one in the
    /// same phase. Admission refuses a cycle.
    pub after: Vec<Name>,
    pub limits: Limits,
    pub run: EntryFn,
}

/// The complete declaration of a module.
#[derive(Clone, Debug)]
pub struct ModuleDescriptor {
    pub key: ModuleKey,
    pub api: ApiVersion,
    pub code: CodeIdentity,
    pub schemas: Vec<StateSchema>,
    pub entries: Vec<EntryDescriptor>,
}

impl ModuleDescriptor {
    /// The full name of one of this module's entries: `provider::module/entry`.
    pub fn entry_path(&self, entry: &EntryDescriptor) -> String {
        format!("{}/{}", self.key, entry.key)
    }

    /// The mechanical identity of the descriptor: everything admission reads,
    /// with schema shapes by digest. The entry function pointer is not part of
    /// it — the code identity stands for the code.
    pub fn identity_digest(&self) -> u64 {
        let mut d = Digest::new();
        d.str(&self.key.provider).str(&self.key.key);
        d.u16(self.api.major).u16(self.api.minor);
        match &self.code {
            CodeIdentity::StaticNative {
                crate_name,
                version,
            } => {
                d.u8(1).str(crate_name).str(version);
            }
        }
        d.u32(self.schemas.len() as u32);
        for schema in &self.schemas {
            d.u64(schema.shape_digest());
        }
        d.u32(self.entries.len() as u32);
        for entry in &self.entries {
            d.str(&entry.key).str(&entry.phase.0);
            d.str(&entry.trigger.port.name)
                .u16(entry.trigger.port.version)
                .str(&entry.trigger.selector);
            for list in [&entry.reads, &entry.requests] {
                d.u32(list.len() as u32);
                for port in list {
                    d.str(&port.name).u16(port.version);
                }
            }
            d.u32(entry.writes.len() as u32);
            for key in &entry.writes {
                key.digest_into(&mut d);
            }
            d.u32(entry.after.len() as u32);
            for after in &entry.after {
                d.str(after);
            }
            d.u32(entry.limits.max_requests);
        }
        d.finish()
    }
}
