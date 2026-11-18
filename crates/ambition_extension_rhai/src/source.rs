//! Prepared script libraries and invocation-local module environments.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use ambition_extension_sdk::digest::Digest;
use rhai::{AST, ASTNode, Engine, EvalAltResult, Expr, GlobalRuntimeState, Module, ModuleResolver, Position, Scope, Stmt};

/// Source supplied during preparation. The name is for diagnostics only.
pub struct LibrarySource {
    pub name: String,
    pub text: String,
}

/// Bounds for one source graph and each invocation that uses it.
#[derive(Clone, Copy, Debug)]
pub struct LibraryLimits {
    pub max_libraries: usize,
    pub max_source_bytes: usize,
    pub max_depth: usize,
    pub max_imports: usize,
}

impl Default for LibraryLimits {
    fn default() -> Self {
        Self { max_libraries: 64, max_source_bytes: 1_048_576, max_depth: 32, max_imports: 128 }
    }
}

pub(crate) struct CompiledSources {
    pub entry: AST,
    pub libraries: Vec<String>,
    pub digest: u64,
}

/// Library IDs are relative to the source bundle, with slash separators.
fn check_library_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains(['\\', '\0', ':'])
        || id.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("library ID {id:?} must be a bundle-relative path without empty, dot, or parent segments"));
    }
    Ok(())
}

fn imports(ast: &AST) -> Result<Vec<String>, String> {
    let mut found = BTreeSet::new();
    let mut error = None;
    ast.walk(&mut |path| {
        if let Some(ASTNode::Stmt(Stmt::Import(import, position))) = path.last() {
            match &import.0 {
                Expr::StringConstant(id, _) => match check_library_id(id) {
                    Ok(()) => { found.insert(id.to_string()); },
                    Err(reason) => error = Some(format!("{position}: {reason}")),
                },
                _ => error = Some(format!("{position}: a library import must use a constant string")),
            }
        }
        error.is_none()
    });
    error.map_or_else(|| Ok(found.into_iter().collect()), Err)
}

pub(crate) fn compile(
    engine: &mut Engine,
    source: &str,
    text: &str,
    limits: LibraryLimits,
    read: &mut impl FnMut(&str) -> Result<LibrarySource, String>,
) -> Result<CompiledSources, String> {
    struct Graph<'a, F> {
        engine: &'a Engine,
        limits: LibraryLimits,
        read: &'a mut F,
        code: BTreeMap<String, AST>,
        text: BTreeMap<String, String>,
        pending: Vec<String>,
        bytes: usize,
    }
    impl<F: FnMut(&str) -> Result<LibrarySource, String>> Graph<'_, F> {
        fn load(&mut self, id: &str) -> Result<(), String> {
            if self.pending.iter().any(|pending| pending == id) {
                return Err(format!("library import cycle: {} -> {id}", self.pending.join(" -> ")));
            }
            if self.code.contains_key(id) { return Ok(()); }
            if self.pending.len() >= self.limits.max_depth {
                return Err(format!("library import depth exceeds {} at {id:?}", self.limits.max_depth));
            }
            if self.text.len() >= self.limits.max_libraries {
                return Err(format!("source graph exceeds {} libraries at {id:?}", self.limits.max_libraries));
            }
            let LibrarySource { name, text } = (self.read)(id)
                .map_err(|reason| format!("library {id:?}: {reason}"))?;
            self.bytes = self.bytes.checked_add(text.len()).ok_or("source byte count overflow")?;
            if self.bytes > self.limits.max_source_bytes {
                return Err(format!("source graph exceeds {} bytes at {name}", self.limits.max_source_bytes));
            }
            let mut ast = self.engine.compile(&text).map_err(|e| format!("{name}: {e}"))?;
            ast.set_source(&name);
            let imports = imports(&ast).map_err(|reason| format!("{name}: {reason}"))?;
            self.text.insert(id.to_owned(), text);
            self.pending.push(id.to_owned());
            for next in imports {
                self.load(&next).map_err(|reason| format!("{name} imports {next:?}: {reason}"))?;
            }
            self.pending.pop();
            self.code.insert(id.to_owned(), ast);
            Ok(())
        }
    }
    if text.len() > limits.max_source_bytes {
        return Err(format!("{source}: source graph exceeds {} bytes", limits.max_source_bytes));
    }
    let mut entry = engine.compile(text).map_err(|e| format!("{source}: {e}"))?;
    entry.set_source(source);
    let mut graph = Graph {
        engine, limits, read, code: BTreeMap::new(), text: BTreeMap::new(),
        pending: Vec::new(), bytes: text.len(),
    };
    for id in imports(&entry).map_err(|reason| format!("{source}: {reason}"))? {
        graph.load(&id).map_err(|reason| format!("{source} imports {id:?}: {reason}"))?;
    }
    let mut digest = Digest::new();
    digest.str("ambition.rhai.sources/1").str(text).u64(graph.text.len() as u64);
    for (id, text) in &graph.text { digest.str(id).str(text); }
    let libraries = graph.code.keys().cloned().collect();
    let resolver = Libraries(graph.code);
    engine.set_module_resolver(resolver);
    engine.set_max_modules(limits.max_imports);
    Ok(CompiledSources { entry, libraries, digest: digest.finish() })
}

/// Only syntax trees are shared. Each import creates an environment in its invocation.
struct Libraries(BTreeMap<String, AST>);

impl ModuleResolver for Libraries {
    fn resolve(
        &self,
        _: &Engine,
        _: Option<&str>,
        path: &str,
        pos: Position,
    ) -> Result<Arc<Module>, Box<EvalAltResult>> {
        Err(EvalAltResult::ErrorRuntime(
            format!("library {path:?} requires an invocation work budget").into(), pos,
        ).into())
    }

    fn resolve_raw(
        &self,
        engine: &Engine,
        global: &mut GlobalRuntimeState,
        _: &mut Scope,
        path: &str,
        pos: Position,
    ) -> Result<Arc<Module>, Box<EvalAltResult>> {
        let ast = self.0.get(path)
            .ok_or_else(|| Box::new(EvalAltResult::ErrorModuleNotFound(path.into(), pos)))?;
        // Share the caller's budget, but not its variables or module environment.
        Module::eval_ast_as_new_raw(engine, &mut Scope::new(), global, ast).map(Arc::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PreparedScript;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn prepare(text: &str, libraries: &[(&str, &str)], budget: u64) -> PreparedScript {
        PreparedScript::with_libraries("main.rhai", text, Default::default(), budget, Default::default(), |id| {
            let text = libraries.iter().find(|(name, _)| *name == id)
                .ok_or_else(|| format!("missing {id}"))?.1;
            Ok(LibrarySource { name: id.into(), text: text.into() })
        }).unwrap()
    }

    #[test]
    fn library_closures_share_one_invocation_but_cannot_retain_state_between_calls() {
        let script = prepare(
            "import \"counter\" as counter; fn read() { counter::next() * 10 + counter::next() }",
            &[("counter", "const counter = { let n = 0; || { n += 1; n } }; fn next() { global::counter.call() }")],
            200_000,
        );
        for _ in 0..3 {
            assert_eq!(script.engine.call_fn::<rhai::INT>(&mut Scope::new(), &script.ast, "read", ()).unwrap(), 12);
        }
    }

    #[test]
    fn imported_work_uses_the_callers_remaining_operation_budget() {
        let work = "let sum = 0; for n in 0..40 { sum += n; }";
        let library = format!("{work} export sum;");
        let caller = format!("{work} sum");
        let combined = format!("import \"work\" as work; {work} sum + work::sum");
        let measure = |text: &str| {
            let mut script = prepare(text, &[("work", &library)], 0);
            let spent = Arc::new(AtomicU64::new(0));
            let seen = spent.clone();
            Arc::get_mut(&mut script.engine).unwrap().on_progress(move |operations| {
                seen.fetch_max(operations, Ordering::Relaxed);
                None
            });
            script.engine.eval_ast::<rhai::INT>(&script.ast).unwrap();
            spent.load(Ordering::Relaxed)
        };
        let root_cost = measure(&caller);
        let import_cost = measure("import \"work\" as work; work::sum");
        let full_cost = measure(&combined);
        let budget = root_cost.max(import_cost);
        assert!(full_cost > budget, "both independent workloads must fit below their combined cost");
        let script = prepare(&combined, &[("work", &library)], budget);
        let error = script.engine.eval_ast::<rhai::INT>(&script.ast).unwrap_err();
        assert!(error.to_string().contains("operations"), "{error}");
    }

    #[test]
    fn shared_library_source_is_captured_once_and_changes_the_generation() {
        let root = "import \"a\" as a; import \"b\" as b; a::value() + b::value()";
        let mut reads = BTreeMap::<String, usize>::new();
        let make = |offset: i32, reads: &mut BTreeMap<String, usize>| {
            PreparedScript::with_libraries("main.rhai", root, Default::default(), 200_000, Default::default(), |id| {
                *reads.entry(id.into()).or_default() += 1;
                let text = match id {
                    "a" | "b" => "import \"common\" as common; fn value() { common::value() }".to_owned(),
                    "common" => format!("fn value() {{ {offset} }}"),
                    _ => return Err(id.into()),
                };
                Ok(LibrarySource { name: format!("provider/{id}"), text })
            }).unwrap()
        };
        let old = make(3, &mut reads);
        assert_eq!(old.engine.eval_ast::<rhai::INT>(&old.ast).unwrap(), 6);
        assert_eq!(reads["common"], 1);
        let new = make(5, &mut BTreeMap::new());
        assert_eq!(new.engine.eval_ast::<rhai::INT>(&new.ast).unwrap(), 10);
        assert_ne!(old.digest, new.digest);
        assert_eq!(old.engine.eval_ast::<rhai::INT>(&old.ast).unwrap(), 6);
    }

    #[test]
    fn invalid_import_graphs_are_refused_before_invocation() {
        for (root, library, expected) in [
            ("import \"a\" as a;", "import \"a\" as a;", "cycle"),
            ("import \"missing\" as m;", "", "missing"),
            ("fn read(id) { import id as m; }", "", "constant string"),
            ("import \"../outside\" as m;", "", "bundle-relative"),
        ] {
            let error = PreparedScript::with_libraries("main.rhai", root, Default::default(), 200_000, Default::default(), |id| {
                if id != "a" { return Err(format!("missing {id}")); }
                Ok(LibrarySource { name: "a".into(), text: library.into() })
            }).err().expect("the invalid graph must be refused");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn the_host_can_bound_library_count() {
        let root = "import \"a\" as a;";
        let limits = LibraryLimits { max_libraries: 1, ..Default::default() };
        let error = PreparedScript::with_libraries("main.rhai", root, Default::default(), 200_000, limits, |id| {
            Ok(LibrarySource { name: id.into(), text: "import \"b\" as b;".into() })
        }).err().expect("the second library exceeds the host bound");
        assert!(error.contains("exceeds 1 libraries"), "{error}");
    }
}
