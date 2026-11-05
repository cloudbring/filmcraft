//! cloudbring/artcraft-algorithms#224: the pre-migration true-peak interpolator (vendored in
//! `tests/old/oversample.rs`) against `oversample`, which now runs on `craft-resample`.
//!
//! What changed in the arithmetic. The prototype and its per-phase normalisation are the same
//! (`kaiser_prototype(factor, 1, 16, 8.0)` is the old design, so the f32 coefficients must be equal
//! bit for bit; checked below). The old `push_peak` summed each phase with eight partial sums
//! (vectorisable); `craft-resample` sums taps in order, newest sample first, which is what the old
//! code's sequential reference did. So: new == old sequential bit for bit, and new == old
//! vectorised to f32 summation-order rounding (a few ulps, tolerance below).
//!
//! Ratios. The shipping filmcraft polyphase is an integer interpolator (M = 1) with
//! `true_peak_factor`: 4x at 44.1/48/22.05 kHz, 2x at 88.2/96 kHz, 1x above. Those are the rates
//! exercised. (The `StreamResampler` on origin/main is linear interpolation, not polyphase; the
//! sinc `StreamResampler` of fork branch algo/46 is not merged and not a rational polyphase.)

use filmcraft_audio_dsp::oversample::{Interpolator, MAX_FACTOR, PolyphaseBank, TAPS_PER_PHASE, true_peak_factor};

#[path = "old/oversample.rs"]
mod old;

/// Deterministic white noise in [-0.9, 0.9] (xorshift64*).
fn noise(len: usize, seed: u64) -> Vec<f32> {
    let mut s = seed | 1;
    (0..len)
        .map(|_| {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            let v = s.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 40;
            (v as f64 / (1u64 << 24) as f64 * 1.8 - 0.9) as f32
        })
        .collect()
}

fn sine(len: usize, rate: f64, hz: f64) -> Vec<f32> {
    (0..len).map(|n| (0.8 * (std::f64::consts::TAU * hz * n as f64 / rate).sin()) as f32).collect()
}

fn chirp(len: usize, rate: f64) -> Vec<f32> {
    let (f0, f1, t) = (50.0, 0.45 * rate, len as f64 / rate);
    (0..len)
        .map(|n| {
            let tn = n as f64 / rate;
            (0.8 * (std::f64::consts::TAU * (f0 * tn + (f1 - f0) * tn * tn / (2.0 * t))).sin()) as f32
        })
        .collect()
}

/// Impulses further apart than the filter is long: each output has at most one non-zero term.
fn impulses(len: usize) -> Vec<f32> {
    let mut v = vec![0.0; len];
    for (k, at) in [0, 100, 400, len / 2, len - 100, len - 1].into_iter().enumerate() {
        v[at] = if k % 2 == 0 { 1.0 } else { -0.5 };
    }
    v
}

fn signals(rate: f64) -> Vec<(&'static str, bool, Vec<Vec<f32>>)> {
    let n = 6_000;
    vec![
        ("sine mono", false, vec![sine(n, rate, 1_000.0)]),
        ("fs/4 sine", false, vec![sine(n, rate, rate / 4.0)]),
        ("chirp mono", false, vec![chirp(n, rate)]),
        ("noise mono", false, vec![noise(n, 1)]),
        ("impulses mono", true, vec![impulses(n)]),
        ("silence mono", true, vec![vec![0.0; n]]),
        ("sine+noise stereo", false, vec![sine(n, rate, 440.0), noise(n, 7)]),
        ("chirp+impulses stereo", false, vec![chirp(n, rate), impulses(n)]),
        ("3 samples", false, vec![vec![0.5, -0.25, 0.75]]),
        ("empty", true, vec![Vec::new()]),
    ]
}

const RATES: [f64; 6] = [22_050.0, 44_100.0, 48_000.0, 88_200.0, 96_000.0, 192_000.0];

/// Largest allowed |old vectorised - new| in a peak or phase value (signals peak at 0.9).
const TOLERANCE: f32 = 2e-6;

#[test]
fn coefficients_are_bit_identical() {
    for factor in [2, 4, 8] {
        let (a, b) = (old::PolyphaseBank::new(factor), PolyphaseBank::new(factor));
        let (mut ia, mut ib) = (old::Interpolator::default(), Interpolator::default());
        // An impulse reads each coefficient once: phase p of input q is h[p + q*L].
        let (mut oa, mut ob) = ([0.0f32; MAX_FACTOR], [0.0f32; MAX_FACTOR]);
        for n in 0..TAPS_PER_PHASE + 2 {
            let x = if n == 0 { 1.0 } else { 0.0 };
            ia.phases(&a, x, &mut oa[..factor]);
            ib.push_phases(&b, x, &mut ob);
            assert!(oa[..factor].iter().zip(&ob).all(|(p, q)| p.to_bits() == q.to_bits()), "factor {factor} tap {n}: {oa:?} vs {ob:?}");
        }
    }
}

#[test]
fn new_core_matches_the_old_core() {
    let (mut samples, mut identical_seq, mut identical_vec) = (0usize, 0usize, 0usize);
    let (mut worst_vec, mut worst_phase) = (0.0f32, 0.0f32);
    for rate in RATES {
        let factor = true_peak_factor(rate);
        let (old_bank, new_bank) = (old::PolyphaseBank::new(factor), PolyphaseBank::new(factor));
        for (name, exact, channels) in signals(rate) {
            for (c, x) in channels.iter().enumerate() {
                let (mut vec_ip, mut seq_ip, mut phase_ip, mut new_ip, mut new_phase_ip) = (
                    old::Interpolator::default(),
                    old::Interpolator::default(),
                    old::Interpolator::default(),
                    Interpolator::default(),
                    Interpolator::default(),
                );
                let (mut op, mut np) = ([0.0f32; MAX_FACTOR], [0.0f32; MAX_FACTOR]);
                for (n, &v) in x.iter().enumerate() {
                    let (a, s, b) = (vec_ip.push_peak(&old_bank, v), seq_ip.push_peak_sequential(&old_bank, v), new_ip.push_peak(&new_bank, v));
                    let diff = (a - b).abs();
                    worst_vec = worst_vec.max(diff);
                    samples += 1;
                    identical_vec += usize::from(a.to_bits() == b.to_bits());
                    // (old `push_peak_sequential` has no factor-1 shortcut, so it is only compared above 1x.)
                    if factor > 1 {
                        identical_seq += usize::from(s.to_bits() == b.to_bits());
                        assert_eq!(s.to_bits(), b.to_bits(), "{rate} {name} ch{c} n{n}: sequential {s} vs new {b}");
                    }
                    assert!(!exact || a.to_bits() == b.to_bits(), "{rate} {name} ch{c} n{n}: {a} vs {b} must be bit-identical");
                    assert!(diff <= TOLERANCE, "{rate} {name} ch{c} n{n}: |{a} - {b}| = {diff:e}");
                    // Every phase, not just the peak (factor 1 has a single phase; skip it).
                    if factor > 1 {
                        phase_ip.phases(&old_bank, v, &mut op[..factor]);
                        new_phase_ip.push_phases(&new_bank, v, &mut np);
                        for p in 0..factor {
                            worst_phase = worst_phase.max((op[p] - np[p]).abs());
                        }
                    }
                }
            }
        }
    }
    assert!(worst_phase <= TOLERANCE, "worst phase difference {worst_phase:e}");
    println!(
        "equivalence: {samples} peaks; new == old sequential bit-identical for {identical_seq}; new == old vectorised bit-identical for {identical_vec}; worst |old vectorised - new| peak {worst_vec:e}, any phase {worst_phase:e}"
    );
}

/// Not a gate: prints per-sample cost of the old (vectorised) and new `push_peak` (use `--release`).
#[test]
fn timing_is_reported() {
    let x = noise(48_000 * 60, 11);
    let (ob, nb) = (old::PolyphaseBank::new(4), PolyphaseBank::new(4));
    let (mut oi, mut ni) = (old::Interpolator::default(), Interpolator::default());
    let t = std::time::Instant::now();
    let a = x.iter().fold(0.0f32, |p, &v| p.max(oi.push_peak(&ob, v)));
    let (told, t) = (t.elapsed(), std::time::Instant::now());
    let b = x.iter().fold(0.0f32, |p, &v| p.max(ni.push_peak(&nb, v)));
    println!("60 s mono 48 kHz, 4x true peak: old {told:?}, new {:?}", t.elapsed());
    assert!((a - b).abs() < TOLERANCE);
}
