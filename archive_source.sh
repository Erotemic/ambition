# Take some history for review

TAR_FPATH=ambition-source.tar
git-well archive-source --depth=100 --format=tar -o "$TAR_FPATH"

# Add new file

python scripts/generate_agent_index.py

xdev dirstats crates -D 4 --exclude_dnames "debug_traces" ".worktrees" ".agent" > ".agent/dirstats-crates-summary.txt"
xdev dirstats crates --exclude_dnames "debug_traces"  ".worktrees"  ".agent" > ".agent/dirstats-crates-full.txt"
xdev dirstats . -D 4 --exclude_dnames "debug_traces"  ".worktrees" ".agent" > ".agent/dirstats-user-repo-summary.txt"
xdev dirstats . --exclude_dnames "debug_traces"  ".worktrees" ".agent" > ".agent/dirstats-user-repo-full.txt"

tar -rf "$TAR_FPATH" .agent

