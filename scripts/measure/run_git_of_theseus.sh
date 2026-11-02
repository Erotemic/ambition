uv pip install git-of-theseus


# Analyze repository history in daily intervals
git-of-theseus-analyze . \
    --interval 86400 \
    --cohortfm "%Y-%m-%d" \
    --procs 4 \
    --ignore-whitespace \
    --outdir ./git-of-theseus

# Author line plots
git-of-theseus-line-plot \
    ./git-of-theseus/authors.json \
    --outfile ./git-of-theseus/authors-line.png

git-of-theseus-line-plot \
    ./git-of-theseus/authors.json \
    --normalize \
    --outfile ./git-of-theseus/authors-line-norm.png

# Stack plots
git-of-theseus-stack-plot \
    ./git-of-theseus/cohorts.json \
    --outfile ./git-of-theseus/cohorts-stack.png

git-of-theseus-stack-plot \
    ./git-of-theseus/authors.json \
    --outfile ./git-of-theseus/authors-stack.png

git-of-theseus-stack-plot \
    ./git-of-theseus/authors.json \
    --normalize \
    --outfile ./git-of-theseus/authors-stack-norm.png

git-of-theseus-stack-plot \
    ./git-of-theseus/exts.json \
    --outfile ./git-of-theseus/ext-stack.png

# Survival plot
git-of-theseus-survival-plot \
    ./git-of-theseus/survival.json \
    --outfile ./git-of-theseus/survival-survival.png



python - <<'PY'
import json
import collections
import numpy as np
from matplotlib import pyplot as plt

input_fpath = './git-of-theseus/survival.json'
output_fpath = './git-of-theseus/survival-survival-days.png'

DAY = 24 * 60 * 60

commit_history = json.load(open(input_fpath))

deltas = collections.defaultdict(lambda: np.zeros(2))
total_n = 0

for commit, history in commit_history.items():
    t0, orig_count = history[0]
    total_n += orig_count
    last_count = orig_count

    for t, count in history[1:]:
        deltas[t - t0] += (count - last_count, 0)
        last_count = count

    deltas[history[-1][0] - t0] += (-last_count, -orig_count)

P = 1.0
xs = []
ys = []

for t in sorted(deltas.keys()):
    delta_k, delta_n = deltas[t]
    xs.append(t / DAY)
    ys.append(100.0 * P)
    P *= 1 + delta_k / total_n
    total_n += delta_n

    if P < 0.05:
        break

plt.figure(figsize=(13, 8))
plt.style.use('ggplot')
plt.plot(xs, ys)
plt.xlabel('Days')
plt.ylabel('%')
plt.ylim([0, 100])
plt.title('% of lines still present in code after n days')
plt.tight_layout()
plt.savefig(output_fpath)
print(f'wrote {output_fpath}')
PY
