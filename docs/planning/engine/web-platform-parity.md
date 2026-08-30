# Web platform parity — closed receipt

**State:** CLOSED 2026-08-14; durable platform rules are in
[`../../concepts/platform-targets.md`](../../concepts/platform-targets.md).

The blank-browser incident came from four separate composition/packaging defects:

1. the browser host hand-spelled a different visible application and omitted the
   shell/route/visual composition;
2. browser asset-source setup did not match packaged logical asset identities;
3. web publication exposed one implementation crate's asset tree rather than the
   product packaging contract;
4. the DOM status line claimed readiness/input behavior it could not observe.

The repair established one shared visible-game composition over platform-specific
host foundations and routed served assets through the normal packaging guard.
Behavioral composition and asset-platform parity tests protect the structural
parts of the fix.

The durable lesson is not browser-specific: **a target build/link proves that the
program compiles, not that it composed the same application.** Platform hosts
share semantic composition and vary only host/device/packaging policy.

Residual manual checks are not an architecture campaign:

- a human browser run should still confirm the launcher/real frame on a machine
  with a browser;
- audit the non-served embedded-assets persona when that packaging mode becomes a
  real shipping/development path.

The detailed HTTP tables, dated root-cause investigation and implementation
chronology remain available in git history. Keep this receipt until Phase 2 can
remove the queue's historical link.
