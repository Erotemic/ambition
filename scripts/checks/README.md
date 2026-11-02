# Repository checks

These commands check source contracts, documentation, build output, and authored
content. Each command owns its verdict and exit status. Some checks need
`--check` or `--strict` to enforce findings. Read the help before you rely on a
zero exit status. Source and behavioral tests remain the authority for code.

```bash
python3 scripts/checks/check_doc_links.py --help
python3 scripts/checks/check_planning_citations.py --help
python3 scripts/checks/check_absence_contracts.py --check
```

`scripts/required_checks.py` remains the main diff-based check entry point.
Tests remain in `scripts/tests/`. This move adds no new policy or waiver.
