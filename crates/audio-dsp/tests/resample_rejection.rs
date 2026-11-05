//! Image rejection of the true-peak interpolator, old core against new
//! (cloudbring/artcraft-algorithms#202 "67 -> 150 dB" claim, measured under #224).
//!
//! Method (coherent single-tone sweep, rectangular window, no leakage):
//! * Input `N = 8192` samples at rate `fs` of `0.5 sin(2 pi k n / N)` (bin `k`, an exact number of
//!   cycles), rounded to f32 like real audio, preceded by 256 warm-up samples of the same tone.
//! * The interpolator turns each input into `factor` outputs; interleaved they are the signal at
//!   `factor * fs`, `N * factor` samples whose DFT bin spacing equals the input's, so the tone is
//!   still bin `k` and everything else is a spur.
//! * Bins up to `N/2` are baseband (noise floor of f32 arithmetic and coefficient rounding); bins
//!   above `N/2` hold the images of the tone around multiples of `fs`. Reported per tone:
//!   passband gain (dB), the largest image bin (dBc), the largest baseband spur (dBc).
//! * Summary: worst image over the swept tones up to 0.4 fs (the audible/metered band), and the
//!   weakest image rejection.
//!
//! Run `cargo test --release -p filmcraft-audio-dsp --test resample_rejection -- --nocapture`.

use filmcraft_audio_dsp::oversample::{Interpolator, MAX_FACTOR, PolyphaseBank};

#[path = "old/oversample.rs"]
mod old;

const N: usize = 8_192;
const WARM: usize = 256;
const AMP: f64 = 0.5;

/// In-place radix-2 FFT (f64), `re.len()` a power of two.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -std::f64::consts::TAU / len as f64;
        for start in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (wr, wi) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                (re[a], im[a]) = (re[a] + tr, im[a] + ti);
            }
        }
        len <<= 1;
    }
}

fn db(r: f64) -> f64 {
    20.0 * r.max(1e-30).log10()
}

/// Returns (gain dB, worst image dBc, worst baseband spur dBc) for a tone at bin `k`.
fn measure(factor: usize, k: usize, phases: &mut dyn FnMut(f32, &mut [f32])) -> (f64, f64, f64) {
    let tone = |n: usize| (AMP * (std::f64::consts::TAU * (k * n) as f64 / N as f64).sin()) as f32;
    let mut out = [0.0f32; MAX_FACTOR];
    for n in 0..WARM {
        phases(tone(N - WARM + n), &mut out); // the tone is N-periodic, so this is its true past
    }
    let mut re = Vec::with_capacity(N * factor);
    for n in 0..N {
        phases(tone(n), &mut out);
        re.extend(out[..factor].iter().map(|&v| f64::from(v)));
    }
    let mut im = vec![0.0; re.len()];
    fft(&mut re, &mut im);
    let mag: Vec<f64> = re.iter().zip(&im).map(|(r, i)| r.hypot(*i) * 2.0 / N as f64).collect();
    let fund = mag[k];
    // N bins per input rate: output bin b is b * fs / N; the output Nyquist is factor * N / 2.
    let half_out = mag.len() / 2;
    let (mut image, mut base) = (0.0f64, 0.0f64);
    for (b, &m) in mag.iter().enumerate().take(half_out + 1).skip(1) {
        if b == k {
            continue;
        }
        if b > N / 2 {
            image = image.max(m);
        } else {
            base = base.max(m);
        }
    }
    (db(fund / AMP / factor as f64), db(image / fund), db(base / fund))
}

fn run(label: &str, factor: usize, mut make: impl FnMut() -> Box<dyn FnMut(f32, &mut [f32])>) -> (f64, f64) {
    println!("== {label}, factor {factor}: tone (fraction of fs), gain dB, worst image dBc, worst baseband spur dBc");
    let (mut worst, mut worst_all) = (-400.0f64, -400.0f64);
    for frac in [0.02, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45] {
        let k = (frac * N as f64) as usize;
        let mut ip = make();
        let (gain, image, base) = measure(factor, k, &mut *ip);
        println!("  {:.4} {:8.4} {:9.1} {:9.1}", k as f64 / N as f64, gain, image, base);
        worst_all = worst_all.max(image);
        if frac <= 0.4 {
            worst = worst.max(image);
        }
    }
    println!("  summary: worst image up to 0.4 fs {worst:.1} dBc (weakest rejection {:.1} dB); incl. 0.45 fs {worst_all:.1} dBc", -worst);
    (worst, worst_all)
}

#[test]
fn old_and_new_reject_alike() {
    for factor in [4, 2] {
        let ob = old::PolyphaseBank::new(factor);
        let nb = PolyphaseBank::new(factor);
        let (o, _) = run("old core (vendored pre-#224)", factor, || {
            let (mut ip, bank) = (old::Interpolator::default(), ob.clone());
            Box::new(move |x, out| ip.phases(&bank, x, &mut out[..factor]))
        });
        let (n, _) = run("new core (craft-resample)", factor, || {
            let (mut ip, bank) = (Interpolator::default(), nb.clone());
            Box::new(move |x, out| ip.push_phases(&bank, x, out))
        });
        assert!((o - n).abs() < 3.0, "factor {factor}: old {o} vs new {n}");
    }
}
