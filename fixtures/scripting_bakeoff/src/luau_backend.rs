//! Luau (through `mlua`) behind the backend contract.
//!
//! One sandboxed state per module source: the chunk runs once and returns a
//! table of entry functions. Each invocation passes a fresh `inv` table.
//! The sandbox makes globals read-only, but a chunk-level `local` is an
//! upvalue that a function CAN change and that outlives the invocation: a
//! hidden state the host cannot see or roll back. The bake-off checks for it
//! (`hidden_state_survives`); a production adapter would have to refuse it
//! statically or rerun the chunk per invocation.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::{invoke_script, Backend, ScriptCall};
use ambition_extension_sdk::ModuleDescriptor;
use mlua::{Lua, LuaSerdeExt, SerializeOptions, Table};
use serde_json::Value as J;

pub struct LuauBackend {
    state: Mutex<(Lua, Table)>,
    /// The chunk's bytecode, when each invocation runs it afresh: then no
    /// chunk-level local outlives an invocation (no hidden state), at the
    /// cost of running the chunk each time.
    fresh: Option<Vec<u8>>,
    budget: Arc<AtomicU64>,
    max_operations: u64,
    module: ModuleDescriptor,
}

impl LuauBackend {
    /// Compile and run `source` (named `name` in diagnostics). A syntax error
    /// is refused here; a misspelt variable is not (Luau resolves globals at
    /// run time), unless the separate analyzer is run on the source.
    pub fn new(name: &str, source: &str, module: ModuleDescriptor, max_operations: u64) -> Result<Self, String> {
        Self::build(name, source, module, max_operations, false)
    }

    /// As [`LuauBackend::new`], with the chunk run afresh for each invocation.
    pub fn fresh(name: &str, source: &str, module: ModuleDescriptor, max_operations: u64) -> Result<Self, String> {
        Self::build(name, source, module, max_operations, true)
    }

    fn build(name: &str, source: &str, module: ModuleDescriptor, max_operations: u64, fresh: bool) -> Result<Self, String> {
        let lua = Lua::new();
        lua.sandbox(true).map_err(|e| e.to_string())?;
        let budget = Arc::new(AtomicU64::new(0));
        let spent = budget.clone();
        // Luau calls this at function calls and loop back edges: a bound on
        // the work of one invocation.
        lua.set_interrupt(move |_| {
            if spent.fetch_add(1, Ordering::Relaxed) >= max_operations {
                return Err(mlua::Error::runtime("the invocation's operation budget is spent"));
            }
            Ok(mlua::VmState::Continue)
        });
        let bytecode = mlua::Compiler::new().compile(source).map_err(|e| e.to_string())?;
        let entries: Table =
            lua.load(&bytecode[..]).set_name(format!("@{name}")).eval().map_err(|e| e.to_string())?;
        let fresh = fresh.then_some(bytecode);
        Ok(Self { state: Mutex::new((lua, entries)), fresh, budget, max_operations, module })
    }
}

impl ScriptCall for LuauBackend {
    fn call(&self, entry: &str, inv: J) -> Result<J, String> {
        let guard = self.state.lock().map_err(|_| "the Luau state is poisoned".to_owned())?;
        let (lua, entries) = &*guard;
        self.budget.store(0, Ordering::Relaxed);
        let _ = self.max_operations;
        let entries = match &self.fresh {
            Some(bytecode) => lua.load(&bytecode[..]).eval::<Table>().map_err(|e| e.to_string())?,
            None => entries.clone(),
        };
        let function: mlua::Function = entries.get(entry).map_err(|e| format!("no entry {entry}: {e}"))?;
        let options = SerializeOptions::new().serialize_none_to_null(false).serialize_unit_to_null(false);
        let value = lua.to_value_with(&inv, options).map_err(|e| e.to_string())?;
        function.call::<()>(value.clone()).map_err(|e| e.to_string())?;
        lua.from_value::<J>(value).map_err(|e| e.to_string())
    }
}

impl Backend for LuauBackend {
    fn invoke(&self, _module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        invoke_script(&self.module, entry, input, self)
    }
}
