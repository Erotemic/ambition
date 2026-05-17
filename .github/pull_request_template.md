## Summary

<!-- 1–3 bullets: what changed and why. -->

## Source-of-truth checklist

When this PR closes one or more `TODO.md` items, confirm both halves of
the source-of-truth pact:

- [ ] Removed the corresponding line(s) from `TODO.md`
- [ ] Added matching `FEATURES.md` entry (status badge + brief
      description + file:line links)
- [ ] If any `docs/*.md` source doc tracked the same item (e.g.
      `docs/recipes/mechanics-checklist.md`, `docs/planning/path-forward.md`,
      `docs/planning/tech-debt-log.md`, `docs/recipes/crate-split-plan.md`,
      `docs/recipes/events-refactor-plan.md`, `docs/systems/character-ai-refactor.md`), updated it too

If this PR is purely a refactor / bug fix that does not close a
`TODO.md` item, leave the checklist unchecked.

## Test plan

- [ ] `cargo test -p ambition_engine`
- [ ] `cargo test -p ambition_sandbox --lib`
- [ ] If LDtk authoring changed: `python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk`
- [ ] If gameplay changed: brief manual playtest notes (which room, what verbs)
