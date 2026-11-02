#!/bin/bash
# Run the app_it binary until it fails, preserving the FULL output of the failing run.
# The row's standing instruction: record the test name, the assertion OR PANIC TEXT,
# the SHA and the machine. A panicking victim's message is the entire diagnosis.
cd "$(git rev-parse --show-toplevel)"
export PATH=$HOME/.cargo/bin:$PATH CARGO_INCREMENTAL=0
S="${1:-$(mktemp -d)}"
SHA=$(git rev-parse --short HEAD)
for i in $(seq 1 12); do
  out=$S/flake_run_$i.log
  cargo test -j 4 -p ambition_app --test app_it > $out 2>&1
  rc=$?
  line=$(grep -E "^test result:" $out | tail -1)
  echo "run $i rc=$rc  $line"
  if [ $rc -ne 0 ]; then
    echo "=== FAILING RUN $i, sha $SHA, host $(hostname) ==="
    grep -E "^failures:|^    [a-z_]+::|panicked at|assertion" $out | head -40
    cp $out $S/flake_FAILING.log
    exit 1
  fi
done
echo "12 clean runs at $SHA on $(hostname) — no reproduction"
