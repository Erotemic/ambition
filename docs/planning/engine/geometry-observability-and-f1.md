# Geometry observability — gameplay truth in F1 and headless inspection

**State:** DONE, 2026-11-17. GOBS-0 through GOBS-6 are complete.
**Commit:** `ce8fbaf68`.
**Contract, producer coverage and commands:**
[developer tools](../../systems/developer-tools.md).

The adapters existed, but Ambition did not install their collector and renderer.
A test called the collector directly and missed that gap. The host now installs
`ObservedGeometryPlugin`. Portal parts retain exact source primitives, cuts,
paths and affine maps. Unstamped strikes use their owners' rooms, as combat does.
The report records the session, live room, subject and simulation tick.

F1 has provider, view, room and body selection. Authoring and Collision use
resolved world queries. Navigation starts with all graphs in the room. The
legend names empty, missing-subject and unavailable answers. Native body and
projectile parts, interaction doors and material refusals are visible.

**Guards:** `app_it::observed_geometry` presses F1 through the visible host's
schedule, compares F1 and headless reports, holds two rooms live, separates two
view subjects in one room, rejects a foreign subject, and observes a miss
followed by a real sandbag hit. The view crate tests exact scaled circular portal
caps and owner-scoped strikes. Render tests check cap outlines and refusal
labels. An isolated observer test run builds without a renderer.

**Measured cost:** debug build, local tools, 21 samples per provider. Reports
are complete; no shapes are truncated. Commands use `inspect_geometry --cost`.

| Room and context | Ticks | Pieces | Whole report median | JSON bytes |
| --- | ---: | ---: | ---: | ---: |
| `central_hub_complex`, room view | 120 | 290 | 260 us | 116618 |
| `portal_lab`, Player 1 | 30 | 151 | 303 us | 62270 |
| `boss_main_stage`, room view | 60 | 51 | 112 us | 21983 |

**Standing rules:** domain owners supply their actual queries or read models.
`world_pieces` handles new world fields exhaustively; other domains register
an observer beside their capability. Geometry stays at the simulation tick
pose. Presentation comparison is explicit. Disabled collection returns before
it scans bodies or calls providers. A refused query has no active fallback box.
Broad spatial semantics and runtime geometry editing remain separate work.

## Planned portal provenance extension

The [portal delivery plan](portal-parts-similarity-and-chirality.md) extends
existing owner observations as P1–P6 land. Report the actual directed binding,
topology revision/effective time, source material and accepted motion interval,
contact chart/time, and typed refusal. For a view comparison also name the
observer, presented sample/cursor and final projection, plus admitted capture
and ready-image state. Keep these as projections of actual owner results.

This makes a root remap, stale sample, unproved exclusion or unavailable picture
distinguishable without manually tracing several crates. It does not authorize
a second geometry calculator, an always-on query history, or a separate
inspection registry. Retain disabled-collection early exits and explicit
simulation-versus-presentation sample labels. The GOBS completion receipt above
remains valid for the existing report; this extension is planned.
