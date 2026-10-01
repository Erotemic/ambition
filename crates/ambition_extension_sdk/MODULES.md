# `ambition_extension_sdk` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_extension_sdk** — The portable procedural extension contract.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`abi`](src/abi.rs) | The loaded-module ABI, `ambition-ext-1`: the bytes a host and a guest exchange, and the guest half that runs an entry from them. |
| [`digest`](src/digest.rs) | A small deterministic 64-bit digest (FNV-1a). |
| [`invoke`](src/invoke.rs) | The invocation an entry receives. |
| [`module`](src/module.rs) | Module and entry descriptors. |
| [`port`](src/port.rs) | Ports: the typed doors between a module and a domain. |
| [`schema`](src/schema.rs) | State schemas, logical values and records. |
| [`typed`](src/typed.rs) | Typed records: a Rust struct for a state schema, with no handwritten codec. |
| [`wire`](src/wire.rs) | The portable wire encoding: little-endian, length-prefixed, no type tags. |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
