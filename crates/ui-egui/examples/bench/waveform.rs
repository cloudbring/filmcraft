use std::hint::black_box;
use std::time::Instant;

use filmcraft_ui_egui::waveform_peaks::PeakPyramid;
use serde_json::{Value, json};

use crate::Opts;
use crate::playback::{ms, pct};

pub fn run(options: &Opts) -> Vec<Value> {
    let buckets: usize = 2 * 60 * 60 * 48_000 / 256;
    let peaks: Vec<(f32, f32)> = (0..buckets)
        .map(|index| {
            let amplitude = ((index * 17 % 997) + 1) as f32 / 997.0;
            (-amplitude, amplitude * 0.75)
        })
        .collect();
    let started = Instant::now();
    let pyramid: Option<PeakPyramid> = (options.only.as_deref() != Some("flat")).then(|| peaks.iter().copied().collect());
    let build_ms = ms(started.elapsed());
    let context = egui::Context::default();
    let columns = 1000;
    let frames = if options.quick { 40 } else { 200 };
    let mut rows = Vec::new();
    for _ in 0..options.repeat {
        for visible_s in [7200.0, 1800.0, 300.0, 30.0, 1.0] {
            let mut times = Vec::new();
            for frame in 0..frames + 10 {
                let started = Instant::now();
                let mut output = context.run_pass(egui::RawInput::default(), |context| {
                    let painter = context.layer_painter(egui::LayerId::background());
                    let mut mesh = egui::Mesh::default();
                    let start = (7200.0 - visible_s) * 0.37;
                    for column in 0..columns {
                        let first = ((start + visible_s * column as f64 / columns as f64) * 48_000.0 / 256.0) as usize;
                        let end = (((start + visible_s * (column + 1) as f64 / columns as f64) * 48_000.0 / 256.0) as usize).max(first + 1);
                        let (low, high) = match &pyramid {
                            Some(pyramid) => black_box(pyramid).range(first..end),
                            None => black_box(&peaks)
                                .iter()
                                .skip(first)
                                .take(end - first)
                                .fold((0f32, 0f32), |(low, high), &(min, max)| (low.min(min), high.max(max))),
                        };
                        if high - low > 0.002 {
                            mesh.add_colored_rect(
                                egui::Rect::from_min_max(egui::pos2(column as f32, 100.0 - high * 90.0), egui::pos2(column as f32 + 1.0, 101.0 - low * 90.0)),
                                egui::Color32::GREEN,
                            );
                        }
                    }
                    painter.add(mesh);
                });
                output.textures_delta.clear();
                black_box(output);
                if frame >= 10 {
                    times.push(ms(started.elapsed()));
                }
            }
            rows.push(json!({
                "visible_s": visible_s, "columns": columns, "frames": frames,
                "clip_hours": 2, "sample_rate": 48_000, "channels": 2,
                "method": if pyramid.is_some() { "pyramid" } else { "flat" },
                "pyramid_build_ms": build_ms,
                "pair_payload_bytes": if pyramid.is_some() { (2 * buckets - buckets.count_ones() as usize) * 8 } else { buckets * 8 },
                "repaint_p50_ms": pct(&times, 0.5), "repaint_p95_ms": pct(&times, 0.95),
            }));
        }
    }
    rows
}
