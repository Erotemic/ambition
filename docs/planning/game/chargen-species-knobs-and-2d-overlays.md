# Chargen: species head knobs and per-view 2D overlay art

Status: plan (2026-10-29), not built. The 2D-overlay mechanism is owned by the
CalculexAmbition agent (first subject: Nikobolt Tesseract). This page is the
research behind the plan, for whoever builds either half.

## What Jon asked

> "we could also really improve a lot of the 3D character generator if 1. we
> did more parameterization of the different species heads, and two we had
> custom 2D artwork for every angle that effectively overlaid on to the part
> and increased its Fidelity and 2D charm. the 3D model is great for
> consistency, but I think we use it as a reference to really make some of the
> character features pop. we can even have it optional to toggle on or off to
> make some of the characters more or less unique" (2026-10-29)

Rulings it must keep:

- **3D places, 2D draws.** The cast's own 2D drawings draw; the 3D model only
  places them (2026-10-25, "never remake the cast with chargen"). Overlays are
  the same idea for generated characters.
- **Art is authored assets**, never drawn in a shader.
- **SVG views in a grid**: one SVG per subject, a cell per view, clones for
  shared parts; no stacked whole-view alternates (only face-type swaps
  overlay).
- **All angles; mirror only what is symmetric.** An asymmetric feature (a
  scar, a parted fringe, a monocle) needs its own left views, never a mirror.
- No drop shadows; feet on the floor.

## 1. The overlay mechanism

- **Art.** Per subject, one SVG grid: a cell per chargen view key (`front`,
  `front_3q`, `side`, `back_3q`, `back`; left views mirrored only for
  symmetric features). A cell holds one feature's drawing (nose, ears, a
  marking, a moustache, a hand) with its anchor point marked.
- **Placement from the 3D model.** An overlay names a landmark on a part (a 3D
  point) and an axis or scale reference. In each view it is placed at the
  projected landmark, scaled by the projected size and turned by the projected
  axis, and drawn as its own rig `Part` on that part's bone, either OVER the
  3D-drawn feature or REPLACING it.
- **Toggle.** A spec field (a head-wide `overlay`, or per feature) so a
  character can be more or less unique. Measured by `applies.py`, switchable
  in the studio.
- **Part-generic, heads first.** Any rig part or bone; Jon's ask is about
  features in general (Nikobolt needs hair, moustache, torso, hands).
- **Faces follow expressions.** An eye or mouth overlay must follow the face
  channels (`face.face_channels`: per emotion and viseme opacity parts, and
  the `face_blink` overlay part). Static features first.

### Where things are (renderer `ambition_sprite2d_renderer/chargen/`)

- `species.py`: `HeadModel` per species: `keys` (rows of
  `(f, rx, rzf, rzb, cz, sq)`, f from crown 0 to chin 1, in head heights `hh`),
  `bumps` (relief functions of `(alpha, y)`), `layout` (where the face is:
  `eye_f`, `brow_f`, `eye_alpha`, `eye_w`, `iris`, `nose_tip_f`, `mouth_f`,
  `mouth_alpha`, `nose` kind, `pupil`, beak keys `beak`, `beak_w`, `beak_h`,
  `beak_col`, `cere`, `beak_droop`), `ears`, `hair`, `muzzle_f`,
  `muzzle_alpha`, `throat`, `neck_f`, `skull_keys`, `sheen`.
- Landmarks come for free from the loft: `fig.head.point(alpha, y, out)` is the
  3D surface point (alpha from the front toward the character's left, `y =
  fig.fy(f)`), `ctx.p(point)` projects it, `ctx.view.facing(alpha)` and
  `drawkit.visible(ctx, alpha, thresh)` say whether it faces the camera. Good
  anchors: eye centre `(±eye_alpha, eye_f)`, nose tip `(0, nose_tip_f)`, mouth
  corners `(±mouth_alpha, mouth_f)`, ear bases (`headdraw._animal_ears`).
  Scale reference: `hh`, or the projected eye spacing.
- `headdraw.py`: `head_parts` (the `head`, `head_behind`, `head_ears` parts),
  `_species_features` (noses, beaks), the face parts, hair and hats.
  `draw.py` / `rig.py` assemble parts per view into the multiview rig, and
  `drawkit.Part(name, bone, z, els, label, feature=...)` is a part.

## 2. Species head knobs

Each species is one fixed model today, plus `head.muzzle` (0.6 to 1.4) and
`head.ears` (`default`, `floppy`, `pointed`). Proposed knobs, drawn from the
seed like every other field:

- **dog**: muzzle length, stop depth, skull breadth, jowls (flews), ear length
  and fold and set, nose size, coat markings (mask, blaze, saddle; floppy ears
  already take the hair colour).
- **pigeon**: beak length and curve, cere size, sheen strength, an eye ring.
- **horse**: face length, a blaze or star. **cat**: ear size, cheek fluff,
  stripes. **bird**: crest style. **sea lion**: whisker pads.
  **alligator**: snout length and width, scute rows.

## Traps (measured while making the dog and pigeon, renderer 5d0f0bf)

- `generate()` draws EVERY field in a fixed order, then applies overrides, so
  an edit changes only its own field. Draw a NEW field after all existing
  ones, or every saved seed changes.
- A species is named in many places: `SPECIES` is in BOTH `spec.py` and
  `species.py`; `HEAD_SCALE`, `COATS` and `_BUILDERS` (`species.py`);
  `hair_thickness`, the eye shape and eye colour tables and `coat_colors`
  (`headdraw.py`); the neck radius factor (`body.py`); and two explicit
  species lists in `tests/test_chargen.py`.
- `applies.py` measures which knobs draw anything per species; the studio
  caches it by `applies.version()`.
- Tests: `tests/test_chargen.py`. The species subset with every library clip's
  pose audit (`-k "species or dog or pigeon or rig_derives"`) takes about
  7 minutes. The audit checks the head stays on its neck in every view.
- Fast look: render head crops in five views for a few seeds with
  `sheets._head_crop` and `sheets._grid` (about 8 s a species; run with
  `PYTHONPATH=<renderer>`).
- Open weak spots: the pigeon's beak reads a little snout-like from the front;
  the pointed-ear dog reads foxy; the cat is untouched.
