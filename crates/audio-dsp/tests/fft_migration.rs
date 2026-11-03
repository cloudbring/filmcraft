use filmcraft_audio_dsp::{AudioEffect, effects::FftFilter};
use std::{hint::black_box, time::Instant};

fn signal(length: usize) -> Vec<f32> {
    (0..length).map(|index| ((index * 17 % 101) as f32 - 50.0) / 200.0).collect()
}

#[test]
fn spectral_effects_do_not_allocate() {
    for id in ["fft_filter", "pitch_shifter", "denoise", "dereverb", "convolution_reverb"] {
        for rate in [48_000, 96_000, 192_000] {
            let mut effect = filmcraft_audio_dsp::create_effect(id, rate as f32, 1).unwrap();
            let mut samples = signal(16_384);
            let allocations = allocation_counter::measure(|| {
                effect.process(&mut [&mut samples]);
                effect.reset();
                effect.process(&mut [&mut samples]);
            });
            assert_eq!(allocations.count_total, 0, "{id} at {rate} Hz");
        }
    }
}

#[test]
#[ignore = "manual before/after comparison; set FILMCRAFT_FFT_BASELINE and optionally FILMCRAFT_FFT_RECORD"]
fn filter_migration_vectors() {
    let root = std::path::PathBuf::from(std::env::var_os("FILMCRAFT_FFT_BASELINE").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    for rate in [48_000, 96_000, 192_000] {
        for shaped in [false, true] {
            let mut filter = FftFilter::new(rate as f32, 1);
            if shaped {
                assert!(filter.set_param("p5_gain", -30.0));
                assert!(filter.set_param("interp", 0.0));
            }
            let mut samples = signal(65_536);
            for block in samples.chunks_mut(127) {
                filter.process(&mut [block]);
            }
            let path = root.join(format!("filter-{rate}-{shaped}.f32le"));
            if std::env::var_os("FILMCRAFT_FFT_RECORD").is_some() {
                let bytes: Vec<u8> = samples.iter().flat_map(|sample| sample.to_le_bytes()).collect();
                std::fs::write(path, bytes).unwrap();
            } else {
                let bytes = std::fs::read(path).unwrap();
                assert_eq!(bytes.len(), samples.len() * 4);
                let mut worst = 0.0f32;
                let mut changed = 0;
                for (sample, bytes) in samples.iter().zip(bytes.as_chunks::<4>().0) {
                    let previous = f32::from_le_bytes(*bytes);
                    assert!(sample.is_finite());
                    worst = worst.max((sample - previous).abs());
                    changed += usize::from(sample.to_bits() != previous.to_bits());
                }
                eprintln!("filter rate={rate} shaped={shaped}: {changed}/{} changed, max error {worst:e}", samples.len());
                assert!(worst <= 1e-6);
            }
        }
    }
}

#[test]
#[ignore = "manual warmed STFT benchmark; run in release with --nocapture --test-threads=1"]
fn stft_timing() {
    for rate in [48_000, 96_000, 192_000] {
        let mut filter = FftFilter::new(rate as f32, 1);
        let input = signal(65_536);
        let mut samples = input.clone();
        filter.process(&mut [&mut samples]);
        let mut timings = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            for _ in 0..30 {
                samples.copy_from_slice(&input);
                filter.process(&mut [black_box(&mut samples)]);
                black_box(&samples);
            }
            timings.push(start.elapsed().as_secs_f64() * 1e6 / 30.0);
        }
        timings.sort_by(f64::total_cmp);
        eprintln!("STFT {rate} Hz: {:.3} us / 65536 samples (median of 7 x 30)", timings[3]);
    }
}
