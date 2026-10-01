//! Admission: match each module's requirements against the ports this
//! composition installed, and fix one serial entry order.
//!
//! Admission is App-local and pure. It reads descriptors and offers; it never
//! calls an entry. Every refusal names the module and the reason. A
//! composition with any refusal does not start (see
//! [`crate::ExtensionHostPlugin`]).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use ambition_extension_sdk::digest::Digest;
use ambition_extension_sdk::{
    ApiVersion, EntryCode, EntryDescriptor, EntryFn, ModuleDescriptor, ModuleKey,
    Phase, PortKey, PortRole, SaveEligibility, SchemaKey, StateSchema,
};

/// The code of a loaded module file: something that runs entry `entry` of
/// module `module` from `ambition-ext-1` input bytes and returns output bytes.
/// An `Err` is a deterministic fault of that invocation (a trap, fuel run
/// out), never a reason to skip it on one peer.
pub trait ModuleBackend: Send + Sync + 'static {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String>;
}

/// Where a declared module's code is.
#[derive(Clone)]
pub enum ModuleCode {
    /// Rust functions in this process (`EntryCode::Native`).
    Native,
    /// Module number `module` of a loaded file.
    Loaded {
        backend: Arc<dyn ModuleBackend>,
        module: u32,
        /// The file it came from (a path, as the composition names it). A
        /// reload of that file replaces every module that came from it, as
        /// one set: a module the new build no longer exports leaves.
        artifact: Arc<str>,
    },
}

/// One module as a composition declares it.
#[derive(Clone)]
pub struct DeclaredModule {
    pub descriptor: ModuleDescriptor,
    pub code: ModuleCode,
    /// True when this module explicitly REPLACES a module with the same key
    /// (a developer's loaded build of a module the game also links). Without
    /// it, two modules with one key refuse.
    pub replaces: bool,
}

impl From<ModuleDescriptor> for DeclaredModule {
    fn from(descriptor: ModuleDescriptor) -> Self {
        Self {
            descriptor,
            code: ModuleCode::Native,
            replaces: false,
        }
    }
}

/// How the host runs one admitted entry.
#[derive(Clone)]
pub enum EntryRunner {
    Native(EntryFn),
    Loaded {
        backend: Arc<dyn ModuleBackend>,
        module: u32,
        entry: u32,
    },
}

impl std::fmt::Debug for EntryRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(_) => f.write_str("Native"),
            Self::Loaded { module, entry, .. } => write!(f, "Loaded({module}, {entry})"),
        }
    }
}

/// One installed port. The only way to make an offer is the install function
/// that also adds the adapter that answers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortOffer {
    pub key: PortKey,
    pub role: PortRole,
    pub phase: Phase,
    /// The domain that owns the port, for diagnostics.
    pub owner: &'static str,
}

/// Why admission refused a module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ApiVersion {
        module: ModuleKey,
        built: ApiVersion,
        host: ApiVersion,
    },
    DuplicateModule(ModuleKey),
    DuplicateEntry {
        module: ModuleKey,
        entry: String,
    },
    InvalidSchema {
        schema: SchemaKey,
        reason: String,
    },
    DuplicateSchema(SchemaKey),
    /// A module declared a schema in another provider's namespace.
    ForeignSchema {
        module: ModuleKey,
        schema: SchemaKey,
    },
    /// The host has no support for this attachment or save policy yet. It
    /// refuses instead of treating the state as something it is not.
    UnsupportedPolicy {
        schema: SchemaKey,
        policy: String,
    },
    /// The entry needs a port this composition did not install, or installed
    /// with another role or in another phase.
    MissingPort {
        entry: String,
        port: PortKey,
        role: PortRole,
        phase: Phase,
    },
    /// The entry writes a schema its module did not declare.
    UndeclaredState {
        entry: String,
        schema: SchemaKey,
    },
    /// `IdlePolicy::ResetStateExcept` keeps a schema the entry does not write.
    IdleKeepsUnwrittenState {
        entry: String,
        schema: SchemaKey,
    },
    UnknownOrderTarget {
        entry: String,
        after: String,
    },
    /// `after` names an entry in another phase.
    CrossPhaseOrder {
        entry: String,
        after: String,
    },
    OrderCycle {
        entries: Vec<String>,
    },
    /// Two adapters answer one port in one phase.
    DuplicateOffer(PortKey),
    /// The entry's code is not where its module's code is (a loaded entry in
    /// a native module, or the reverse).
    NoCode { entry: String },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// One admitted entry.
#[derive(Clone, Debug)]
pub struct AdmittedEntry {
    pub module: ModuleKey,
    /// `provider::module/entry`.
    pub path: String,
    pub descriptor: EntryDescriptor,
    pub runner: EntryRunner,
}

/// One admitted schema with its shape digest.
#[derive(Clone, Debug)]
pub struct AdmittedSchema {
    pub schema: StateSchema,
    pub shape: u64,
}

/// The sealed result of admission.
#[derive(Clone, Debug, Default)]
pub struct Admitted {
    /// Entries in serial execution order: dependency order, and declaration
    /// order where no dependency decides.
    pub entries: Vec<AdmittedEntry>,
    pub schemas: BTreeMap<SchemaKey, AdmittedSchema>,
    /// The offers this admission was sealed against.
    pub offers: Vec<PortOffer>,
    /// The identity of the admitted set: every module's descriptor digest and
    /// every offer. A changed composition gives a different digest.
    pub digest: u64,
    /// Modules replaced by an explicit replacement, as `replaced -> by`.
    pub replaced: Vec<String>,
}

impl Admitted {
    /// True when some admitted entry is bound to this trigger and selector.
    /// A trigger adapter asks this so it does not build values nobody reads.
    pub fn wants(&self, port: &PortKey, selector: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.descriptor.trigger.port == *port && e.descriptor.trigger.selector == selector)
    }

    pub fn entries_in<'a>(&'a self, phase: &'a Phase) -> impl Iterator<Item = &'a AdmittedEntry> + 'a {
        self.entries
            .iter()
            .filter(move |e| e.descriptor.phase == *phase)
    }
}

/// Admit `modules` against `offers`. All refusals are reported, not only the
/// first.
pub fn admit(
    host_api: ApiVersion,
    offers: &[PortOffer],
    declared: &[DeclaredModule],
) -> Result<Admitted, Vec<Refusal>> {
    let mut refusals = Vec::new();
    let (declared, replaced) = resolve_replacements(declared, &mut refusals);
    let mut schemas: BTreeMap<SchemaKey, AdmittedSchema> = BTreeMap::new();
    let mut entries: Vec<AdmittedEntry> = Vec::new();
    let mut seen_modules: Vec<&ModuleKey> = Vec::new();

    for (i, offer) in offers.iter().enumerate() {
        if offers[..i]
            .iter()
            .any(|o| o.key == offer.key && o.phase == offer.phase)
        {
            refusals.push(Refusal::DuplicateOffer(offer.key.clone()));
        }
    }

    for declared_module in &declared {
        let module = &declared_module.descriptor;
        if seen_modules.contains(&&module.key) {
            refusals.push(Refusal::DuplicateModule(module.key.clone()));
            continue;
        }
        seen_modules.push(&module.key);

        if module.api.major != host_api.major || module.api.minor > host_api.minor {
            refusals.push(Refusal::ApiVersion {
                module: module.key.clone(),
                built: module.api,
                host: host_api,
            });
            continue;
        }

        for schema in &module.schemas {
            if let Some(refusal) = schema_refusal(module, schema) {
                refusals.push(refusal);
                continue;
            }
            if schemas.contains_key(&schema.key) {
                refusals.push(Refusal::DuplicateSchema(schema.key.clone()));
                continue;
            }
            schemas.insert(
                schema.key.clone(),
                AdmittedSchema {
                    schema: schema.clone(),
                    shape: schema.shape_digest(),
                },
            );
        }

        let mut entry_keys: Vec<&str> = Vec::new();
        for entry in &module.entries {
            let path = module.entry_path(entry);
            if entry_keys.contains(&entry.key.as_ref()) {
                refusals.push(Refusal::DuplicateEntry {
                    module: module.key.clone(),
                    entry: path,
                });
                continue;
            }
            entry_keys.push(&entry.key);
            refusals.extend(entry_port_refusals(&path, entry, offers));
            for key in &entry.writes {
                if !module.schemas.iter().any(|s| s.key == *key) {
                    refusals.push(Refusal::UndeclaredState {
                        entry: path.clone(),
                        schema: key.clone(),
                    });
                }
            }
            if let ambition_extension_sdk::IdlePolicy::ResetStateExcept(keep) = &entry.on_idle {
                for key in keep.iter().filter(|key| !entry.writes.contains(key)) {
                    refusals.push(Refusal::IdleKeepsUnwrittenState {
                        entry: path.clone(),
                        schema: key.clone(),
                    });
                }
            }
            let runner = match (&declared_module.code, entry.run) {
                (ModuleCode::Native, EntryCode::Native(run)) => EntryRunner::Native(run),
                (ModuleCode::Loaded { backend, module, .. }, EntryCode::Loaded { index }) => {
                    EntryRunner::Loaded {
                        backend: backend.clone(),
                        module: *module,
                        entry: index,
                    }
                }
                _ => {
                    refusals.push(Refusal::NoCode { entry: path });
                    continue;
                }
            };
            entries.push(AdmittedEntry {
                module: module.key.clone(),
                path,
                descriptor: entry.clone(),
                runner,
            });
        }
    }

    let ordered = match order_entries(&entries) {
        Ok(order) => order,
        Err(mut order_refusals) => {
            refusals.append(&mut order_refusals);
            Vec::new()
        }
    };

    if !refusals.is_empty() {
        return Err(refusals);
    }

    let mut d = Digest::new();
    d.u16(host_api.major).u16(host_api.minor);
    d.u32(declared.len() as u32);
    for module in &declared {
        d.u64(module.descriptor.identity_digest());
    }
    d.u32(offers.len() as u32);
    for offer in offers {
        d.str(&offer.key.name)
            .u16(offer.key.version)
            .u8(role_code(offer.role))
            .str(&offer.phase.0);
    }

    Ok(Admitted {
        entries: ordered.into_iter().map(|i| entries[i].clone()).collect(),
        schemas,
        offers: offers.to_vec(),
        digest: d.finish(),
        replaced,
    })
}

/// Apply explicit replacements: where several modules share a key and
/// exactly one says `replaces`, that one stays. Every other shared key stays
/// a duplicate for the main loop to refuse.
fn resolve_replacements(
    declared: &[DeclaredModule],
    refusals: &mut Vec<Refusal>,
) -> (Vec<DeclaredModule>, Vec<String>) {
    let mut kept: Vec<DeclaredModule> = Vec::new();
    let mut replaced = Vec::new();
    for module in declared {
        let key = &module.descriptor.key;
        let same: Vec<&DeclaredModule> = declared.iter().filter(|m| m.descriptor.key == *key).collect();
        let replacers = same.iter().filter(|m| m.replaces).count();
        if same.len() == 1 || replacers == 0 {
            kept.push(module.clone());
        } else if replacers > 1 {
            if !refusals.contains(&Refusal::DuplicateModule(key.clone())) {
                refusals.push(Refusal::DuplicateModule(key.clone()));
            }
        } else if module.replaces {
            replaced.push(format!(
                "{key}: {:?} -> {:?}",
                same.iter().find(|m| !m.replaces).map(|m| &m.descriptor.code),
                module.descriptor.code
            ));
            kept.push(module.clone());
        }
    }
    (kept, replaced)
}

fn role_code(role: PortRole) -> u8 {
    match role {
        PortRole::Trigger => 1,
        PortRole::Observation => 2,
        PortRole::Request => 3,
    }
}

fn schema_refusal(module: &ModuleDescriptor, schema: &StateSchema) -> Option<Refusal> {
    if let Err(reason) = schema.validate() {
        return Some(Refusal::InvalidSchema {
            schema: schema.key.clone(),
            reason: format!("{reason:?}"),
        });
    }
    if schema.key.provider != module.key.provider {
        return Some(Refusal::ForeignSchema {
            module: module.key.clone(),
            schema: schema.key.clone(),
        });
    }
    // Body- and session-attached transient state have a host store and a
    // retirement road (`store`). The save roads are I5 work; until they exist,
    // a module that needs one is refused, never silently downgraded.
    if schema.save != SaveEligibility::Transient {
        return Some(Refusal::UnsupportedPolicy {
            schema: schema.key.clone(),
            policy: format!("save {:?}", schema.save),
        });
    }
    None
}

fn entry_port_refusals(path: &str, entry: &EntryDescriptor, offers: &[PortOffer]) -> Vec<Refusal> {
    let mut needs: Vec<(&PortKey, PortRole)> = vec![(&entry.trigger.port, PortRole::Trigger)];
    needs.extend(entry.reads.iter().map(|p| (p, PortRole::Observation)));
    needs.extend(entry.requests.iter().map(|p| (p, PortRole::Request)));
    needs
        .into_iter()
        .filter(|(port, role)| {
            !offers
                .iter()
                .any(|o| o.key == **port && o.role == *role && o.phase == entry.phase)
        })
        .map(|(port, role)| Refusal::MissingPort {
            entry: path.to_owned(),
            port: port.clone(),
            role,
            phase: entry.phase.clone(),
        })
        .collect()
}

/// Kahn's algorithm. Among ready entries, the earliest declared runs first, so
/// the order depends only on the descriptors.
fn order_entries(entries: &[AdmittedEntry]) -> Result<Vec<usize>, Vec<Refusal>> {
    let index: HashMap<&str, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| (e.path.as_str(), i))
        .collect();
    let mut refusals = Vec::new();
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); entries.len()];
    for (i, entry) in entries.iter().enumerate() {
        for after in &entry.descriptor.after {
            match index.get(after.as_ref()) {
                None => refusals.push(Refusal::UnknownOrderTarget {
                    entry: entry.path.clone(),
                    after: after.to_string(),
                }),
                Some(&j) if entries[j].descriptor.phase != entry.descriptor.phase => {
                    refusals.push(Refusal::CrossPhaseOrder {
                        entry: entry.path.clone(),
                        after: after.to_string(),
                    })
                }
                Some(&j) => preds[i].push(j),
            }
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let mut done = vec![false; entries.len()];
    let mut order = Vec::with_capacity(entries.len());
    while order.len() < entries.len() {
        let next = (0..entries.len()).find(|&i| !done[i] && preds[i].iter().all(|&p| done[p]));
        match next {
            Some(i) => {
                done[i] = true;
                order.push(i);
            }
            None => {
                let entries = (0..entries.len())
                    .filter(|&i| !done[i])
                    .map(|i| entries[i].path.clone())
                    .collect();
                return Err(vec![Refusal::OrderCycle { entries }]);
            }
        }
    }
    Ok(order)
}
