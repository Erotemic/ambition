# Tentative benchmark idea: meta-process side quest derails artifact delivery

## Status

Tentative / not yet reproducible.

This is a benchmark idea, not a finished benchmark candidate. The observed
failure is plausible and potentially valuable, but we do not yet know how to
reproduce it reliably.

## Observed behavior

During an overlay-generation task, the user introduced a side question about
whether a failure should be recorded as a lessons-learned journal entry or a
benchmark candidate. The agent inspected process/dev guidance, then appeared to
over-prioritize that meta-process task.

Instead of returning quickly to the concrete overlay deliverable, the agent
began re-reading process or skill instructions, restarted planning, emitted
status updates without producing the artifact, and eventually had to be stopped.

The apparent loop shape was:

```text
concrete artifact task
→ side question about process classification
→ inspect dev/process guidance
→ over-weight process compliance
→ re-read meta instructions
→ restart or re-plan the main task
→ delay artifact handoff
```

## Why this might matter

This tests a failure mode that is not primarily about coding ability. It is
about maintaining the active task stack under interruption.

A capable agent should be able to hold all of these simultaneously:

```text
primary task: deliver the overlay
side task: classify/update dev guidance
user constraint: do not over-test; user will debug locally
interaction constraint: do not keep restarting or re-planning
```

The desired behavior is to answer or patch the side question narrowly, then
resume the concrete deliverable without turning the side quest into the main
task.

## Tentative benchmark form

A future benchmark might give the agent:

1. A repo/archive with a concrete artifact task, such as producing an overlay zip.
2. A dev folder containing guidance about journals and benchmark candidates.
3. A mid-task user interruption asking whether the current failure belongs in
   one of those locations.
4. A follow-up user message emphasizing speed and local iteration.

Expected behavior:

* Inspect only the relevant dev/process file.
* Make the smallest useful classification or doc edit.
* Preserve the main task state.
* Return to the artifact deliverable.
* Avoid rereading unrelated skill/process docs.
* Avoid restarting the whole task.
* Avoid expanding validation beyond the user's requested scope.
* Hand off a usable artifact or clearly state the minimal remaining blocker.

Failure signals:

* Repeatedly re-reading meta docs, skill docs, or instructions.
* Treating the side question as the primary task.
* Saying the work is nearly done but continuing to plan instead of shipping.
* Restarting from scratch after already having enough context.
* Over-testing despite explicit user direction to hand off quickly.
* Producing a prompt for a future agent only after the user has to stop the task.

## Reproducibility gap

We do not yet know what exact ingredients trigger the loop.

Possible contributing factors:

* A process document with an arguably wrong criterion.
* A user asking for classification while the agent is already under artifact
  pressure.
* The agent having recently made a bad overlay and trying to compensate with
  extra process compliance.
* Tool/skill instructions that require reading meta guidance before acting.
* The model confusing "use the dev folder" with "re-enter a full procedural
  workflow."

A useful next step would be to try small controlled variants and see whether
the same derailment appears:

```text
Variant A: side question only, no requested doc edit
Variant B: side question plus requested doc edit
Variant C: side question after a failed patch
Variant D: side question before any patch work starts
Variant E: explicit "do not let this interrupt the main task" instruction
```

## Candidate benchmark title ideas

* Preserve artifact delivery after meta-process interruption
* Meta-process side quest should not derail the main task
* Do not loop on process docs during overlay handoff
* Maintain task stack after benchmark-vs-journal interruption

## Notes for future refinement

This should not become a benchmark until there is a prompt sequence that
reliably distinguishes good behavior from bad behavior. The benchmark should
measure task-stack preservation and prioritization, not mere compliance with a
specific document wording.

The key question is:

```text
Can the agent answer a process side quest without losing the concrete deliverable?
```
