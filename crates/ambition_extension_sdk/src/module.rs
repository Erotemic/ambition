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
    /// A loaded executable: its ABI name and the digest of its exact bytes.
    /// A guest says `Loaded` with digest 0; the host that loads the bytes
    /// fills the digest in.
    Loaded { abi: Name, digest: u64 },
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

    /// A body uses the item it holds this tick. Guarantees: the body's
    /// control frame, kinematics and gravity frame for this tick are settled;
    /// requests submitted here are consumed THIS tick, before the effect and
    /// projectile executors run. In the Ambition runtime this is
    /// `ItemPickupSet::WieldedAbilities`.
    pub const WIELDED_USE: Phase = Phase::new("wielded_use");
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

/// What the host does with an IDLE trigger: one the trigger's domain marks as
/// having nothing to act on this tick (for the boss special trigger: the key
/// is neither pressed nor telegraphed).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum IdlePolicy {
    /// Call the entry anyway.
    #[default]
    Invoke,
    /// Do not call the entry; put each of its declared records back to its
    /// initial value. For an entry whose rule is "an idle tick ends what I was
    /// doing", this is the same result without the call, and it is what makes
    /// a loaded module cost nothing on the ticks it has nothing to do.
    ResetState,
}

/// The function an entry runs. The in-process Rust binding calls it directly.
pub type EntryFn = fn(&mut Invocation<'_>) -> Result<(), Fault>;

/// Where an entry's code is.
#[derive(Clone, Copy, Debug)]
pub enum EntryCode {
    /// A Rust function in this process.
    Native(EntryFn),
    /// Entry number `index` of a loaded module. The host's backend for that
    /// module runs it; the descriptor alone cannot.
    Loaded { index: u32 },
}

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
    pub on_idle: IdlePolicy,
    pub run: EntryCode,
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
            CodeIdentity::Loaded { abi, digest } => {
                d.u8(2).str(abi).u64(*digest);
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
            d.u8(match entry.on_idle {
                IdlePolicy::Invoke => 0,
                IdlePolicy::ResetState => 1,
            });
        }
        d.finish()
    }
}

impl ModuleDescriptor {
    /// The descriptor on the wire. Entry code is not encoded: a decoded
    /// descriptor's entries are `EntryCode::Loaded` in declaration order.
    pub fn put(&self, out: &mut Vec<u8>) {
        use crate::wire::*;
        put_str(out, &self.key.provider);
        put_str(out, &self.key.key);
        put_u16(out, self.api.major);
        put_u16(out, self.api.minor);
        put_u32(out, self.schemas.len() as u32);
        for schema in &self.schemas {
            schema.put(out);
        }
        put_u32(out, self.entries.len() as u32);
        for entry in &self.entries {
            put_str(out, &entry.key);
            put_str(out, &entry.phase.0);
            entry.trigger.port.put(out);
            put_str(out, &entry.trigger.selector);
            for list in [&entry.reads, &entry.requests] {
                put_u32(out, list.len() as u32);
                for port in list {
                    port.put(out);
                }
            }
            put_u32(out, entry.writes.len() as u32);
            for key in &entry.writes {
                key.put(out);
            }
            put_u32(out, entry.after.len() as u32);
            for after in &entry.after {
                put_str(out, after);
            }
            put_u32(out, entry.limits.max_requests);
            put_u8(
                out,
                match entry.on_idle {
                    IdlePolicy::Invoke => 0,
                    IdlePolicy::ResetState => 1,
                },
            );
        }
    }

    /// Decode a descriptor that a loaded module published. Its code identity
    /// is `Loaded` with digest 0 until the host fills it in.
    pub fn read(r: &mut crate::wire::WireReader<'_>, abi: &'static str) -> Result<Self, crate::wire::WireError> {
        let key = ModuleKey {
            provider: r.str()?.to_owned().into(),
            key: r.str()?.to_owned().into(),
        };
        let api = ApiVersion {
            major: r.u16()?,
            minor: r.u16()?,
        };
        let n = r.read_len()?;
        let schemas = (0..n)
            .map(|_| StateSchema::read(r))
            .collect::<Result<Vec<_>, _>>()?;
        let n = r.read_len()?;
        let mut entries = Vec::with_capacity(n.min(256));
        for index in 0..n {
            let key: Name = r.str()?.to_owned().into();
            let phase = Phase(r.str()?.to_owned().into());
            let port = PortKey::read(r)?;
            let selector: Name = r.str()?.to_owned().into();
            let mut lists = [Vec::new(), Vec::new()];
            for list in &mut lists {
                let m = r.read_len()?;
                *list = (0..m).map(|_| PortKey::read(r)).collect::<Result<_, _>>()?;
            }
            let [reads, requests] = lists;
            let m = r.read_len()?;
            let writes = (0..m).map(|_| SchemaKey::read(r)).collect::<Result<_, _>>()?;
            let m = r.read_len()?;
            let after = (0..m)
                .map(|_| r.str().map(|s| Name::from(s.to_owned())))
                .collect::<Result<_, _>>()?;
            let limits = Limits {
                max_requests: r.u32()?,
            };
            let on_idle = match r.u8()? {
                0 => IdlePolicy::Invoke,
                1 => IdlePolicy::ResetState,
                other => return Err(crate::wire::WireError::BadTag(other)),
            };
            entries.push(EntryDescriptor {
                key,
                phase,
                trigger: TriggerBinding { port, selector },
                reads,
                writes,
                requests,
                after,
                limits,
                on_idle,
                run: EntryCode::Loaded {
                    index: index as u32,
                },
            });
        }
        Ok(Self {
            key,
            api,
            code: CodeIdentity::Loaded {
                abi: Name::Borrowed(abi),
                digest: 0,
            },
            schemas,
            entries,
        })
    }
}
