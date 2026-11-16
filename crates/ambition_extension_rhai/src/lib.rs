//! The Rhai backend of the extension host.
//!
//! A script supplies the entries of a module the game already declares: the
//! module's descriptor (its entries, phases, ports, schemas and limits) is
//! the declared one, and the script gives the code. An entry the script
//! defines (a function named like the entry, with no parameters) runs from
//! the script; an entry it does not define runs the module's native code, so
//! a module can move to script one entry at a time.
//!
//! Or a script declares a module of its own, which the game does not link:
//! its `fn declare()` returns the declaration ([`declare`]), and every entry
//! is the script's.
//!
//! An entry runs with the invocation bound as `this`:
//!
//! ```text
//! fn turret() {
//!     let c = this.state.cadence;
//!     c.cooldown_s -= this.dt;
//!     ...
//!     this.requests.push(#{ port: "sound", value: #{ cue: "world.rock.hit", at: this.trigger.position } });
//!     this.state.cadence = c;
//! }
//! ```
//!
//! (`inv` fields: [`ambition_extension_sdk::script`].) The SDK decodes and
//! encodes the invocation exactly as for a compiled guest, so every check a
//! compiled module meets applies; a fault stages nothing.
//!
//! What the engine is configured to guarantee:
//!
//! - **The engine's numbers.** `f32_float`; and float comparisons are exact
//!   ([`exact_float_comparisons`]): Rhai's built-ins compare within
//!   `f32::EPSILON`, which fired the sentry a tick early in the bake-off.
//! - **No hidden state or input.** A Rhai function cannot see a mutable
//!   global, each invocation gets a fresh scope and a fresh `this`, there is
//!   no clock (`no_time`) and no import from files.
//! - **Bounded work.** An operation budget, call depth, expression depth and
//!   data sizes; a script that exceeds one faults.
//! - **Early refusal.** A syntax error or an undeclared variable refuses the
//!   file when it loads (`strict_variables`); reading a field a table lacks
//!   is an error, not `()`.

use std::sync::Arc;

pub mod declare;

use ambition_extension_host::ModuleBackend;
use ambition_extension_sdk::script::{ScriptPorts, ScriptValue, invoke_script};
use ambition_extension_sdk::{EntryCode, ModuleDescriptor, ModuleKey};
use rhai::{AST, Dynamic, Engine, FLOAT, INT};

/// The code identity's ABI: the language and the engine configuration that
/// gives a script its meaning. A change to the configuration (the number
/// type, the comparisons, the binding) is a new ABI name.
pub const ABI: &str = "rhai-f32-exact/1";

/// The operations one invocation may spend.
pub const DEFAULT_MAX_OPERATIONS: u64 = 200_000;

struct ScriptedModule {
    /// The declared module, with its native code: an entry the script does
    /// not define runs it.
    native: ModuleDescriptor,
    ast: AST,
    /// For each entry, whether the script defines it.
    scripted: Vec<bool>,
}

/// One script file's modules, ready to run.
pub struct RhaiModules {
    engine: Engine,
    modules: Vec<ScriptedModule>,
    keys: Vec<ModuleKey>,
    ports: Arc<ScriptPorts>,
    /// The file, for diagnostics.
    source: Arc<str>,
}

/// The configured engine (see the crate documentation).
pub fn engine(max_operations: u64) -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(max_operations);
    engine.set_max_call_levels(32);
    engine.set_max_expr_depths(64, 64);
    engine.set_max_string_size(4096);
    engine.set_max_array_size(1024);
    engine.set_max_map_size(256);
    engine.set_strict_variables(true);
    engine.set_fail_on_invalid_map_property(true);
    engine.set_module_resolver(rhai::module_resolvers::DummyModuleResolver::new());
    exact_float_comparisons(&mut engine);
    engine
}

/// `request(alias, value)`: the request `#{ port: alias, value }` with its
/// value checked against the port's description and its defaults filled, for
/// `this.requests.push(..)`. A misspelt field, a wrong kind, a port that is
/// not a request, or no such port is an error AT THE CALL, so it names the
/// script's line and column. (A plain map pushed to `this.requests` is
/// checked too, but only after the entry returns, without a position.)
fn register_request(engine: &mut Engine, ports: Arc<ScriptPorts>) {
    engine.register_fn(
        "request",
        move |alias: &str, value: rhai::Map| -> Result<rhai::Map, Box<rhai::EvalAltResult>> {
            let entry = ports.by_alias(alias).ok_or_else(|| {
                let names: Vec<&str> = ports
                    .entries()
                    .iter()
                    .filter(|e| e.role == ambition_extension_sdk::PortRole::Request)
                    .map(|e| e.alias.as_ref())
                    .collect();
                format!("request: no port named `{alias}` (the request ports are {names:?})")
            })?;
            if entry.role != ambition_extension_sdk::PortRole::Request {
                return Err(format!("request: `{alias}` is not a request port").into());
            }
            let checked = from_dynamic(&Dynamic::from_map(value))
                .and_then(|v| (entry.check)(&v))
                .map_err(|e| format!("request `{alias}`: {e}"))?;
            let mut request = rhai::Map::new();
            request.insert("port".into(), Dynamic::from(alias.to_owned()));
            request.insert("value".into(), to_dynamic(&checked));
            Ok(request)
        },
    );
}

/// Rhai's built-in float comparisons are tolerant: within `f32::EPSILON`,
/// absolute when either side is zero, relative otherwise (`1.1e-8 > 0.0` is
/// false). The engine compares exactly, and so must its scripts. The
/// `unchecked` feature would compare exactly but also drop the operation
/// budget; instead the fast built-in operators are turned off and exact
/// comparisons registered for every float pairing.
pub fn exact_float_comparisons(engine: &mut Engine) {
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

/// A compiled source and the bindings used to compile it.
struct CompiledScript {
    engine: Engine,
    ast: AST,
    ports: Arc<ScriptPorts>,
    source: Arc<str>,
    digest: u64,
}

impl CompiledScript {
    fn new(
        source: &str,
        text: &str,
        ports: ScriptPorts,
        max_operations: u64,
    ) -> Result<Self, String> {
        let ports = Arc::new(ports);
        let mut engine = engine(max_operations);
        register_request(&mut engine, ports.clone());
        let mut ast = engine.compile(text).map_err(|e| format!("{source}: {e}"))?;
        ast.set_source(source);
        Ok(Self {
            engine,
            ast,
            ports,
            source: Arc::from(source),
            digest: ambition_extension_sdk::digest::digest_bytes(text.as_bytes()),
        })
    }

    fn declaration(&self) -> Result<ModuleDescriptor, String> {
        if !declares_a_module(&self.ast) {
            return Err(format!(
                "{}: declares no module (no `fn declare()`)",
                self.source
            ));
        }
        let table: Dynamic = self
            .engine
            .call_fn(&mut rhai::Scope::new(), &self.ast, "declare", ())
            .map_err(|e| format!("{}: declare(): {e}", self.source))?;
        declare::descriptor(&table, &self.ports).map_err(|e| format!("{}: {e}", self.source))
    }

    fn bind(
        self,
        native: ModuleDescriptor,
        require_all: bool,
    ) -> Result<(Arc<RhaiModules>, Vec<ModuleDescriptor>), String> {
        let scripted: Vec<bool> = native
            .entries
            .iter()
            .map(|entry| {
                self.ast
                    .iter_functions()
                    .any(|f| f.name == entry.key.as_ref() && f.params.is_empty())
            })
            .collect();
        if require_all {
            if let Some((missing, _)) = native
                .entries
                .iter()
                .zip(&scripted)
                .find(|(_, scripted)| !**scripted)
            {
                return Err(format!(
                    "{}: declares entry \"{}\" and defines no function `{}()` for it",
                    self.source, missing.key, missing.key
                ));
            }
        }
        if !scripted.iter().any(|s| *s) {
            let names: Vec<&str> = native.entries.iter().map(|e| e.key.as_ref()).collect();
            return Err(format!(
                "{}: defines none of the entries of module {} (functions with no parameters named {names:?})",
                self.source, native.key
            ));
        }
        let mut declared = native.clone();
        declared.code = ambition_extension_sdk::CodeIdentity::Loaded {
            abi: ABI.into(),
            digest: self.digest,
        };
        for (index, entry) in declared.entries.iter_mut().enumerate() {
            entry.run = EntryCode::Loaded {
                index: index as u32,
            };
        }
        let keys = vec![native.key.clone()];
        let backend = Arc::new(RhaiModules {
            engine: self.engine,
            modules: vec![ScriptedModule {
                native,
                ast: self.ast,
                scripted,
            }],
            keys,
            ports: self.ports,
            source: Arc::from(format!("{} (generation {:016x})", self.source, self.digest)),
        });
        Ok((backend, vec![declared]))
    }
}

impl RhaiModules {
    /// Compile `text` as the code of `native`. Entries absent from the script
    /// keep their native code. A syntax error names `source` and its position.
    pub fn load(
        source: &str,
        text: &str,
        native: ModuleDescriptor,
        ports: ScriptPorts,
        max_operations: u64,
    ) -> Result<(Arc<Self>, Vec<ModuleDescriptor>), String> {
        CompiledScript::new(source, text, ports, max_operations)?.bind(native, false)
    }

    /// Compile a script that declares its own module through `fn declare()`.
    /// Every declared entry must have a function in the script.
    pub fn load_declared(
        source: &str,
        text: &str,
        ports: ScriptPorts,
        max_operations: u64,
    ) -> Result<(Arc<Self>, Vec<ModuleDescriptor>), String> {
        let compiled = CompiledScript::new(source, text, ports, max_operations)?;
        let declared = compiled.declaration()?;
        compiled.bind(declared, true)
    }

    /// Compile one file. Its declaration supplies the module when present.
    /// Otherwise, `stem` selects a linked module. Compilation precedes lookup
    /// so invalid source always reports its syntax error and position.
    pub fn load_file(
        source: &str,
        text: &str,
        stem: &str,
        linked: &[ambition_extension_host::DeclaredModule],
        ports: ScriptPorts,
        max_operations: u64,
    ) -> Result<(Arc<Self>, Vec<ModuleDescriptor>), String> {
        let compiled = CompiledScript::new(source, text, ports, max_operations)?;
        let own_declaration = declares_a_module(&compiled.ast);
        let declared = if own_declaration {
            compiled.declaration()?
        } else {
            module_for_file(stem, linked)?.clone()
        };
        compiled.bind(declared, own_declaration)
    }

    /// Whether entry `entry` of module `module` runs from the script.
    pub fn is_scripted(&self, module: u32, entry: u32) -> bool {
        self.modules
            .get(module as usize)
            .and_then(|m| m.scripted.get(entry as usize))
            .copied()
            .unwrap_or(false)
    }
}

impl ModuleBackend for RhaiModules {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Result<Vec<u8>, String> {
        let m = self
            .modules
            .get(module as usize)
            .ok_or_else(|| format!("no module {module}"))?;
        if !m.scripted.get(entry as usize).copied().unwrap_or(false) {
            return Ok(ambition_extension_sdk::abi::invoke_one(
                &m.native, entry, input,
            ));
        }
        Ok(invoke_script(
            &m.native,
            entry,
            input,
            self.ports.as_ref(),
            |key, inv| {
                let mut this = to_dynamic(&inv);
                let options = rhai::CallFnOptions::new()
                    .eval_ast(true)
                    .bind_this_ptr(&mut this);
                let _: Dynamic = self
                    .engine
                    .call_fn_with_options(options, &mut rhai::Scope::new(), &m.ast, key, ())
                    .map_err(|e| format!("{}: {e}", self.source))?;
                from_dynamic(&this).map_err(|e| format!("{}: inv: {e}", self.source))
            },
        ))
    }

    fn module_index(&self, key: &ModuleKey) -> Option<u32> {
        self.keys
            .iter()
            .position(|known| known == key)
            .map(|index| index as u32)
    }
}

/// Whether the compiled script declares its own module (`fn declare()`).
pub fn declares_a_module(ast: &AST) -> bool {
    ast.iter_functions()
        .any(|f| f.name == "declare" && f.params.is_empty())
}

/// A script value as a Rhai value.
pub fn to_dynamic(value: &ScriptValue) -> Dynamic {
    match value {
        ScriptValue::Unit => Dynamic::UNIT,
        ScriptValue::Bool(v) => Dynamic::from_bool(*v),
        ScriptValue::Int(v) => Dynamic::from_int(*v as INT),
        ScriptValue::Num(v) => Dynamic::from_float(*v as FLOAT),
        ScriptValue::Text(v) => Dynamic::from(v.clone()),
        ScriptValue::List(items) => Dynamic::from_array(items.iter().map(to_dynamic).collect()),
        ScriptValue::Map(fields) => Dynamic::from_map(
            fields
                .iter()
                .map(|(k, v)| (k.as_str().into(), to_dynamic(v)))
                .collect(),
        ),
    }
}

/// A Rhai value as a script value. A value of another type (a function
/// pointer, a custom type) is an error naming it.
pub fn from_dynamic(value: &Dynamic) -> Result<ScriptValue, String> {
    if value.is_unit() {
        return Ok(ScriptValue::Unit);
    }
    if let Ok(v) = value.as_bool() {
        return Ok(ScriptValue::Bool(v));
    }
    if let Ok(v) = value.as_int() {
        return Ok(ScriptValue::Int(v as i64));
    }
    if let Ok(v) = value.as_float() {
        return Ok(ScriptValue::Num(v as f32));
    }
    if value.is_string() {
        return Ok(ScriptValue::Text(
            value
                .clone()
                .into_string()
                .map_err(|t| format!("text: {t}"))?,
        ));
    }
    if value.is_array() {
        let items = value
            .clone()
            .into_array()
            .map_err(|t| format!("list: {t}"))?;
        return Ok(ScriptValue::List(
            items.iter().map(from_dynamic).collect::<Result<_, _>>()?,
        ));
    }
    if value.is_map() {
        let map = value.clone().cast::<rhai::Map>();
        return Ok(ScriptValue::Map(
            map.iter()
                .map(|(k, v)| Ok((k.to_string(), from_dynamic(v)?)))
                .collect::<Result<_, String>>()?,
        ));
    }
    Err(format!(
        "a value of type {} cannot leave a script",
        value.type_name()
    ))
}

/// The linked module a script file supplies code for: the file's stem names
/// it, by key (`sentry.rhai`) or by provider and key (`ambition.sentry.rhai`).
/// No match, or a key two providers share, is an error naming the choices.
pub fn module_for_file<'a>(
    stem: &str,
    linked: &'a [ambition_extension_host::DeclaredModule],
) -> Result<&'a ModuleDescriptor, String> {
    let matches: Vec<&ModuleDescriptor> = linked
        .iter()
        .map(|m| &m.descriptor)
        .filter(|d| {
            stem == d.key.key.as_ref() || stem == format!("{}.{}", d.key.provider, d.key.key)
        })
        .collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => {
            let mut names: Vec<String> = linked
                .iter()
                .map(|m| m.descriptor.key.key.to_string())
                .collect();
            names.sort();
            names.dedup();
            Err(format!(
                "the game links no module named {stem:?}; it links {names:?}"
            ))
        }
        many => Err(format!(
            "{stem:?} names {} linked modules ({:?}): name the file <provider>.<key>",
            many.len(),
            many.iter().map(|d| d.key.to_string()).collect::<Vec<_>>()
        )),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn invalid_source_reports_its_position_before_native_lookup() {
        let error = super::RhaiModules::load_file(
            "new_mechanic.rhai",
            "fn declare() {\n    let value = ;\n}",
            "new_mechanic",
            &[],
            Default::default(),
            super::DEFAULT_MAX_OPERATIONS,
        )
        .err()
        .expect("the invalid source is refused");
        assert!(
            error.contains("new_mechanic.rhai") && error.contains("line 2"),
            "{error}"
        );
        assert!(!error.contains("links no module"), "{error}");
    }
}
