# Repository maintenance

These commands archive, inspect, prune, or repair repository and build state.
Read each command's help before use. Commands that remove files keep their own
selection, dry-run, and confirmation rules. Target cleanup requires the target
bind described in the root agent guide. A directory move does not change those
rules. Do not run cleanup as a command-discovery check.

Examples:

```bash
python3 scripts/maintenance/archive_agent_source.py --help
python3 scripts/maintenance/prune_profile_bundles.py --help
python3 scripts/maintenance/sweep_target.py --help
```

Tests remain in `scripts/tests/`.
