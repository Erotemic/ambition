# Project build and distribution — Engine 1.0 program

**State:** OPEN / LATER — workflow direction is clear; external-project format and release targets remain product-driven.

## Goal

Make the supported path from project checkout to shippable game explicit:

```text
clone/create
 -> configure providers/capabilities
 -> author
 -> validate/prepare/generate
 -> build
 -> package
 -> run/test
 -> distribute/update
```

Ambition already has setup scripts, content preparation, asset management,
quality profiles and desktop/Android work. The missing piece is one coherent
product contract rather than local knowledge across scripts.

## Program areas

- fresh-clone reproducibility;
- capability-driven build/features;
- generated asset ownership and cache policy;
- content-pack/asset dependency manifests;
- development versus shipping profiles;
- desktop/Android and later web/other targets when product demand exists;
- package-size/startup/load diagnostics;
- external SDK-project templates only when the public SDK is ready.

## Candidate crate/tool shape

Build orchestration is likely tool-side rather than a runtime Bevy plugin.
Runtime asset/package manifests should live in narrow crates consumed by the
engine. Avoid a giant project manager process embedded in `ambition_app`.

## Open design questions — deliberately unresolved

- What is the eventual external project layout?
- Which generated artifacts are checked in versus built on demand?
- Is web a first-class release target or only a later experiment?
- How are third-party capability/plugin versions locked and validated?
- What asset hot-reload behavior is required for agent iteration?
- What belongs in Cargo features versus runtime/provider configuration?
