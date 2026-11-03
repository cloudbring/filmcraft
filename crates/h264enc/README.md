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
