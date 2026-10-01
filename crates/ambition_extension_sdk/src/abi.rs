//! The loaded-module ABI, `ambition-ext-1`: the bytes a host and a guest
//! exchange, and the guest half that runs an entry from them.
//!
//! A guest exports three functions and its linear memory:
//!
//! | export | signature | meaning |
//! |---|---|---|
//! | `amb_alloc` | `(len: u32) -> u32` | a buffer of `len` bytes for the host to write into |
//! | `amb_describe` | `() -> u64` | the module list ([`describe`]), as `ptr << 32 \| len` |
//! | `amb_invoke` | `(module: u32, entry: u32, ptr: u32, len: u32) -> u64` | run one entry ([`invoke`]) |
//!
//! It imports nothing: no clock, no files, no entropy. The host builds a new
//! instance for each call, so no guest global outlives an invocation; all
//! state that lasts is in the records the host gives and takes back.
//! [`export_modules!`](crate::export_modules) writes the exports.
//!
//! Input (host → guest): tick `u64`; occurrence `opt u32`; trigger port and
//! bytes; observations `[(port, bytes)]`; state `[(schema key, record)]`.
//! Output (guest → host): `0` then state `[(schema key, record)]` and
//! requests `[(port, bytes)]`; or `1` then a fault message.

use crate::invoke::{HostParts, Observation, OwnedPayload, Payload, StagedRequest};
use crate::module::{EntryCode, ModuleDescriptor};
use crate::port::PortKey;
use crate::schema::{Record, SchemaKey, StateSchema};
use crate::wire::{self, Reader, WireError};
use crate::{Fault, Invocation};

/// The ABI name a loaded module's code identity records.
pub const ABI: &str = "ambition-ext-1";

/// The module list a guest publishes.
pub fn describe(modules: &[ModuleDescriptor]) -> Vec<u8> {
    let mut out = Vec::new();
    wire::put_u32(&mut out, modules.len() as u32);
    for module in modules {
        module.put(&mut out);
    }
    out
}

/// Decode a guest's module list.
pub fn read_description(bytes: &[u8]) -> Result<Vec<ModuleDescriptor>, WireError> {
    wire::decode_all(bytes, |r| {
        let n = r.read_len()?;
        (0..n).map(|_| ModuleDescriptor::read(r, ABI)).collect()
    })
}

/// One invocation's input, as the host holds it.
pub struct InvocationInput<'a> {
    pub tick: u64,
    pub occurrence: Option<u32>,
    pub trigger_port: &'a PortKey,
    pub trigger: &'a [u8],
    pub observations: &'a [(PortKey, Vec<u8>)],
    pub state: &'a [(SchemaKey, Record)],
}

impl InvocationInput<'_> {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        wire::put_u64(&mut out, self.tick);
        wire::put_opt(&mut out, self.occurrence, wire::put_u32);
        self.trigger_port.put(&mut out);
        wire::put_bytes(&mut out, self.trigger);
        wire::put_u32(&mut out, self.observations.len() as u32);
        for (port, bytes) in self.observations {
            port.put(&mut out);
            wire::put_bytes(&mut out, bytes);
        }
        wire::put_u32(&mut out, self.state.len() as u32);
        for (key, record) in self.state {
            key.put(&mut out);
            let mut bytes = Vec::new();
            record.put_wire(&mut bytes);
            wire::put_bytes(&mut out, &bytes);
        }
        out
    }
}

/// What a guest returned.
#[derive(Debug)]
pub enum InvocationOutput {
    Ok {
        state: Vec<(SchemaKey, Record)>,
        requests: Vec<(PortKey, Vec<u8>)>,
    },
    Fault(String),
}

/// Decode a guest's output. `schema` gives the schema of each record key; a
/// key it does not know is an error.
pub fn read_output<'s>(
    bytes: &[u8],
    schema: impl Fn(&SchemaKey) -> Option<&'s StateSchema>,
) -> Result<InvocationOutput, WireError> {
    wire::decode_all(bytes, |r| match r.u8()? {
        0 => {
            let n = r.read_len()?;
            let mut state = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                let key = SchemaKey::read(r)?;
                let record_bytes = r.bytes()?;
                let schema = schema(&key).ok_or(WireError::BadTag(0))?;
                let record = wire::decode_all(record_bytes, |r| schema.read_record(r))?;
                state.push((key, record));
            }
            let n = r.read_len()?;
            let mut requests = Vec::with_capacity(n.min(256));
            for _ in 0..n {
                let port = PortKey::read(r)?;
                requests.push((port, r.bytes()?.to_vec()));
            }
            Ok(InvocationOutput::Ok { state, requests })
        }
        1 => Ok(InvocationOutput::Fault(r.str()?.to_owned())),
        other => Err(WireError::BadTag(other)),
    })
}

fn fault_bytes(message: &str) -> Vec<u8> {
    let mut out = vec![1];
    wire::put_str(&mut out, message);
    out
}

/// The guest half: decode the input, run entry `entry` of module `module`,
/// encode what it staged. A decode failure or a fault is a fault output.
pub fn invoke(modules: &[ModuleDescriptor], module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
    match invoke_inner(modules, module, entry, input) {
        Ok(out) => out,
        Err(message) => fault_bytes(&message),
    }
}

fn invoke_inner(
    modules: &[ModuleDescriptor],
    module: u32,
    entry: u32,
    input: &[u8],
) -> Result<Vec<u8>, String> {
    let module = modules
        .get(module as usize)
        .ok_or_else(|| format!("no module {module}"))?;
    let descriptor = module
        .entries
        .get(entry as usize)
        .ok_or_else(|| format!("no entry {entry} in {}", module.key))?;
    let EntryCode::Native(run) = descriptor.run else {
        return Err(format!("entry {} has no code in this guest", descriptor.key));
    };

    let mut r = Reader::new(input);
    let decoded = (|| -> Result<_, WireError> {
        let tick = r.u64()?;
        let occurrence = r.opt(Reader::u32)?;
        let trigger_port = PortKey::read(&mut r)?;
        let trigger = r.bytes()?;
        let n = r.read_len()?;
        let mut observations = Vec::with_capacity(n.min(64));
        for _ in 0..n {
            let port = PortKey::read(&mut r)?;
            observations.push(Observation {
                port,
                value: OwnedPayload::Wire(r.bytes()?.to_vec()),
            });
        }
        let n = r.read_len()?;
        let mut state = Vec::with_capacity(n.min(64));
        for _ in 0..n {
            let key = SchemaKey::read(&mut r)?;
            let bytes = r.bytes()?;
            let schema = module
                .schemas
                .iter()
                .find(|s| s.key == key)
                .ok_or(WireError::BadTag(0))?;
            state.push((key, wire::decode_all(bytes, |r| schema.read_record(r))?));
        }
        r.finish()?;
        Ok((tick, occurrence, trigger_port, trigger, observations, state))
    })()
    .map_err(|e| format!("the invocation input does not decode: {e}"))?;
    let (tick, occurrence, trigger_port, trigger, observations, mut state) = decoded;

    let mut requests: Vec<StagedRequest> = Vec::new();
    let mut call = Invocation::from_host(HostParts {
        entry: descriptor,
        tick,
        occurrence,
        trigger_port: &trigger_port,
        trigger: Payload::Wire(trigger),
        observations: &observations,
        state: &mut state,
        requests: &mut requests,
    });
    run(&mut call).map_err(|fault: Fault| fault.to_string())?;
    drop(call);

    let mut out = vec![0];
    wire::put_u32(&mut out, state.len() as u32);
    for (key, record) in &state {
        key.put(&mut out);
        let mut bytes = Vec::new();
        record.put_wire(&mut bytes);
        wire::put_bytes(&mut out, &bytes);
    }
    wire::put_u32(&mut out, requests.len() as u32);
    for request in &requests {
        request.port.put(&mut out);
        let mut bytes = Vec::new();
        (request.encode)(request.value.as_ref(), &mut bytes);
        wire::put_bytes(&mut out, &bytes);
    }
    Ok(out)
}

/// Write the `ambition-ext-1` exports for a function that lists this
/// crate's modules. It expands to nothing except on `wasm32`.
///
/// ```ignore
/// ambition_extension_sdk::export_modules!(crate::modules);
/// ```
#[macro_export]
macro_rules! export_modules {
    ($modules:path) => {
        #[cfg(target_arch = "wasm32")]
        mod __ambition_extension_exports {
            fn publish(bytes: ::std::vec::Vec<u8>) -> u64 {
                let bytes = ::std::mem::ManuallyDrop::new(bytes);
                ((bytes.as_ptr() as u64) << 32) | bytes.len() as u64
            }

            #[no_mangle]
            pub extern "C" fn amb_alloc(len: u32) -> u32 {
                let buffer = ::std::mem::ManuallyDrop::new(::std::vec::Vec::<u8>::with_capacity(
                    len as usize,
                ));
                buffer.as_ptr() as u32
            }

            #[no_mangle]
            pub extern "C" fn amb_describe() -> u64 {
                publish($crate::abi::describe(&$modules()))
            }

            #[no_mangle]
            pub extern "C" fn amb_invoke(module: u32, entry: u32, ptr: u32, len: u32) -> u64 {
                // SAFETY: the host wrote `len` bytes at `ptr`, a buffer
                // `amb_alloc` returned, before this call.
                let input = unsafe { ::std::slice::from_raw_parts(ptr as *const u8, len as usize) };
                publish($crate::abi::invoke(&$modules(), module, entry, input))
            }
        }
    };
}
