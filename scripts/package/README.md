# Application packaging

These commands build, inspect, or assemble desktop, single-executable,
external-consumer, and web distributions. They coordinate the existing Cargo
and content-publication owners. They do not define a second asset selection.

```bash
python3 scripts/package/package_single_exe.py --help
python3 scripts/package/package_outlander.py --help
python3 scripts/package/package_project.py --help
python3 scripts/package/web_bundle.py --help
python3 scripts/package/package_asset_guard.py --help
```

Use existing artifacts when the command supports that option. A command that
builds Rust needs the required target bind. Tests remain in `scripts/tests/`.

`package_project.py` uses a prebuilt host and `project_export`. The Rust tool
reads the project and content manifests and materializes selected library
files. The Python command uses the existing byte audit and file-open probe.
It runs no compiler. The host must support `--headless-ticks` and report its
project and module composition. The optional presentation probe also needs
`--presentation-frames`; Pulse Forge's host provides both modes. See the
[example commands](../../examples/projects/pulse_forge/README.md).
