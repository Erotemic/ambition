// POISON FIXTURE — a deliberately-violating source file.
//
// The self-tests point `forbidden-source-reference` at this dir and assert the
// rule REACTS. It is under `fixtures/`, not the package's `src/`, so cargo never
// compiles it. Do not "fix" the violations below — they are the point.
//
// This comment names ambition_content and BodyKinematics on purpose: a
// whole-line comment must NOT trip the scan (prose is exempt).

use ambition_content::intro::spawn_boss; // a real forbidden cross-crate ref

pub fn leaks(state: &BodyKinematics) {
    // The identifier `BodyKinematics` above is live sim state a presentation
    // crate must never name.
    let _ = state;
    let legacy = SandboxRuntime; // ALLOW_LEGACY_RUNTIME — this line is exempt
    let _ = legacy;
    let ok = GroundItemVisual::default(); // must NOT trip a whole-ident `GroundItem`
    let _ = ok;
}
