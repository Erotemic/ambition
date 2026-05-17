# Path forward

This is the current sequencing guide. Keep it short; move old milestone prose to `docs/archive/` when it lands or stops applying.

## Immediate priorities

1. **Trustworthy docs.** Keep ADRs/current/concepts aligned with the real system. Delete or archive stale migration notes.
2. **Data-driven Bevy ECS.** Continue moving runtime integration toward authored/generated data -> components/entities/systems.
3. **LDtk authority.** Use LDtk and `ambition_ldtk_tools` for world changes; retire old RON room assumptions from docs and code comments.
4. **Movement/combat validation.** Fix wall-cling/OOB and transition edge cases with focused tests and trace evidence.
5. **Platform smoke path.** Keep desktop, web, Android/mobile touch, controller, and Steam Deck paths healthy.
6. **Tool visibility.** Document the active tools enough that agents know what to call instead of hand-editing structured assets.

## Near-term doc work

- Convert stale system docs into either current docs or archived historical evidence.
- Keep `docs/mechanics/expressibility-checklist.md` synced with landed mechanics.
- Add concept pages only when they encode durable invariants or edit protocols.
- Add retrieval evals for tool lookup and platform-sensitive failures.

## Near-term code work outside this docs pass

- Remove old RON level-design code only in a dedicated code cleanup with tests.
- Continue shrinking broad sandbox modules into Bevy/ECS-oriented slices.
- Strengthen LDtk runtime projection tests.
- Add platform smoke checks as scripts or CI jobs when practical.

## Validation habit

After docs/index changes:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

After code changes, use concept pages to select focused tests before broad commands.
