# Plugin refactor stale-doc index

This index captures documentation themes that may conflict with the pluginized
platformer-runtime direction. Treat it as a review queue, not proof that every
referenced document is wrong.

## Search topics

- Docs that describe `engine_core` as the final reusable runtime boundary.
- Docs that assume reusable mechanics will remain in `ambition_sandbox` forever.
- Docs that normalize `app/plugins.rs` as the central place for subsystem system
registration.
- Docs that describe portal, item, boss, or content systems as sandbox-local by
default rather than plugin-owned behavior with Ambition-specific adapters.
- Docs that treat headless as only a test harness instead of a supported build
persona.
- Docs that imply LDtk conversion should know every optional mechanic directly.

## First-pass notes

- The generated inventory baseline now gives a concrete snapshot for measuring
whether `app/plugins.rs` registrations and raw spawn sites trend down.
- `docs/planning/plugin_refactor/` is the source of truth for the current staged
refactor plan until follow-up system docs are promoted.
- ADR 0019 records the decision to use same-crate proto-boundaries before real
crate extraction.
