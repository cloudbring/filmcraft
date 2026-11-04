# Speedup re-measure: waveform repaint (artcraft-algorithms#192, claim C05 from #106)

Claim: "filmcraft waveform repaint 9.2x" (whole 2 h clip, 1,000 columns; 0.235 -> 0.026 ms).

- OLD: merge-base `eb59ec09daaea519c9c8fffa404a594ae50d00e0`. NEW: `529c5c1e8e4b6be393e147bc6316a4643dda8529` (`algo/106-waveform-pyramid`).
- Harness: `tools/perf-remeasure/` (standalone workspace), craft-bench from craft-libs `94c6d76e5bc2c936d3db5ce5e19d70213e8390c0`.
  Run: `cargo run --release -p perf-remeasure -- WAVE_DIR docs/verification/data 40` (`WAVE_DIR` holds raw f32le mono 48 kHz files, see Inputs).
- OLD is vendored (`draw_waveform` is `pub(super)` and needs a `FilmcraftApp`; the old crate exports nothing callable): the per-column
  `peaks.iter().skip(s0).take(s1-s0)` abs-max loop, cited in the file header. NEW is `crates/ui-egui/src/waveform_peaks.rs` compiled in
  by `#[path]` (std-only; avoids the patched egui/wgpu stack). The column-to-bucket mapping, the dynamic-waveform log scale and the pixel fill are shared code.
  This is not the egui painter: egui mesh building and the volume-envelope lookup are not timed.
- Machine: Apple M6, 12 cores, 32 GiB, macOS Darwin 27.0.0, rustc 1.99.0, release. Run 2026-11-04 6:21 PM EST.
  Load average 3.2-3.7 on 12 cores (threshold 6.0), so every report is `publishable: true`, `contended: false`.
- Method: 40 runs per side per input, ABBA/BAAB alternation, 2 warm-ups, 4,000 bootstrap resamples, wall and process CPU time.
  craft-bench reports new/old; below, speedup = old/new and the CI is inverted.
- Equivalence: before any timing, per-column amplitude (f32, bit-equal) and the width x 90 8-bit pixel buffer were required equal on every input
  (assert, plus craft-bench `.check`). All 72 inputs passed. The min/max the pyramid returns equals the flat scan's by the branch's own proptests; here it is checked on real buckets.

## Inputs

Generated with ffmpeg into the scratchpad `wave/` directory: `speech` (5 min, pink noise, band-limited, 4.5 Hz tremolo, gated 2.6 s on / 1.1 s near-silent),
`music` (4 min, four partials with vibrato and a slow swell), `silence` (2 min digital zero), `soundcraft` (three 0.1 s fixtures from
`soundcraft/crates/audio-io/tests/fixtures/tone.{mp3,ogg,m4a}`, tiled to 5 min). Each is run native and tiled to 2 h (bucket vector repeated), at visible spans
whole / 1/4 / 1/16 / 30 s / 1 s with 1,000 columns, and the whole-clip span also at 3,840 columns.
No "real projects": filmcraft has no project corpus in `/Users/emm/ArtCraft/corpus` (it holds CAD and Office files only); these are synthetic-but-realistic audio, so this is L3 on method, not on input realism.
Soundcraft fixtures are 0.1 s long and are tiled, so they test the shape of a tone, not real content.

## Results by input class (geometric mean over that class's inputs, wall time, OLD=shipped scan)

| class | inputs | speedup [95% CI] | CPU-time speedup [95% CI] | publishable |
|---|---|---|---|---|
| speech | 12 | 2.69x [2.67, 2.71] | 2.68x [2.66, 2.71] | yes |
| music | 12 | 2.51x [2.48, 2.54] | 2.51x [2.47, 2.53] | yes |
| silence | 12 | 3.73x [3.60, 3.78] | 3.64x [3.41, 3.78] | yes |
| soundcraft (tiled) | 36 | 2.85x [2.81, 2.87] | 2.84x [2.81, 2.86] | yes |

By clip length and zoom, speech (music, silence and soundcraft agree within the CIs, apart from silence being faster on 2 h):

| input | OLD | NEW | speedup [95% CI] |
|---|---|---|---|
| 2 h, whole clip, 1,000 columns (the claim's shape) | 2.493 ms | 60.7 us | 41.08x [39.83, 42.04] |
| 2 h, whole clip, 3,840 columns | 2.602 ms | 226.7 us | 11.48x [11.20, 11.78] |
| 2 h, 1/4 visible | 651 us | 59.5 us | 10.95x [10.81, 11.32] |
| 2 h, 1/16 visible | 189 us | 46.8 us | 4.04x [3.90, 4.13] |
| 5 min native, whole, 1,000 columns | 133 us | 43.3 us | 3.08x [2.97, 3.14] |
| 5 min native, 30 s visible | 35.0 us | 31.1 us | 1.13x [1.09, 1.15] |
| 5 min native, 1 s visible | 18.7 us | 20.0 us | 0.93x [0.92, 0.96] (slower) |

The 1 s span is 0.78-0.97x on every non-silent input: the pyramid is a small constant slower than scanning 1-2 buckets. In absolute terms it is 1-8 us.

## Why the number is not 9.2x: the baseline

The claim's own bench (`examples/bench/waveform.rs`) did not time the shipped code as OLD. It timed a `(min, max)` fold, which the compiler vectorises;
the shipped loop is a dependent `m.max(a.abs()).max(b.abs())` chain, about 13x slower per bucket. Same synthetic 2 h buckets, 1,000 columns, 40 runs/side:

| OLD baseline | measured on | whole 2 h clip | geomean over the 5 spans |
|---|---|---|---|
| claim's min/max fold | columns only (mapping + reducer) | 12.57x [12.35, 12.73] | 2.42x [2.40, 2.43] |
| claim's min/max fold | with 1,000 x 90 pixel buffer | 3.30x [3.24, 3.33] | 1.43x [1.42, 1.44] |
| shipped abs-max chain | columns only | 110.8x [65.0, 163.2] | 7.44x [6.06, 8.27] |
| shipped abs-max chain | with pixel buffer | 42.27x [41.43, 43.33] | 4.22x [4.19, 4.25] |

The claimed 9.2x sits inside this range (12.6x on the reducer alone, 3.3x with a raster; the real painter's egui mesh overhead lies between). It is a
single-condition number: it needs a 2 h clip and a fully zoomed-out view. Raw: `data/waveform-claim-shape-*.json`. The "columns only" rows are 3-23 us, near timer
resolution at the narrow spans, so their CIs are wide there.

## Verdict

- Direction: confirmed. Zero output differences on 72 inputs, no CI that includes 1.0 except two columns-only rows at 1 s (inconclusive).
- 9.2x: reproduced only for a 2 h clip viewed whole (3.3x to 110x depending on the baseline and the shared raster cost). It is not a general figure.
  For 2-5 minute clips, which is a normal timeline, the gain is 1.2-3.2x at full zoom-out (1,000 to 3,840 columns) and about 1.0-1.15x at 30 s or less; at 1 s it is a 3-20% regression of a few microseconds.
- Correct status page wording: "waveform repaint up to ~40x on a 2 h clip zoomed out, ~3x on a 5-minute clip, no gain below 30 s visible; claim's 9.2x baseline was a vectorised fold, not the shipped loop".
- Not measured: the egui painter itself, `build` cost (one-time 6.5 ms per 2 h in the branch's own bench), real project audio.

Raw JSON: `data/waveform-{speech,music,silence,soundcraft}.json`, `data/waveform-claim-shape-*.json`; per-input table: `data/waveform-summary.md`.
