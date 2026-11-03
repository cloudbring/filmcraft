//! Device-thread work only. No decoding, synchronization primitives, or buffer growth here.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use craft_audio_rt::{PanicGuard, SpscProducer};

pub(super) type Fill = Box<dyn FnMut(&mut [f32], usize) + Send>;

pub(super) struct Callback {
    fill: Option<Fill>,
    return_to_ui: SpscProducer<Fill>,
    guard: PanicGuard,
}

impl Callback {
    pub(super) fn new(fill: Fill, return_to_ui: SpscProducer<Fill>) -> Self {
        Self { fill: Some(fill), return_to_ui, guard: PanicGuard::new() }
    }

    pub(super) fn render(&mut self, output: &mut [f32], channels: usize, played: &AtomicU64, failed: &AtomicBool) {
        let Some(fill) = self.fill.as_mut() else {
            output.fill(0.0);
            return;
        };
        if failed.load(Ordering::Relaxed) || !self.guard.render(&mut |block: &mut [f32]| fill(block, channels), output) {
            failed.store(true, Ordering::Relaxed);
            output.fill(0.0);
            return;
        }
        played.fetch_add((output.len() / channels.max(1)) as u64, Ordering::Relaxed);
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        if let Some(fill) = self.fill.take() {
            let _ = self.return_to_ui.push(fill);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_audio_rt::Spsc;

    #[test]
    fn callback_counts_frames_and_returns_its_fill_on_drop() {
        let (sender, mut receiver) = Spsc::new(1).split();
        let (played, failed) = (AtomicU64::new(0), AtomicBool::new(false));
        let mut callback = Callback::new(Box::new(|buf, _| buf.fill(0.25)), sender);
        let mut output = [1.0; 8];
        callback.render(&mut output, 2, &played, &failed);
        assert_eq!((output, played.load(Ordering::Relaxed)), ([0.25; 8], 4));
        callback.render(&mut output, 0, &played, &failed);
        assert_eq!(played.load(Ordering::Relaxed), 12);
        drop(callback);
        assert!(receiver.pop().is_some());
    }

    #[test]
    fn steady_state_device_callback_makes_no_allocations() {
        let (sender, _) = Spsc::new(1).split();
        let (played, failed) = (AtomicU64::new(0), AtomicBool::new(false));
        let mut callback = Callback::new(Box::new(|buf, _| buf.fill(0.25)), sender);
        let mut output = [0.0; 512];
        callback.render(&mut output, 2, &played, &failed);
        let allocations = allocation_counter::measure(|| {
            for _ in 0..100 {
                callback.render(&mut output, 2, &played, &failed);
            }
        });
        assert_eq!(allocations.count_total, 0);
        assert_eq!(played.load(Ordering::Relaxed), 101 * 256);
    }

    #[test]
    fn panic_silences_current_and_later_blocks() {
        let (sender, _) = Spsc::new(1).split();
        let (played, failed) = (AtomicU64::new(0), AtomicBool::new(false));
        let mut callback = Callback::new(
            Box::new(|buf, _| {
                buf.fill(0.5);
                panic!("mixer bug")
            }),
            sender,
        );
        let mut output = [1.0; 4];
        callback.render(&mut output, 2, &played, &failed);
        assert_eq!(output, [0.0; 4]);
        assert!(failed.load(Ordering::Relaxed));
        output.fill(1.0);
        callback.render(&mut output, 2, &played, &failed);
        assert_eq!((output, played.load(Ordering::Relaxed)), ([0.0; 4], 0));
    }

    #[test]
    fn device_callback_has_no_blocking_or_allocating_operations() {
        let source = include_str!("audio_callback.rs").split("#[cfg(test)]").next().unwrap();
        for forbidden in ["Mutex", "mpsc", "try_lock", "Vec::", "reserve(", "resize(", "Box::new", "println!", "eprintln!"] {
            assert!(!source.contains(forbidden), "device callback contains {forbidden}");
        }
    }
}
