
# Tool authoring policy

A tool should have a clear status before agents rely on it.

## Status labels

- **active**: safe to use for current workflows.
- **experimental**: useful reference, not a runtime asset source.
- **archived**: kept for history only.
- **unknown**: inspect before use.

## New or promoted tools must document

- purpose,
- primary command,
- inputs,
- outputs,
- whether outputs may become runtime assets,
- validation command,
- generated-file policy.

## Generated outputs

Generated outputs are local by default. A tool must provide an explicit install/publish step before outputs should enter `crates/ambition_actors/assets/` or another runtime path.

## Agent rule

If a tool workflow becomes relevant to future agents, update both the tool README and `docs/tools/index.md` or the appropriate focused tool doc.
