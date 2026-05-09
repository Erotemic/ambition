# UI Navigation Test Benchmark Candidates

This file is intentionally separate from the main benchmark question files so
parallel documentation edits do not collide with small follow-up fixes from the
`ui_nav` extraction.

## Question: refactoring a UI label helper with alignment gutters

### Context

A Rust game UI refactor extracted shared list/window helpers into a new
`ui_nav` folder module. The helper `decorate_windowed_label(label, index,
selected, total, capacity)` adds overflow hints to the first and last visible
rows of a clipped list. To keep labels visually aligned, rows without an up
marker still receive a two-space left gutter when the list is windowed.

A test failed after the extraction:

```text
assertion `left == right` failed
  left: "  A"
 right: "A"
```

The implementation returns `"  A"` for the first row of a windowed list at the
top edge, because there is no `↑ ` marker but the label should align with rows
that do have one.

### Task

Update the test without changing the helper behavior. Preserve the visual
alignment contract and make the assertion describe why the leading spaces are
intentional.

### Expected answer

Change the expected value from `"A"` to `"  A"` and add a short test comment
that the two-space gutter is intentional for windowed-row alignment.
