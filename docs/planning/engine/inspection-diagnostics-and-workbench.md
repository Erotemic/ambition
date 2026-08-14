# Inspection, diagnostics and workbench — Engine 1.0 program

**State:** OPEN — machine-readable inspection is the priority; a monolithic graphical editor is not.

## Goal

Make the engine explain itself well enough that an LLM agent or human developer
can understand current runtime/content state without repository archaeology or a
collection of unrelated debug hotkeys.

The first-class product is **structured introspection**. A future GUI workbench
may consume the same APIs.

## Questions the engine should answer

- What authored/prepared definition produced this entity/body/item?
- Who controls this body and which view observes it?
- Why does this body have this capability?
- Where is this persistent actor/item and why does it exist?
- What room/region owns this instance?
- What rollback/save state participates?
- Why was this authored reference rejected?
- What facts did this character observe and why did it choose this action?
- What happened during these simulation ticks?

## Program areas

- structured entity/domain inspection;
- preparation/provenance queries;
- world/item/actor accounting audits;
- pause/step/headless capture where useful;
- trace/replay/rollback inspection;
- collision/navigation/world-residency visualization data;
- profiler/compile/runtime measurements surfaced through stable reports;
- concise agent review products.

## Candidate crate / Bevy ecosystem value

A generic inspection registry/protocol may become a reusable Bevy plugin if it
can introspect domain-provided views without depending on Ambition content.
Avoid one reflection-heavy god inspector that requires every internal type to be
public.

## Open design questions — deliberately unresolved

- Reflection registry, explicit typed inspectors, or a hybrid?
- In-process query API versus trace/report artifacts?
- How much historical state should be retained by default?
- What is safe/cheap enough for shipping builds?
- How do we expose deterministic state without exposing private implementation
  topology as public SDK?
- Which inspection pieces are generic enough to become independently consumable Bevy crates?
