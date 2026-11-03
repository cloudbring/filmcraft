//! AAC transform adapters for craft-fft (ISO/IEC 14496-3 §4.6.18).
//!
//! AAC applies asymmetric transition windows outside the transform. The shared facade currently
//! applies a full sine window, so these adapters undo it using only the well-conditioned middle
//! half (window >= sqrt(1/2)), and restore the codec's unscaled convention.

use std::f64::consts::PI;
use std::sync::OnceLock;

use craft_fft::{Error, TdacWindow};

pub struct Mdct {
    plan: craft_fft::Mdct<f32>,
    buffer: Vec<f32>,
}

impl Mdct {
    pub fn new(coefficients: usize) -> Result<Self, Error> {
        let plan = craft_fft::Mdct::new(coefficients, TdacWindow::Sine)?;
        Ok(Self { plan, buffer: vec![0.0; 2 * coefficients] })
    }

    pub fn forward(&mut self, input: &[f32], output: &mut [f32]) -> Result<(), Error> {
        let length = self.plan.coefficients_len();
        if input.len() != 2 * length {
            return Err(Error::Length { expected: 2 * length, actual: input.len() });
        }
        let half = length / 2;
        let window = self.plan.window();
        for index in 0..half {
            let left = length - 1 - index;
            let right = 3 * half - 1 - index;
            self.buffer[left] = (input[left] - input[index]) / window[left];
            self.buffer[right] = (input[right] + input[3 * half + index]) / window[right];
        }
        self.plan.process_into(&self.buffer, output)
    }
}

pub struct MdctScratch {
    plan: craft_fft::Imdct<f32>,
    gains: Vec<f32>,
}

impl MdctScratch {
    pub fn new(coefficients: usize) -> Result<Self, Error> {
        let plan = craft_fft::Imdct::new(coefficients, TdacWindow::Sine)?;
        let half = coefficients / 2;
        let gains = plan.window()[half..3 * half].iter().map(|window| half as f32 / window).collect();
        Ok(Self { plan, gains })
    }

    pub fn inverse_with(&mut self, spectrum: &[f32], output: &mut [f32]) -> Result<(), Error> {
        self.plan.process_into(spectrum, output)?;
        let length = self.plan.coefficients_len();
        let half = length / 2;
        for (sample, gain) in output[half..3 * half].iter_mut().zip(&self.gains) {
            *sample *= gain;
        }
        for index in 0..half {
            output[index] = -output[length - 1 - index];
            output[3 * half + index] = output[3 * half - 1 - index];
        }
        Ok(())
    }
}

/// Window shape (`window_shape` bit): 0 = sine, 1 = Kaiser–Bessel derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowShape {
    #[default]
    Sine,
    Kbd,
}

impl WindowShape {
    pub fn from_bit(b: bool) -> Self {
        if b { WindowShape::Kbd } else { WindowShape::Sine }
    }
    pub fn bit(self) -> u32 {
        (self == WindowShape::Kbd) as u32
    }
}

fn sine_half(m: usize) -> Vec<f32> {
    let n = 2 * m;
    (0..m).map(|i| (PI / n as f64 * (i as f64 + 0.5)).sin() as f32).collect()
}

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let y = x * x / 4.0;
    for k in 1..60 {
        term *= y / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

fn kbd_half(m: usize, alpha: f64) -> Vec<f32> {
    // w[n] = sqrt(Σ_{j<=n} W(j) / Σ_{j<=M} W(j)), W the Kaiser kernel of length M+1.
    let kernel: Vec<f64> = (0..=m)
        .map(|j| {
            let r = (j as f64 - m as f64 / 2.0) / (m as f64 / 2.0);
            bessel_i0(PI * alpha * (1.0 - r * r).max(0.0).sqrt())
        })
        .collect();
    let total: f64 = kernel.iter().sum();
    let mut acc = 0.0;
    (0..m)
        .map(|n| {
            acc += kernel[n];
            (acc / total).sqrt() as f32
        })
        .collect()
}

/// Rising half (length N/2) of the window of the given shape; `long` selects N = 2048, else N = 256.
pub fn window(shape: WindowShape, long: bool) -> &'static [f32] {
    static W: OnceLock<[Vec<f32>; 4]> = OnceLock::new();
    let w = W.get_or_init(|| [sine_half(1024), kbd_half(1024, 4.0), sine_half(128), kbd_half(128, 6.0)]);
    let i = match (long, shape) {
        (true, WindowShape::Sine) => 0,
        (true, WindowShape::Kbd) => 1,
        (false, WindowShape::Sine) => 2,
        (false, WindowShape::Kbd) => 3,
    };
    &w[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing_including_first_call_does_not_allocate() {
        for length in [128, 1024] {
            let mut forward = Mdct::new(length).unwrap();
            let mut inverse = MdctScratch::new(length).unwrap();
            let input = vec![0.25; 2 * length];
            let mut spectrum = vec![0.0; length];
            let mut output = vec![0.0; 2 * length];
            let allocations = allocation_counter::measure(|| {
                for _ in 0..16 {
                    forward.forward(&input, &mut spectrum).unwrap();
                    inverse.inverse_with(&spectrum, &mut output).unwrap();
                }
            });
            assert_eq!(allocations.count_total, 0);
        }
    }

    #[test]
    fn invalid_lengths_do_not_panic_or_modify_output() {
        for length in [0, 1, 127, usize::MAX] {
            assert!(Mdct::new(length).is_err());
            assert!(MdctScratch::new(length).is_err());
        }
        let mut forward = Mdct::new(128).unwrap();
        let mut inverse = MdctScratch::new(128).unwrap();
        let mut spectrum = [7.0; 128];
        let mut output = [7.0; 256];
        assert!(forward.forward(&[0.0; 255], &mut spectrum).is_err());
        assert!(forward.forward(&[0.0; 256], &mut spectrum[..127]).is_err());
        assert!(inverse.inverse_with(&spectrum[..127], &mut output).is_err());
        assert!(inverse.inverse_with(&spectrum, &mut output[..255]).is_err());
        assert_eq!(spectrum, [7.0; 128]);
        assert_eq!(output, [7.0; 256]);
    }

    fn direct(x: &[f32]) -> Vec<f64> {
        let m = x.len() / 2;
        (0..m).map(|k| (0..2 * m).map(|n| x[n] as f64 * (PI / m as f64 * (n as f64 + 0.5 + m as f64 / 2.0) * (k as f64 + 0.5)).cos()).sum()).collect()
    }

    #[test]
    fn forward_matches_definition() {
        for m in [128usize, 1024] {
            let x: Vec<f32> = (0..2 * m).map(|i| ((i * 7919) % 101) as f32 / 50.0 - 1.0).collect();
            let mut out = vec![0f32; m];
            Mdct::new(m).unwrap().forward(&x, &mut out).unwrap();
            let d = direct(&x);
            for k in 0..m {
                assert!((out[k] as f64 - d[k]).abs() < 1e-3 * (m as f64).sqrt(), "m={m} k={k} {} {}", out[k], d[k]);
            }
        }
    }

    #[test]
    fn inverse_matches_definition() {
        for m in [128, 1024] {
            let spec: Vec<f32> = (0..m).map(|i| ((i * 31) % 17) as f32 - 8.0).collect();
            let mut y = vec![0f32; 2 * m];
            MdctScratch::new(m).unwrap().inverse_with(&spec, &mut y).unwrap();
            for n in 0..2 * m {
                let d: f64 = (0..m).map(|k| spec[k] as f64 * (PI / m as f64 * (n as f64 + 0.5 + m as f64 / 2.0) * (k as f64 + 0.5)).cos()).sum();
                assert!((y[n] as f64 - d).abs() < 1e-3, "m={m} n={n}");
            }
        }
    }

    #[test]
    fn tdac_perfect_reconstruction() {
        let m = 1024;
        let mut t = Mdct::new(m).unwrap();
        let mut inverse = MdctScratch::new(m).unwrap();
        for shape in [WindowShape::Sine, WindowShape::Kbd] {
            let w = window(shape, true);
            let win = |n: usize| if n < m { w[n] } else { w[2 * m - 1 - n] };
            let x: Vec<f32> = (0..4 * m).map(|i| ((i as f32) * 0.013).sin() + ((i * 13) % 7) as f32 * 0.05).collect();
            let mut out = vec![0f32; 4 * m];
            for f in 0..3 {
                let seg: Vec<f32> = (0..2 * m).map(|n| x[f * m + n] * win(n)).collect();
                let mut spec = vec![0f32; m];
                t.forward(&seg, &mut spec).unwrap();
                for v in spec.iter_mut() {
                    *v *= 2.0;
                }
                let mut y = vec![0f32; 2 * m];
                inverse.inverse_with(&spec, &mut y).unwrap();
                for n in 0..2 * m {
                    out[f * m + n] += y[n] * win(n) / m as f32;
                }
            }
            for n in m..3 * m {
                assert!((out[n] - x[n]).abs() < 1e-4, "{shape:?} n={n} {} {}", out[n], x[n]);
            }
        }
    }
}
