# Benchmark results

Artifact `bench/artifacts/2026-09-08-f6c3adb3.json`, protocol D-066, commit `f6c3adb3`, 2026-09-08. 10 cold launches; per row the median of per-launch medians, the CV across launches, and the gate G = max(1.20, 1 + 3·CV). Two numbers whose ratio lies inside [1/G, G] are indistinguishable.

Hardware: Apple M1 Max, 10 cores, Darwin 25.5.0 arm64. Toolchains: rustc 1.95.0 (59807616e 2026-04-14); wasm-pack 0.15.0; maturin 1.13.1; bun 1.3.12; Python 3.12.12. Profile: release.

## python

| Benchmark | Median | CV | Gate |
|---|---|---|---|
| `crusty_compile_complex` | 13.33 µs | 0.007 | 1.20 |
| `crusty_compile_simple` | 679 ns | 0.011 | 1.20 |
| `crusty_config_evaluate_simple` | 285 ns | 0.009 | 1.20 |
| `crusty_config_load_compound` | 7.29 µs | 0.010 | 1.20 |
| `crusty_config_load_simple` | 6.15 µs | 0.009 | 1.20 |
| `crusty_evaluate_complex_hit` | 522 ns | 0.010 | 1.20 |
| `crusty_evaluate_complex_miss` | 403 ns | 0.005 | 1.20 |
| `crusty_evaluate_exact_hit` | 173 ns | 0.004 | 1.20 |
| `crusty_evaluate_exact_miss` | 135 ns | 0.003 | 1.20 |
| `crusty_evaluate_regex_hit` | 226 ns | 0.004 | 1.20 |
| `crusty_evaluate_regex_miss` | 171 ns | 0.004 | 1.20 |
| `crusty_http_config_evaluate` | 165 ns | 0.005 | 1.20 |
| `crusty_http_config_load` | 5.77 µs | 0.012 | 1.20 |
| `crusty_trace` | 2.71 µs | 0.005 | 1.20 |
| `floor_flat_map` | 571 ns | 0.006 | 1.20 |
| `floor_option_string` | 65 ns | 0.002 | 1.20 |
| `floor_u32` | 32 ns | 0.004 | 1.20 |

### Against the boundary floor

| Path | vs `floor_u32` | vs `floor_option_string` | vs `floor_flat_map` |
|---|---|---|---|
| `crusty_evaluate_exact_hit` | 5.41x (G=1.20) | 2.68x (G=1.20) | 0.30x (G=1.20) |
| `crusty_evaluate_exact_miss` | 4.22x (G=1.20) | 2.09x (G=1.20) | 0.24x (G=1.20) |
| `crusty_config_evaluate_simple` | 8.91x (G=1.20) | 4.41x (G=1.20) | 0.50x (G=1.20) |
| `crusty_http_config_evaluate` | 5.16x (G=1.20) | 2.56x (G=1.20) | 0.29x (G=1.20) |

## typescript

| Benchmark | Median | CV | Gate |
|---|---|---|---|
| `crusty_compile_complex` | 21.71 µs | 0.025 | 1.20 |
| `crusty_compile_simple` | 2.33 µs | 0.067 | 1.20 |
| `crusty_complex_hit` | 1.43 µs | 0.013 | 1.20 |
| `crusty_complex_miss` | 1.30 µs | 0.012 | 1.20 |
| `crusty_config_evaluate_simple` | 746 ns | 0.023 | 1.20 |
| `crusty_config_load_compound` | 8.94 µs | 0.019 | 1.20 |
| `crusty_config_load_simple` | 7.94 µs | 0.009 | 1.20 |
| `crusty_evaluate` | 1.42 µs | 0.010 | 1.20 |
| `crusty_exact_hit` | 795 ns | 0.010 | 1.20 |
| `crusty_exact_miss` | 683 ns | 0.007 | 1.20 |
| `crusty_http_config_evaluate` | 1.18 µs | 0.014 | 1.20 |
| `crusty_http_config_load` | 8.09 µs | 0.010 | 1.20 |
| `crusty_regex_hit` | 958 ns | 0.029 | 1.20 |
| `crusty_regex_miss` | 723 ns | 0.010 | 1.20 |
| `crusty_trace` | 4.17 µs | 0.024 | 1.20 |
| `floor_flat_map` | 2.25 µs | 0.017 | 1.20 |
| `floor_option_string` | 150 ns | 0.012 | 1.20 |
| `floor_u32` | 2 ns | 0.003 | 1.20 |

### Against the boundary floor

| Path | vs `floor_u32` | vs `floor_option_string` | vs `floor_flat_map` |
|---|---|---|---|
| `crusty_exact_hit` | 361.50x (G=1.20) | 5.29x (G=1.20) | 0.35x (G=1.20) |
| `crusty_exact_miss` | 310.32x (G=1.20) | 4.55x (G=1.20) | 0.30x (G=1.20) |
| `crusty_config_evaluate_simple` | 339.05x (G=1.20) | 4.97x (G=1.20) | 0.33x (G=1.20) |
| `crusty_http_config_evaluate` | 538.18x (G=1.20) | 7.88x (G=1.20) | 0.53x (G=1.20) |

