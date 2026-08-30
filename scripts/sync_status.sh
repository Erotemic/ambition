#!/usr/bin/env bash
# WHAT IS UNMERGED, UNPUSHED, OR UNCOMMITTED — across every branch, worktree and
# submodule in one answer.
#
# ⭐⭐ THE QUESTION IS "is anything I did going to be lost", and it has FIVE
# independent answers, which is why eyeballing `git status` does not settle it:
#
#   1. work in a WORKTREE that was never committed
#   2. a BRANCH with commits that are not in main
#   3. a branch whose commits are not on the REMOTE
#   4. a SUBMODULE whose checkout has commits the superproject does not record
#   5. a submodule whose own commits are not on ITS remote
#
# A repo can be clean by every one of the first four and still be losing work by
# the fifth. Each is reported separately and the exit code is the number of
# things that need attention.
#
#   --fetch      refresh remotes first (default; --no-fetch to skip)
#   --quiet      only print the problems and the verdict
#
# ⛔ IT CHANGES NOTHING. No fetch --prune, no merge, no push, no submodule
# update. A status tool that mutates is a tool nobody dares run when they are
# worried, which is the only time it matters.
set -uo pipefail

# ⛔⛔ THE REPO IS THE ONE YOU ARE IN, AND A NON-REPO IS AN ERROR RATHER THAN A
# CLEAN BILL. This was `dirname $BASH_SOURCE/..`, which is correct only while the
# script sits in `scripts/` of the repo it is about — and a poison test copied it
# to /tmp, where it measured `/`, found no branches, no submodules and no dirty
# files, and printed **"✔ everything is committed, merged and pushed"**. A status
# tool that answers confidently about a directory it never looked at is worse
# than no tool, because the whole point of running it is to trust the answer.
if ! repo="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  echo "sync_status: not inside a git repository (cwd $(pwd))" >&2
  exit 2
fi
cd "$repo"

do_fetch=1
quiet=0
for arg in "$@"; do
  case "$arg" in
    --no-fetch) do_fetch=0 ;;
    --fetch) do_fetch=1 ;;
    -q|--quiet) quiet=1 ;;
    -h|--help) sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "sync_status: unknown argument '$arg'" >&2; exit 2 ;;
  esac
done

problems=0
# ⛔ THE FILE LIST IS CAPPED. A worktree another agent is mid-refactor in has
# thirty-odd dirty files, and printing all of them buries the ONE line that says
# a branch is unpushed. The count is always exact; the list is a sample.
LIST_CAP=6
say()  { (( quiet )) || printf '%s\n' "$*"; }
warn() { printf '%s\n' "$*"; problems=$(( problems + 1 )); }

# The branch everything is measured against. Read rather than assumed: a repo
# whose default is `master` or `develop` gets the right answer without an edit.
main_ref="$(git symbolic-ref -q --short refs/remotes/origin/HEAD 2>/dev/null || true)"
[[ -n "$main_ref" ]] || main_ref="origin/main"
git rev-parse --verify -q "$main_ref" >/dev/null || main_ref="main"

if (( do_fetch )); then
  say "[sync] fetching (use --no-fetch to skip)…"
  # ⛔ NOT `--prune`: this tool does not delete anything, including refs.
  git fetch --all --recurse-submodules --quiet 2>/dev/null || \
    warn "[sync] ⚠ fetch failed — every 'behind/ahead' number below may be stale"
fi

say "[sync] measuring against $main_ref"
say ""

# ── 1 + 2 + 3: worktrees, their branches, and their remotes ─────────────────
say "── worktrees ────────────────────────────────────────────────────────────"
while IFS= read -r line; do
  wt="${line%% *}"
  [[ -d "$wt" ]] || continue
  branch="$(git -C "$wt" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"

  # ⛔ UNCOMMITTED WORK IS THE FIRST QUESTION, because it is the only one with no
  # copy anywhere. `--ignore-submodules=dirty` so a submodule's own uncommitted
  # work is reported ONCE, in its own section, rather than twice.
  dirty="$(git -C "$wt" status --porcelain --ignore-submodules=dirty | wc -l)"
  if (( dirty > 0 )); then
    warn "[sync] ⚠ $wt ($branch): $dirty uncommitted file(s)"
    (( quiet )) || git -C "$wt" status --short --ignore-submodules=dirty | head -n "$LIST_CAP" | sed 's/^/[sync]     /'
    (( quiet )) || (( dirty <= LIST_CAP )) || echo "[sync]     … and $(( dirty - LIST_CAP )) more (git -C $wt status)"
  fi

  if [[ "$branch" == "HEAD" ]]; then
    # ⛔ ONLY IF THE COMMIT IS ACTUALLY AT RISK. A detached HEAD parked on a
    # commit that IS in main has nothing to lose — which is the state one of this
    # repo's own worktrees sits in, and warning about it every run is how a ⚠
    # stops being read. What is dangerous is a detached HEAD carrying commits no
    # branch points at.
    head="$(git -C "$wt" rev-parse --short HEAD)"
    if git merge-base --is-ancestor "$head" "$main_ref" 2>/dev/null; then
      printf '[sync] %-28s %s\n' "(detached)" "$head — already in $main_ref, nothing at risk"
    else
      warn "[sync] ⚠ $wt: DETACHED HEAD at $head, and it is NOT in $main_ref — \
those commits belong to no branch and are one 'git checkout' from unreachable"
    fi
    continue
  fi

  # 2: is this branch's work in main?
  read -r ahead behind <<<"$(git rev-list --left-right --count "$branch...$main_ref" 2>/dev/null || echo '? ?')"
  # 3: and is it on its remote?
  upstream="$(git rev-parse --abbrev-ref "$branch@{upstream}" 2>/dev/null || true)"
  if [[ -z "$upstream" ]]; then
    unpushed="no upstream"
  else
    read -r up_ahead up_behind <<<"$(git rev-list --left-right --count "$branch...$upstream" 2>/dev/null || echo '? ?')"
    unpushed="$up_ahead unpushed, $up_behind unpulled"
  fi
  printf '[sync] %-28s %-30s %s\n' "$branch" "ahead-of-main=$ahead behind=$behind" "$unpushed"

  if [[ "$ahead" != "0" && "$ahead" != "?" ]]; then
    warn "[sync] ⚠ $branch has $ahead commit(s) NOT in $main_ref"
  fi
  # ⛔ A MISSING UPSTREAM IS ONLY A PROBLEM IF THE BRANCH HOLDS WORK. This warned
  # unconditionally and so flagged `smash-presentation` — a branch tracking
  # nothing whose every commit is already in main, i.e. a branch with nothing to
  # lose. A status tool that cries about branches that are fully merged trains
  # its reader to skim the ⚠ lines, which is the only thing it produces.
  if [[ -z "$upstream" ]]; then
    if [[ "$ahead" != "0" && "$ahead" != "?" ]]; then
      warn "[sync] ⚠ $branch tracks nothing AND has $ahead commit(s) not in \
$main_ref — that work exists only on this machine"
    fi
  elif [[ "$up_ahead" != "0" && "$up_ahead" != "?" ]]; then
    warn "[sync] ⚠ $branch has $up_ahead commit(s) not pushed to $upstream"
  fi
done < <(git worktree list)

# Branches with no worktree are still branches, and still hold work.
say ""
say "── branches with no worktree ────────────────────────────────────────────"
checked_out="$(git worktree list --porcelain | awk '/^branch /{sub("refs/heads/","",$2); print $2}')"
while IFS= read -r branch; do
  grep -qxF "$branch" <<<"$checked_out" && continue
  read -r ahead behind <<<"$(git rev-list --left-right --count "$branch...$main_ref" 2>/dev/null || echo '? ?')"
  printf '[sync] %-28s ahead-of-main=%-6s behind=%s\n' "$branch" "$ahead" "$behind"
  if [[ "$ahead" != "0" && "$ahead" != "?" ]]; then
    warn "[sync] ⚠ $branch has $ahead commit(s) NOT in $main_ref"
    # ⛔ THE SAME UPSTREAM RULE AS THE WORKTREE LOOP, and it is here because the
    # two loops disagreed: a branch with no worktree was never asked whether its
    # commits had reached a remote at all.
    up="$(git rev-parse --abbrev-ref "$branch@{upstream}" 2>/dev/null || true)"
    if [[ -z "$up" ]]; then
      warn "[sync] ⚠ $branch tracks nothing, so those commits are on this machine only"
    else
      read -r u_ahead _ <<<"$(git rev-list --left-right --count "$branch...$up" 2>/dev/null || echo '? ?')"
      [[ "$u_ahead" == "0" || "$u_ahead" == "?" ]] || \
        warn "[sync] ⚠ $branch has $u_ahead commit(s) not pushed to $up"
    fi
  fi
done < <(git for-each-ref --format='%(refname:short)' refs/heads/)

# ── 4 + 5: submodules ──────────────────────────────────────────────────────
#
# ⛔⛔ A SUBMODULE HAS TWO WAYS TO BE OUT OF SYNC AND THEY ARE OPPOSITES. The
# superproject records ONE sha; the checkout can be AHEAD of it (work the parent
# has not recorded — `git submodule status` prefixes `+`) or the checkout can be
# BEHIND it (a rebase moved the parent's pointer and nobody updated the working
# tree). Only the first loses work, and the second is what makes a build fail
# for reasons that look nothing like a submodule.
say ""
say "── submodules ───────────────────────────────────────────────────────────"
#
# ⛔⛔ EVERY WORKTREE HAS ITS OWN SUBMODULE CHECKOUTS, and this loop used to walk
# only the primary one. With five worktrees and five submodules that is five of
# twenty-five actually inspected. It was not hypothetical: `agent-worktree3` held
# two uncommitted measurement appends in ITS copy of
# `dev/ambition_dev_measurements` — data that existed on one disk, in a slot
# whose branch was about to be deleted, and the sweep reported the repo clean of
# it while looking straight at the same submodule in another directory.
for wt_root in $(git worktree list --porcelain | awk '/^worktree /{print $2}'); do
  [[ -f "$wt_root/.gitmodules" ]] || continue
  (( quiet )) || [[ "$wt_root" == "$repo" ]] || say "[sync] ── in worktree $wt_root"
  while IFS= read -r rel; do
    path="$wt_root/$rel"
    [[ -d "$path/.git" || -f "$path/.git" ]] || { warn "[sync] ⚠ $path is not initialised (git submodule update --init)"; continue; }
    recorded="$(git -C "$wt_root" ls-tree HEAD "$rel" | awk '{print $3}')"
    actual="$(git -C "$path" rev-parse HEAD 2>/dev/null || echo '?')"
    sub_dirty="$(git -C "$path" status --porcelain | wc -l)"
    sub_branch="$(git -C "$path" rev-parse --abbrev-ref HEAD 2>/dev/null || echo '?')"

    printf '[sync] %-34s %s\n' "$rel" "$sub_branch @ ${actual:0:9}"

    if (( sub_dirty > 0 )); then
      warn "[sync] ⚠ $path: $sub_dirty uncommitted file(s) INSIDE the submodule"
      (( quiet )) || git -C "$path" status --short | head -n "$LIST_CAP" | sed 's/^/[sync]     /'
      (( quiet )) || (( sub_dirty <= LIST_CAP )) || echo "[sync]     … and $(( sub_dirty - LIST_CAP )) more"
    fi
    if [[ "$recorded" != "$actual" ]]; then
      # Which way? Both are reported, because they are different problems.
      if git -C "$path" merge-base --is-ancestor "$recorded" "$actual" 2>/dev/null; then
        warn "[sync] ⚠ $path is AHEAD of what the superproject records \
(${recorded:0:9} -> ${actual:0:9}) — commit the pointer or the work is invisible to everyone else"
      elif git -C "$path" merge-base --is-ancestor "$actual" "$recorded" 2>/dev/null; then
        warn "[sync] ⚠ $path checkout is BEHIND the recorded pointer \
(${actual:0:9} < ${recorded:0:9}) — run 'git submodule update' or you are building against old content"
      else
        warn "[sync] ⚠ $path has DIVERGED from the recorded pointer \
(recorded ${recorded:0:9}, checked out ${actual:0:9})"
      fi
    fi
    # 5: the submodule's OWN remote.
    #
    # ⛔⛔ A SUBMODULE IN A WORKTREE IS ALWAYS DETACHED, so "tracks no upstream"
    # is the normal state and warning on it fired for EVERY submodule in EVERY
    # worktree — twenty warnings that meant nothing, burying the two that did.
    # The question is never "is there an upstream", it is "does this checkout
    # hold commits no remote has", so ask that directly and fall back to the
    # remote's default branch when HEAD names no upstream of its own.
    sub_up="$(git -C "$path" rev-parse --abbrev-ref '@{upstream}' 2>/dev/null || true)"
    if [[ -z "$sub_up" ]]; then
      for cand in origin/HEAD origin/main origin/master; do
        git -C "$path" rev-parse --verify -q "$cand" >/dev/null && { sub_up="$cand"; break; }
      done
    fi
    if [[ -z "$sub_up" ]]; then
      warn "[sync] ⚠ $path has no remote at all — everything in it is on this machine only"
    else
      s_ahead="$(git -C "$path" rev-list --count "$sub_up..HEAD" 2>/dev/null || echo '?')"
      if [[ "$s_ahead" != "0" && "$s_ahead" != "?" ]]; then
        warn "[sync] ⚠ $path has $s_ahead commit(s) no remote has (vs $sub_up)"
      fi
    fi
  done < <(git -C "$wt_root" config -f "$wt_root/.gitmodules" --get-regexp '^submodule\..*\.path$' | awk '{print $2}')
done

say ""
if (( problems == 0 )); then
  echo "[sync] ✔ everything is committed, merged into $main_ref, and pushed."
else
  echo "[sync] ✘ $problems thing(s) need attention — see the ⚠ lines above."
fi
exit $(( problems > 0 ))
