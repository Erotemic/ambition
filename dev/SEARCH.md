# Searching Ambition engineering memory

Use this when you hit a confusing bug, plan a refactor, or see an error that feels familiar.

## Choose the right corpus

| Situation | Search |
|---|---|
| Runtime symptom, weird behavior, failed local/device test | `dev/journals/` |
| Refactor planning, invariant preservation, agent mistake avoidance | `dev/benchmark-candidates/` |
| Rust module split / visibility / derives / imports | both |
| Build command uncertainty | both |
| LDtk/editor/asset interop | both |
| Android, web, overlay packaging, generated assets | both |

## Common searches

```bash
rg -n "movement|collision|sweep|teleport|wall|ledge|body mode" dev/journals dev/benchmark-candidates
rg -n "module split|re-export|visibility|extension trait|Self: Sized|derive|attribute" dev/journals dev/benchmark-candidates
rg -n "LDtk|LoadingZone|activeArea|IntGrid|cWid|editor|roundtrip|defUid|spec drift|world_x|--replace-existing|--secondary-world" dev/journals dev/benchmark-candidates
rg -n "Bevy|resource|system tuple|event|message|ParamSet|visibility" dev/journals dev/benchmark-candidates
rg -n "cargo test|single filter|unexpected argument|command grammar" dev/journals dev/benchmark-candidates
rg -n "Android|APK|asset|manifest|logcat|android_main|Gradle" dev/journals dev/benchmark-candidates
rg -n "overlay|stale base|clobber|entrypoint|feature flag" dev/journals dev/benchmark-candidates
rg -n "audio|music|director|adaptive|procedural|squeaky|cue" dev/journals dev/benchmark-candidates
rg -n "input|ControlFrame|edge|held|touch|menu|semantic" dev/journals dev/benchmark-candidates
```

## Search protocol

1. Search with the user's symptom words first.
2. Search with subsystem words second.
3. Search with the likely failure class third.
4. Read only the matching entries plus their linked benchmark questions or journals.
5. Promote durable rules into `docs/concepts/`, `docs/recipes/`, or `docs/adr/` when the lesson becomes current project policy.

Do not load the whole corpus into context. It is a lookup memory, not a cold-start essay.


## Generated localization aids

After symptom/invariant search, use generated indexes when you need concrete files, symbols, or tests:

```bash
python scripts/check_agent_kb.py
cat .agent/index/test_map.json
cat .agent/index/symbol_index.json
```

The generated indexes are navigation aids. Trust source code and current docs over generated summaries if they disagree.
