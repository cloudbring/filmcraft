//! Audio played ahead: playback mixes on its own thread into a ring buffer that stays
//! [`LEAD_S`] ahead of the device, and the device callback only copies samples out of it.
//!
//! Mixing (decoding AAC, effects, opening a clip's file at a cut) used to run inside the device
//! callback, which has one I/O buffer (~10 ms) to return. On a busy Mac any stall there missed the
//! device's deadline, heard as static and dropouts that were different on every play. Now a stall
//! only drains part of the lead; the callback never decodes, allocates or takes a lock.
//!
//! When the ring does run dry (an underrun), the callback plays silence for the missing frames and
//! counts them, and the mixer skips the same span, so sound stays locked to the picture.
//!
//! Desktop only: the web build has no threads and keeps mixing in its callback.

use craft_audio_rt::Spsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

/// How far ahead of the device the mixer keeps the ring (s). Edits made while playing are heard
/// this much later.
pub const LEAD_S: f64 = 0.2;
/// Frames mixed per block on the mixing thread.
pub const BLOCK_FRAMES: usize = 1024;
/// Ceiling on mixed channels (large interfaces and aggregate devices stay well under it).
const MAX_CHANNELS: usize = 64;
const MAX_CALLBACK_FRAMES: usize = 65_536;

/// Underrun counters for the current (or last) play, readable from any thread (`perf.stats`).
#[derive(Debug, Default)]
pub struct AudioStats {
    /// Device callbacks served.
    pub callbacks: AtomicU64,
    /// Callbacks the ring could not fill completely.
    pub underruns: AtomicU64,
    /// Frames played as silence because the ring was empty.
    pub missing_frames: AtomicU64,
    /// Fewest frames left in the ring after a callback, once it first filled to the lead
    /// (`u64::MAX` until then).
    pub min_lead_frames: AtomicU64,
}

impl AudioStats {
    pub fn reset(&self) {
        self.callbacks.store(0, Ordering::Relaxed);
        self.underruns.store(0, Ordering::Relaxed);
        self.missing_frames.store(0, Ordering::Relaxed);
        self.min_lead_frames.store(u64::MAX, Ordering::Relaxed);
    }

    pub fn to_json(&self, sample_rate: u32) -> serde_json::Value {
        let min = self.min_lead_frames.load(Ordering::Relaxed);
        serde_json::json!({
            "callbacks": self.callbacks.load(Ordering::Relaxed),
            "underruns": self.underruns.load(Ordering::Relaxed),
            "missingFrames": self.missing_frames.load(Ordering::Relaxed),
            "minLeadMs": if min == u64::MAX || sample_rate == 0 { serde_json::Value::Null } else { serde_json::json!(min as f64 * 1e3 / sample_rate as f64) },
        })
    }
}

/// Stops the mixing thread when the device callback that owns it is dropped (stream stopped).
struct StopOnDrop(Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

/// Mixes `mix(cursor, buf, channels)` (interleaved frames from device frame `cursor`) on its own
/// thread, from `start`, and returns the device callback that plays it (`fill(buf, channels)`).
/// The first block is mixed before this returns, so playback starts with sound. The thread runs
/// until the returned callback is dropped.
pub fn spawn<M>(mix: M, start: i64, sample_rate: u32, channels: usize, stats: Arc<AudioStats>) -> Box<dyn FnMut(&mut [f32], usize) + Send>
where
    M: FnMut(i64, &mut [f32], usize) + Send + 'static,
{
    spawn_with_reset(mix, start, sample_rate, channels, stats, None)
}

/// A scrub-position change invalidates samples already mixed for the previous position without
/// reopening the device. The mixer waits until the callback has drained the old generation.
#[derive(Default)]
pub struct ResetState {
    requested: AtomicU64,
    acknowledged: AtomicU64,
}

impl ResetState {
    pub fn request(&self) {
        self.requested.fetch_add(1, Ordering::Release);
    }
}

pub fn spawn_resettable<M>(
    mix: M,
    start: i64,
    sample_rate: u32,
    channels: usize,
    stats: Arc<AudioStats>,
) -> (Box<dyn FnMut(&mut [f32], usize) + Send>, Arc<ResetState>)
where
    M: FnMut(i64, &mut [f32], usize) + Send + 'static,
{
    let generation = Arc::new(ResetState::default());
    let fill = spawn_with_reset(mix, start, sample_rate, channels, stats, Some(generation.clone()));
    (fill, generation)
}

fn spawn_with_reset<M>(
    mut mix: M,
    start: i64,
    sample_rate: u32,
    channels: usize,
    stats: Arc<AudioStats>,
    reset: Option<Arc<ResetState>>,
) -> Box<dyn FnMut(&mut [f32], usize) + Send>
where
    M: FnMut(i64, &mut [f32], usize) + Send + 'static,
{
    stats.reset();
    let ch = channels.clamp(1, MAX_CHANNELS);
    let bounded_rate = sample_rate.min(384_000) as usize;
    let lead = ((bounded_rate as f64 * LEAD_S) as usize).max(BLOCK_FRAMES * 2);
    // One block of headroom beyond the lead plus the largest callback we serve. Larger
    // pathological driver buffers get silence, not a callback-side allocation.
    let capacity = lead.saturating_add(BLOCK_FRAMES).saturating_add(MAX_CALLBACK_FRAMES).saturating_mul(ch);
    let (mut producer, mut consumer) = Spsc::<f32>::new(capacity).split();
    let mut block = vec![0.0; BLOCK_FRAMES * ch];
    mix(start, &mut block, ch);
    producer.push_slice(&block);
    // Frames the callback played as silence that the mixer has not skipped yet.
    let skip = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    {
        let (skip, stop) = (skip.clone(), stop.clone());
        let generation = reset.clone();
        let body = move || {
            let mut cursor = start + BLOCK_FRAMES as i64;
            while !stop.load(Ordering::Acquire) {
                if generation.as_ref().is_some_and(|state| state.acknowledged.load(Ordering::Acquire) != state.requested.load(Ordering::Acquire)) {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                if capacity.saturating_sub(producer.slots()) + block.len() > lead * ch {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                cursor += skip.swap(0, Ordering::AcqRel) as i64;
                block.fill(0.0);
                let current_generation = generation.as_ref().map(|state| state.requested.load(Ordering::Acquire));
                mix(cursor, &mut block, ch);
                cursor += BLOCK_FRAMES as i64;
                if generation.as_ref().map(|state| state.requested.load(Ordering::Acquire)) != current_generation {
                    continue;
                }
                // silence played while this block was mixing: its first frames are already late
                let late = skip.swap(0, Ordering::AcqRel) as usize;
                let drop = late.min(BLOCK_FRAMES);
                cursor += (late - drop) as i64;
                producer.push_slice(block.get(drop * ch..).unwrap_or_default());
            }
        };
        let spawned = std::thread::Builder::new().name("filmcraft-audio-mix".into()).spawn(move || {
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)).is_err() {
                log::error!("audio mixing stopped after a panic; playback continues silent");
            }
        });
        if let Err(e) = spawned {
            log::error!("audio mixing thread: {e}");
        }
    }
    let guard = StopOnDrop(stop);
    let mut primed = false;
    let mut seen_generation = 0;
    Box::new(move |buf: &mut [f32], dev_ch: usize| {
        let _keep = &guard;
        stats.callbacks.fetch_add(1, Ordering::Relaxed);
        // a device wider or narrower than the mix (it changed since `channels` was read) still
        // plays the channels both have
        let dev_ch = dev_ch.max(1);
        if let Some(generation) = &reset {
            let latest = generation.requested.load(Ordering::Acquire);
            if latest != seen_generation {
                seen_generation = latest;
                while consumer.pop().is_some() {}
                generation.acknowledged.store(latest, Ordering::Release);
            }
        }
        let have = (buf.len() / dev_ch).min(consumer.slots() / ch);
        for frame in buf.chunks_mut(dev_ch).take(have) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                *sample = if channel < ch { consumer.pop().unwrap_or(0.0) } else { 0.0 };
            }
            for _ in frame.len()..ch {
                let _ = consumer.pop();
            }
        }
        if let Some(rest) = buf.get_mut(have * dev_ch..) {
            rest.fill(0.0);
        }
        let n = buf.len() / dev_ch;
        if have < n {
            stats.underruns.fetch_add(1, Ordering::Relaxed);
            stats.missing_frames.fetch_add((n - have) as u64, Ordering::Relaxed);
            skip.fetch_add((n - have) as u64, Ordering::AcqRel);
        }
        let left = consumer.slots();
        primed |= left + 2 * BLOCK_FRAMES * ch > lead * ch;
        if primed {
            stats.min_lead_frames.fetch_min((left / ch) as u64, Ordering::Relaxed);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_audio_rt::AtomicParam;

    #[test]
    fn underrun_is_silence_and_counts_missing_frames_without_allocating() {
        let stats = Arc::new(AudioStats::default());
        let mut first = true;
        let mut fill = spawn(
            move |_, buf: &mut [f32], _| {
                if !first {
                    std::thread::sleep(Duration::from_millis(100));
                }
                first = false;
                buf.fill(0.5);
            },
            0,
            48_000,
            2,
            stats.clone(),
        );
        let mut output = [1.0; 8192];
        let allocations = allocation_counter::measure(|| fill(&mut output, 2));
        assert_eq!(allocations.count_total, 0);
        assert!(output[..BLOCK_FRAMES * 2].iter().all(|sample| *sample == 0.5));
        assert!(output[BLOCK_FRAMES * 2..].iter().all(|sample| *sample == 0.0));
        assert_eq!(stats.underruns.load(Ordering::Relaxed), 1);
        assert_eq!(stats.missing_frames.load(Ordering::Relaxed), 4096 - BLOCK_FRAMES as u64);
    }

    #[test]
    fn reset_discards_old_grain_and_plays_new_samples() {
        let level = Arc::new(AtomicParam::new(0.25));
        let worker_level = level.clone();
        let (mut fill, reset) = spawn_resettable(move |_, buf: &mut [f32], _| buf.fill(worker_level.load()), 0, 48_000, 2, Arc::new(AudioStats::default()));
        let mut output = [0.0; BLOCK_FRAMES * 2];
        fill(&mut output, 2);
        assert!(output.iter().all(|sample| *sample == 0.25));
        level.store(0.75);
        reset.request();
        fill(&mut output, 2);
        assert!(output.iter().all(|sample| *sample != 0.25), "old samples are discarded");
        let mut ready = false;
        for _ in 0..50 {
            std::thread::sleep(Duration::from_millis(2));
            fill(&mut output, 2);
            if output.contains(&0.75) {
                ready = true;
                assert!(output.iter().all(|sample| *sample == 0.75 || *sample == 0.0));
                break;
            }
        }
        assert!(ready, "the worker resumes after the callback acknowledges the reset");
    }
}
