# Repository history across Git epochs

**Owner:** repository provenance and review procedure. This documents an existing
maintainer policy: active history is deliberately shortened, retired history is
preserved in cold storage, and the operation will be repeated. An active clone is
not the complete historical evidence surface.

## Authoritative locations

Read the active repository's `.git-epoch.yaml` before resolving an old citation.
At this review baseline it identifies this store:

```text
https://github.com/Erotemic/ambition-history.git
```

The history repository has three different surfaces:

| Surface | Meaning |
| --- | --- |
| Its `main` branch and README | Human-facing instructions; not the archived Ambition source tree |
| `refs/meta/main`, file `manifest.yaml` | Machine-authoritative repository, epoch, archived-ref and boundary records |
| `refs/epochs/<repository>/<epoch>/...` | Original archived Git refs and their reachable objects, preserving original object IDs |

The repository key is significant. Ambition and its music, SFX, sprite,
measurement and map subrepositories have separate epoch histories in the same
store. An old submodule commit is not necessarily an Ambition commit. Follow the
manifest's repository and submodule records rather than guessing from a pathname.

A normal branch-oriented clone of the history store does not fetch these custom
refs. Seeing only its README on GitHub does not mean the archive is empty. A
normal `git log HEAD`, or fetching more history from the active `origin`, does
not cross a deliberately created root. Shallow-export incompleteness and an
epoch boundary are different conditions and can occur together.

## What is preserved and what reconstruction means

Retired commits, trees, blobs and recorded refs retain their original object IDs.
The active successor is a new root; adding an actual parent would change its
commit ID. The manifest records the relationship between that successor and the
retired tip. It supplies logical continuity without pretending those two commits
already have a parent edge in Git.

There are three separate verification claims:

1. **Object recovery:** the original object can be fetched and inspected by its
   original ID, including the source tree a historical citation describes.
2. **Boundary equivalence:** the successor represents the retired tip's source,
   accounting for explicitly recorded submodule epoch translations.
3. **Reconstructed traversal/workspace:** a local history view walks the verified
   boundaries, and a historical checkout can resolve the original submodule
   gitlinks from the corresponding archived repositories.

Do not substitute a successful lookup of one commit for all three claims.
Preservation covers the manifest's archived refs and objects reachable from them.
Before a rollover, name any detached work or otherwise-unreachable commit that
must be preserved; Git garbage collection cannot infer that policy afterward.

### Exact trees and recursive equivalence

A leaf repository can have an exact-tree boundary. A superproject that rolls its
submodules at the same time can have different old and new tree IDs because the
gitlinks changed. Its boundary is then recursively equivalent: every changed
path must be an enumerated gitlink translation, and each translated child boundary
must itself verify. Ordinary files, executable bits, symlinks and unlisted
submodule paths must remain equivalent according to the recorded boundary policy.
Do not accept any arbitrary tree difference under the word "recursive."

The inspected manifest records Ambition's first boundary as recursive and its
five child boundaries as exact-tree. Thus an assertion that every Ambition epoch
boundary has identical superproject trees would reject the documented design.
Future reviews must discover the current records rather than hardcode epoch 0,
epoch 1, a fixed number of children or a particular branch list.

## Recover a citation without rewriting the active checkout

These standard Git commands add an isolated local ref namespace. They do not
checkout a branch, alter the index, edit source files, change `origin`, install
replacement refs or truncate anything. Fetching archived objects increases local
storage; use a disposable evidence repository when keeping the working clone
small is important.

```bash
cd ~/code/ambition
cat .git-epoch.yaml

git ls-remote https://github.com/Erotemic/ambition-history.git \
  'refs/meta/*' 'refs/epochs/*'

git fetch --no-tags --no-write-fetch-head \
  https://github.com/Erotemic/ambition-history.git \
  'refs/meta/main:refs/ambition-history/meta/main' \
  'refs/epochs/ambition/*:refs/ambition-history/epochs/ambition/*'

git rev-parse refs/ambition-history/meta/main
git show refs/ambition-history/meta/main:manifest.yaml
```

The two wildcard refspecs on this page are deliberately different: this working
checkout example imports only the Ambition repository; the scratch-store example
below imports every repository. Use the appropriate repository key for a child.
Fetch every recorded branch/tag needed by the citation, not only archived `main`.
Do not use `--prune`, force a ref update, or overwrite a divergent evidence ref
merely to make a check pass. Retain the inspected manifest object ID in the
verification receipt so a later appended manifest does not change the evidence.

After a successful fetch, use ordinary `git show`, `git cat-file` and
`git log --all` against the original citation. Expand an abbreviated ID to its
full commit ID after resolution. A symbol removed by a commit is inspected in
that commit's parent; a symbol introduced by it is inspected in the commit itself.
Neither operation requires checking out old code.

For a separate complete evidence store:

```bash
HISTORY_EVIDENCE=$(mktemp -d "${TMPDIR:-/tmp}/ambition-history-evidence.XXXXXX")
git init --bare "$HISTORY_EVIDENCE"
git -C "$HISTORY_EVIDENCE" fetch --no-tags \
  https://github.com/Erotemic/ambition-history.git \
  'refs/meta/main:refs/meta/main' \
  'refs/epochs/*:refs/epochs/*'
git -C "$HISTORY_EVIDENCE" show refs/meta/main:manifest.yaml
git -C "$HISTORY_EVIDENCE" fsck --full
printf 'Evidence store: %s\n' "$HISTORY_EVIDENCE"
```

This store contains archived epochs, not subsequent active commits. Fetch the
required active refs into a separate namespace when an investigation spans both.
For a continuous local traversal, use Git Epoch's verified reconstruction route
from the installed tool's documentation. A manually built inspection view must
first verify each manifest boundary, work in a disposable clone, and distinguish
local replacement/graft objects from preserved originals. Never publish
replacement refs or rewrite the active branch as part of a review. A visual
continuous log alone proves neither original identity nor recursive equivalence.

## Review and citation diagnostics

Classify a lookup before drawing a historical conclusion:

| Result | Allowed conclusion / next action |
| --- | --- |
| Resolves in active checkout | Inspect the cited original object and relevant source |
| Absent locally, resolves under archived refs | Archived evidence; retain the original citation and repository identity |
| Active epoch is shallow and cited object belongs to that epoch | Obtain the missing active objects; epoch hydration is not a substitute |
| Archive cannot be reached or applicable refs were not fetched | Evidence unavailable in this environment; no conclusion that it never existed |
| Still absent after all applicable active/archived refs and repository keys are checked | Unresolved citation; investigate a typo, wrong repository or incomplete preservation |
| Source claim contradicted by inspected objects | Correct the claim with the inspected evidence, independently of the storage scheme |

The current `scripts/check_planning_citations.py` is a local-object/source linter.
Its suggested `git log -S ... --all` only searches locally available refs. That
command cannot establish "never existed" in an epoch-only or shallow checkout.
Do not delete old citations or label them fabricated merely to obtain a zero exit
status. A future diagnostic change should distinguish unavailable evidence from
an inspected contradiction; it need not download the archive in every normal CI
run. Keep the fast local check and an explicit history-hydrated audit job separate.

The first architecture overlay's **192 unresolved commit references** were a
local lookup result. They were not evidence of lost history. They have not all
been resolved by this follow-up: remote archive metadata was inspected, but a
full archive fetch/reconstruction was unavailable in the review container.

### Worked example: a red citation guard that was red about its own container

2026-09-10. An outside review of 52 commits reported the citation ratchet RED on
two commit hashes it could not resolve, and was careful to say so rather than
call them fabricated — citing the rule in the table above.

Re-run in the working checkout:

    citation guard:  9 passed
    git merge-base --is-ancestor 414019ec9 HEAD   ->  ancestor
    git merge-base --is-ancestor 1659e5402 HEAD   ->  ancestor

⇒ **Both resolve and both are reachable.** No hydration, no allow-list, no hash
correction. The finding was true of the review container and false of the
repository.

⭐ **THE METHOD IS THE DURABLE PART, NOT THE VERDICT.** The next such report will
name different hashes. What transfers is the order: **re-run the guard where the
objects live BEFORE changing anything the guard names.** An edit made from a
red-elsewhere reading would have deleted two good citations.

⚠ **AND THE SAME SHAPE ARRIVED THREE TIMES ON ONE DAY, from three people who
were not comparing notes**: a stale object store reported *"not present"*; an
accidental unpushed stash reported *"present"* for objects no clone can read;
and this container's broken `git` enumeration turned a whole guard red. ⇒ **An
absence is a claim about the store you looked in.** Say *"not present in this
checkout at `<sha>`"*, and for a presence claim, name the ref that reaches it.

## Repeatable rollover acceptance

This is a verification contract, not authorization to run another truncation.
Before advancing a repository to a successor epoch:

- archive all preservation-required refs and reachable objects, including child
  repositories referenced by retained gitlinks; publish a committed manifest
  describing exact original IDs, namespace and verification receipts;
- verify from an independent evidence clone: ref tips, object connectivity,
  available bundle checksums where bundles are used, boundary equivalence and
  recursive child translations; merely retaining a local reflog is insufficient;
- verify the successor's relationship to the retired tip, then retrieve a
  historical citation, an archived side branch and a historical submodule
  checkout without depending on the pre-rollover workstation;
- verify a walk across **two or more** boundaries, not only the new adjacent
  pair. Reject missing/ambiguous child translations, boundary loops and links to
  uncommitted manifest records. Keep active source refs and archive namespaces
  separate throughout.

The verification receipt identifies the manifest revision, repository keys,
boundaries exercised, original object IDs and commands/results. An archive receipt
from an earlier rollover remains evidence about that rollover; it does not prove
that a later one completed correctly.

## Evidence for this documentation

Inspected through the GitHub connector on 2026-09-08: the history store README,
its metadata ref and `manifest.yaml`. Metadata commit:
`b7394f043ed3fffdccbc2dad05670dd8b744c9ad`. <!-- cite-ok: external history-store commit -->
The manifest records the first committed rollout on 2026-09-06. Its recorded
checks are archival receipts, not checks executed by this review.

Upstream command references: [Git fetch](https://git-scm.com/docs/git-fetch) and
[Git replace](https://git-scm.com/docs/git-replace). Git Epoch is responsible for
the repository-specific reconstruction protocol; generic Git replacement support
does not validate an epoch manifest on its own.
