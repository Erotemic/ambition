# Optimization report collector

Run from the repository root:

```bash
./run_optimization_report.sh
```

Outputs are written under:

```text
target/optimization_reports/<UTC timestamp>/
```

The main files to give to an LLM are:

```text
llm_optimization_report.md
ambition_optimization_report_<UTC timestamp>.zip
```

Useful modes:

```bash
./run_optimization_report.sh --quick       # skip build --timings and release/distribution builds
./run_optimization_report.sh --clean       # cargo clean before measuring
./run_optimization_report.sh --long-tests  # include slow/noisy deep probes and full diagnostics
./run_optimization_report.sh --strict      # return non-zero if any probe fails
```

Default mode keeps diagnostics concise:

- `cargo check` uses `--message-format=short` unless `--long-tests` is set.
- the Markdown report includes capped failure excerpts, not entire compiler logs.
- raw stdout/stderr are still captured under `logs/`, so critical information is not discarded.
- slow/noisy symbol-level tools (`cargo-bloat`, `cargo-llvm-lines`) run only with `--long-tests`.

The script is stdlib-only Python plus shell. Optional tools are skipped cleanly
when missing.
