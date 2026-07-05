# Current risks

Keep this short. Put detailed procedures in concepts, systems, recipes, or dev-memory entries.

## High-risk areas

- **Spatial / geometry:** collision edge cases, swept AABBs, ledge contacts, blink destination search, loading-zone placement, room-transition repair, camera/world transforms, moving hazards/platforms.
- **Data-driven world flow:** LDtk entity names, IntGrid meanings, runtime ECS projection, hot reload, static-map/web embedding, and validators.
- **Platform feature composition:** desktop devtools, web, Android, mobile touch, controller, Steam Deck asset roots, and headless tests use different feature sets.
- **Sim/presentation boundaries:** gameplay logic should emit messages/events; presentation owns audio, VFX, sprites, UI, and debug overlays.
- **Broad file replacement:** replacing app/input/world files can silently clobber platform entrypoints or typed event seams.

## Review rules

- Search `dev/` for matching prior mistakes before non-trivial edits.
- Add `AMBITION_REVIEW:` comments for geometry that is plausible but hard to prove.
- Update ADRs when a durable decision changes.
- Archive or delete stale docs instead of keeping contradictory guidance alive.
- Regenerate `.agent/` indexes after docs/code/test moves.

## Useful searches

```bash
rg -n "AMBITION_REVIEW|wall_cling|ledge|sweep|loading zone|LDtk|android|web|SteamDeck" crates docs dev
rg -n "module split|re-export|visibility|extension trait|stale component" dev
```
