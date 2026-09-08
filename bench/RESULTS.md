# Benchmark results

No current results. The last report here compared five implementations and was
dated 2026-02-10; it described engines that D-051 retired, and its numbers were
never reproduced under a stated protocol. It is in git history at `1b058f3b`.

What exists now:

- `just bench-rust` — the engine, under divan
- `just bench-xuma-crust-py` — the PyO3 boundary, under pytest-benchmark
- `just bench-xuma-crust-wasm` — the WASM boundary, under mitata
- `just bench-smoke` — every benchmark once, in CI, so one that stops
  measuring anything fails

None of these is a regression gate. A result belongs in this file only when it
carries the commit, hardware, toolchain and profile it was measured on, and a
noise floor to read it against. That protocol is the benchmark item in
`PLAN.md`.
