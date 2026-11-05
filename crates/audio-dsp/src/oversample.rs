//! Polyphase FIR interpolator used for true-peak estimation (BS.1770-4 Annex 2 style).
//!
//! The prototype is a Kaiser-windowed sinc low-pass (cut-off at the original Nyquist) split into
//! `factor` phases of [`TAPS_PER_PHASE`] taps; each phase is normalised to unity DC gain. The
//! filter bank and its arithmetic are `craft-resample`'s, whose Lean proof shows the polyphase form
//! equals zero-stuff, filter, keep every sample: `PolyphaseBank` is `craft_resample::Polyphase`
//! with `kaiser_prototype(factor, 1, TAPS_PER_PHASE, 8.0)`.

use craft_resample::{Polyphase, kaiser_prototype};

/// Taps per polyphase branch.
pub const TAPS_PER_PHASE: usize = 16;

/// Oversampling factor giving an effective rate of at least 176.4 kHz (4× at 44.1/48 kHz,
/// 2× at 88.2/96 kHz, 1× above), as recommended by BS.1770-4 for true-peak metering.
pub fn true_peak_factor(sample_rate: f64) -> usize {
    if sample_rate < 88_200.0 {
        4
    } else if sample_rate < 176_400.0 {
        2
    } else {
        1
    }
}

/// Largest oversampling factor a [`PolyphaseBank`] accepts (the true-peak factors are 1, 2 and 4).
pub const MAX_FACTOR: usize = 8;

/// Kaiser beta of the interpolation prototype.
const BETA: f64 = 8.0;

/// Polyphase coefficient bank shared by all channels.
#[derive(Clone, Debug)]
pub struct PolyphaseBank {
    factor: usize,
    /// `phase(p)[q]` multiplies `x[n - q]` for output phase `p`; `None` only if the design were
    /// rejected, which the fixed parameters below cannot do (then outputs are the input itself).
    poly: Option<Polyphase<f32>>,
}

impl PolyphaseBank {
    /// Interpolator by `factor` (clamped to `1..=`[`MAX_FACTOR`]).
    pub fn new(factor: usize) -> Self {
        let factor = factor.clamp(1, MAX_FACTOR);
        let proto = kaiser_prototype(factor, 1, TAPS_PER_PHASE, BETA).unwrap_or_default();
        Self::from_prototype(&proto, factor)
    }

    /// A bank from any prototype of at most `factor * TAPS_PER_PHASE` taps (shorter ones are
    /// zero-padded).
    fn from_prototype(h: &[f64], factor: usize) -> Self {
        let mut padded = vec![0.0f32; factor * TAPS_PER_PHASE];
        for (d, &c) in padded.iter_mut().zip(h) {
            *d = c as f32;
        }
        Self { factor, poly: Polyphase::from_prototype(&padded, factor, 1).ok() }
    }

    pub fn factor(&self) -> usize {
        self.factor
    }
    /// Approximate delay (in input samples) between an input sample and the interpolated
    /// values around it.
    pub fn delay(&self) -> usize {
        TAPS_PER_PHASE / 2
    }
}

/// Per-channel interpolator history.
#[derive(Clone, Debug)]
pub struct Interpolator {
    /// Double-written ring so a contiguous window of TAPS_PER_PHASE is always available.
    hist: [f32; 2 * TAPS_PER_PHASE],
    pos: usize,
}

impl Default for Interpolator {
    fn default() -> Self {
        Interpolator { hist: [0.0; 2 * TAPS_PER_PHASE], pos: 0 }
    }
}

impl Interpolator {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Push one input sample; writes the `factor` interpolated output samples produced for it to
    /// `out[..factor]` (`out` must hold at least `factor`; fewer are written otherwise).
    #[inline]
    pub fn push_phases(&mut self, bank: &PolyphaseBank, x: f32, out: &mut [f32]) {
        let count = bank.factor.min(out.len());
        let Some(out) = out.get_mut(..count) else { return };
        // Oldest sample first, so window[TAPS-1-q] = x[n - q].
        if let Some(slot) = self.hist.get_mut(self.pos) {
            *slot = x;
        }
        if let Some(slot) = self.hist.get_mut(self.pos + TAPS_PER_PHASE) {
            *slot = x;
        }
        self.pos = (self.pos + 1) % TAPS_PER_PHASE;
        let window = self.hist.get(self.pos..self.pos + TAPS_PER_PHASE).unwrap_or_default();
        match &bank.poly {
            // The window ends at input index TAPS-1; output p of that input is upsampled index
            // (TAPS-1) * factor + p.
            Some(poly) => poly.process_from_into(window, ((TAPS_PER_PHASE - 1) * bank.factor) as u128, out),
            None => out.fill(x),
        }
    }

    /// Push one input sample; returns the maximum absolute value of the `factor` interpolated
    /// output samples produced for it.
    #[inline]
    pub fn push_peak(&mut self, bank: &PolyphaseBank, x: f32) -> f32 {
        if bank.factor == 1 {
            return x.abs();
        }
        let mut out = [0.0f32; MAX_FACTOR];
        self.push_phases(bank, x, &mut out);
        out.iter().fold(0.0f32, |peak, y| peak.max(y.abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_have_unity_dc_and_interpolate_sines() {
        let bank = PolyphaseBank::new(4);
        let poly = bank.poly.as_ref().expect("bank");
        for p in 0..4 {
            let s: f32 = poly.phase(p).expect("phase").iter().sum();
            assert!((s - 1.0).abs() < 1e-5);
        }
        // Sine at fs/4 with 45° phase: samples at ±0.707, true peak 1.0.
        let mut ip = Interpolator::default();
        let mut peak = 0.0f32;
        for n in 0..400 {
            let x = (std::f64::consts::FRAC_PI_2 * n as f64 + std::f64::consts::FRAC_PI_4).sin() as f32;
            let p = ip.push_peak(&bank, x);
            if n > 40 {
                peak = peak.max(p);
            }
        }
        assert!((peak - 1.0).abs() < 0.02, "peak {peak}");
    }

    /// The streaming interpolator is `craft-resample`'s polyphase on the whole signal: output
    /// `n * factor + p` of the stream is what `push_phases` writes for input `n`, phase `p`.
    #[test]
    fn streaming_equals_the_whole_signal_polyphase() {
        let mut rng = crate::testutil::Rng::new(5);
        let x: Vec<f32> = (0..2_000).map(|_| rng.uniform()).collect();
        for factor in [2, 4, 8] {
            let bank = PolyphaseBank::new(factor);
            let poly = bank.poly.as_ref().expect("bank");
            let all = poly.process(&x, 0, x.len() * factor);
            let (mut ip, mut out) = (Interpolator::default(), [0.0f32; MAX_FACTOR]);
            for (n, &v) in x.iter().enumerate() {
                ip.push_phases(&bank, v, &mut out);
                assert_eq!(out[..factor], all[n * factor..(n + 1) * factor], "factor {factor} n {n}");
            }
        }
    }

    // The Lean-generated vectors of craft-resample (craft-libs b0574cf `crates/resample/tests/
    // vectors/lean-resample.csv`) through this app's path: `from_prototype` -> `push_phases`, in f32.
    // The app interpolates (M = 1) with at most TAPS_PER_PHASE taps per phase, so those rows apply;
    // the rest have M > 1, which the interpolator does not implement.
    #[test]
    fn lean_vectors_through_the_app_path() {
        const TABLE: &str = include_str!("../tests/vectors/lean-resample.csv");
        let rational = |t: &str| -> f64 {
            let (n, d) = t.split_once('/').expect("fraction");
            n.parse::<f64>().expect("num") / d.parse::<f64>().expect("den")
        };
        let list = |t: &str| -> Vec<f64> { t.split(' ').filter(|v| !v.is_empty()).map(rational).collect() };
        let u = 2.0f64.powi(-24);
        let (mut used, mut skipped, mut worst_ratio) = (0, 0, 0.0f64);
        for line in TABLE.lines().skip(1) {
            let f: Vec<&str> = line.split(',').collect();
            let (up, down): (usize, usize) = (f[1].parse().expect("L"), f[2].parse().expect("M"));
            let (x, h, y) = (list(f[6]), list(f[7]), list(f[8]));
            let t = h.len().div_ceil(up);
            if down != 1 || t > TAPS_PER_PHASE || up > MAX_FACTOR {
                skipped += 1;
                continue;
            }
            // `up == 1` is the identity-rate case: push_peak shortcuts it, push_phases still filters.
            let bank = PolyphaseBank::from_prototype(&h, up);
            let (mut ip, mut out) = (Interpolator::default(), [0.0f32; MAX_FACTOR]);
            let mut got = Vec::new();
            // input, then zeros to flush the filter's tail
            for n in 0..x.len() + TAPS_PER_PHASE {
                ip.push_phases(&bank, x.get(n).map_or(0.0, |&v| v as f32), &mut out);
                got.extend_from_slice(&out[..up]);
            }
            for (m, &want) in y.iter().enumerate() {
                let (i, p) = ((m / up) as i64, m % up);
                let mag: f64 = (0..t)
                    .map(|q| {
                        let xv = usize::try_from(i - q as i64).ok().and_then(|j| x.get(j)).copied().unwrap_or(0.0);
                        (h.get(p + q * up).copied().unwrap_or(0.0) * xv).abs()
                    })
                    .sum();
                let bound = (t as f64 + 4.0) * u * mag;
                let err = (f64::from(got[m]) - want).abs();
                if bound > 0.0 {
                    worst_ratio = worst_ratio.max(err / bound);
                }
                assert!(err <= bound, "{} {up}/{down} m={m}: err {err:e} > {bound:e}", f[0]);
            }
            used += 1;
        }
        println!("lean vectors via the interpolator: {used} rows used, {skipped} skipped (M > 1), worst error/bound {worst_ratio:.3}");
        assert!(used >= 2);
    }
}
