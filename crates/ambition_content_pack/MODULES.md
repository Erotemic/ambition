# `ambition_content_pack` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_content_pack** — Content-pack compiler.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`diagnostic`](src/diagnostic.rs) | Structured diagnostics — the compiler's output when it refuses. |
| [`draft`](src/draft.rs) | `ContentPackDraft` — the authored side, read but not yet meaningful. |
| [`identity`](src/identity.rs) | The stable identities a prepared pack assigns. |
| [`prepared`](src/prepared.rs) | `PreparedContentPack` — the value the pipeline produces. |
| [`refs`](src/refs.rs) | References — the two safe forms, and the one that is not. |
| [`schema`](src/schema.rs) | Schema registration — how a capability contributes an authored content family WITHOUT editing one central closed enum. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
