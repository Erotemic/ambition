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
//! * Each call gets a NEW instance. No guest global, heap or table survives an
//!   invocation; everything that lasts is in the records the host passes in
//!   and takes back (extension-state-and-execution: "VM memory is scratch").
//! * Work is metered by FUEL, a count of executed instructions. A call that
//!   runs out faults the same way on every machine; load never decides it.
//! * The interpreter canonicalizes NaN results (`wasmi/deterministic`).
//!
//! ⚠ Not a security sandbox claim beyond what the interpreter gives. And a
//! module's floating-point transcendentals (`sin`, `atan2`) are the guest's
//! own compiled code, so they can differ from the native host's in the last
//! bit; a native module and its WASM build are two executables, not one.

use std::sync::Arc;

use ambition_extension_host::ModuleBackend;
use ambition_extension_sdk::abi;
use ambition_extension_sdk::digest::digest_bytes;
use ambition_extension_sdk::{CodeIdentity, ModuleDescriptor, Name};
use wasmi::{Config, Engine, Instance, Linker, Memory, Module, Store};

/// The work one invocation may do, in fuel units (about one per executed
/// instruction). Generous: a fan of seven shots costs a few thousand.
pub const DEFAULT_FUEL: u64 = 20_000_000;

/// Why a `.wasm` file could not become modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The bytes are not a valid module.
    Compile(String),
    /// The module does not have the `ambition-ext-1` exports, or imports
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
        let this = Arc::new(Self {
            engine,
            module,
            fuel,
        });
        let description = this.describe().map_err(LoadError::Describe)?;
        let mut modules = abi::read_description(&description)
            .map_err(|e| LoadError::Describe(format!("the description does not decode: {e}")))?;
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

fn trap(e: wasmi::Error) -> String {
    match e.as_trap_code() {
        Some(code) => format!("trap: {code}"),
        None => e.to_string(),
    }
}

impl ModuleBackend for WasmModules {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String> {
        let (mut store, instance, memory) = self.instantiate()?;
        let alloc = instance
            .get_typed_func::<u32, u32>(&store, "amb_alloc")
            .map_err(|e| format!("amb_alloc: {e}"))?;
        let invoke = instance
            .get_typed_func::<(u32, u32, u32, u32), u64>(&store, "amb_invoke")
            .map_err(|e| format!("amb_invoke: {e}"))?;
        let len = u32::try_from(input.len()).map_err(|_| "input too long".to_owned())?;
        let ptr = alloc.call(&mut store, len).map_err(trap)?;
        memory
            .write(&mut store, ptr as usize, input)
            .map_err(|e| format!("input out of bounds: {e}"))?;
        let packed = invoke
            .call(&mut store, (module, entry, ptr, len))
            .map_err(trap)?;
        Self::read_packed(&store, memory, packed)
    }
}

/// The target a loaded module is built for.
pub const WASM_TARGET: &str = "wasm32-unknown-unknown";

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
        let wat = r#"(module
            (memory (export "memory") 1)
            (func (export "amb_alloc") (param i32) (result i32) (i32.const 1024))
            (func (export "amb_describe") (result i64)
                ;; zero modules: four zero bytes at offset 0
                (i64.const 4))
            (func (export "amb_invoke") (param i32 i32 i32 i32) (result i64)
                (loop $forever (br $forever))
                (i64.const 0)))"#;
        let bytes = wat::parse_str(wat).unwrap();
        let (modules, described) = WasmModules::load_with_fuel(&bytes, 10_000).unwrap();
        assert!(described.is_empty());
        let fault = modules.invoke(0, 0, &[]).unwrap_err();
        assert!(fault.contains("fuel"), "{fault}");
    }
}
