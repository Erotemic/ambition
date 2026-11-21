# `ambition_extension_sdk` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_extension_sdk** — The portable procedural extension contract.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abi`](src/abi.rs) | The loaded-module ABI, `ambition-ext-3`: the bytes a host and a guest exchange, and the guest half that runs an entry from them. |
| [`action`](src/action.rs) | Scoped replies from domain actions. |
| [`choices`](src/choices.rs) | One declaration for an enum's native, wire, and script choices. |
| [`contract`](src/contract.rs) | Compare module declarations across code backends. |
| [`data`](src/data.rs) | Bounded logical data for schema-selected record initializers. |
| [`digest`](src/digest.rs) | A small deterministic 64-bit digest (FNV-1a). |
| [`invoke`](src/invoke.rs) | The invocation an entry receives. |
| [`module`](src/module.rs) | Module and entry descriptors. |
| [`object`](src/object.rs) | References to one live object occurrence in a gameplay session. |
| [`port`](src/port.rs) | Ports: the typed doors between a module and a domain. |
| [`schema`](src/schema.rs) | State schemas, logical values and records. |
| [`script`](src/script.rs) | What a script sees: one neutral value for an invocation, and the conversions that make it. |
| [`structure`](src/structure.rs) | One structural description of a port's value (runtime authoring audit P2): the declaration that gives the value's wire encoding, its script form and a machine-readable [`Shape`]. |
| [`typed`](src/typed.rs) | Typed records: a Rust struct for a state schema, with no handwritten codec. |
| [`wire`](src/wire.rs) | The portable wire encoding: little-endian, length-prefixed, no type tags. |

_15 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
