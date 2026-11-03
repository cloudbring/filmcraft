//! Append-only min/max overviews shared by timeline and source-monitor waveform painters.
//!
//! Level zero keeps the original 256-sample buckets, including the final partial bucket.
//! Higher levels contain completed pairs of children; appending never rewrites a bucket.
//! Exact queries scan at most eight buckets for small ranges; larger half-open ranges
//! are partitioned into aligned power-of-two blocks, reading at most two buckets per level
//! without including any samples outside the range.
//!
//! N decoded buckets need fewer than 2N min/max pairs, at 8 bytes each. At 48 kHz that is
//! under 10,800,000 bytes/hour (10.30 MiB), independent of channel count: as before, each
//! audio stream's waveform uses its first channel. Vec growth reserves at most 4N pairs
//! plus four per level (21.60 MB/hour during building); `finish` compacts the cache.
//! Level descriptors add O(log N) bytes. No PCM or duration-sized allocation is retained.

use std::ops::{Deref, Range};

/// A cached, incrementally built pyramid of 256-sample min/max buckets.
#[derive(Debug, Default)]
pub struct PeakPyramid {
    levels: Vec<Vec<(f32, f32)>>,
    extrema: (f32, f32),
}

impl PeakPyramid {
    /// Append a decoded bucket and newly completed ancestors in amortized O(1) time.
    pub fn push(&mut self, peak: (f32, f32)) {
        self.extrema = merge(self.extrema, peak);
        let mut combined = peak;
        let mut level = 0;
        loop {
            if level == self.levels.len() {
                self.levels.push(Vec::new());
            }
            let Some(buckets) = self.levels.get_mut(level) else { break };
            buckets.push(combined);
            if buckets.len() % 2 != 0 {
                break;
            }
            let Some(&left) = buckets.get(buckets.len() - 2) else { break };
            combined = merge(left, combined);
            level += 1;
        }
    }

    /// Release spare build capacity before publishing the immutable clip cache.
    pub fn finish(&mut self) {
        for level in &mut self.levels {
            level.shrink_to_fit();
        }
        self.levels.shrink_to_fit();
    }

    /// Zero-inclusive min/max of the original buckets in `range`, in O(log N) time.
    /// Empty, reversed, and out-of-bounds ranges behave like the former flat scan.
    pub fn range(&self, range: Range<usize>) -> (f32, f32) {
        let mut start = range.start;
        let end = range.end.min(self.len());
        let mut peak = (0f32, 0f32);
        if start >= end {
            return peak;
        }
        if end - start <= 8 {
            return self.get(start..end).unwrap_or_default().iter().copied().fold(peak, merge);
        }
        while start < end {
            let level = start.trailing_zeros().min((end - start).ilog2());
            let Some(&bucket) = self.levels.get(level as usize).and_then(|buckets| buckets.get(start >> level)) else { break };
            peak = merge(peak, bucket);
            start += 1usize << level;
        }
        peak
    }

    /// Source amplitude for waveform normalization, cached during decoding.
    pub fn amplitude(&self) -> f32 {
        self.extrema.0.abs().max(self.extrema.1.abs())
    }
}

impl Deref for PeakPyramid {
    type Target = [(f32, f32)];

    fn deref(&self) -> &Self::Target {
        self.levels.first().map(Vec::as_slice).unwrap_or_default()
    }
}

impl FromIterator<(f32, f32)> for PeakPyramid {
    fn from_iter<T: IntoIterator<Item = (f32, f32)>>(iter: T) -> Self {
        let mut peaks = Self::default();
        for peak in iter {
            peaks.push(peak);
        }
        peaks.finish();
        peaks
    }
}

fn merge(left: (f32, f32), right: (f32, f32)) -> (f32, f32) {
    (left.0.min(right.0), left.1.max(right.1))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn random(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        *seed >> 16
    }

    pub(crate) fn flat(peaks: &[(f32, f32)], range: Range<usize>) -> (f32, f32) {
        peaks.iter().skip(range.start).take(range.end.saturating_sub(range.start)).copied().fold((0.0, 0.0), merge)
    }

    pub(crate) fn random_peaks(seed: &mut u64, len: usize) -> PeakPyramid {
        (0..len).map(|_| (-((random(seed) % 65536) as f32) / 32768.0, (random(seed) % 65536) as f32 / 32768.0)).collect()
    }

    #[test]
    fn every_range_after_every_append_matches_flat() {
        let mut pyramid = PeakPyramid::default();
        let mut buckets = Vec::new();
        for index in 0..129 {
            let peak = (-((index * 17 % 29) as f32) / 29.0, (index * 13 % 31) as f32 / 31.0);
            buckets.push(peak);
            pyramid.push(peak);
            for start in 0..=index + 2 {
                for end in start..=index + 3 {
                    assert_eq!(pyramid.range(start..end), flat(&buckets, start..end), "append {index}, {start}..{end}");
                }
            }
            assert_eq!(pyramid.amplitude(), buckets.iter().fold(0f32, |peak, &(low, high)| peak.max(low.abs()).max(high.abs())));
            let pairs: usize = pyramid.levels.iter().map(Vec::len).sum();
            let reserved: usize = pyramid.levels.iter().map(Vec::capacity).sum();
            assert!(pairs < 2 * buckets.len());
            assert!(reserved <= 4 * buckets.len() + 4 * pyramid.levels.len());
        }
    }

    #[test]
    fn hostile_ranges_and_nonfinite_buckets_match_flat() {
        let empty = PeakPyramid::default();
        assert_eq!(empty.range(0..usize::MAX), (0.0, 0.0));
        let peaks: PeakPyramid = [(f32::NAN, f32::NAN), (-0.5, 0.25), (f32::NEG_INFINITY, f32::INFINITY)].into_iter().collect();
        let mut ranges = vec![0..0, 0..1, 0..2, 0..3, 1..usize::MAX, usize::MAX..usize::MAX];
        ranges.push(Range { start: 3, end: 2 });
        for range in ranges {
            assert_eq!(peaks.range(range.clone()), flat(&peaks, range));
        }
    }

    #[test]
    fn compaction_preserves_queries_and_allows_more_appends() {
        let mut peaks: PeakPyramid = (0..1001).map(|index| (-(index as f32), index as f32)).collect();
        assert!(peaks.levels.iter().all(|level| level.len() == level.capacity()));
        peaks.push((-2000.0, 3000.0));
        assert_eq!(peaks.range(999..1002), (-2000.0, 3000.0));
        assert_eq!(peaks.range(0..999), (-998.0, 998.0));
    }

    #[test]
    fn randomized_ranges_match_flat() {
        let mut seed = 106;
        for _ in 0..64 {
            let len = (random(&mut seed) % 8192) as usize;
            let peaks = random_peaks(&mut seed, len);
            for _ in 0..512 {
                let start = (random(&mut seed) % (len + 256) as u64) as usize;
                let end = (random(&mut seed) % (len + 256) as u64) as usize;
                assert_eq!(peaks.range(start..end), flat(&peaks, start..end));
            }
        }
    }
}
