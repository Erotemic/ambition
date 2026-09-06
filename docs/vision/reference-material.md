# Reference material

External reference links are inspiration and research context. They do not override Ambition's current docs, ADRs, or source code.

## Metroidvania / level design

- PC Gamer — "How to design a great Metroidvania map". Useful for blockout-first world design and Team Cherry's 64x64 black-tile discussion.
- Game Maker's Toolkit — Boss Keys playlist. Useful comparative analysis of nonlinear world design.
- Hugo Bille — "Analysis of Super Metroid". Useful for invisible teaching, map rooms, and exploration scope.
- Game Developer — "The Foundation of Metroidvania Design". Good concise checklist around movement, upgrades, and environments.
- Dreamnoid — "How to create your own Metroidvania". Practical small-team planning notes.
- Kayin — "The Mechanics of a Metroidvania are Tools, not the Destination". Good caution against genre cargo-culting.
- The Level Design Book — Wayfinding. Useful for landmarks and player information design.

## Audio and tools

- FMOD Celeste getting-started material.
- Lena Raine interviews on game-audio workflow.
- Generated-audio tooling notes in `docs/tools/generated-audio-tools.md` and `tools/ambition_music_renderer/README.md` are the local authority.

## Font candidates

- Inter — UI/dialog candidate under the SIL Open Font License.
- JetBrains Mono — monospace/debug HUD candidate under the SIL Open Font License.
- Atkinson Hyperlegible / Atkinson Hyperlegible Mono — accessibility-focused UI/debug candidates.

Use `scripts/grab_font_assets.py` to download the current bundled-font set into the ignored sandbox asset tree before force-adding or IPFS-tracking accepted font assets.


This is a very cool way to implement visuals for 2d portals:
https://medium.com/@AtaTrkgl/ingression-how-we-made-seamless-portals-in-2d-b16080ecfabc


Another 2d portal reference:
https://github.com/MaximilianMcC/2D-Portal

https://www.gamedeveloper.com/programming/making-2d-portals-using-shaders

https://github.com/atjallen/portal-2D
