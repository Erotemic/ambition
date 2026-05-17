# LLM documentation cleanup overlay — 2026-05-17

This overlay cleans up Ambition's LLM-facing documentation by reducing stale authority, consolidating scattered subsystem docs, and adding a stricter active-doc link/path check.

## Major changes

- Deletes the GitHub pull request template because it referenced stale docs and retired LDtk commands.
- Rewrites `FEATURES.md` into a capability matrix instead of a stale changelog.
- Rewrites `TODO.md` into a tiny active queue instead of a completed-work ledger.
- Consolidates architecture, input, pause/menu, settings, abilities, blink, body modes, and projectile docs into current canonical docs.
- Moves planning-shaped docs out of recipes.
- Moves external reference material into vision docs.
- Prunes old session dumps, overlay readmes, staging patches, and noisy archived handoffs.
- Adds `scripts/check_doc_links.py` for active-doc markdown link checks and stale-path checks.
- Adds `scripts/apply_llm_docs_cleanup.sh` because overlay zip extraction cannot delete files by itself.

## Apply order

```bash
unzip -o ~/Downloads/ambition-llm-doc-cleanup-overlay.zip -d ~/code/ambition
cd ~/code/ambition
bash scripts/apply_llm_docs_cleanup.sh
```

The apply script removes consolidated/pruned files, regenerates `.agent/` indexes, and runs the doc checks.
