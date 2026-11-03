//! Dependency-free Smith–Gossett windowed-sinc sample-rate conversion.
//!
//! A 256-tap squared Blackman–Harris window and 129 precomputed phases suppress images and
//! aliases. The cutoff follows the rate ratio. Absolute rational source positions make output
//! bit-identical across block sizes, including negative timeline coordinates and seeks.
//! Source pulls are contiguous and non-overlapping. Reading 128 source frames ahead compensates
//! the filter delay: output stays aligned to the timeline, with no extra frames or duration drift.
//! Finite sources supply silence outside their extent, including the lookahead at the end; this
//! flushes the filter in the final requested block rather than appending delayed audio.
//!
//! [`StreamResampler::process`] retains the allocating, owned-block mixer API. For preallocated
//! sources and destinations, construct with [`StreamResampler::with_capacity`] and use
//! [`StreamResampler::process_into`]; that path never allocates, including on a seek.

use std::{f64::consts::PI, ops::Range};

const TAPS: usize = 256;
const PHASES: usize = 128;
const LEFT: i64 = TAPS as i64 / 2 - 1;
const RIGHT: i64 = TAPS as i64 / 2;
const MAX_CHANNELS: usize = 256;

/// Converts a planar stream from `from` Hz to `to` Hz, block by block.
#[derive(Debug, Clone)]
pub struct StreamResampler {
    from: u32,
    to: u32,
    kernel: Vec<[f64; TAPS]>,
    /// Source frames `[held_start, held_start + held[c].len())` kept from the last pull.
    held_start: i64,
    held: Vec<Vec<f32>>,
    /// The output frame the next block is expected to start at (contiguous playback).
    next_out: Option<i64>,
}

/// Maximum source frames fetched in a single resampler pull.
pub const MAX_RESAMPLE_SOURCE_SPAN: usize = 384_000 * 10;

struct Span {
    first: i64,
    remainder: i64,
    start: i64,
    end: i64,
    next_out: i64,
}

impl StreamResampler {
    /// Rates of zero are treated as 1 Hz rather than dividing by zero.
    pub fn new(from: u32, to: u32) -> Self {
        let (from, to) = (from.max(1), to.max(1));
        let cutoff = 0.90 * (f64::from(to) / f64::from(from)).min(1.0);
        let kernel = if from == to {
            Vec::new()
        } else {
            (0..=PHASES)
                .map(|phase| {
                    let mut coefficients = [0.0; TAPS];
                    for (tap, coefficient) in coefficients.iter_mut().enumerate() {
                        let offset = tap as f64 - LEFT as f64 - phase as f64 / PHASES as f64;
                        let angle = PI * offset / RIGHT as f64;
                        let window = 0.35875 + 0.48829 * angle.cos() + 0.14128 * (2.0 * angle).cos() + 0.01168 * (3.0 * angle).cos();
                        let sinc_angle = PI * cutoff * offset;
                        let sinc = if sinc_angle.abs() < 1e-12 { 1.0 } else { sinc_angle.sin() / sinc_angle };
                        *coefficient = cutoff * sinc * window.powi(2);
                    }
                    let sum: f64 = coefficients.iter().sum();
                    coefficients.iter_mut().for_each(|coefficient| *coefficient /= sum);
                    coefficients
                })
                .collect()
        };
        Self { from, to, kernel, held_start: 0, held: Vec::new(), next_out: None }
    }

    /// Reserve storage for an allocation-free planar stream, with up to `max_output_frames`
    /// per call. Invalid or excessive sizes return an error rather than allocating unboundedly.
    pub fn with_capacity(from: u32, to: u32, channels: usize, max_output_frames: usize) -> Result<Self, &'static str> {
        if channels == 0 || channels > MAX_CHANNELS || max_output_frames == 0 || max_output_frames > MAX_RESAMPLE_SOURCE_SPAN {
            return Err("invalid resampler capacity");
        }
        let capacity = (max_output_frames as u128 * u128::from(from.max(1))).div_ceil(u128::from(to.max(1))) + TAPS as u128;
        if capacity * channels as u128 > MAX_RESAMPLE_SOURCE_SPAN as u128 {
            return Err("resampler source capacity exceeds limit");
        }
        let mut resampler = Self::new(from, to);
        resampler.held = (0..channels).map(|_| Vec::with_capacity(capacity as usize)).collect();
        Ok(resampler)
    }

    pub fn rates(&self) -> (u32, u32) {
        (self.from, self.to)
    }

    /// Exact output length for a finite source, rounding only once at its end.
    pub fn output_frames(&self, source_frames: u64) -> u64 {
        (u128::from(source_frames) * u128::from(self.to)).div_ceil(u128::from(self.from)).min(u128::from(u64::MAX)) as u64
    }

    fn source_index(&self, out: i64) -> (i128, i64) {
        let p = out as i128 * self.from as i128;
        let to = self.to as i128;
        (p.div_euclid(to), p.rem_euclid(to) as i64)
    }

    fn span(&self, out: i64, n: usize) -> Option<Span> {
        if n == 0 || n > MAX_RESAMPLE_SOURCE_SPAN {
            return None;
        }
        let next_out = out.checked_add(i64::try_from(n).ok()?)?;
        let (first, remainder) = self.source_index(out);
        let (last, _) = self.source_index(next_out - 1);
        let (left, right) = if self.from == self.to { (0, 0) } else { (LEFT, RIGHT) };
        let start = i64::try_from(first - i128::from(left)).ok()?;
        let end = i64::try_from(last + i128::from(right) + 1).ok()?;
        if i128::from(end) - i128::from(start) > MAX_RESAMPLE_SOURCE_SPAN as i128 {
            return None;
        }
        Some(Span { first: i64::try_from(first).ok()?, remainder, start, end, next_out })
    }

    fn begin(&mut self, out: i64, span: &Span) {
        if self.next_out != Some(out) {
            self.held_start = span.start;
            self.held.iter_mut().for_each(Vec::clear);
            self.next_out = None;
        }
    }

    fn missing(&self, span: &Span) -> (i64, usize) {
        let end = self.held_start + self.held.first().map_or(0, Vec::len) as i64;
        (end, usize::try_from(i128::from(span.end) - i128::from(end)).unwrap_or(0))
    }

    fn render(&self, span: &Span, destination: &mut [&mut [f32]]) {
        let to = i64::from(self.to);
        let (step, rem_step) = (i64::from(self.from / self.to), i64::from(self.from % self.to));
        let (mut source, mut remainder) = (span.first, span.remainder);
        let left = if self.kernel.is_empty() { 0 } else { LEFT };
        for frame in 0..destination.first().map_or(0, |channel| channel.len()) {
            let offset = usize::try_from(i128::from(source) - i128::from(left) - i128::from(self.held_start)).unwrap_or(usize::MAX);
            let phase = remainder as f64 * PHASES as f64 / to as f64;
            let lower = (phase.floor() as usize).min(PHASES - 1);
            let weight = phase - lower as f64;
            for (output, history) in destination.iter_mut().zip(&self.held) {
                let sample = if self.kernel.is_empty() {
                    history.get(offset).copied().unwrap_or(0.0)
                } else if let (Some(input), Some(first), Some(second)) =
                    (offset.checked_add(TAPS).and_then(|end| history.get(offset..end)), self.kernel.get(lower), self.kernel.get(lower + 1))
                {
                    input
                        .iter()
                        .zip(first.iter().zip(second))
                        .map(|(&sample, (&first, &second))| f64::from(sample) * (first + weight * (second - first)))
                        .sum::<f64>() as f32
                } else {
                    0.0
                };
                if let Some(output) = output.get_mut(frame) {
                    *output = sample;
                }
            }
            remainder += rem_step;
            source = source.saturating_add(step);
            if remainder >= to {
                remainder -= to;
                source = source.saturating_add(1);
            }
        }
    }

    fn finish(&mut self, span: &Span) {
        let (next_first, _) = self.source_index(span.next_out);
        let left = if self.kernel.is_empty() { 0 } else { LEFT };
        let available = self.held.first().map_or(0, Vec::len);
        let drop = (next_first - i128::from(left) - i128::from(self.held_start)).clamp(0, available as i128) as usize;
        for history in &mut self.held {
            history.drain(..drop.min(history.len()));
        }
        self.held_start += drop as i64;
        self.next_out = Some(span.next_out);
    }

    /// Fill equal-length caller-owned planes without allocating. Returns `false` and silence
    /// for invalid coordinates, channel counts or insufficient reserved capacity. Construct
    /// with `with_capacity` first; do not mix this method with the allocating `process` API.
    ///
    /// `pull(start, range, planes)` fills only `planes[channel][range]` with contiguous source
    /// frames starting at `start`. Planes already have the required length; leave absent input
    /// as zero, and do not resize them. Finite sources must zero-pad beyond both ends.
    pub fn process_into(&mut self, out: i64, destination: &mut [&mut [f32]], mut pull: impl FnMut(i64, Range<usize>, &mut [Vec<f32>])) -> bool {
        destination.iter_mut().for_each(|channel| channel.fill(0.0));
        let n = destination.first().map_or(0, |channel| channel.len());
        if destination.len() != self.held.len() || self.held.is_empty() || destination.iter().any(|channel| channel.len() != n) {
            return false;
        }
        if n == 0 {
            return true;
        }
        let Some(span) = self.span(out, n) else { return false };
        self.begin(out, &span);
        let (start, want) = self.missing(&span);
        let offset = self.held.first().map_or(0, Vec::len);
        let end = offset.saturating_add(want);
        if self.held.iter().any(|channel| channel.capacity() < end) {
            return false;
        }
        if want > 0 {
            self.held.iter_mut().for_each(|channel| channel.resize(end, 0.0));
            pull(start, offset..end, &mut self.held);
            self.held.iter_mut().flatten().filter(|sample| !sample.is_finite()).for_each(|sample| *sample = 0.0);
        }
        self.render(&span, destination);
        self.finish(&span);
        true
    }

    /// Output frames `[out, out + n)`, planar. `pull(start, frames)` returns source frames
    /// `[start, start + frames)`, planar (missing samples read as silence). A block that does not
    /// continue the previous one (a seek) starts the stream afresh. Requests over
    /// `MAX_RESAMPLE_SOURCE_SPAN` output frames return empty planes; unrepresentable source
    /// coordinates or source spans above that limit return silence without invoking `pull`.
    /// Up to 256 channels are retained. Source and destination storage are each bounded to
    /// `MAX_RESAMPLE_SOURCE_SPAN` total samples; larger requests return empty planes.
    /// This owned-block compatibility API allocates.
    pub fn process(&mut self, out: i64, n: usize, mut pull: impl FnMut(i64, usize) -> Vec<Vec<f32>>) -> Vec<Vec<f32>> {
        if n == 0 || n > MAX_RESAMPLE_SOURCE_SPAN / self.held.len().max(1) {
            return vec![Vec::new(); self.held.len().max(1)];
        }
        let Some(span) = self.span(out, n) else {
            return vec![vec![0.0; n]; self.held.len().max(1)];
        };
        if self.next_out != Some(out) {
            self.held.clear();
        }
        self.begin(out, &span);
        let (start, want) = self.missing(&span);
        if want > 0 {
            let got = pull(start, want);
            if self.held.is_empty() {
                let channels = got.len().clamp(1, MAX_CHANNELS);
                if n > MAX_RESAMPLE_SOURCE_SPAN / channels || want > MAX_RESAMPLE_SOURCE_SPAN / channels {
                    self.next_out = None;
                    return vec![Vec::new(); channels];
                }
                self.held = vec![Vec::new(); channels];
            }
            let samples = self.held.first().map_or(0, Vec::len).saturating_add(want);
            if samples > MAX_RESAMPLE_SOURCE_SPAN / self.held.len() {
                self.next_out = None;
                return vec![Vec::new(); self.held.len()];
            }
            for (c, h) in self.held.iter_mut().enumerate() {
                let src = got.get(c).map(Vec::as_slice).unwrap_or_default();
                h.extend((0..want).map(|i| src.get(i).copied().filter(|sample| sample.is_finite()).unwrap_or(0.0)));
            }
        }
        let mut dst = vec![vec![0.0f32; n]; self.held.len()];
        self.render(&span, &mut dst.iter_mut().map(Vec::as_mut_slice).collect::<Vec<_>>());
        self.finish(&span);
        dst
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sine at `hz`, sampled at `rate`, from absolute frame `start`; logs every pull.
    fn sine(hz: f64, rate: u32, pulls: &mut Vec<(i64, usize)>) -> impl FnMut(i64, usize) -> Vec<Vec<f32>> + '_ {
        move |start, frames| {
            pulls.push((start, frames));
            let ch: Vec<f32> = (0..frames).map(|i| (2.0 * std::f64::consts::PI * hz * (start + i as i64) as f64 / rate as f64).sin() as f32).collect();
            vec![ch.clone(), ch]
        }
    }

    fn convert(from: u32, to: u32, blocks: &[usize], total: usize) -> (Vec<f32>, Vec<(i64, usize)>) {
        let mut r = StreamResampler::new(from, to);
        let mut pulls = Vec::new();
        let mut out = Vec::new();
        let mut pos = 0i64;
        let mut k = 0;
        while (pos as usize) < total {
            let n = blocks[k % blocks.len()].min(total - pos as usize);
            let b = r.process(pos, n, sine(1000.0, from, &mut pulls));
            assert_eq!(b.len(), 2);
            assert!(b.iter().all(|c| c.len() == n));
            out.extend_from_slice(&b[0]);
            pos += n as i64;
            k += 1;
        }
        (out, pulls)
    }

    #[test]
    fn output_is_the_same_however_the_stream_is_cut() {
        for (from, to) in [(48_000, 44_100), (44_100, 48_000), (48_000, 96_000), (96_000, 48_000)] {
            let (whole, _) = convert(from, to, &[20_000], 20_000);
            for blocks in [&[512][..], &[333], &[1, 7, 512, 64]] {
                let (cut, _) = convert(from, to, blocks, 20_000);
                assert_eq!(cut, whole, "{from}→{to} in blocks {blocks:?}");
            }
        }
    }

    /// `process` as it was with one i128 division per output sample.
    fn reference_process(r: &mut StreamResampler, out: i64, n: usize, mut pull: impl FnMut(i64, usize) -> Vec<Vec<f32>>) -> Vec<Vec<f32>> {
        let (from, to) = (r.from as i128, r.to as i128);
        let index = |out: i64| {
            let p = out as i128 * from;
            (p.div_euclid(to) as i64, (p.rem_euclid(to) as f64 / to as f64) as f32)
        };
        if n == 0 {
            return vec![Vec::new(); r.held.len().max(1)];
        }
        let (first, _) = index(out);
        let (last, _) = index(out.saturating_add(n as i64 - 1));
        if r.next_out != Some(out) || r.held.is_empty() {
            r.held_start = first - 1;
            r.held.clear();
        }
        let held_end = r.held_start + r.held.first().map_or(0, Vec::len) as i64;
        if last + 1 > held_end {
            let want = ((last + 1 - held_end) as usize).min(MAX_RESAMPLE_SOURCE_SPAN);
            let got = pull(held_end, want);
            if r.held.is_empty() {
                r.held = vec![Vec::new(); got.len().max(1)];
            }
            for (c, h) in r.held.iter_mut().enumerate() {
                let src = got.get(c).map(Vec::as_slice).unwrap_or_default();
                h.extend((0..want).map(|i| src.get(i).copied().unwrap_or(0.0)));
            }
        }
        let mut dst = vec![vec![0.0f32; n]; r.held.len()];
        for (i, frame) in (out..).take(n).enumerate() {
            let (s, frac) = index(frame);
            let k = usize::try_from(s - r.held_start).unwrap_or(0);
            for (d, h) in dst.iter_mut().zip(&r.held) {
                let a = k.checked_sub(1).and_then(|j| h.get(j)).copied().unwrap_or(0.0);
                let b = h.get(k).copied().unwrap_or(0.0);
                if let Some(x) = d.get_mut(i) {
                    *x = a + (b - a) * frac;
                }
            }
        }
        let (next_first, _) = index(out.saturating_add(n as i64));
        let drop = (next_first - 1 - r.held_start).clamp(0, r.held.first().map_or(0, Vec::len) as i64) as usize;
        for h in &mut r.held {
            h.drain(..drop);
        }
        r.held_start += drop as i64;
        r.next_out = Some(out.saturating_add(n as i64));
        dst
    }

    #[test]
    fn stepped_source_positions_match_exact_rational_positions() {
        for (from, to) in
            [(48_000, 44_100), (44_100, 48_000), (96_000, 48_000), (48_000, 48_000), (48_000, 96_000), (44_100, 22_050), (7, 384_000), (176_400, 1_001)]
        {
            let resampler = StreamResampler::new(from, to);
            let mut k = 0;
            for start in [0i64, -5_000, 1_000_000_007, -3, i64::MAX / 2_000_000] {
                let mut pos = start;
                for _ in 0..12 {
                    let n = [1, 7, 333, 512, 64][k % 5];
                    k += 1;
                    let (mut source, mut remainder) = resampler.source_index(pos);
                    for frame in pos..pos + n as i64 {
                        assert_eq!((source, remainder), resampler.source_index(frame));
                        source += i128::from(from / to);
                        remainder += i64::from(from % to);
                        if remainder >= i64::from(to) {
                            source += 1;
                            remainder -= i64::from(to);
                        }
                    }
                    pos += n as i64;
                }
            }
        }
    }

    #[test]
    fn a_tone_converts_to_the_same_tone() {
        let (from, to) = (48_000u32, 44_100u32);
        let (out, _) = convert(from, to, &[512], 20_000);
        for (o, x) in out.iter().enumerate() {
            let t = o as f64 / to as f64;
            let want = (2.0 * std::f64::consts::PI * 1000.0 * t).sin() as f32;
            assert!((x - want).abs() < 2e-5, "frame {o}: {x} vs {want}");
        }
    }

    #[test]
    fn the_source_is_pulled_as_one_continuous_stream() {
        for (from, to) in [(48_000, 44_100), (44_100, 48_000)] {
            let (_, pulls) = convert(from, to, &[512, 333], 20_000);
            for w in pulls.windows(2) {
                assert_eq!(w[0].0 + w[0].1 as i64, w[1].0, "{from}→{to}: {:?} then {:?}", w[0], w[1]);
            }
        }
    }

    #[test]
    fn a_seek_starts_afresh_and_hostile_input_stays_finite() {
        let mut r = StreamResampler::new(48_000, 44_100);
        let mut pulls = Vec::new();
        let _ = r.process(0, 512, sine(1000.0, 48_000, &mut pulls));
        let _ = r.process(1_000_000, 512, sine(1000.0, 48_000, &mut pulls));
        assert_eq!(pulls.last().map(|p| p.0), Some(1_000_000 * 48_000 / 44_100 - LEFT));
        // a mixer returning too few channels and samples, and zero rates
        let mut r = StreamResampler::new(0, 0);
        let b = r.process(5, 64, |_, _| vec![vec![1.0; 3]]);
        assert_eq!(b.len(), 1);
        assert!(b[0].iter().all(|x| x.is_finite()));
        assert!(r.process(0, 0, |_, _| Vec::new()).iter().all(Vec::is_empty));
    }

    fn amplitude(samples: &[f32], frequency: f64, rate: u32) -> f64 {
        let (mut real, mut imaginary) = (0.0, 0.0);
        for (frame, &sample) in samples.iter().enumerate() {
            let angle = 2.0 * PI * frequency * frame as f64 / f64::from(rate);
            real += f64::from(sample) * angle.cos();
            imaginary += f64::from(sample) * angle.sin();
        }
        2.0 * real.hypot(imaginary) / samples.len() as f64
    }

    #[test]
    fn images_are_rejected_more_than_one_hundred_decibels() {
        let (converted, _) = convert(48_000, 44_100, &[512], 20_000);
        let mut old = StreamResampler::new(48_000, 44_100);
        let linear = reference_process(&mut old, 0, 20_000, sine(1000.0, 48_000, &mut Vec::new()));
        let rejection = |samples: &[f32]| {
            let samples = &samples[1000..1000 + 4410];
            let fundamental = amplitude(samples, 1000.0, 44_100);
            let image = (100..22_050)
                .step_by(100)
                .filter(|frequency| *frequency != 1000)
                .map(|frequency| amplitude(samples, f64::from(frequency), 44_100))
                .fold(0.0, f64::max);
            20.0 * (fundamental / image).log10()
        };
        let sinc = rejection(&converted);
        let linear = rejection(&linear[0]);
        println!("48k → 44.1k: sinc {sinc:.2} dB image rejection; old linear {linear:.2} dB");
        assert!(sinc > 100.0, "{sinc}");
        assert!(sinc > linear + 40.0, "sinc {sinc}, linear {linear}");
    }

    #[test]
    fn downsampling_rejects_above_nyquist_tones() {
        for (from, to, frequency, alias) in [(48_000, 44_100, 23_000.0, 21_100.0), (48_000, 12_000, 10_000.0, 2000.0), (96_000, 48_000, 30_000.0, 18_000.0)] {
            let mut resampler = StreamResampler::new(from, to);
            let converted = resampler.process(0, 20_000, sine(frequency, from, &mut Vec::new()));
            let residual = amplitude(&converted[0][1000..1000 + to as usize / 10], alias, to);
            assert!(residual < 1e-5, "{from} → {to}: {residual}");
        }
    }

    fn finite_convert(source: &[Vec<f32>], from: u32, to: u32, blocks: &[usize]) -> Vec<Vec<f32>> {
        let mut resampler = StreamResampler::new(from, to);
        let length = resampler.output_frames(source[0].len() as u64) as usize;
        let mut converted = vec![Vec::new(); source.len()];
        let (mut position, mut block) = (0, 0);
        while position < length {
            let frames = blocks[block % blocks.len()].min(length - position);
            let output = resampler.process(position as i64, frames, |start, count| {
                source
                    .iter()
                    .map(|channel| {
                        (0..count)
                            .map(|frame| usize::try_from(start + frame as i64).ok().and_then(|frame| channel.get(frame)).copied().unwrap_or(0.0))
                            .collect()
                    })
                    .collect()
            });
            for (destination, channel) in converted.iter_mut().zip(output) {
                destination.extend(channel);
            }
            position += frames;
            block += 1;
        }
        converted
    }

    #[test]
    fn round_trip_preserves_length_and_compensates_filter_delay() {
        for (from, to) in [(48_000, 44_100), (44_100, 48_000), (96_000, 48_000), (48_000, 12_000)] {
            let mut source = vec![vec![0.0; from as usize / 10]; 3];
            let center = source[0].len() / 2;
            for (channel, gain) in source.iter_mut().zip([1.0, -0.5, 0.25]) {
                channel[center] = gain;
            }
            let down = finite_convert(&source, from, to, &[1, 7, 333, 512]);
            assert!(down.iter().all(|channel| channel.len() == to as usize / 10));
            let up = finite_convert(&down, to, from, &[333, 64]);
            assert!(up.iter().all(|channel| channel.len() == source[0].len()));
            for (channel, gain) in up.iter().zip([1.0, -0.5, 0.25]) {
                let peak = channel.iter().enumerate().max_by(|left, right| left.1.abs().total_cmp(&right.1.abs())).unwrap().0;
                assert_eq!(peak, center, "{from} → {to}");
                assert!(channel[peak] * gain > 0.0);
            }
        }
    }

    #[test]
    fn finite_sources_flush_edges_without_appending_or_rounding_each_chunk() {
        let mut source = vec![vec![0.0; 4801]; 2];
        source[0][4800] = 1.0;
        source[1][0] = -1.0;
        let whole = finite_convert(&source, 48_000, 44_100, &[20_000]);
        let chunks = finite_convert(&source, 48_000, 44_100, &[1, 7, 512, 333]);
        assert_eq!(whole, chunks);
        assert_eq!(whole[0].len(), 4411);
        assert!(whole[0].last().unwrap().abs() > 0.1, "last source impulse must not be dropped");
        assert!(whole[1][0] < -0.5);
        assert_eq!(StreamResampler::new(48_000, 44_100).output_frames(48_000 * 60 * 60 * 24), 44_100 * 60 * 60 * 24);
        assert_eq!(StreamResampler::new(1, u32::MAX).output_frames(u64::MAX), u64::MAX);
    }

    #[test]
    fn unity_is_a_bit_exact_undelayed_bypass() {
        let source = vec![(0..1025).map(|frame| (frame as f32 * 0.123).sin()).collect::<Vec<_>>(); 6];
        assert_eq!(finite_convert(&source, 48_000, 48_000, &[1, 7, 512]), source);
        let mut resampler = StreamResampler::new(48_000, 48_000);
        assert!(resampler.kernel.is_empty());
        let mut pulls = Vec::new();
        resampler.process(0, 512, sine(440.0, 48_000, &mut pulls));
        assert_eq!(pulls, [(0, 512)]);
    }

    #[test]
    fn preallocated_processing_and_seeks_do_not_allocate() {
        for (from, to) in [(48_000, 44_100), (44_100, 48_000), (48_000, 48_000), (48_000, 12_000)] {
            let mut resampler = StreamResampler::with_capacity(from, to, 3, 512).unwrap();
            let mut buffers: [Vec<f32>; 3] = std::array::from_fn(|_| vec![0.0; 512]);
            let [left, right, center] = &mut buffers;
            let mut planes = [left.as_mut_slice(), right.as_mut_slice(), center.as_mut_slice()];
            let allocations = allocation_counter::measure(|| {
                for start in [0, 512, 1024, -5000, -4488, 1_000_000] {
                    assert!(resampler.process_into(start, &mut planes, |_, range, channels| {
                        for (channel, value) in channels.iter_mut().zip([0.25, -0.5, 0.75]) {
                            channel[range.clone()].fill(value);
                        }
                    }));
                    for (channel, expected) in planes.iter().zip([0.25, -0.5, 0.75]) {
                        assert!(channel.iter().all(|sample| (sample - expected).abs() < 1e-6));
                    }
                }
            });
            assert_eq!(allocations.count_total, 0);
            assert_eq!(allocations.bytes_total, 0);
        }
    }

    #[test]
    fn preallocated_output_matches_owned_blocks() {
        let mut resampler = StreamResampler::with_capacity(48_000, 44_100, 2, 512).unwrap();
        let (expected, _) = convert(48_000, 44_100, &[20_000], 20_000);
        let mut buffers = [vec![0.0; 512], vec![0.0; 512]];
        let mut actual = Vec::new();
        for position in (0..20_000).step_by(512) {
            let frames = 512.min(20_000 - position);
            let [left, right] = &mut buffers;
            let mut planes = [&mut left[..frames], &mut right[..frames]];
            assert!(resampler.process_into(position as i64, &mut planes, |start, range, channels| {
                for channel in channels {
                    for (frame, sample) in channel[range.clone()].iter_mut().enumerate() {
                        *sample = (2.0 * PI * 1000.0 * (start + frame as i64) as f64 / 48_000.0).sin() as f32;
                    }
                }
            }));
            actual.extend_from_slice(planes[0]);
        }
        assert_eq!(actual, expected);
    }

    #[test]
    fn hostile_coordinates_and_capacities_do_not_panic_or_pull() {
        for (from, to) in [(0, 0), (u32::MAX, 1), (1, u32::MAX)] {
            let mut resampler = StreamResampler::new(from, to);
            for position in [i64::MIN, i64::MAX] {
                let block = resampler.process(position, 512, |_, _| vec![vec![f32::NAN; 3]]);
                assert!(block.iter().flatten().all(|sample| sample.is_finite()));
            }
        }
        assert!(StreamResampler::with_capacity(48_000, 44_100, 0, 512).is_err());
        assert!(StreamResampler::with_capacity(u32::MAX, 1, 2, 512).is_err());
        assert!(StreamResampler::with_capacity(1, 1, usize::MAX, usize::MAX).is_err());
        let mut resampler = StreamResampler::with_capacity(48_000, 44_100, 1, 1).unwrap();
        let mut output = [1.0; 1024];
        assert!(!resampler.process_into(0, &mut [&mut output], |_, _, _| panic!("must not pull with insufficient capacity")));
        assert!(output.iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn failed_preallocated_seek_does_not_poison_contiguous_playback() {
        let mut resampler = StreamResampler::with_capacity(48_000, 44_100, 1, 32).unwrap();
        let fill = |_: i64, range: Range<usize>, channels: &mut [Vec<f32>]| channels[0][range].fill(0.25);
        let mut output = [0.0; 32];
        assert!(resampler.process_into(0, &mut [&mut output], fill));
        let mut oversized = [0.0; 1024];
        assert!(!resampler.process_into(10_000, &mut [&mut oversized], |_, _, _| panic!("insufficient capacity")));
        assert!(resampler.process_into(32, &mut [&mut output], fill));
        assert!(output.iter().all(|sample| (*sample - 0.25).abs() < 1e-6));
    }

    #[test]
    fn owned_blocks_bound_total_storage_across_channels() {
        let mut resampler = StreamResampler::new(1, u32::MAX);
        resampler.process(0, 1, |_, frames| vec![vec![0.25; frames]; 2]);
        for position in [1, i64::MAX] {
            let oversized = resampler.process(position, MAX_RESAMPLE_SOURCE_SPAN, |_, _| panic!("oversized output must not pull"));
            assert_eq!(oversized.len(), 2);
            assert!(oversized.iter().all(Vec::is_empty));
        }
    }
}
