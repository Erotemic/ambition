# Actor effects and modifier composition

**State:** numeric phases 0–5 DONE (2026-11-16). Effects6a's typed claim stores
and own-key release are implemented (`d577ffc5`, 2026-11-21); its live
occurrence, retirement and restore boundary remain open. The follow-up below
precedes PLANNED 6b/6c. Source review at `3ed50e318392` adds no new runtime
receipt. [The queue](../queue.md) selects execution order.

**Scope:** source-owned contributions, their application lifetime, and their
typed projections into actor semantics. Resources and payments stay with
[actor resources](composable-actor-resources.md). Immutable contribution
definitions use [capability linking and prepared facets](capability-linking-and-prepared-facets.md).

## Completed numeric slice

Commit `a65b8a3a` introduced
`ambition_platformer2d_core::effects::{BodyEffects, EffectSource, NumericAttribute,
NumericOp, fold}`. Equipment uses the shared Add/Mul fold. Speed Shoes and
maneuver gravity use source-owned entries in `BodyEffects`; the dedicated
`MomentumBoost` and gravity scale/timer pair are deleted.
<!-- cite-ok: deleted names -->

The implemented rule is one numeric operation per (source, attribute).
Reapplication replaces that entry; another source remains independent.
`body.effects` is rollback state. The app witness
`two_effects_on_one_number_expire_apart_and_rewind_across_both` proves
cross-source expiry and rewind.

Preserve these results. Never overwrite an intrinsic value and later restore
a saved old value. Keep resource amounts, numeric attributes, capabilities,
defense and behavior under their own typed authorities. The remaining work
concerns who owns a contribution, not another numeric modifier engine.

## Finding: ownership has several independent dimensions

The source review at `6895e5168638` found these representations:

| Source owner | Initial identity or lifetime | Limit at that baseline |
| --- | --- | --- |
| `crates/ambition_platformer2d_core/src/effects.rs` | One operation and timer per `(EffectSource, NumericAttribute)` | One application cannot both add and multiply the same attribute. Its second operation replaces the first. Several attributes repeat the lifetime. |
| `crates/ambition_platformer2d_core/src/ability_projection.rs` | One `AbilityContribution` variant per static source name | A source cannot both lend swimming and impose a flight ceiling without extra source names. Those names need coordinated withdrawal. |
| `crates/ambition_combat/src/extension/contact.rs` | Every `ControlHoldPort` producer writes the `Sequence` bit | One module can release another module's hold. The adapter discards the producer identity. |
| `crates/ambition_platformer2d_actor_monolith/src/body/transform_beat.rs` and the pose port in `combat::extension::contact` | Both write or remove `ActorAnimOverride` | A transform's expiry can remove a pose supplied by another owner. `PinnedRow.writer` only partly solves this for named clips. |
| `crates/ambition_combat/src/empowerment.rs` | One `Empowered` value with one timer and trait set | Independent temporary grants cannot coexist. A form and one grant can coexist through `empowerment_of`. |

These were source-derived counterexamples at the initial baseline. For
control, the sequence is sufficient to expose that implementation's mismatch:
A holds, B holds, A releases, and the shared bit is clear. A and B are
independent loaded owners on the same body. The port's
contract says another holder remains held.

Effects6a subsequently replaced the shared script hold bit and competing pose
setters with typed per-application claims. Its receipt below establishes
own-key withdrawal; the follow-up addresses the remaining identity/lifetime
boundary rather than proposing those stores again.

The earlier plan already allowed one effect to contribute to several
domains. That statement is insufficient. The current APIs also collapse
**definition, application and contribution identity**. A more general timer
around those APIs would retain the defect.

## Selected model

Keep these facts separate. The names describe roles, not fixed Rust names.

| Fact | Meaning |
| --- | --- |
| Effect definition | Authored semantics and typed prepared contribution plans |
| Issuer occurrence | Provenance: the native owner or behavior attachment that issued the application |
| Lifetime owner and binding | The body, source occurrence, room or session whose declared boundary ends the application |
| Application key | This independently revocable application |
| Refresh key | The explicit policy that makes a later application replace or refresh an earlier one |
| Contribution channel | One typed term supplied by that application |

One application can contain two numeric terms on one attribute, or a
capability lend and ceiling, without pretending that these terms have different
owners. It can also contribute to several domains. Retiring the application
withdraws all its terms at the declared publication boundary.

An application is authoritative state. The resolved outputs are projections.
Do not keep a timer per output, a saved old output, or a marker that says an
independent output was applied.

### Issuer and application identity

Reuse the existing stable object and behavior-attachment occurrence identity.
A script request's `Submitted<T>` already carries `state_scope`, `entry`,
subject scope and optional move occurrence. Preserve the relevant issuer
identity through the adapter. A module path alone does not distinguish two
attachments of that module.

An entry path is not necessarily the application owner either. A start handler
and a completion handler in one attachment can name the same local grant key.
They must be able to release that application's terms without authority over a
different attachment's terms.

A native owner supplies an equally explicit occurrence and local application
key. Use a deterministic local ordinal only when the stacking policy needs
independent applications. No portable key contains a Bevy `Entity`, component
id, wall-clock timestamp, or global cache insertion order. Do not assume that
the host's execution-order `InvocationId` is a persistent application id.

Definition identity does not establish refresh policy. Preserve current intent:

- a second pair of Speed Shoes refreshes its own pair;
- a second quasar refreshes the quasar;
- a second transformation restarts the transform beat;
- two independent script attachments hold separate claims.

Refresh authority comes from the admitted policy. The target's quasar policy
can accept another pickup into the same quasar application. Reusing an authored
source path does not grant authority to refresh or revoke an unrelated
application.

### Issuer provenance does not own every lifetime

Select the lifetime binding at admission. Reuse the existing body, object,
room and session vocabulary in
[instance lifetime](instance-lifetime-provenance-and-persistence.md); do not
create another scope taxonomy.

- A source-bound claim, such as a script hold or pose, ends when its live
  source releases it or retires.
- A granted interval can transfer to the target body. Its declared clock and
  target lifecycle then own expiry. Consuming its pickup or retiring its caster
  does not end it. The issuer remains provenance, not a required live entity.
- A room- or session-owned application follows that scope's stated retirement
  boundary, even when its issuer has already gone.

The transfer and accepted revokers are explicit. A detached script must lose
its source-bound claims; it must not remove a target-owned poison or buff that
the domain has already granted. A definition change does not silently move
this ownership.

### Share ownership; keep the reduction laws typed

The common facility owns application identity, validity and lifetime.
A domain owns the type, meaning and reduction of its contributions.

| Domain | Reduction |
| --- | --- |
| Numeric values | Add terms, then multiply terms; apply semantic clamps at the attribute owner |
| Capabilities | Union lends, intersect physical ceilings, and apply access permissions under the current ability law |
| Control holds | Suppress control while any eligible hold is live |
| Defense | Combine independent semantic reasons |
| Pose | Select by an explicit priority and tie policy; keep losing proposals so they can become visible later |
| Time | Use the time owner's clock domain, regime and arbitration policy |

Numeric staging does not make floating-point arithmetic associative. Use a
canonical source/channel order when exact reproducibility is required. Pose
selection is not a sum, and an access grant is not a physical capability.

Keep simple derived facts simple. Submersion, a form's permanent traits, and
the fixed possession/mount relation need not become allocated effect objects.
Their adapters can supply typed facts without a stored timer. The protocol
must support multiplicity where it exists, not impose one physical layout on
all sources.

### Data placement and dependency direction

Put the small identity and owned-term utilities below their users. Keep domain
schemas and prepared payloads with their capability owners. The core must not
gain dependencies on combat, sprite, time-policy or extension-host code to
interpret every contribution type.

The prepared application binds typed facets through the selected composition.
There is no central enum of all gameplay mutations, `Any` registry, arbitrary
attribute map, or common message bus that owns the reducers.

Use target-local sparse storage for live applications unless a measured
customer needs independent entity identity. A body with no dynamic
applications needs no allocation. Index the relevant domain terms during
preparation or activation. A movement read must not scan unrelated pose,
inventory or story state.

### Activation, clocks and read cuts

Validate all required contribution facets before publication. If one required
facet cannot bind, reject the application without a partial hold or pose.

The application advances once on its declared clock. A consumer reads its
typed projection; it does not spend the interval. Preserve each migrated
customer's current clock until a deliberate gameplay change selects another.
In particular, a transform beat uses the unscaled simulation-step duration
so that its own slowdown does not extend it. `WorldTime::wall_dt` is a
per-step snapshot; no new OS-clock read belongs in this state.

Publication follows the existing semantic phases. State exactly when a request
takes effect. For example, a control request submitted in `BODY_CONTACT`
currently affects the next brain read; this work must not silently move it
earlier. At that next declared read cut, every term from the application sees
the same live or retired decision. At transform expiry, defense, pose and
clock-policy readers must not observe different lifetimes.

Atomic retirement means atomic visibility at those cuts. It does not require
one exclusive system to mutate every domain. Domain caches can rebuild in
their own ordered systems, provided no consumer can read a stale output between
retirement and projection.

### Rollback, removal and reload

Snapshot the application key, issuer occurrence, lifetime binding and state, dynamic
parameters and the identity of the prepared meaning that interprets them.
Retain that immutable meaning while a snapshot can name it. A rewind must not
resolve an old application through the current replacement definition.

Derived masks, resolved numbers and pose winners must rebuild from the restored
authority before their next read. Do not independently restore a cached winner
and a source table that can disagree.

A lifetime owner's removal retires its applications through the same protocol.
For source-bound claims this includes behavior detachment and module replacement.
For target-owned grants it includes the target's declared retirement boundary,
not the issuer's death. Room/session bindings follow their own boundaries.
A replacement module at the same authored path is not the old live occurrence.
Preserve a source-bound claim across replacement only through an admitted
mapping. A transferred grant retains its admitted meaning until expiry or an
explicit migration; it does not depend on the issuing module remaining loaded.

Track lifetime-owner-to-target bindings when applications cross targets. Reuse
the existing scope lifecycle; do not add a whole-world cleanup scan. A failed
reload candidate cannot retire applications in the active generation. A
cross-target binding does not authorize writes across independent rollback
histories: [causal world commit](causal-world-commit.md) governs that crossing.

## Phase 6 — implement the shared ownership protocol

### 6a. Preserve issuer identity in control and pose

Start with two independent loaded owners on the same body. Each acquires its own hold
and pose claim. Release A while B remains active. The body stays held and
B's pose remains available. An explicit pose policy determines the winner;
registration order does not.

Run a native `TransformBeat` over the same body. Its expiry removes only its
pose and defense contributions. A later script pose must survive. Replace the
per-adapter source encodings with the common application identity and typed
owned terms. Do not fix this with one new `ControlHold` bit per script.

**6a record (2026-11-21).** The common application identity is
`ambition_platformer2d_core::applications`:

- `Issuer` is either `Native(name)` or `Attachment { occurrence, module }`.
- `ApplicationKey` is an issuer plus a local key.
- `OwnedTerms<T>` holds one term per key in key order. It never stores an
  `Entity` and never uses arrival order.
- A script submission's key is the `SimId` text of its state scope plus the
  `provider::module` of its entry. Two entries of one attachment share it.

| Domain | Storage | Projection | Writers |
| --- | --- | --- | --- |
| Holds | `HoldClaims` | `ControlHold::Claimed`, cleared with the last claim | `lower_control_holds` claims and releases under the submitter's key |
| Poses | `PoseClaims` | `ActorAnimOverride` is the winner: highest priority, then lowest key | `lower_pose_pins` at `SCRIPT_POSE_PRIORITY` 0; `TransformBeat` under `native("transform_beat")` at 10 |

- **Holds.** The script adapter no longer shares the native `Sequence` bit.
  Native authorities keep their own bits.
- **Poses.** Losing proposals are kept, so they show again when the winner
  withdraws. `TransformBeat`'s untouchability stays its own invulnerability
  reason.
- **Rollback.** Both stores are rollback components, probed by value.

Witness: `transform_beat::tests::each_owner_withdraws_only_its_own_hold_and_pose`.
Two attachment keys hold and pose one body. The tie goes to the lower key
whatever order the claims arrive in. A releases, and B's hold and pose
remain. A transformation shows over B's pose, and its expiry leaves B's pose.
B releases, and the body is free.

**Not yet:**

- The Rhai real path is not run. The adapters only map a submission to its
  key, and the key rule is unit-tested in core.
- An attachment that retires does not yet withdraw its claims.

### 6a follow-up — bind the live occurrence and restore the authority

**State: identity and withdrawal IMPLEMENTED 2026-11-21 for the
body-contact producer (receipt at the end of this section). The restore
projection is stated as a snapshot-resident compatibility projection, not
rebuilt. Status re-application on one tick and portable detach are open.**
Keep `OwnedTerms<T>`, independent
native hold bits, deterministic pose priority and the recorded overlap
witness. The term stores are the useful implemented mechanism; source
identity and lifecycle still need to meet the selected model above.

#### Preserve the actual attachment occurrence

[submitter_key](../../../crates/ambition_combat/src/extension/contact.rs)
currently uses the state scope's `SimId` text and substitutes `""` when no
ID exists. [Issuer::Attachment](../../../crates/ambition_platformer2d_core/src/applications.rs)
calls this string an occurrence, but a semantic state address and a live
attachment occurrence are different facts.

[Behavior attachment](../../../crates/ambition_platformer2d_runtime/src/object_behaviors.rs)
already allocates an `ObjectOccurrence` for every private binding state.
Its [owned_object_state_id](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/object_scope.rs)
instead derives a `SimId` from the owner ObjectRef and binding key. Two live
bindings of one module have different keys and are already distinct;
attachment admission rejects a duplicate live binding key. Do not describe
that supported composition as currently colliding.

If a binding is retired and recreated under the same key on the same owner,
its semantic state ID repeats while its actual occurrence changes. The
current key would let old withdrawal and new claims share an identity. This
is a required retirement/replacement witness for this follow-up, not a
demonstrated current portable script path: no portable detach operation was
found in the reviewed source. Native lifecycle and future detach support
must not freeze the semantic-address alias into the application contract.

Bind a script issuer to its admitted state occurrence and session. Reuse
the existing occurrence contract; [ActionRecipient](../../../crates/ambition_platformer2d_shared_tangle/src/action_results.rs)
already distinguishes exact occurrence from optional semantic subject text.
The application adapter needs that identity law, not the mailbox requirement
of `ActionRecipient::resolve`. Missing required identity is a refusal; do not
manufacture an empty issuer. Preserve a start/completion entry pair's ability
to address one application without giving another attachment that authority.

Native owners keep explicit native identity and declared target/source
lifetime. Their existing one-application key remains sufficient until their
stacking policy requires independent local keys. A diagnostic module path,
invocation sequence or mutable entity address is not a replacement occurrence.
Changing the key representation must retain an explicit deterministic pose
tie policy; it must not replace key ordering with insertion order.

#### Withdraw by the declared lifetime, not by provenance alone

Complete source-bound hold/pose withdrawal at the existing attachment or
scope retirement boundary. Track the affected applications through that
lifecycle; do not add a whole-world scan or separate domain cleanup clocks.
A retired source cannot later revoke the replacement attachment's claims.
The extension host's `RequestStage::Retire` runs only when its phase has
submitted work. Its name does not make it a source-lifetime maintenance
boundary: withdrawal must also run when no script request was submitted.

This rule must not revoke a granted native interval merely because its
pickup, caster or original script retired. The accepted target/room/session
lifetime remains authoritative for transferred grants. Sharing an issuer
does not force all its independently requested holds and poses to become
one inseparable interval.

For 6b, one quasar application owns one interval and supplies typed defense,
contact and music contributions. Domains keep their reduction/read policies;
they do not each copy the expiry clock. Preserve current refresh behavior.
When an interval uses data-defined prepared meaning, retain that immutable
meaning for the application's replay lifetime under
[capability linking](capability-linking-and-prepared-facets.md) and
[content generations](content-generation-and-reload.md). Do not reinterpret
an old application through a newly active catalog. Simple unit holds and
native enum-valued pose claims do not need a content-generation handle solely
to fit this model. Content meaning and live occurrence allocation are distinct.

#### Restore claims, then publish their projections before the declared reads

Current [character rollback registration](../../../crates/ambition_characters/src/rollback_registration.rs)
stores `ControlHolds` and `HoldClaims`; current
[sprite rollback registration](../../../crates/ambition_sprite_sheet/src/rollback_registration.rs)
stores `ActorAnimOverride` and `PoseClaims`. The Claimed bit and winning
pose are therefore stored alongside their source terms. The current command
helpers update them together; this review found no production pose writer
bypassing those helpers and demonstrated no divergent-restore failure.

Complete the authority boundary before adding multi-domain intervals:
restore authoritative applications/typed claims, then rebuild affected
projections at the restore/deferred-publication boundary before their first
consumer. Native control-hold bits remain their owners' authoritative facts;
only `ControlHold::Claimed` derives from `HoldClaims`. Keep typed
consumer components rather than making every domain query a generic effect bag.

If a compatibility projection remains snapshot-resident during migration,
state its derived status and atomic restoration requirement. Remove its
independent authoritative restoration only when the replacement projection
cut exists. [claim_pose](../../../crates/ambition_sprite_sheet/src/character/anim.rs)
returns early for an unchanged term, so the next identical submission is not
a substitute for rebuilding a cleared/restored winner.

Control and pose retain their existing distinct read cuts: a hold affects
the next ordinary brain read, while a pose can affect the current drawn tick.
A shared application identity does not erase that schedule distinction.

**Exit:** use the real submission adapter for independent bindings and shared
start/completion entries; refuse missing identity; retire and recreate a
binding without old releases reaching the new one; preserve target-owned
grants after issuer retirement; rewind across acquisition and retirement
with the same claims and projections at each declared read. The replacement
case must identify whether it uses the native lifecycle or a newly supported
portable detach path. This completes 6a's boundary, then enables 6b/6c; it
does not introduce a dynamic reducer registry or a general effect language.

#### Follow-up receipt (2026-11-21)

**Identity.** `Issuer::Attachment` now carries an `OccurrenceKey`: session,
allocation room and sequence of the state's live `ObjectOccurrence`. It no
longer carries `SimId` text. In
[contact](../../../crates/ambition_combat/src/extension/contact.rs),
`submitter_key` allocates the state's occurrence through
`ensure_object_occurrence`; the counter and stamp rewind with the room. A
state with no session or identity is refused with a warning, so no `""`
issuer is made up. Entries of one module on one state share the issuer, and
a remade object is another issuer.

**Withdrawal.** `withdraw_unbound_attachment_claims` runs on every tick of
`body_contact`, whether or not anything was submitted. It is installed
through the new host hook `install_extension_lowering_maintenance`, which
sits after the submitted lowerings and is not gated on the outbox. It visits
only bodies that carry a hold or pose claim, and withdraws each attachment
claim that names the body's own occurrence once that module is no longer
bound there (its status ended or its brain changed).

**Scope.** In today's production, every `ControlHoldPort` / `PosePinPort`
submitter is a body-contact entry whose state scope is the body it holds.
Claims another object issued are left to their owner, and native claims are
untouched.

**Witness.** `extension::contact::tests::a_hold_is_withdrawn_when_its_attachment_is_no_longer_bound`
uses the real host and adapters:
1. A status-bound rule holds its body under the body's occurrence and module.
2. The status ends, and on a tick that submits nothing the claim and the
   `Claimed` bit go.
3. A body remade under the same name gets a different occurrence.
4. A touched body with no session submits, and is refused rather than held.

The native lifecycle supplies the replacement; no portable detach operation
exists.

**Restore.** `HoldClaims` / `ControlHolds` and `PoseClaims` /
`ActorAnimOverride` stay snapshot-resident pairs. They are registered in one
snapshot and restored together, and the claim helpers are their only
writers. The `Claimed` bit and the winning pose are therefore a derived
projection that must restore atomically with its claims. It is not rebuilt
at the restore boundary, and no divergent restore was observed.

**Open:**
- A status that ends and is re-applied before the next maintenance pass
  keeps one issuer key. Status applications have no occurrence of their
  own.
- The transferred-grant rule (6b) is not exercised.
- The Rhai path is exercised only through the existing Mary-O/Sanic lanes.

### 6b. Make one native interval supply several domains

Use Mary-O's pocket quasar in `game/ambition_demo_mary_o/src/star.rs`.
It currently turns an equipment token into `Empowered`, projects
untouchability/contact harm, claims music, and ends at course completion.

The quasar becomes one identified application. Course completion retires the
quasar, not an entire component that a separate empowerment can own.
Admission transfers the interval to the body, so consumption or retirement of
the pickup token does not retire the granted quasar.
Presentation observes that application. Music must not infer “quasar” from
the presence of any `Empowered` value.

Keep the current refresh behavior and the current native damage path.
Contact harm continues to submit ordinary hits. A grant does not become a
second combat reducer. A permanent super form remains a character fact.

**6b landed (2026-11-22).** The pocket quasar is one application on the
body: `star::PocketQuasar { remaining }` owns the star's single interval.
- **Clock.** `burn_pocket_quasars` runs it at the room's pace.
- **Defense and contact.** They read the quasar's held grant
  (`QUASAR_GRANT`, `Empowered` with no clock of its own), so the grant can
  neither outlive nor undercut the interval. Contact harm still submits
  ordinary hits through `apply_contact_harm`.
- **Music and the shader.** `star_music_is_owed`, `play_star_music` and the
  three shader queries observe `PocketQuasar`, not any `Empowered`.
- **Retirement.** Course completion and death call `retire_quasar`, which
  removes the application and withdraws the grant only while it is still
  the quasar's. An empowerment another granter holds stays (witness:
  `victory_retires_the_quasar_and_not_another_granters_empowerment`).
- **Transfer.** Admission spends the token the tick it is collected, so the
  pickup's consumption cannot end the star. Re-collecting refreshes the
  interval.
- **Other granters.** A body already empowered by another granter keeps that
  grant: the quasar burns (theme and look) without overwriting it.
- **Rollback.** Schema 528 (`content.mary_o_pocket_quasar`).

Open: `Empowered` remains one slot per body. Two simultaneous granters do not
compose; the second keeps the first's traits. A typed contribution set waits
for a second real granter.

### 6c. Reuse the protocol for numeric and capability channels

Migrate one current numeric source through the same ownership and lifetime
road. Keep its explicit refresh rule and clock. Permit two channels on one
attribute and a lend plus ceiling from one application. Each application has
one interval, regardless of the number of terms it supplies.

Delete the migrated per-channel timers and independent cleanup rules. The
result must remove four competing ownership protocols: script holds, script
poses, the selected native interval, and numeric source lifetime. A utility
that merely wraps each existing setter without taking ownership does not
complete the slice.

The existing numeric fold and typed ability/defense laws remain useful.
Extract only the shared identity, owned-term storage and lifecycle operations
that these customers need. Do not add a status language or periodic behavior
scheduler. A later poison effect sends damage requests to combat.

## Evidence required for completion

Use a focused real-path scenario plus narrow reduction checks where needed.
The scenario must distinguish:

- two independent issuers from repeated application under one refresh key;
- two terms on one attribute from two separate effect owners;
- a control/pose release from removal of another owner's claim;
- one interval's end from the continued life of a concurrent grant;
- a rewind before application from a rewind before retirement;
- source-bound detachment from a granted interval that survives its issuer;
- successful replacement from a refused reload.

Inspect the same semantic facts at the declared read cuts. Preserve the
current transform/quasar/Speed Shoes behavior. Measure the empty case and the
selected overlap case before claiming a runtime improvement. No new source-text
guard, global census, or full-buff test matrix is required.

This phase is complete when a native mechanic and independently loaded behavior
can share one revocable application protocol without sharing their reducer,
clock policy, schema owner or game vocabulary. The protocol must make the next
compound mechanic require typed declarations and policy, not another set of
paired apply/restore systems.
