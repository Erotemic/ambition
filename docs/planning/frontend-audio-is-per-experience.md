# Frontend audio — remaining work

> **Verified against `7e3510f5c` (2026-09-17); previously `008b44120`
> (2026-09-02) and `cecd01ca` (2026-08-13), each finding the same thing.**
> Route-keyed frontend audio is implemented and still is: `FrontendAudioProfile`
> is declared by a route, `FrontendAudioRegistry` (`ambition_audio/src/selection.rs`)
> holds every declaration *"keyed by the route that owns"* it in its own words,
> and it has four readers at HEAD — provider composition, the game shell's
> session, and two systems in the actor monolith's audio plugin. The full
> migration record is archived at
> `../archive/planning-superseded/2026-08-13/frontend-audio-is-per-experience.md` (docs/archive/planning-superseded/2026-08-13/frontend-audio-is-per-experience.md — removed from the checkout 2026-09-05; still in git history).
>
> ⚠ **THREE RECEIPTS WERE STACKED HERE AND ARE NOW ONE.** A receipt dates the
> CHECK, so a page that keeps every check it ever passed makes the reader work
> out which one is current. The dates above are the history; the sha is the
> claim.
>
> ⇒ The remaining item below is unchanged and still a *"when the product wants
> it"* consumer rather than missing architecture: nothing declares a stage theme
> and a winner-card theme as two routes, and `MusicDirectorState` is what a
> within-experience change goes through today.

## Remaining

- ▢ **Use route declarations for music changes inside one experience.** A stage
  theme and winner-card theme should be separate route declarations instead of a
  process-global music switch. The architecture is already capable of this; add
  a real consumer when the product wants the transition.

`provider` and `experience` still share string vocabulary at some call sites by
an explicit maintainer decision. Do not open a rename campaign from this item.
