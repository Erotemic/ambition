//! The WebAssembly backend of the extension host.
//!
//! A `.wasm` file built against the SDK (`ambition_extension_sdk::export_modules!`)
//! is a [`WasmModules`]: the host admits its descriptors like any module's and
//! runs its entries through [`ModuleBackend::invoke`]. A change to the module
//! rebuilds the module crate for `wasm32-unknown-unknown`; the engine is not
//! compiled or linked again (extension-model D1).
//!
//! ⭐ DETERMINISM IS BY CONSTRUCTION, NOT BY CARE.
//!
//! * The module may import NOTHING. Loading refuses a module with an import,
//!   so it has no clock, no files and no entropy.
//! * Each call starts from the file's STARTING IMAGE. No guest global, heap or
//!   table survives an invocation; everything that lasts is in the records the
//!   host passes in and takes back (extension-state-and-execution: "VM memory
//!   is scratch"). The instance is reused only when loading proved it can be
//!   put back exactly: every mutable global exported, no code that changes a
//!   table or a passive segment ([`restorable_globals`]); its memory and
//!   globals are then set back to the image after each call, and a call that
//!   grows memory drops it. Such a file's image is taken after one guest
//!   allocation, so the allocator already holds its heap. Any other file gets
//!   a new instance per call.
//! * Work is metered by FUEL, a count of executed instructions. A call that
//!   runs out faults the same way on every machine; load never decides it.
//! * The interpreter canonicalizes NaN results (`wasmi/deterministic`).
//!
//! ⚠ Not a security sandbox claim beyond what the interpreter gives. And a
//! module's floating-point transcendentals (`sin`, `atan2`) are the guest's
//! own compiled code, so they can differ from the native host's in the last
//! bit; a native module and its WASM build are two executables, not one.

use std::sync::{Arc, Mutex};

use ambition_extension_host::ModuleBackend;
use ambition_extension_sdk::abi;
use ambition_extension_sdk::digest::digest_bytes;
use ambition_extension_sdk::{CodeIdentity, ModuleDescriptor, ModuleKey, Name};
use wasmi::{Config, Engine, Global, Instance, Linker, Memory, Module, Store, TypedFunc, Val};

/// The work one invocation may do, in fuel units (about one per executed
/// instruction). Generous: a fan of seven shots costs a few thousand.
pub const DEFAULT_FUEL: u64 = 20_000_000;

/// Why a `.wasm` file could not become modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The bytes are not a valid module.
    Compile(String),
    /// The module does not have the `ambition-ext-3` exports, or imports
    /// something.
    Abi(String),
    /// `amb_describe` failed or returned bytes that do not decode.
    Describe(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// One loaded `.wasm` file and the modules it describes.
pub struct WasmModules {
    engine: Engine,
    module: Module,
    fuel: u64,
    /// The key of each module, in the file's own order: the number
    /// `amb_invoke` takes.
    keys: Vec<ModuleKey>,
    /// The export names of the mutable globals when an instance can be put
    /// back to its starting image, or why it cannot.
    restorable: Result<Vec<String>, String>,
    /// The instance a call reuses, at its starting image; empty while a call
    /// holds it (a concurrent call starts its own) or before the first call.
    held: Mutex<Option<Held>>,
}

impl WasmModules {
    /// Load `bytes`, ask the module to describe itself, and stamp each
    /// descriptor with the digest of the exact bytes.
    pub fn load(bytes: &[u8]) -> Result<(Arc<Self>, Vec<ModuleDescriptor>), LoadError> {
        Self::load_with_fuel(bytes, DEFAULT_FUEL)
    }

    pub fn load_with_fuel(
        bytes: &[u8],
        fuel: u64,
    ) -> Result<(Arc<Self>, Vec<ModuleDescriptor>), LoadError> {
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, bytes).map_err(|e| LoadError::Compile(e.to_string()))?;
        if let Some(import) = module.imports().next() {
            return Err(LoadError::Abi(format!(
                "a module imports nothing; this one imports {}::{}",
                import.module(),
                import.name()
            )));
        }
        let mut this = Self {
            engine,
            module,
            fuel,
            keys: Vec::new(),
            restorable: restorable_globals(bytes),
            held: Mutex::new(None),
        };
        let description = this.describe().map_err(LoadError::Describe)?;
        let mut modules = abi::read_description(&description)
            .map_err(|e| LoadError::Describe(format!("the description does not decode: {e}")))?;
        this.keys = modules.iter().map(|module| module.key.clone()).collect();
        let this = Arc::new(this);
        let digest = digest_bytes(bytes);
        for module in &mut modules {
            module.code = CodeIdentity::Loaded {
                abi: Name::Borrowed(abi::ABI),
                digest,
            };
        }
        Ok((this, modules))
    }

    fn instantiate(&self) -> Result<(Store<()>, Instance, Memory), String> {
        let mut store = Store::new(&self.engine, ());
        store.set_fuel(self.fuel).map_err(|e| e.to_string())?;
        let linker = Linker::<()>::new(&self.engine);
        let instance = linker
            .instantiate_and_start(&mut store, &self.module)
            .map_err(|e| format!("instantiate: {e}"))?;
        let memory = instance
            .get_memory(&store, "memory")
            .ok_or("the module exports no `memory`")?;
        Ok((store, instance, memory))
    }

    /// Whether a call reuses one instance put back to its starting image, or
    /// why each call starts a new one.
    pub fn reuses_instances(&self) -> Result<(), &str> {
        self.restorable.as_ref().map(|_| ()).map_err(String::as_str)
    }

    fn start(&self) -> Result<Held, String> {
        let (mut store, instance, memory) = self.instantiate()?;
        let alloc = instance
            .get_typed_func::<u32, u32>(&store, "amb_alloc")
            .map_err(|e| format!("amb_alloc: {e}"))?;
        let invoke = instance
            .get_typed_func::<(u32, u32, u32, u32), u64>(&store, "amb_invoke")
            .map_err(|e| format!("amb_invoke: {e}"))?;
        if self.restorable.is_ok() {
            // The guest's allocator takes its heap with `memory.grow` on its
            // first allocation, so a call from the bare instance always grows
            // memory and the instance could never be put back. The image is
            // taken once the allocator holds its first heap: a fixed function
            // of the file, the same for every call and every peer.
            alloc.call(&mut store, 1).map_err(trap)?;
        }
        let image = memory.data(&store).to_vec();
        let globals = match &self.restorable {
            Ok(names) => names
                .iter()
                .map(|name| {
                    let global = instance
                        .get_global(&store, name)
                        .ok_or_else(|| format!("exported global `{name}` is missing"))?;
                    Ok((global, global.get(&store)))
                })
                .collect::<Result<_, String>>()?,
            Err(_) => Vec::new(),
        };
        Ok(Held {
            store,
            memory,
            alloc,
            invoke,
            image,
            globals,
        })
    }

    fn read_packed(store: &Store<()>, memory: Memory, packed: u64) -> Result<Vec<u8>, String> {
        let ptr = (packed >> 32) as usize;
        let len = (packed & 0xffff_ffff) as usize;
        let mut out = vec![0u8; len];
        memory
            .read(store, ptr, &mut out)
            .map_err(|e| format!("result out of bounds: {e}"))?;
        Ok(out)
    }

    fn describe(&self) -> Result<Vec<u8>, String> {
        let (mut store, instance, memory) = self.instantiate()?;
        let describe = instance
            .get_typed_func::<(), u64>(&store, "amb_describe")
            .map_err(|e| format!("amb_describe: {e}"))?;
        let packed = describe.call(&mut store, ()).map_err(trap)?;
        Self::read_packed(&store, memory, packed)
    }
}

/// Why an instance of `bytes` cannot be put back to its starting image after
/// a call, or the export names of its mutable globals when it can. A call
/// changes an instance's memory, its mutable globals and, through the
/// table and passive-segment instructions, its tables and segments. Memory
/// is copied back and a global is set back only when it is exported, so
/// the file qualifies when every mutable global is exported and no code
/// touches a table or a passive segment. (A call that grows memory is
/// caught at restore: the instance is dropped.)
fn restorable_globals(bytes: &[u8]) -> Result<Vec<String>, String> {
    use wasmparser::{ExternalKind, Operator, Parser, Payload};
    let mut mutable = Vec::new();
    let mut exported = std::collections::BTreeMap::new();
    for payload in Parser::new(0).parse_all(bytes) {
        match payload.map_err(|e| e.to_string())? {
            Payload::GlobalSection(reader) => {
                for (index, global) in reader.into_iter().enumerate() {
                    if global.map_err(|e| e.to_string())?.ty.mutable {
                        mutable.push(index as u32);
                    }
                }
            }
            Payload::ExportSection(reader) => {
                for export in reader {
                    let export = export.map_err(|e| e.to_string())?;
                    if export.kind == ExternalKind::Global {
                        exported.insert(export.index, export.name.to_owned());
                    }
                }
            }
            Payload::CodeSectionEntry(body) => {
                let mut operators = body.get_operators_reader().map_err(|e| e.to_string())?;
                while !operators.eof() {
                    match operators.read().map_err(|e| e.to_string())? {
                        Operator::TableSet { .. }
                        | Operator::TableGrow { .. }
                        | Operator::TableFill { .. }
                        | Operator::TableCopy { .. }
                        | Operator::TableInit { .. }
                        | Operator::ElemDrop { .. }
                        | Operator::MemoryInit { .. }
                        | Operator::DataDrop { .. } => {
                            return Err("its code changes a table or a passive segment".to_owned());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    mutable
        .into_iter()
        .map(|index| {
            exported
                .get(&index)
                .cloned()
                .ok_or_else(|| format!("mutable global {index} is not exported"))
        })
        .collect()
}

/// A started instance, the image it started from, and the two entries a
/// call uses.
struct Held {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    invoke: TypedFunc<(u32, u32, u32, u32), u64>,
    image: Vec<u8>,
    globals: Vec<(Global, Val)>,
}

impl Held {
    /// Put the instance back to its image, or say it cannot be: a call grew
    /// its memory, which never shrinks.
    fn restore(&mut self) -> bool {
        let data = self.memory.data_mut(&mut self.store);
        if data.len() != self.image.len() {
            return false;
        }
        data.copy_from_slice(&self.image);
        self.globals
            .iter()
            .all(|(global, value)| global.set(&mut self.store, value.clone()).is_ok())
    }
}

impl Held {
    fn call(&mut self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String> {
        let len = u32::try_from(input.len()).map_err(|_| "input too long".to_owned())?;
        let ptr = self.alloc.call(&mut self.store, len).map_err(trap)?;
        self.memory
            .write(&mut self.store, ptr as usize, input)
            .map_err(|e| format!("input out of bounds: {e}"))?;
        let packed = self
            .invoke
            .call(&mut self.store, (module, entry, ptr, len))
            .map_err(trap)?;
        WasmModules::read_packed(&self.store, self.memory, packed)
    }
}

fn trap(e: wasmi::Error) -> String {
    match e.as_trap_code() {
        Some(code) => format!("trap: {code}"),
        None => e.to_string(),
    }
}

impl ModuleBackend for WasmModules {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String> {
        let reusable = self.restorable.is_ok();
        let taken = if reusable {
            self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take()
        } else {
            None
        };
        let mut held = match taken {
            Some(held) => held,
            None => self.start()?,
        };
        held.store.set_fuel(self.fuel).map_err(|e| e.to_string())?;
        let result = held.call(module, entry, input);
        if reusable && held.restore() {
            *self.held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(held);
        }
        result
    }

    fn module_index(&self, key: &ModuleKey) -> Option<u32> {
        self.keys.iter().position(|known| known == key).map(|index| index as u32)
    }
}

/// The target a loaded module is built for.
pub const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// The stack a loaded module is linked with, in bytes. Rust's wasm default
/// is 1 MiB, and every instance zeroes its linear memory, so with a fresh
/// instance per call the default stack was about half the cost of a call
/// (M1, 2026-11-28: a sentry call went from about 89 to 59 µs at 128 KiB).
/// The stack sits first in memory, so a guest that overflows it traps, the
/// same on every peer. `scripts/regen/build_extension_modules.sh` links with
/// the same value.
pub const MODULE_STACK_BYTES: u32 = 128 * 1024;

/// The global a loaded module is linked to export: Rust's stack pointer,
/// its one mutable global, so a call can reuse an instance and set it back
/// ([`restorable_globals`]). `scripts/regen/build_extension_modules.sh`
/// exports it too.
pub const MODULE_STACK_POINTER: &str = "__stack_pointer";

/// True when the toolchain that builds `workspace_root` has the standard
/// library for [`WASM_TARGET`]. Without it, cargo fails with "can't find crate
/// for `core`", which does not say what to do.
fn wasm_target_installed(workspace_root: &std::path::Path) -> bool {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let Ok(out) = std::process::Command::new(rustc)
        .current_dir(workspace_root)
        .args(["--print", "sysroot"])
        .output()
    else {
        // Let cargo report it.
        return true;
    };
    let sysroot = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    std::path::Path::new(&sysroot).join("lib").join("rustlib").join(WASM_TARGET).join("lib").is_dir()
}

/// Build a workspace module crate for `wasm32-unknown-unknown` and return the
/// `.wasm` path. For tests and developer tools: it runs Cargo, so the engine
/// never calls it.
///
/// It uses its own target directory under `target/`, so it does not wait on
/// the lock of a Cargo process that is running the caller.
pub fn build_module_crate(workspace_root: &std::path::Path, package: &str) -> Result<std::path::PathBuf, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let target_dir = workspace_root.join("target").join("extension-modules");
    if !wasm_target_installed(workspace_root) {
        return Err(format!(
            "the rust target {WASM_TARGET} is not installed; \
             run `rustup target add {WASM_TARGET}` (or scripts/setup/web_prereq.sh)"
        ));
    }
    let status = std::process::Command::new(cargo)
        .current_dir(workspace_root)
        .args([
            "rustc",
            "--quiet",
            "-p",
            package,
            "--target",
            WASM_TARGET,
            "--release",
            "--crate-type",
            "cdylib",
            "--target-dir",
        ])
        .arg(&target_dir)
        .arg("--")
        .arg(format!("-Clink-arg=-zstack-size={MODULE_STACK_BYTES}"))
        .arg(format!("-Clink-arg=--export={MODULE_STACK_POINTER}"))
        .status()
        .map_err(|e| format!("could not run cargo: {e}"))?;
    if !status.success() {
        return Err(format!("cargo rustc for {package} failed: {status}"));
    }
    let wasm = target_dir
        .join(WASM_TARGET)
        .join("release")
        .join(format!("{package}.wasm"));
    wasm.exists()
        .then_some(wasm.clone())
        .ok_or_else(|| format!("cargo succeeded and {} is missing", wasm.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guest that imports a function is refused: a module has no clock, no
    /// files and no entropy to import.
    #[test]
    fn a_module_that_imports_anything_is_refused() {
        let wat = r#"(module (import "env" "now" (func $now (result i64))) (memory (export "memory") 1))"#;
        let bytes = wat::parse_str(wat).unwrap();
        assert!(matches!(WasmModules::load(&bytes), Err(LoadError::Abi(_))));
    }

    /// Fuel ends a runaway entry with a fault, the same on every machine.
    #[test]
    fn an_entry_that_never_returns_runs_out_of_fuel() {
        let description = abi::describe(&[]);
        let data: String = description.iter().map(|byte| format!("\\{byte:02x}")).collect();
        let wat = format!(r#"(module
            (memory (export "memory") 1)
            (data (i32.const 0) "{data}")
            (func (export "amb_alloc") (param i32) (result i32) (i32.const 1024))
            (func (export "amb_describe") (result i64)
                (i64.const {length}))
            (func (export "amb_invoke") (param i32 i32 i32 i32) (result i64)
                (loop $forever (br $forever))
                (i64.const 0)))"#, length = description.len());
        let bytes = wat::parse_str(&wat).unwrap();
        let (modules, described) = WasmModules::load_with_fuel(&bytes, 10_000).unwrap();
        assert!(described.is_empty());
        let fault = modules.invoke(0, 0, &[]).unwrap_err();
        assert!(fault.contains("fuel"), "{fault}");
    }

    /// A file whose `amb_invoke` bumps a global and a byte of memory, then
    /// returns both: a call that saw an earlier call's writes returns 2s.
    /// `global` declares the global, `grow` runs before the result is written.
    fn counting_module(global: &str, grow: bool) -> Vec<u8> {
        let description = abi::describe(&[]);
        let data: String = description.iter().map(|byte| format!("\\{byte:02x}")).collect();
        let grow = if grow { "(drop (memory.grow (i32.const 1)))" } else { "" };
        let wat = format!(r#"(module
            (memory (export "memory") 1)
            {global}
            (data (i32.const 0) "{data}")
            (func (export "amb_alloc") (param i32) (result i32) (i32.const 1024))
            (func (export "amb_describe") (result i64) (i64.const {length}))
            (func (export "amb_invoke") (param i32 i32 i32 i32) (result i64)
                (global.set $count (i32.add (global.get $count) (i32.const 1)))
                (i32.store8 (i32.const 2048) (i32.add (i32.load8_u (i32.const 2048)) (i32.const 1)))
                {grow}
                (i32.store8 (i32.const 3000) (global.get $count))
                (i32.store8 (i32.const 3001) (i32.load8_u (i32.const 2048)))
                (i32.store8 (i32.const 3002) (i32.wrap_i64 (i64.extend_i32_u (memory.size))))
                (i64.or (i64.shl (i64.const 3000) (i64.const 32)) (i64.const 3))))"#,
            length = description.len());
        wat::parse_str(&wat).unwrap()
    }

    /// A reused instance is put back to its starting image: what a call wrote
    /// to a global or to memory is gone at the next call.
    #[test]
    fn a_reused_instance_forgets_what_a_call_wrote() {
        let bytes = counting_module(r#"(global $count (export "count") (mut i32) (i32.const 0))"#, false);
        let (modules, _) = WasmModules::load(&bytes).unwrap();
        assert_eq!(modules.reuses_instances(), Ok(()));
        for _ in 0..3 {
            assert_eq!(modules.invoke(0, 0, &[]).unwrap(), vec![1, 1, 1]);
        }
        assert!(modules.held.lock().unwrap().is_some(), "the instance was kept for reuse");
    }

    /// A global the host cannot reach cannot be set back, so the file gets a
    /// new instance for every call.
    #[test]
    fn a_file_with_a_hidden_mutable_global_starts_each_call_new() {
        let bytes = counting_module("(global $count (mut i32) (i32.const 0))", false);
        let (modules, _) = WasmModules::load(&bytes).unwrap();
        assert!(modules.reuses_instances().is_err());
        for _ in 0..3 {
            assert_eq!(modules.invoke(0, 0, &[]).unwrap(), vec![1, 1, 1]);
        }
    }

    /// Memory never shrinks, so an instance a call grew is dropped and the
    /// next call sees the starting size.
    #[test]
    fn a_call_that_grows_memory_does_not_leave_it_grown() {
        let bytes = counting_module(r#"(global $count (export "count") (mut i32) (i32.const 0))"#, true);
        let (modules, _) = WasmModules::load(&bytes).unwrap();
        for _ in 0..3 {
            assert_eq!(modules.invoke(0, 0, &[]).unwrap(), vec![1, 1, 2]);
        }
    }
}

/// M1 measurement, not a check: what one call of the game's module file
/// costs, split into instantiation and the call. Needs the file built
/// (`scripts/regen/build_extension_modules.sh`).
///
/// `cargo test -p ambition_extension_wasm --release -- --ignored --nocapture m1_`
#[cfg(test)]
mod m1_measurement_tests {
    use super::*;

    #[test]
    #[ignore = "measurement: prints the per-call costs of the built module file"]
    fn m1_cost_of_one_call() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/extension-modules/wasm32-unknown-unknown/release/ambition_content_modules.wasm");
        let bytes = std::fs::read(&path).expect("build the module file first");
        let (modules, described) = WasmModules::load(&bytes).unwrap();
        let n = 300;
        let start = std::time::Instant::now();
        for _ in 0..n {
            let _ = modules.instantiate().unwrap();
        }
        let instantiate = start.elapsed().as_secs_f64() * 1e6 / n as f64;
        // A call with an input the guest refuses at decode: it pays the
        // instantiation (or the restore of a reused instance), the guest's
        // module table and the decode, and runs no entry.
        let start = std::time::Instant::now();
        for _ in 0..n {
            let _ = modules.invoke(0, 0, &[0xff]);
        }
        let call = start.elapsed().as_secs_f64() * 1e6 / n as f64;
        println!(
            "M1: {} modules; instantiate {instantiate:.1} us; a refused call {call:.1} us; reuses instances: {:?}",
            described.len(),
            modules.reuses_instances()
        );
    }
}

