# Repository history across Git epochs

**Owner:** repository provenance. The maintainer shortens active history on
purpose and keeps retired history in a cold store. This will happen again. An
active clone is therefore not the complete history. Use this page when a cited
commit is absent from your checkout.

## Where the history is

Read `.git-epoch.yaml` at the repository root first. It names the history store:

```text
https://github.com/Erotemic/ambition-history.git
```

The store has three surfaces:

| Surface | Meaning |
| --- | --- |
| Its `main` branch and README | Human instructions. Not the archived Ambition source tree. |
| `refs/meta/main`, file `manifest.yaml` | The machine authority: repositories, epochs, archived refs and boundary records |
| `refs/epochs/<repository>/<epoch>/...` | The original archived refs and their objects, with original object IDs |

The repository key is significant. Ambition and its music, SFX, sprite,
measurement and map subrepositories have separate epoch histories in the same
store. An old submodule commit is not necessarily an Ambition commit. Follow the
manifest's repository and submodule records. Do not guess from a pathname.

A normal branch-oriented clone of the history store does not fetch these custom
refs. Seeing only its README on GitHub does not mean the archive is empty. A
normal `git log HEAD`, or a deeper fetch from the active `origin`, does not cross
an epoch root. A shallow active clone is a different condition, and both can
apply at once.

Archived commits keep their original IDs. The active successor is a new root
commit with no parent edge to the retired tip. The manifest records that
relationship.

## Recover a citation

These commands add an isolated local ref namespace. They do not change the
checkout, the index, `origin` or any branch. They do add objects to the local
store; use a disposable repository if local size matters.

```bash
cat .git-epoch.yaml

git ls-remote https://github.com/Erotemic/ambition-history.git \
  'refs/meta/*' 'refs/epochs/*'

git fetch --no-tags --no-write-fetch-head \
  https://github.com/Erotemic/ambition-history.git \
  'refs/meta/main:refs/ambition-history/meta/main' \
  'refs/epochs/ambition/*:refs/ambition-history/epochs/ambition/*'

git show refs/ambition-history/meta/main:manifest.yaml
```

This imports only the `ambition` repository key. Use the matching key for a
submodule commit. Then use `git show`, `git cat-file` or `git log --all` on the
original ID. Inspect a symbol that a commit removed in that commit's parent.

For a separate evidence store with every repository:

```bash
HISTORY_EVIDENCE=$(mktemp -d "${TMPDIR:-/tmp}/ambition-history-evidence.XXXXXX")
git init --bare "$HISTORY_EVIDENCE"
git -C "$HISTORY_EVIDENCE" fetch --no-tags \
  https://github.com/Erotemic/ambition-history.git \
  'refs/meta/main:refs/meta/main' \
  'refs/epochs/*:refs/epochs/*'
git -C "$HISTORY_EVIDENCE" show refs/meta/main:manifest.yaml
```

This store holds archived epochs only. Fetch active refs into a separate
namespace when a question spans both.

Rules:

- Do not use `--prune`, force a ref update, or overwrite an evidence ref to make
  a check pass.
- Do not publish replacement refs or rewrite the active branch.
- Record the manifest object ID you inspected in any receipt.
- A successful lookup of one object does not prove boundary equivalence or a
  continuous traversal. For those, use Git Epoch's own reconstruction procedure.

## Classify a lookup before you conclude

| Result | Allowed conclusion / next action |
| --- | --- |
| Resolves in active checkout | Inspect the object and the relevant source |
| Absent locally, resolves under archived refs | Archived evidence. Keep the original citation and repository key. |
| Active clone is shallow and the object belongs to the active epoch | Fetch the missing active objects. The archive does not replace them. |
| Archive not reachable, or applicable refs not fetched | Evidence unavailable here. Do not conclude the object never existed. |
| Absent after all applicable active and archived refs and keys are checked | Unresolved citation. Look for a typo, a wrong repository key or a preservation gap. |
| Inspected objects contradict the source claim | Correct the claim with the inspected evidence |

An absence is a claim about the store you looked in. Write "not present in this
checkout at `<sha>`". For a presence claim, name the ref that reaches the object.

`scripts/check_planning_citations.py` checks local objects only. Its suggested
`git log -S ... --all` searches local refs only. It cannot prove "never existed"
in a shallow or epoch-limited clone. Do not delete a citation or call it
fabricated to get a zero exit status. When a guard is red on a citation, re-run
it where the objects live before you change anything the guard names.
