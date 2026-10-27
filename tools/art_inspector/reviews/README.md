# Art review notes

Save notes from a character page with:

```bash
python3 scripts/art_inspector.py serve --open
```

The service writes one JSON file per note under `notes/`. Keep these files in
Git. Each note records the character identity and exact published generation.
The generated inspector directory and browser storage do not own these notes.

Read the task queue with `python3 scripts/art_inspector.py notes backlog --json`.
Open notes remain tasks after regeneration. Check the new art, then use
`notes resolve NOTE_UUID --resolution "Fix summary"`. Use `notes reopen NOTE_UUID`
to put a resolved note back in the queue.

See the [tool guide](../README.md#character-review-notes) for the full workflow.
