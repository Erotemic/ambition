# Rhai library environments can retain hidden state

## Question

Can a shared compiled library keep mutable gameplay state outside the host's
records, even when each entry has a fresh scope?

## Evidence

Yes. A native probe on 2026-11-18 used locked Rhai 1.27.0, strict variables,
and a 200,000-operation limit. It compiled this library:

```rhai
const counter = {
    let n = 0;
    || { n += 1; n }
};
fn next() { global::counter.call() }
```

`Module::eval_ast_as_new` made one module. `StaticModuleResolver` retained it.
Two separate evaluations of this entry returned `1`, then `2`:

```rhai
import "counter" as counter;
counter::next()
```

Each evaluation had a fresh scope. Rebuilding the module environment before
each evaluation returned `1` each time. A constant can hold a closure with
mutable captured values. Constant module variables alone do not prevent
hidden state.

## Design consequence

A library loader must preserve the host's state contract. Code can be cached;
mutable environments must not persist between invocations outside the host's
records. Check operation budgets across imports, import cycles, and module
initialization together. A loader that resets an operation budget at each
import can evade the entry limit.

The original loader disabled imports. The captured-library compiler in
`c12a4e89c` creates invocation-local environments and shares the caller's work
budget. Its owner tests reproduce this case. Keep mutable gameplay state in
host-owned records when changing the library cache.

## Script-visible source names

Rhai catch blocks expose source names and messages. A division error in a
library included its installation path in the caught map. Equal code installed
in two directories then produced different script values with the same code
identity. The source compiler uses the canonical entry name and library IDs
inside the VM. Physical paths stay in host diagnostics. The owner test covers
caught errors in both an entry function and an imported function.
