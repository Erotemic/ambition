# Comment and documentation debloat: pass 2

## Scope

This pass targets low-churn source and documentation rather than current architecture work.
The selection scan used the visible Git epoch and comment density. Every file edited in this
pass had one non-merge touch in that visible history before this change. Because Ambition
periodically truncates active history, that is a conflict-avoidance signal, not a claim about
the file's lifetime history.

The pass covers:

- 18 isolated `ambition_entity_catalog::smash_*` technique-vocabulary modules;
- the stable content-artifact envelope and engine-schema registry;
- the SDK README and API reference;
- the minimal SDK fixture and the two SDK documentation ratchets;
- deletion of the superseded `docs/sdk/api-prototype.md` design record.

It deliberately avoids A10 candidate-world/session construction, rollback admission,
identity/provenance implementation, and other current architecture seams.
`smash_capture.rs` and `smash_repertoire.rs` are also deferred: both are low-churn, but they
encode denser shared grammar and deserve an isolated semantic review before comment removal.

## Quantitative result

These measurements describe only the files selected for this pass. They are observations,
not deletion targets.

| Scope | Before | After | Change |
|---|---:|---:|---:|
| Selected files, lines | 5,630 | 3,716 | -1,914 |
| Production Rust, lines | 3,128 | 1,884 | -1,244 |
| Production Rust full-line comments | 1,605 | 361 | -1,244 |
| Production Rust rustdoc lines | 1,480 | 337 | -1,143 |
| 18 technique-vocabulary files, lines | 2,751 | 1,591 | -1,160 |
| 18 technique-vocabulary full-line comments | 1,481 | 321 | -1,160 |
| SDK Markdown lines | 1,207 | 721 | -486 |
| Minimal fixture Rust lines | 964 | 810 | -154 |
| Minimal fixture full-line comments | 220 | 67 | -153 |
| Python ratchet `#` comments | 35 | 25 | -10 |

`docs/sdk/api-prototype.md` accounted for 340 of the removed Markdown lines. It explicitly
identified itself as a historical slice-A design record, said the implementation had landed,
and directed readers to current SDK guidance. Git remains the provenance for that design.

## What was removed

### Technique vocabulary

The selected `smash_*` modules had large blocks of review chronology, maintainer quotes,
dated bug stories, decorative warning markers, and repeated explanations of validation that
was already visible in the code. Those were reduced to the durable contract at the owning
abstraction:

- what the payload means;
- which subsystem owns runtime behavior;
- non-obvious units and bounds;
- why a validation rule is mechanical rather than balance policy;
- what a test witnesses when the failure would otherwise be silent.

No production Rust logic changed in these files.

### Content artifact and schema registry

`ambition_content_pack::artifact` now states the durable distinctions directly:

- section codecs belong to their domains;
- envelope and section versions answer different compatibility questions;
- parsing is not admission;
- admission returns every independent refusal;
- unknown section kinds may be carried without being acted on.

`ambition_engine_schemas` now documents the one-owner registry and capability ownership
without retaining the migration story about the two deleted hand-maintained lists, dependency
closure measurements, or dated move history.

### SDK documentation

The SDK docs now describe the current public surface rather than the blind-run campaign that
found it. Historical debugging narratives, slice labels, dated corrections, old measurements,
and closed-gap prose were removed. Current failure semantics and operational constraints were
kept.

One stale current-state claim was fixed while consolidating the reference: the page said that
there was no public seam for driving input to a named seat. The current facade exports
`drive_slot_frame`, and the stable minimal fixture already tests the primary-seat and explicit-
seat seams. The reference now documents both.

The SDK `Known gaps` section now contains current gaps only. Closed gaps are not retained as
strike-through history.

### SDK fixtures and ratchets

Test comments now say what boundary each test witnesses instead of which campaign slice or
blind run caused the test to exist. The two documentation ratchets keep their contracts but
lose the investigation narrative. The removal of `api-prototype.md` also removed the last two
references to that superseded file.

## What was deliberately preserved

The cleanup retains explanations for:

- semantic ownership across ruleset, movement, combat, mount, portal, and item systems;
- payload units where confusing one quantity for another changes authored behavior;
- validation that prevents silent no-op or impossible techniques;
- the independent-envelope/section-version contract;
- parse-versus-admission semantics;
- the single engine-schema registry and capability ownership;
- SDK composition ordering and rollback preconditions;
- rollback baseline, liveness, generation, snapshot-order, and entity-registration contracts;
- Bevy/GGRS or Cargo behavior a consumer cannot infer from the call site;
- test non-vacuity where a superficially passing test could prove nothing.

## Deferred low-churn targets

These were identified but intentionally not edited:

- `crates/ambition_entity_catalog/src/smash_capture.rs`;
- `crates/ambition_entity_catalog/src/smash_repertoire.rs`;
- `docs/concepts/anti-llmism-style-guide.md`;
- large runtime/test modules with only a few visible-history touches but architecture-sensitive
  rollback, session, identity, or A10 semantics.

`smash_capture.rs` and `smash_repertoire.rs` are the strongest next source-comment candidates,
but their shared grammar means a dedicated semantic pass is safer than applying the style used
for isolated technique payloads.

## Validation

- `git diff --check`: pass.
- SDK documentation ratchets:
  `python3 -m pytest -q scripts/tests/test_sdk_api_reference_is_current.py scripts/tests/test_sdk_docs_name_real_modules.py`
  -> **9 passed**.
- A diff audit verified that every changed line in the 20 production crate Rust files is a
  comment/rustdoc or blank line. The same is true for
  `fixtures/minimal_game/src/minimal_experience.rs`.
- `fixtures/minimal_game/tests/boots.rs` changes are comments plus one test failure-message
  cleanup; no test logic changed.
- `scripts/tests/test_sdk_docs_name_real_modules.py` changes include one assertion diagnostic
  string so it no longer names the deleted prototype; the assertion logic is unchanged.
- `python3 scripts/check_doc_links.py` still reports exactly the two pre-existing missing
  generated-inventory links: `README.md -> .agent/README.md` and
  `docs/README.md -> ../.agent/README.md`. No changed SDK document introduced a broken link.
- `rustfmt` is not installed in this review environment, so no formatter check was available.
  No Rust behavior or syntax was intentionally changed.
