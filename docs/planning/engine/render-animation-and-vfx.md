# Render, animation and VFX — Engine 1.0 program

**State:** OPEN — existing implementation is substantial; long-term subsystem boundaries are under-specified.

## Goal

Give Ambition a coherent first-class 2D presentation architecture that remains
usable with one or many local views and does not leak presentation authority back
into deterministic simulation.

Current capability already spans `ambition_render`, `ambition_vfx`, character
sprite metadata, screen effects, HUD, quality profiles and capture tooling. The
plan is to converge these pieces, not replace them with a new renderer.

## Program areas

- sprite/material rendering and layering;
- animation state/vocabulary and authored timing;
- VFX/particles and combat/world-effect presentation;
- 2D lighting/shadows/material effects where Ambition needs them;
- view-local camera and screen effects;
- quality/resolution/residency policies across desktop/mobile/multiview;
- render extraction/read models that remain downstream of simulation;
- agent-native visual diagnostics and concise review captures.

## Candidate crate / Bevy shape

Presentation domains should be ordinary Bevy plugins downstream of read models.
Do not turn `ambition_render` into a second application composition root.

Potential reusable slices (animation selection, view-local screen effects,
quality policy, sprite metadata) should be evaluated independently with the
crate strategy rather than splitting by file count.

## Open design questions — deliberately unresolved

- Does Ambition need an explicit reusable animation graph/state-machine model, or
  are typed action/pose vocabularies sufficient?
- Which VFX are authored simulation events versus purely presentation inference?
- What lighting model is worth supporting in a sprite-first game?
- How should view-local effects behave when adaptive split views merge/split?
- Which quality/residency policies belong in generic rendering versus game
  content?
- Which render capabilities are valuable enough to publish as Bevy plugins?
