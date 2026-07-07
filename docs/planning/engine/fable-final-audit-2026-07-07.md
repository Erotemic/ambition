# FABLE FINAL AUDIT — 2026-07-07 (the last fable pass)

Whole-repo audit after the opus/codex decomposition landing (E1a–e, E2
projectiles, W3 world/LDtk split, W-queue step 3 lowering proof, asset-manager
carve, sprite-sheet absorb, encounter mint, boss tail, `gameplay_core` →
`ambition_actors`, `game/` re-home). **Findings are appended IN PRIORITY ORDER
as they land — treat every entry as a plan item even if the session cut off
before it was folded into the ledgers/cards.** Anything here that contradicts
an older card wins (it is the newer ruling).

Audit order (most valuable first):
1. Dep-graph / tier audit — does the crate DAG match architecture.md's arrows?
2. `ambition_actors` (68k) — the residual monolith's next decomposition line.
3. Facade/shim census — the E7/E8 dissolution checklist, made explicit.
4. Ruling-compliance spot checks (W3 zero-LDtk, [W-e] hard error, GeoId
   adoption, SweepSample adopters, Tier-0 purity).
5. Subtle-correctness greps (query order, time domains, pushout, Entity
   identity, seam races).
6. Full test gate.
7. Elegance directions newly visible in the post-carve structure.

## Findings

(appended below, newest last)
