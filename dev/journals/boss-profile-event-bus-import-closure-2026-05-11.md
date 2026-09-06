# Boss profile validation: feature bus import closure miss

During the boss behavior profile patch, the engine projectile tests passed, but
sandbox compilation failed in `features/bus.rs` because `GameplayEffect` was used
without an explicit local import.

This is another occurrence of the Rust module/import closure failure mode. The
bus module had previously relied on nearby facade imports and re-exports, but a
child implementation file that names `GameplayEffect` should import it directly:

```rust
use crate::features::events::GameplayEffect;
```

Occurrence count in this refactor series: module/test import-closure and
visibility-closure misses hit #7.

Pattern to watch for before handoff: any child module that uses a type defined
in a sibling file (`events::GameplayEffect`, `model::Foo`, `ui::helper`) needs an
explicit import or a deliberately visible facade re-export. Do not assume
`use super::*` will make new sibling-defined implementation types available in a
stable way.
