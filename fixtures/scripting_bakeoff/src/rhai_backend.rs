//! Rhai behind the backend contract.
//!
//! One `Engine` and one compiled `AST` per module source, shared and
//! immutable. Each invocation binds a fresh `inv` value as `this` and a fresh
//! `Scope`: nothing a script does outlives its invocation except what it
//! writes into `inv.state`. `f32_float` makes Rhai's number the engine's
//! `f32`, so the arithmetic is the native module's.

use crate::{invoke_script, Backend, ScriptCall};
use ambition_extension_sdk::ModuleDescriptor;
use serde_json::Value as J;

pub struct RhaiBackend {
    engine: rhai::Engine,
    ast: rhai::AST,
    module: ModuleDescriptor,
}

impl RhaiBackend {
    /// Compile `source` (named `name` in diagnostics) for `module`. A syntax
    /// error, or a variable no scope declares, is refused here, before any
    /// invocation.
    pub fn new(name: &str, source: &str, module: ModuleDescriptor, max_operations: u64) -> Result<Self, String> {
        let mut engine = rhai::Engine::new();
        engine.set_max_operations(max_operations);
        engine.set_max_call_levels(32);
        engine.set_max_expr_depths(64, 64);
        // A read of a variable no scope declares is a compile error.
        engine.set_strict_variables(true);
        // A read of a field a table does not have is an error, not `()`.
        engine.set_fail_on_invalid_map_property(true);
        exact_float_comparisons(&mut engine);
        let mut ast = engine.compile(source).map_err(|e| format!("{name}: {e}"))?;
        ast.set_source(name);
        Ok(Self { engine, ast, module })
    }
}

impl ScriptCall for RhaiBackend {
    fn call(&self, entry: &str, inv: J) -> Result<J, String> {
        let mut this = rhai::serde::to_dynamic(&inv).map_err(|e| e.to_string())?;
        let options = rhai::CallFnOptions::new().eval_ast(true).bind_this_ptr(&mut this);
        let _: rhai::Dynamic = self.engine
            .call_fn_with_options::<rhai::Dynamic>(options, &mut rhai::Scope::new(), &self.ast, entry, ())
            .map_err(|e| e.to_string())?;
        rhai::serde::from_dynamic::<J>(&this).map_err(|e| e.to_string())
    }
}

impl Backend for RhaiBackend {
    fn invoke(&self, _module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        invoke_script(&self.module, entry, input, self)
    }
}

/// Rhai's built-in float comparisons are tolerant: within `f32::EPSILON`,
/// absolute when either side is zero, relative otherwise (`0.1 + 0.2 == 0.3`
/// is true; `1.1e-8 > 0.0` is false). The native module compares exactly, so
/// a cooldown of 1.1e-8 fired a tick early (bake-off, tick 15). The
/// `unchecked` feature compares exactly but also drops the operation budget;
/// instead the built-in fast operators are turned off and exact IEEE
/// comparisons registered for every float pairing.
pub fn exact_float_comparisons(engine: &mut rhai::Engine) {
    use rhai::{FLOAT, INT};
    engine.set_fast_operators(false);
    macro_rules! exact {
        ($op:literal, $f:expr) => {
            engine.register_fn($op, |a: FLOAT, b: FLOAT| -> bool { $f(a, b) });
            engine.register_fn($op, |a: FLOAT, b: INT| -> bool { $f(a, b as FLOAT) });
            engine.register_fn($op, |a: INT, b: FLOAT| -> bool { $f(a as FLOAT, b) });
        };
    }
    exact!("==", |a: FLOAT, b: FLOAT| a == b);
    exact!("!=", |a: FLOAT, b: FLOAT| a != b);
    exact!(">", |a: FLOAT, b: FLOAT| a > b);
    exact!(">=", |a: FLOAT, b: FLOAT| a >= b);
    exact!("<", |a: FLOAT, b: FLOAT| a < b);
    exact!("<=", |a: FLOAT, b: FLOAT| a <= b);
}
