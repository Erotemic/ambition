# ambition_sfx_pack

One-shot tool that packs the output of `tools/ambition_sfx_renderer/`
into a single binary `.sfxbank` file consumed by the Rust-side
`ambition_sfx_bank` crate.

Stdlib-only Python; no install required.

## Usage

```bash
python3 tools/ambition_sfx_pack/pack.py --dump
```

Defaults:

- input:  `tools/ambition_sfx_renderer/output/`
- output: `crates/ambition_gameplay_core/assets/audio/sfx.bank`
- `--dump` also writes `sfx.bank.txt` alongside for diff-friendly inspection.

## Format

See the docstring at the top of `pack.py` for the binary layout.
The Rust reader is `crates/ambition_sfx_bank/`.
