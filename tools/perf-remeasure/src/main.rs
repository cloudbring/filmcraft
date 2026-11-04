//! Re-measure of filmcraft waveform repaint, old flat scan vs new min/max pyramid (#106, #192).
//!
//! OLD is vendored: filmcraft `crates/ui-egui/src/panels/timeline/waveforms.rs::draw_waveform`
//! at merge-base eb59ec09daaea519c9c8fffa404a594ae50d00e0..., lines 38-42 (the per-column
//! `for (a, b) in peaks.iter().skip(s0).take(s1 - s0) { m = m.max(a.abs()).max(b.abs()) }`),
//! because `draw_waveform` is `pub(super)` and needs a full `FilmcraftApp`; nothing in the old
//! crate is callable from outside, so a renamed git dependency would expose no usable symbol.
//! NEW is `crates/ui-egui/src/waveform_peaks.rs` (`PeakPyramid::range`) included by `#[path]`.
//! Everything else in the repaint (column to bucket mapping, dynamic-waveform log scale, pixel fill)
//! is shared by both sides, so the only difference timed is the reducer.
//!
//! Usage: perf-remeasure WAVE_DIR OUT_DIR [reps]
use std::hint::black_box;
use std::sync::Arc;

use craft_bench::report::Revs;
use craft_bench::{Config, Paired};
// NEW: the committed source file itself, compiled in by path (it is std-only). Depending on the
// `filmcraft-ui-egui` crate would drag in the patched egui/eframe/wgpu stack (several GB) for a
// module that uses none of it.
#[path = "../../../crates/ui-egui/src/waveform_peaks.rs"]
mod waveform_peaks;
use waveform_peaks::PeakPyramid;

const BUCKET: usize = 256;
const SR: f64 = 48_000.0;
const HEIGHT: usize = 90;
const TWO_HOURS_BUCKETS: usize = 2 * 60 * 60 * 48_000 / BUCKET;

struct Clip {
    flat: Vec<(f32, f32)>,
    pyramid: PeakPyramid,
}

struct Case {
    clip: Arc<Clip>,
    start_s: f64,
    visible_s: f64,
    width: usize,
    /// false: time only column mapping + reducer (no pixel buffer)
    raster: bool,
}

#[derive(PartialEq, Debug)]
struct Out {
    /// per-column max(|min|,|max|), the value the old code computed
    amp: Vec<f32>,
    /// width x HEIGHT 8-bit pixel buffer
    pixels: Vec<u8>,
}

/// Same fold as `source_peaks` in waveforms.rs: min/max per 256 samples, final partial bucket kept.
fn buckets(samples: &[f32]) -> Vec<(f32, f32)> {
    samples.chunks(BUCKET).map(|c| c.iter().fold((0f32, 0f32), |(l, h), v| (l.min(*v), h.max(*v)))).collect()
}

fn clip(flat: Vec<(f32, f32)>) -> Arc<Clip> {
    let pyramid: PeakPyramid = flat.iter().copied().collect();
    Arc::new(Clip { flat, pyramid })
}

fn tile(b: &[(f32, f32)], n: usize) -> Vec<(f32, f32)> {
    b.iter().copied().cycle().take(n).collect()
}

fn repaint(case: &Case, reduce: impl Fn(usize, usize) -> f32) -> Out {
    let (w, vis) = (case.width, case.visible_s);
    let mut amp = Vec::with_capacity(w);
    let mut pixels = if case.raster { vec![0u8; w * HEIGHT] } else { Vec::new() };
    for x in 0..w {
        let t0 = case.start_s + vis * x as f64 / w as f64;
        let t1 = case.start_s + vis * (x + 1) as f64 / w as f64;
        let s0 = (t0 * SR / BUCKET as f64) as usize;
        let s1 = ((t1 * SR / BUCKET as f64) as usize).max(s0 + 1);
        let m = reduce(s0, s1);
        amp.push(m);
        // dynamic waveform scale as in draw_waveform: -48 dB..0 dB -> 0..full height
        let h = (((20.0 * m.max(1e-5).log10() + 48.0) / 48.0).clamp(0.0, 1.0) * HEIGHT as f32) as usize;
        for y in 0..if case.raster { h.min(HEIGHT) } else { 0 } {
            pixels[(HEIGHT - 1 - y) * w + x] = 255;
        }
    }
    Out { amp, pixels }
}

fn old(case: &Case) -> Out {
    let peaks = &case.clip.flat;
    repaint(case, |s0, s1| {
        let mut m = 0f32;
        for (a, b) in black_box(peaks).iter().skip(s0).take(s1 - s0) {
            m = m.max(a.abs()).max(b.abs());
        }
        m
    })
}

/// The flat baseline used by the claim's own bench (`crates/ui-egui/examples/bench/waveform.rs`
/// at 529c5c1e): a (min, max) fold, not the shipped abs-max chain above. Kept to explain the gap.
fn old_minmax_fold(case: &Case) -> Out {
    let peaks = &case.clip.flat;
    repaint(case, |s0, s1| {
        let (low, high) = black_box(peaks).iter().skip(s0).take(s1 - s0).fold((0f32, 0f32), |(low, high), &(min, max)| (low.min(min), high.max(max)));
        low.abs().max(high.abs())
    })
}

fn new(case: &Case) -> Out {
    let p = &case.clip.pyramid;
    repaint(case, |s0, s1| {
        let (low, high) = black_box(p).range(s0..s1);
        low.abs().max(high.abs())
    })
}

fn load(dir: &str, name: &str) -> Vec<f32> {
    let b = std::fs::read(format!("{dir}/{name}")).unwrap_or_else(|e| panic!("{name}: {e}"));
    b.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (dir, out_dir) = (a[1].as_str(), a[2].as_str());
    let reps: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(40);
    let revs = Revs { old: Some("eb59ec09daaea519c9c8fffa404a594ae50d00e0".into()), new: Some("529c5c1e8e4b6be393e147bc6316a4643dda8529".into()) };
    std::fs::create_dir_all(out_dir).unwrap();
    // (class, files). Soundcraft fixtures are 0.1 s, so they are tiled to 5 min.
    let classes: [(&str, Vec<&str>); 4] = [
        ("speech", vec!["speech.f32"]),
        ("music", vec!["music.f32"]),
        ("silence", vec!["silence.f32"]),
        ("soundcraft", vec!["sc_tone_mp3.f32", "sc_tone_ogg.f32", "sc_tone_m4a.f32"]),
    ];
    let mut md = String::new();
    for (class, files) in classes {
        let mut inputs: Vec<(String, Case)> = Vec::new();
        for f in files {
            let mut b = buckets(&load(dir, f));
            if class == "soundcraft" {
                b = tile(&b, 5 * 60 * 48_000 / BUCKET);
            }
            let native_s = b.len() as f64 * BUCKET as f64 / SR;
            let two_h = clip(tile(&b, TWO_HOURS_BUCKETS));
            let native = clip(b);
            for (len, c, total) in [("native", native, native_s), ("2h-tiled", two_h, 7200.0)] {
                let label = f.trim_end_matches(".f32");
                let mut spans: Vec<(String, f64)> = vec![("whole".into(), total), ("1/4".into(), total / 4.0), ("1/16".into(), total / 16.0)];
                for s in [30.0, 1.0] {
                    if s < total {
                        spans.push((format!("{s}s"), s));
                    }
                }
                for (zname, vis) in spans {
                    let widths: &[usize] = if zname == "whole" { &[1000, 3840] } else { &[1000] };
                    for &width in widths {
                        inputs.push((
                            format!("{label}/{len}({total:.0}s)/{zname}/w{width}"),
                            Case { clip: c.clone(), start_s: (total - vis) * 0.37, visible_s: vis, width, raster: true },
                        ));
                    }
                }
            }
        }
        // Output equivalence: every input, before any timing (also enforced by .check inside the harness).
        for (n, c) in &inputs {
            assert_eq!(old(c), new(c), "output differs on {n}");
        }
        eprintln!("{class}: {} inputs, outputs identical", inputs.len());
        let cfg = Config { repetitions: reps, resamples: 4000, revs: revs.clone(), ..Config::default() };
        let report = Paired::new(format!("filmcraft waveform repaint, {class}"))
            .old(old)
            .new(new)
            .inputs(inputs)
            .check(|a: &Out, b: &Out| a == b)
            .run(&cfg)
            .expect("bench");
        std::fs::write(format!("{out_dir}/waveform-{class}.json"), report.to_json()).unwrap();
        md.push_str(&summary(class, &report));
        eprintln!("{class}: publishable={} contended={}", report.publishable, report.contended);
    }
    // The claim's own shape: synthetic 2 h clip, 1000 columns, visible spans 7200/1800/300/30/1 s,
    // exactly the buckets of examples/bench/waveform.rs.
    let synth = clip((0..TWO_HOURS_BUCKETS).map(|i| {
        let a = ((i * 17 % 997) + 1) as f32 / 997.0;
        (-a, a * 0.75)
    }).collect());
    let shape = |raster: bool| -> Vec<(String, Case)> {
        [7200.0, 1800.0, 300.0, 30.0, 1.0]
            .into_iter()
            .map(|v| (format!("synthetic-2h/{v}s/w1000{}", if raster { "" } else { "/columns-only" }), Case { clip: synth.clone(), start_s: (7200.0 - v) * 0.37, visible_s: v, width: 1000, raster }))
            .collect()
    };
    for (label, f, raster) in [
        ("claim-shape-shipped-scan", old as fn(&Case) -> Out, true),
        ("claim-shape-minmax-fold", old_minmax_fold, true),
        ("claim-shape-shipped-scan-columns-only", old, false),
        ("claim-shape-minmax-fold-columns-only", old_minmax_fold, false),
    ] {
        let inputs = shape(raster);
        for (n, c) in &inputs {
            assert_eq!(f(c), new(c), "output differs on {n}");
        }
        let cfg = Config { repetitions: reps, resamples: 4000, revs: revs.clone(), ..Config::default() };
        let report = Paired::new(format!("filmcraft waveform repaint, {label}")).old(f).new(new).inputs(inputs).check(|a: &Out, b: &Out| a == b).run(&cfg).expect("bench");
        std::fs::write(format!("{out_dir}/waveform-{label}.json"), report.to_json()).unwrap();
        md.push_str(&summary(label, &report));
        eprintln!("{label}: publishable={} contended={}", report.publishable, report.contended);
    }
    std::fs::write(format!("{out_dir}/waveform-summary.md"), md).unwrap();
}

/// speedup = old/new = 1/ratio; the CI inverts too.
fn summary(class: &str, r: &craft_bench::report::Report) -> String {
    let inv = |x: &craft_bench::report::Ratio| format!("{:.2}x [{:.2}, {:.2}]", 1.0 / x.point, 1.0 / x.ci_hi, 1.0 / x.ci_lo);
    let mut s = format!(
        "\n### {class}  (publishable: {}, contended: {}, load {:?}->{:?}, {} runs/side)\n\n| input | old median | new median | wall speedup [95% CI] | cpu speedup [95% CI] | verdict |\n|---|---|---|---|---|---|\n",
        r.publishable, r.contended, r.guard.load_before, r.guard.load_after, r.config.runs_per_side
    );
    for i in &r.inputs {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            i.name,
            craft_bench::report::fmt_ns(i.wall.old.median_ns),
            craft_bench::report::fmt_ns(i.wall.new.median_ns),
            inv(&i.wall.ratio),
            i.cpu.as_ref().map(|c| inv(&c.ratio)).unwrap_or_default(),
            i.wall.ratio.verdict().label()
        ));
    }
    s.push_str(&format!("| **geomean** | | | **{}** | {} | |\n", inv(&r.aggregate.wall), r.aggregate.cpu.as_ref().map(inv).unwrap_or_default()));
    s
}
