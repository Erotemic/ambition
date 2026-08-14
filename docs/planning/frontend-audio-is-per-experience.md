# Frontend audio — remaining work

> **Verified against `cecd01ca` (2026-08-13).** Route-keyed frontend audio is
> implemented: routes can declare `FrontendAudioProfile`s, composition stores
> them in a registry, activation resolves the selected route, and Smash's select
> route owns its own score. The full migration record is archived at
> [`../archive/planning-superseded/2026-08-13/frontend-audio-is-per-experience.md`](../archive/planning-superseded/2026-08-13/frontend-audio-is-per-experience.md).

## Remaining

- ▢ **Use route declarations for music changes inside one experience.** A stage
  theme and winner-card theme should be separate route declarations instead of a
  process-global music switch. The architecture is already capable of this; add
  a real consumer when the product wants the transition.

`provider` and `experience` still share string vocabulary at some call sites by
an explicit maintainer decision. Do not open a rename campaign from this item.
