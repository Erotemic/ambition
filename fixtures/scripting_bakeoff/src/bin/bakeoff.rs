//! Runs the bake-off and prints what it measured.
//!
//! 1. Equal outputs: one stream of turret invocations (an enemy coming and
//!    going across the range, pauses, a slow tick, an enemy on the turret)
//!    through the native sentry, Rhai and Luau, each carrying its own state
//!    forward; every output compared byte for byte with the native one.
//! 2. Cost: nanoseconds per invocation for each, the same stream repeated.
//! 3. Diagnostics: broken scripts, each language's own message.
//! 4. Budget: an endless loop stops at the operation budget.
//! 5. Hidden state: can a script keep state the host does not hold (so a
//!    replay of one invocation gives another answer)?

use std::time::Instant;

use ambition_combat_port::{ModuleEntityTick, ModuleEntityTickPort};
use ambition_extension_sdk::abi::{read_output, InvocationInput, InvocationOutput};
use ambition_extension_sdk::schema::{Record, SchemaKey};
use ambition_extension_sdk::{ModuleDescriptor, Port};
use scripting_bakeoff::luau_backend::LuauBackend;
use scripting_bakeoff::rhai_backend::RhaiBackend;
use scripting_bakeoff::Backend;

const BUDGET: u64 = 100_000;

struct Native(ModuleDescriptor);

/// The engine's own Rhai backend (`ambition_extension_rhai`), with the ports
/// reachable through their owners' `ScriptPort` forms.
struct Engine(std::sync::Arc<ambition_extension_rhai::RhaiModules>);

impl Backend for Engine {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        use ambition_extension_host::ModuleBackend;
        self.0.invoke(module, entry, input).expect("the backend runs")
    }
}

/// M1's native shared-library arm: the same sentry built as a cdylib behind
/// a trusted C entry table (`native_sentry`). The host owns the output
/// buffer and grows it once; the library is kept loaded as long as the
/// backend, which pins the code a call runs.
struct NativeLibrary {
    _library: libloading::Library,
    invoke: NativeInvoke,
    out: std::cell::RefCell<Vec<u8>>,
}

type NativeInvoke = unsafe extern "C" fn(u32, u32, *const u8, u64, *mut u8, u64) -> u64;

impl NativeLibrary {
    fn load(path: &str) -> Result<Self, String> {
        // SAFETY: a trusted library this repository builds; its entry table
        // is checked by version before any entry is called.
        unsafe {
            let library = libloading::Library::new(path).map_err(|error| format!("{path}: {error}"))?;
            let abi = library
                .get::<extern "C" fn() -> u32>(b"amb_native_abi")
                .map_err(|error| error.to_string())?;
            if abi() != 1 {
                return Err(format!("{path} has entry table version {}, not 1", abi()));
            }
            let invoke = *library.get::<NativeInvoke>(b"amb_native_invoke").map_err(|error| error.to_string())?;
            Ok(Self { _library: library, invoke, out: std::cell::RefCell::new(vec![0; 4096]) })
        }
    }
}

impl Backend for NativeLibrary {
    fn invoke(&self, module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        let mut out = self.out.borrow_mut();
        loop {
            // SAFETY: both buffers are valid for the lengths passed.
            let len = unsafe {
                (self.invoke)(module, entry, input.as_ptr(), input.len() as u64, out.as_mut_ptr(), out.len() as u64)
            };
            assert_ne!(len, u64::MAX, "the native library faulted");
            if len as usize <= out.len() {
                return out[..len as usize].to_vec();
            }
            out.resize(len as usize, 0);
        }
    }
}

/// The game's loaded WebAssembly backend (wasmi, fuel, a new instance per
/// call) running the game's own module file.
struct Wasm(std::sync::Arc<ambition_extension_wasm::WasmModules>, u32);

impl Backend for Wasm {
    fn invoke(&self, _module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        use ambition_extension_host::ModuleBackend;
        self.0.invoke(self.1, entry, input).expect("the wasm module runs")
    }
}

/// The binding alone: the script gives `inv` back unchanged. Its cost is the
/// conversions every script backend here pays, not the language's.
struct Identity(ModuleDescriptor);

impl scripting_bakeoff::ScriptCall for Identity {
    fn call(&self, _entry: &str, inv: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(inv)
    }
}

impl Backend for Identity {
    fn invoke(&self, _module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        scripting_bakeoff::invoke_script(&self.0, entry, input, self)
    }
}

impl Backend for Native {
    fn invoke(&self, _module: u32, entry: u32, input: &[u8]) -> Vec<u8> {
        ambition_extension_sdk::abi::invoke_one(&self.0, entry, input)
    }
}

/// The turret's tick `i`: deterministic, and covering each branch.
fn tick(i: u32) -> (f32, ModuleEntityTick) {
    let dt = if i % 97 == 0 { 0.0 } else if i % 131 == 0 { 1.0 / 30.0 } else { 1.0 / 60.0 };
    let at = [100.0 + (i % 50) as f32 * 0.37, 200.0];
    let enemy = match i % 300 {
        0..=39 => None,
        // On the turret: no direction to fire in.
        40..=44 => Some(at),
        k => {
            let angle = k as f32 * 0.071;
            // In range and out of it: 120 to 760.
            let distance = 120.0 + (k as f32 * 37.0) % 640.0;
            Some([at[0] + distance * angle.cos(), at[1] + distance * angle.sin()])
        }
    };
    (dt, ModuleEntityTick {
        position: at,
        remaining_s: 4.0,
        nearest_enemy: enemy,
        params: Vec::new(),
        touchers: Vec::new(),
    })
}

fn input(i: u32, state: &[(SchemaKey, Record)]) -> Vec<u8> {
    let (dt, value) = tick(i);
    let mut trigger = Vec::new();
    ModuleEntityTickPort::encode(&value, &mut trigger);
    InvocationInput {
        clock: ambition_extension_sdk::InvocationClock::Simulation {tick: i as u64,dt},
        occurrence: None,
        trigger_port: &ModuleEntityTickPort::KEY,
        trigger: &trigger,
        observations: &[],
        state,
    }
    .encode()
}

fn decode(module: &ModuleDescriptor, bytes: &[u8]) -> InvocationOutput {
    read_output(bytes, |key| module.schemas.iter().find(|s| &s.key == key)).expect("the output decodes")
}

fn turret_entry(module: &ModuleDescriptor) -> u32 {
    module.entries.iter().position(|e| e.key == "turret").expect("the sentry has a turret entry") as u32
}

fn initial_state(module: &ModuleDescriptor) -> Vec<(SchemaKey, Record)> {
    let entry = &module.entries[turret_entry(module) as usize];
    module
        .schemas
        .iter()
        .filter(|s| entry.writes.contains(&s.key))
        .map(|s| (s.key.clone(), s.initial_record()))
        .collect()
}

/// Run `ticks` invocations through `backend`, carrying its state; every
/// output.
fn run(module: &ModuleDescriptor, backend: &dyn Backend, ticks: u32) -> Vec<Vec<u8>> {
    let entry = turret_entry(module);
    let mut state = initial_state(module);
    let mut outputs = Vec::new();
    for i in 0..ticks {
        let out = backend.invoke(0, entry, &input(i, &state));
        match decode(module, &out) {
            InvocationOutput::Ok { state: next, .. } => state = next,
            InvocationOutput::Fault(message) => panic!("tick {i}: fault: {message}"),
        }
        outputs.push(out);
    }
    outputs
}

fn compare(name: &str, module: &ModuleDescriptor, native: &[Vec<u8>], other: &[Vec<u8>]) {
    let differing: Vec<usize> = (0..native.len()).filter(|&i| native[i] != other[i]).collect();
    let fired = native
        .iter()
        .filter(|out| matches!(decode(module, out), InvocationOutput::Ok { requests, .. } if !requests.is_empty()))
        .count();
    println!(
        "{name}: {} of {} outputs byte-equal to native ({} ticks fired)",
        native.len() - differing.len(),
        native.len(),
        fired
    );
    if let Some(&first) = differing.first() {
        println!("  first difference at tick {first}:");
        println!("    native {:?}", decode(module, &native[first]));
        println!("    {name:>6} {:?}", decode(module, &other[first]));
    }
}

fn time(module: &ModuleDescriptor, backend: &dyn Backend, ticks: u32, rounds: u32) -> f64 {
    let entry = turret_entry(module);
    let inputs: Vec<Vec<u8>> = {
        let mut state = initial_state(module);
        (0..ticks)
            .map(|i| {
                let bytes = input(i, &state);
                if let InvocationOutput::Ok { state: next, .. } = decode(module, &backend.invoke(0, entry, &bytes)) {
                    state = next;
                }
                bytes
            })
            .collect()
    };
    let start = Instant::now();
    for _ in 0..rounds {
        for bytes in &inputs {
            std::hint::black_box(backend.invoke(0, entry, bytes));
        }
    }
    start.elapsed().as_nanos() as f64 / f64::from(ticks * rounds)
}

/// The ports the engine backend reaches, by the names a script uses.
fn ports() -> ambition_extension_sdk::script::ScriptPorts {
    let mut ports = ambition_extension_sdk::script::ScriptPorts::default();
    ports
        .add::<ModuleEntityTickPort>("module_entity_tick")
        .add::<ambition_projectile_spec::ProjectileSpawnPort>("projectile")
        .add::<ambition_combat_port::BodySoundPort>("sound");
    ports
}

/// One broken script on the engine backend: load it, then run ticks until
/// the first fault (a request mistake shows when the turret first fires).
fn engine_diagnose(module: &ModuleDescriptor, what: &str, source: &str) {
    println!("\n-- {what}");
    let backend = match ambition_extension_rhai::RhaiModules::load(
        "broken.rhai", source, module.clone(), ports(), ambition_extension_rhai::DEFAULT_MAX_OPERATIONS,
    ) {
        Err(e) => return println!("  engine: refused at load: {e}"),
        Ok((backend, _)) => Engine(backend),
    };
    let entry = turret_entry(module);
    let mut state = initial_state(module);
    for i in 0..2000 {
        match decode(module, &backend.invoke(0, entry, &input(i, &state))) {
            InvocationOutput::Fault(e) => return println!("  engine: fault at tick {i}: {e}"),
            InvocationOutput::Ok { state: next, .. } => state = next,
        }
    }
    println!("  engine: ran 2000 ticks without complaint");
}

/// One broken script in each language: compile, then one invocation.
fn diagnose(module: &ModuleDescriptor, what: &str, rhai: &str, luau: &str) {
    println!("\n-- {what}");
    let entry = turret_entry(module);
    let bytes = input(300 + 60, &initial_state(module));
    let show = |lang: &str, built: Result<Box<dyn Backend>, String>| match built {
        Err(e) => println!("  {lang}: refused at compile: {e}"),
        Ok(backend) => match decode(module, &backend.invoke(0, entry, &bytes)) {
            InvocationOutput::Fault(e) => println!("  {lang}: fault at run time: {e}"),
            InvocationOutput::Ok { .. } => println!("  {lang}: ran without complaint"),
        },
    };
    show("rhai", RhaiBackend::new("broken.rhai", rhai, module.clone(), BUDGET).map(|b| Box::new(b) as Box<dyn Backend>));
    show("luau", LuauBackend::new("broken.luau", luau, module.clone(), BUDGET).map(|b| Box::new(b) as Box<dyn Backend>));
}

fn main() {
    let module = ambition_content_modules::sentry::module();
    // The bake-off's own Rhai sentry, as it was measured; the engine row runs
    // the game's shipped script, which names its requests through `request`.
    let rhai_source = include_str!("../../modules/sentry.rhai");
    let engine_source = include_str!("../../../../game/ambition_content_modules/rhai/sentry.rhai");
    let luau_source = include_str!("../../modules/sentry.luau");
    let native = Native(module.clone());
    let rhai = RhaiBackend::new("sentry.rhai", rhai_source, module.clone(), BUDGET).expect("sentry.rhai compiles");
    let luau = LuauBackend::new("sentry.luau", luau_source, module.clone(), BUDGET).expect("sentry.luau compiles");
    let luau_fresh = LuauBackend::fresh("sentry.luau", luau_source, module.clone(), BUDGET).expect("sentry.luau compiles");
    let identity = Identity(module.clone());
    let (engine_rhai, _) = ambition_extension_rhai::RhaiModules::load(
        "sentry.rhai", engine_source, module.clone(), ports(), ambition_extension_rhai::DEFAULT_MAX_OPERATIONS,
    )
    .expect("sentry.rhai loads in the engine backend");
    let engine_rhai = Engine(engine_rhai);
    // Each language with an entry that does nothing: its conversions and call
    // overhead, without the turret's work.
    let rhai_empty = RhaiBackend::new("empty.rhai", "fn turret() { }", module.clone(), BUDGET).expect("compiles");
    let luau_empty = LuauBackend::new("empty.luau", "return { turret = function(inv) end }", module.clone(), BUDGET)
        .expect("compiles");

    let library_path = std::env::var("NATIVE_SENTRY").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/native_sentry/target/release/libnative_sentry.so").to_string()
    });
    let library = NativeLibrary::load(&library_path)
        .map_err(|error| println!("(native library arm skipped: {error}; build native_sentry --release)"))
        .ok();
    let wasm_path = std::env::var("SENTRY_WASM").unwrap_or_else(|_| {
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/extension-modules/wasm32-unknown-unknown/release/ambition_content_modules.wasm")
            .to_string()
    });
    let wasm = std::fs::read(&wasm_path)
        .map_err(|error| error.to_string())
        .and_then(|bytes| ambition_extension_wasm::WasmModules::load(&bytes).map_err(|error| format!("{error:?}")))
        .map(|(backend, _)| {
            use ambition_extension_host::ModuleBackend;
            let index = backend.module_index(&module.key).expect("the module file holds the sentry");
            Wasm(backend, index)
        })
        .map_err(|error| println!("(wasm arm skipped: {wasm_path}: {error}; run scripts/regen/build_extension_modules.sh)"))
        .ok();

    println!("== 1. equal outputs (2000 ticks, each backend carrying its own state)");
    let reference = run(&module, &native, 2000);
    if let Some(library) = &library {
        compare("native shared library", &module, &reference, &run(&module, library, 2000));
    }
    if let Some(wasm) = &wasm {
        compare("wasm (wasmi)", &module, &reference, &run(&module, wasm, 2000));
    }
    compare("rhai", &module, &reference, &run(&module, &rhai, 2000));
    compare("luau", &module, &reference, &run(&module, &luau, 2000));
    compare("engine rhai backend", &module, &reference, &run(&module, &engine_rhai, 2000));
    compare("luau (fresh chunk)", &module, &reference, &run(&module, &luau_fresh, 2000));

    println!("\n== 2. cost per invocation (2000 ticks x 10 rounds, decode + run + encode)");
    for (name, backend) in [
        ("native", &native as &dyn Backend),
        ("binding only (identity script)", &identity),
        ("rhai, empty entry", &rhai_empty),
        ("rhai", &rhai),
        ("engine rhai backend", &engine_rhai),
        ("luau, empty entry", &luau_empty),
        ("luau", &luau),
        ("luau, fresh chunk each call", &luau_fresh),
    ] {
        println!("  {name:>30}: {:>8.0} ns", time(&module, backend, 2000, 10));
    }
    if let Some(library) = &library {
        println!("  {:>30}: {:>8.0} ns", "native shared library", time(&module, library, 2000, 10));
    }
    if let Some(wasm) = &wasm {
        println!("  {:>30}: {:>8.0} ns", "wasm (wasmi, instance per call)", time(&module, wasm, 2000, 10));
    }

    println!("\n== 3. diagnostics");
    let swap = |source: &str, from: &str, to: &str| {
        assert!(source.contains(from), "{from:?} is in the script");
        source.replacen(from, to, 1)
    };
    diagnose(&module, "a syntax error (an unclosed block)",
        &swap(rhai_source, "        return 0.0;\n    }\n    let dx", "        return 0.0;\n    let dx"),
        &swap(luau_source, "        return 0\n    end\n    local dx", "        return 0\n    local dx"));
    diagnose(&module, "a misspelt variable (`cooldwn` for `cooldown`)",
        &swap(rhai_source, "return cooldown;", "return cooldwn;"),
        &swap(luau_source, "return cooldown\n", "return cooldwn\n"));
    diagnose(&module, "a misspelt field read (`c.cooldwn_s -= dt`)",
        &swap(rhai_source, "c.cooldown_s -= this.dt;", "c.cooldwn_s -= this.dt;"),
        &swap(luau_source, "c.cooldown_s -= inv.dt", "c.cooldwn_s -= inv.dt"));
    diagnose(&module, "a misspelt field write (`c.coldown_s = 0.25`)",
        &swap(rhai_source, "c.cooldown_s = global::ARM_DELAY_S;", "c.coldown_s = global::ARM_DELAY_S;"),
        &swap(luau_source, "c.cooldown_s = ARM_DELAY_S", "c.coldown_s = ARM_DELAY_S"));
    diagnose(&module, "a type error (text where a number goes)",
        &swap(rhai_source, "c.cooldown_s -= this.dt;", "c.cooldown_s -= \"soon\";"),
        &swap(luau_source, "c.cooldown_s -= inv.dt", "c.cooldown_s -= \"soon\""));

    println!("\n== 3b. the engine backend's requests, checked by the ports' descriptions");
    engine_diagnose(&module, "a misspelt request field (`speeed`)",
        &swap(engine_source, "speed: global::BOLT_SPEED", "speeed: global::BOLT_SPEED"));
    engine_diagnose(&module, "text where a request's number goes (`damage: \"two\"`)",
        &swap(engine_source, "damage: global::BOLT_DAMAGE", "damage: \"two\""));
    engine_diagnose(&module, "a misspelt port (`projectil`)",
        &swap(engine_source, "request(\"projectile\"", "request(\"projectil\""));
    engine_diagnose(&module, "a missing required field (no `speed`)",
        &swap(engine_source, " speed: global::BOLT_SPEED,", ""));

    println!("\n== 4. budget: an endless loop in the entry ({BUDGET} operations)");
    diagnose(&module, "while true",
        &swap(rhai_source, "    let c = this.state.cadence;", "    loop { }\n    let c = this.state.cadence;"),
        &swap(luau_source, "    local c = inv.state.cadence", "    while true do end\n    local c = inv.state.cadence"));

    println!("\n== 5. hidden state: the same invocation twice on one backend");
    let entry = turret_entry(&module);
    let bytes = input(300 + 60, &initial_state(&module));
    let rhai_hidden = "let calls = 0;\nfn turret() { calls += 1; }";
    match RhaiBackend::new("hidden.rhai", rhai_hidden, module.clone(), BUDGET) {
        Err(e) => println!("  rhai: refused at compile: {e}"),
        Ok(b) => {
            let (a, c) = (b.invoke(0, entry, &bytes), b.invoke(0, entry, &bytes));
            println!("  rhai: compiled; two runs {}", if a == c { "agree" } else { "DIFFER" });
            println!("        first: {:?}", decode(&module, &a));
        }
    }
    let luau_hidden = "local calls = 0\nreturn { turret = function(inv)\n  calls += 1\n  if calls % 2 == 0 then inv.state.cadence.cooldown_s = 9 end\nend }";
    match LuauBackend::new("hidden.luau", luau_hidden, module.clone(), BUDGET) {
        Err(e) => println!("  luau: refused at compile: {e}"),
        Ok(b) => {
            let (a, c) = (b.invoke(0, entry, &bytes), b.invoke(0, entry, &bytes));
            println!("  luau: compiled; two runs {} (a replay of one tick gives another answer)",
                if a == c { "agree" } else { "DIFFER" });
        }
    }
    if let Ok(b) = LuauBackend::fresh("hidden.luau", luau_hidden, module.clone(), BUDGET) {
        let (a, c) = (b.invoke(0, entry, &bytes), b.invoke(0, entry, &bytes));
        println!("  luau, fresh chunk each call: two runs {}", if a == c { "agree" } else { "DIFFER" });
    }
}
