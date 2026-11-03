# filmcraft-h264enc

A clean-room, pure-Rust H.264/AVC encoder for FilmCraft's export path, written from the public
ITU-T H.264 specification (04/2013 edition) and general encoder literature. Layer L0: it depends only on
`filmcraft-bitstream`, `thiserror`, `wide` and (optionally, feature `threads`, on by default) `rayon`. No `unsafe`,
builds for `wasm32-unknown-unknown` with or without `threads`.

## Usage

```rust
use filmcraft_h264enc::{Encoder, EncoderConfig, PacketFormat, Preset, Profile, RateControl, YuvFrame};

let mut cfg = EncoderConfig::new(1920, 1080, 30000, 1001);
cfg.profile = Profile::High;
cfg.preset = Preset::Balanced;              // Speed / Balanced / Quality
cfg.rate = RateControl::Crf(20.0);          // Crf(f32) | Cbr { kbps } | Vbr { target_kbps, max_kbps } | Qp(u8)
cfg.bframes = 2;                            // 0..=3
cfg.keyint = 250;
cfg.format = PacketFormat::LengthPrefixed;  // MP4 samples; or AnnexB (default) for .h264
let mut enc = Encoder::new(cfg)?;
let avcc = enc.avcc();                      // AVCDecoderConfigurationRecord for the MP4 `avcC` box
for (i, pic) in pictures.iter().enumerate() {
    let frame = YuvFrame { y: &pic.y, u: &pic.u, v: &pic.v, y_stride: w, uv_stride: w / 2 };
    for p in enc.encode(&frame, i as i64) { /* p.data, p.pts, p.dts, p.keyframe, p.frame_type */ }
}
for p in enc.flush() { /* ... */ }
```

- Input: planar 4:2:0 8-bit (`YuvFrame` with strides); any even size, non-multiple-of-16 sizes are
  cropped via the SPS frame cropping. `try_encode` returns an error instead of panicking on short planes.
- Output: one `Packet` per picture in decoding order with `pts` (as given), `dts` (reorder-delayed, always
  `<= pts` and strictly increasing), `keyframe` (IDR) and the frame type/QP. Annex-B packets carry an AUD,
  SPS/PPS on every IDR and an identification SEI on the first picture.
- Two-pass: encode once with `cfg.pass = Pass::First`, take `enc.pass_stats()` (serialisable with
  `PassStats::to_text/from_text`), then encode again with `Pass::Second(stats)`.
- `set_recon_capture(true)` + `take_recon()` expose the encoder's reconstructed pictures (used by the tests to
  prove encoder/decoder match).

## Features

| Area | Implemented |
|---|---|
| Profiles | Constrained Baseline (CAVLC, I/P), Main (CABAC, I/P/B), High (CABAC, 8x8 transform, I/P/B); 4:2:0 8-bit, frame coding |
| Parameter sets | SPS (level auto-selected from size/rate/DPB/bitrate, cropping, VUI: SAR, colour description + range, timing, bitstream restriction/reorder), PPS, AUD, user-data SEI, avcC |
| Intra | 16x16 (4 modes, SATD), 4x4 (9 modes, SATD + mode-cost, sequential reconstruction), 8x8 (9 modes with reference filtering, High), chroma (4 modes) |
| Inter P | hexagon integer search from predictors, half- then quarter-sample refinement (SATD) on precomputed 6-tap half-sample planes; 16x16/16x8/8x16/8x8 partitions (Balanced/Quality); P_Skip detection with the normative skip MV |
| Inter B | 1–3 non-reference B-frames, spatial direct / B_Skip, L0/L1/Bi 16x16 and (Balanced/Quality) 16x8/8x16 with a per-partition direction; all B-frames of a mini-GOP are encoded concurrently |
| Residual | 4x4 / 8x8 integer transforms, Intra16x16 and chroma DC Hadamards, deadzone quantisation (intra 1/3, inter 1/6), adaptive 4x4/8x8 inter transform choice, coefficient decimation for inter blocks |
| Entropy | CABAC (all I and cabac_init_idc 0 context tables, verified against the spec text) and CAVLC (all coeff_token/total_zeros/run_before tables, level escapes) |
| Loop filter | normative deblocking (bS 0–4 incl. bi-predicted rules, 8x8-transform edges, chroma QP mapping) in the reconstruction loop |
| Mode decision | SATD + λ·bits for Speed/Balanced; Quality adds RD-lite (SSD + λ·CABAC-estimated bits) between the best intra/inter/skip candidates |
| Rate control | `Qp`, `Crf` (constant quality with I/P/B offsets), 1-pass ABR with complexity model + VBV (`Cbr`, 1-pass `Vbr`), closed-loop 2-pass VBR; variance adaptive quantisation (`aq_strength`), lookahead scene-cut IDR insertion |
| Threading | slice-parallel encoding (MB-row slices, `disable_deblocking_filter_idc = 2` with >1 slice so each slice is also deblocked in parallel), parallel half-sample interpolation and lookahead |

## Verification

### Distortion kernels (#99)

`src/distortion.rs` centralizes SAD, SATD and SSE/SSD for motion search, intra/inter mode
decision, RD-lite and lookahead. SAD and SSE support widths 4/8/16 and heights up to 16,
including 4x4, 8x8, 16x16, 16x8 and 8x16. SATD supports dimensions divisible by four
through 16x16, always summing **halved 4x4 Hadamards**: the 8x8 score is four 4x4
scores, not a new 8x8 transform. This preserves the existing mode decisions.

All vector types are 128-bit (`wide`, permissively licensed, no default features): NEON
on AArch64, SSE2 on x86-64, wasm SIMD128 only with `-C target-feature=+simd128`, and a
scalar fallback otherwise. No target-specific intrinsics or unsafe code are added.
Bounds and checked stride arithmetic are validated before the loops; invalid geometry
receives the maximum cost rather than a partial score. The maximum 16x16 SAD is 65,280,
which fits the unsigned 16-bit reduction; SSE widens signed 16-bit differences into
32-bit dot products before accumulation (maximum block SSE 16,646,400).

Differential tests cover all 65,536 constant byte pairs for all nine 4/8/16-by-4/8/16
block shapes, deterministic random blocks with independent strides and unaligned starts, exact-length
buffers, zero/full blocks, checkerboards, single-pixel impulses, overflow bounds and
invalid geometry. The existing four pinned coded-slice hashes remain unchanged.
Run `cargo test --release -p filmcraft-h264enc --lib distortion` and the optional
`cargo test --release -p filmcraft-h264enc --lib benchmark_kernels -- --ignored --nocapture`.

Measured on 2026-11-03, Apple M6/AArch64, rustup stable rustc 1.99.0, release defaults:
`THREADS=1 cargo run --release -p filmcraft-h264enc --example h264enc_synth --
1920 1080 30 high balanced 2 qp26 1 synth`. All 30 input frames are distinct; input
generation is outside the encode timer. Five interleaved runs of the pre-change
encoder and the SIMD encoder gave median **21.2 -> 23.5 fps** (about 11% faster).
Baseline times were 1.417/1.400/1.393/1.419/1.460 s; SIMD times were
1.256/1.259/1.334/1.281/1.274 s. Both executables were built with explicit rustup
`RUSTC`/`RUSTDOC` paths (the baseline from a temporary HEAD source snapshot).
This is a shared machine, not a claim of a stable platform-wide gain.
Every run produced the same 2,804,462-byte stream,
compared byte-for-byte against the pre-change output. No distortion-time percentage
was measured. The optional microbenchmark compares against the deliberately simple
test reference, not the previous compiler-autovectorized production loops.

Validation on that toolchain: h264enc debug, release and release without default
features each passed 37 tests (one ignored microbenchmark, separately run and passed),
with strict external ffmpeg/ffprobe oracles enabled. Workspace clippy (all targets,
warnings denied), formatting, layers, assets (69) and wasm (49 crates) passed, as did
h264enc checks for wasm without default features, wasm SIMD128 and x86-64 SSE2.
The full workspace test run stopped at the unrelated DNx oracle tests: 806 passed,
two failed and 15 ignored before stopping. Both failures
(`oracle_decode::decode_matches_ffmpeg`, `oracle_encode::encode_decodes_in_ffmpeg`)
were reproduced against the unchanged pre-change DNx source with the same oracles;
the remaining workspace tests were not reached.

### Cached sub-pel reference planes (#111)

The prepared baseline `dc23d047` already implements the requested amortisation:
`Encoder::finalise_job` extends the reconstructed reference's edges and calls
`build_hpel` once before inserting it into the decoded-picture buffer. The H, V
and HV planes cover the padded reference and are shared by every block and slice;
their storage is recycled with the reference-picture pool. Search does not run
the six-tap filter per candidate. A second block/row cache would duplicate this work.

`RefPic::luma_ref` returns direct, strided plane views for integer/half-pel
candidates. Quarter-pel candidates use the two samples selected by `QPEL_SRC`
and `(a + b + 1) >> 1`, as in ITU-T H.264 (04/2013), section 8.4.2.2.1.
In particular, odd/odd phases average the appropriate H/V samples, not four
half-grid neighbours. HV filtering retains unrounded horizontal intermediates
until the final `(+512) >> 10` and clipping. Candidate costs use #99's SIMD SATD.
The candidate set, traversal, visited-set behaviour, MV penalties and strict
less-than tie-breaking are unchanged.

This ticket therefore adds a regression lock, not another production cache.
An independent test oracle filters the clamped integer reference afresh for
every candidate, without using `build_hpel`, `QPEL_SRC` or cached samples.
It checks prediction bytes, motion compensation and SIMD SATD on 36,288 blocks:
all 16 fractional phases, nine rectangular 4/8/16 block sizes, fourteen interior,
edge and corner placements (including negative motion vectors), three picture
sizes including 2x2, and six random/constant/checkerboard/impulse patterns.
Reusing the planes across changing reference pictures is covered. Integer/half
positions also assert that the prediction scratch buffer is untouched.

Run `cargo test --release -p filmcraft-h264enc --lib cached_plane_costs`.
The optional `benchmark_cached_plane_costs` ignored test compares cached costs
with the deliberately simple per-candidate oracle; it does not represent the
pre-change production encoder. A fused quarter-pel-average/SATD experiment was
discarded because it regressed the cached-cost microbenchmark and the paired
end-to-end benchmark. No production hot-loop changes from that experiment remain.

Measured on 2026-11-03 with the same #99 command and explicit rustup stable
`RUSTC`, `RUSTDOC` and toolchain-first `PATH`, using
`CARGO_TARGET_DIR=/Users/emm/ArtCraft/worktrees/filmcraft-111/target-111`:

```sh
THREADS=1 cargo run --release -p filmcraft-h264enc --example h264enc_synth -- \
  1920 1080 30 high balanced 2 qp26 1 synth
```

Five interleaved baseline/final runs measured median **23.1 -> 23.8 fps**;
baseline seconds were 1.298/1.347/1.283/1.221/1.316, final seconds were
1.298/1.258/1.344/1.262/1.234. The median paired time ratio was **1.000x**.
Production code is unchanged, so the median-FPS difference is measurement noise,
not an optimisation gain. The baseline and final executables are themselves
byte-identical (SHA-256 `8e0aa600827efcfbea7387b132aeb3b302725a9467b4e782ec20ea12fe8a6559`).
All ten streams were byte-identical (2,804,462 bytes;
SHA-256 `c0eefc741a0a932fede31e2f349ca75277e9f7e81341467df5751f8eec4a7494`).
Each decoded to 30 frames with ffmpeg 9.0.2, no diagnostics or reconstruction
mismatches, 38.40 dB mean luma PSNR and 22,435.7 kbit/s. PSNR and bitrate deltas
are zero; no candidate-set or tie-breaking changes were made.

Validation: h264enc debug, release and release without default features each
passed 38 tests, including all four existing pinned hashes and strict external
ffmpeg/ffprobe oracles (`FILMCRAFT_REQUIRE_ORACLES=1`); two timing-only tests were
ignored by default and the new cached-plane benchmark was run separately and
passed. Workspace clippy (`--all-targets -- -D warnings`), fmt, layers, assets
(69 attributed assets), and wasm (49 crates) passed with the rustup toolchain.
The full workspace test run stopped after 326 passed, one failed and three
ignored: the unchanged AV1 `hostile_streams_never_panic` fixture generator asks
for 64x48, but this ffmpeg distribution bundles SVT-AV1 1.8, whose minimum is
64x64. This is an oracle-environment failure before AV1 decoding, unrelated to
h264enc; the remaining workspace suites were not reached. No unrelated codec
or test changes were made.

### Encoder conformance

`cargo test --release -p filmcraft-h264enc` (ffmpeg/ffprobe used only as external oracles; tests skip when
absent). Every stream is decoded with `ffmpeg -v warning -err_detect +crccheck+bitstream+buffer+explode -f rawvideo`
and must print nothing (not even a `corrupt decoded frame` warning), produce exactly the number of frames encoded,
and be **bit-identical to the encoder's reconstruction** — this covers Baseline/Main/High, all presets, 0–3
B-frames, 1–4 slices and automatic slicing at 1080p, odd sizes, `testsrc2` and `mandelbrot` sources, and
length-prefixed output re-wrapped with the avcC parameter sets. B-frame display order is checked with ffprobe. A
negative control drops and truncates slices and checks that the oracle reports them.

The oracle leaves ffmpeg's error concealment on. With `-ec 0`, ffmpeg 7.1 and 8.1 print `corrupt decoded frame`
for every picture that has more than one slice, libx264's multi-slice streams included, while the decoded pixels
are unchanged (#74). The bit-exact comparison is what rules out concealment.

Results on the synthetic content (moving gradient, grain noise, text-like edges, sub-pixel panning):

| Test | Result |
|---|---|
| 1080p, High, Speed, CRF 20, 2 B | mean luma PSNR 43.0 dB (IDR 39.1, P/B 43.2–44.5) |
| 640x360 CBR 1500 kbit/s | 1518 kbit/s (+1.2%) |
| 640x360 1-pass VBR 1000 kbit/s | 981 kbit/s (−1.9%) |
| 640x360 2-pass VBR 800 kbit/s | 840 kbit/s (+5.0%) |

Development tool: `cargo run --release -p filmcraft-h264enc --example h264enc_synth -- W H FRAMES
[profile] [preset] [bframes] [qpN|crfN|cbrN|vbrN] [slices] [synth|testsrc2|mandelbrot]` encodes, decodes with
ffmpeg and reports speed, bitrate, PSNR and any encoder/decoder mismatch (`MAP=1` prints a per-MB mismatch map,
`PERF=k` cycles k source pictures for speed runs).

## Limitations / not yet implemented

- Single reference frame per list; no B-pyramid, no weighted prediction, no interlaced/MBAFF, no 4:2:2/4:4:4 or
  high bit depth, no custom scaling matrices, no sub-8x8 partitions, no B_8x8.
- CAVLC is used only for Baseline (High/Main always use CABAC); 8x8 transform is therefore CABAC-only.
- No trellis quantisation, no psy-RD; no HRD/buffering-period SEI (VBV is enforced internally only).
- Threading is slice-based only (no frame-level pipelining); with many slices, slice edges are not deblocked.
