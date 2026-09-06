
# Optimization and reporting tools

## Optimization report

Location: `tools/optimization_report/`

Run from the repository root:

```bash
./run_optimization_report.sh
./run_optimization_report.sh --quick
./run_optimization_report.sh --strict
```

Outputs go under `target/optimization_reports/<timestamp>/` and include an LLM-oriented Markdown report plus a zip of raw diagnostics.

## Coverage helper

Location: `tools/test_coverage_report.sh`

Use when evaluating test coverage, not as a default validation step for every patch.

## Policy

Diagnostic reports are artifacts. Do not commit large generated reports unless a maintainer explicitly asks for them.

## Rust monomorphization and target-footprint census

Location: `scripts/monomorphization_report.py`

Use this when compile/codegen cost or `target/` size looks disproportionate to
source size. The tool keeps four different questions separate rather than
turning "large Rust build" into one metric.

### 1. Retained Cargo artifacts — stable, read-only

```bash
python3 scripts/monomorphization_report.py target
```

This inventories the Cargo target directory that the checkout actually resolves,
including `deps/`, `incremental/`, and build-script output. It also groups Cargo's
hashed artifacts by logical name and reports **all-but-largest retained bytes**
for groups with more than one hash.

That quantity is artifact-variant pressure, not "duplicate code": two hashes may
legitimately represent different features, profiles, rustflags, or targets. The
ranking answers which variant families deserve investigation or sweeping.

### 2. Emitted machine code — stable, read-only, no rebuild

GNU `nm` (or `llvm-nm`) can read an already-built Rust executable, shared library,
or rlib and report the size of each defined text symbol. For example:

```bash
python3 scripts/monomorphization_report.py symbols \
    --find app_it \
    --find libambition_platformer2d_runtime \
    --top 80
```

`--find NAME` chooses the newest matching hashed artifact under `target/*/deps`;
`--artifact PATH` accepts an exact file instead.

The report ranks:

- concrete Rust code symbols by emitted text bytes;
- owner-prefix bytes (`ambition_*`, `bevy_*`, `core`, `alloc`, etc.), which tells
  us whether a problem is first-party or ecosystem code;
- conservative generic families such as `foo::<...>`, with instance count and
  total emitted bytes.

The family report deliberately calls `sum(instances) - largest(instance)` a
**pressure proxy**, never predicted savings. Different monomorphizations can have
genuinely different behavior and machine code. The useful result is a short list
of families worth reading, not a claim that all of those bytes can be erased.

### 3. ELF/rlib section composition — stable, read-only, no rebuild

When an artifact is far larger than its named text, inspect what the remaining
bytes actually are before blaming monomorphized machine code:

```bash
python3 scripts/monomorphization_report.py sections \
    --find app_it \
    --find libambition_platformer2d_actor_monolith \
    --find libambition_platformer2d_runtime
```

This runs `readelf -SW` over the linked ELF or every ELF member of an rlib and
aggregates file-backed sections into code, debug data, relocations, symbol tables,
string tables, read-only data/unwind information, writable/TLS data, and compiler
metadata. `NOBITS` sections such as `.bss` are reported as virtual bytes but never
charged to disk.

The **residual container/header/non-ELF bytes** quantity is deliberately not called
waste. It includes ELF/archive headers and padding and, for Rust rlibs, can include
non-ELF rustc metadata members that `readelf` cannot express as sections. The
measurement tells us whether the next investigation belongs in codegen, debug
information, relocations/symbol names, or rlib packaging.

### 4. rustc's mono-item collector — nightly, explicit, potentially expensive

Current nightly rustc exposes `-Zprint-mono-items`, the same collector output used
by rustc's codegen-unit tests. Capturing it establishes that a generic family is
actually being instantiated by rustc rather than merely looking repetitive in a
linked binary.

```bash
python3 scripts/monomorphization_report.py capture \
    --package ambition_platformer2d_runtime \
    --lib
```

**This mode compiles.** By default it uses an isolated target directory inside
its report directory so unstable/nightly artifacts cannot invalidate or contaminate
the ordinary development cache. That can consume substantial disk. Pass
`--capture-target-dir` when the diagnostic build should live on a disposable
scratch volume.

If a mono-item log was captured some other way, parsing it performs no build:

```bash
python3 scripts/monomorphization_report.py mono-log rustc-mono.log
```

### Reading the result

The first investigation should answer these in order:

1. Is retained disk dominated by many Cargo variants, incremental state, or a few
   enormous link products/rlibs?
2. In one representative shipped/test binary, which owner prefixes account for
   the emitted text?
3. Which generic families multiply into the most text bytes?
4. If the artifact is much larger than its text, which section classes explain
   that multiplier: debug data, relocations, symbol/string tables, compiler
   metadata, or container overhead?
5. Are the expensive families first-party? If yes, inspect whether genericity
   continues below the point where the type still changes the algorithm. The
   preferred refactor is a **small typed/generic shell feeding a large concrete
   core**, not indiscriminate `dyn Trait` conversion.
6. If the families are primarily Bevy/rustc/ecosystem code, optimize feature and
   artifact multiplicity first; source-level churn in Ambition will not remove
   code it does not own.

Generated reports live below `target/monomorphization_reports/` and are diagnostic
artifacts, not committed project state and not a test gate.
