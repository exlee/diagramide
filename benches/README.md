# DiagramIDE interaction workloads

Run the workload suite:

```sh
cargo bench -p diagramide --bench interaction --features perf-workloads
```

`DIAGRAMIDE_BENCH_SAMPLES` sets the number of measured samples (default 30).
The runner does 3 warm-up iterations, then reports median, p95, coefficient of
variation (CV), and a checksum for each fixture. Compare runs on the same
machine, toolchain, power mode, and checkout. Treat a result as real only if it
beats the larger of 5% or twice the baseline CV, with no material regression in
another workload.

The suite covers generated-source rendering, a 120x240 Svgbob canvas edit,
dependency overlay fan-out, 120 grammar viewports, SVG rasterization, and the
Tcl and Ruby evaluators when available. For frame scheduling, message queues,
GPU texture installation, and multi-window interaction, use `--features profile`
with Tracy. Those paths need the real event loop, so no microbenchmark covers
them.

## Initial baseline

Captured on 2026-07-11: macOS 26.3.1, Apple arm64, rustc/cargo 1.93.0, release
benchmark profile. The first run compiled the release graph before measuring.
Tcl was unavailable in the benchmark process.

| Workload | Median | p95 | CV |
| --- | ---: | ---: | ---: |
| Pikchr edit preview | 11.459 us | 15.000 us | 0.247 |
| Svgbob edit preview | 146.250 us | 168.208 us | 0.057 |
| Svgbob canvas 120x240 | 1.533 ms | 1.573 ms | 0.013 |
| Dependency fan-out 160 | 134.291 us | 144.500 us | 0.031 |
| Grammar scroll 120 | 181.459 us | 186.042 us | 0.008 |
| SVG raster 640x360 | 212.250 us | 223.500 us | 0.026 |
| Tcl edit preview | 388.916 us | 416.667 us | 0.035 |
| Ruby edit preview | 16.667 us | 26.458 us | 0.272 |

## Retained optimizations

| Change | Baseline median / p95 | After median / p95 | Result |
| --- | ---: | ---: | --- |
| Reuse the SVG font database | 212.250 / 223.500 us | 183.958 / 194.291 us | 13.3% median and 13.1% p95 faster |
