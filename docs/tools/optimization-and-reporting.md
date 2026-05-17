
# Optimization and reporting tools

## Optimization report

Location: `tools/optimization_report/`

Run from the repository root:

```bash
./run_optimization_report.sh
./run_optimization_report.sh --quick
./run_optimization_report.sh --strict
```

Outputs go under `target/optimization_reports/<timestamp>/` and include an LLM-oriented Markdown report plus a zip of raw diagnostics.

## Coverage helper

Location: `tools/test_coverage_report.sh`

Use when evaluating test coverage, not as a default validation step for every patch.

## Policy

Diagnostic reports are artifacts. Do not commit large generated reports unless a maintainer explicitly asks for them.
