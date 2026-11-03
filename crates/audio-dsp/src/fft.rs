//! Split-complex adapter for the shared, preplanned `craft-fft` kernels.
//!
//! Both directions retain FilmCraft's unscaled convention. Planning allocates; processing does not.

use craft_fft::{Complex, ComplexFft, Direction, Error};

/// A fixed-size FFT with reusable interleaved storage and backend scratch.
pub struct Fft {
    plan: ComplexFft<f32>,
    buffer: Vec<Complex<f32>>,
}

impl Fft {
    /// Plan a power-of-two FFT of at least two samples.
    pub fn new(n: usize) -> Result<Self, Error> {
        if n < 2 || !n.is_power_of_two() {
            return Err(Error::InvalidSize);
        }
        Ok(Self { plan: ComplexFft::new(n)?, buffer: vec![Complex::new(0.0, 0.0); n] })
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// Forward transform, unscaled. Elements beyond the plan length are untouched.
    pub fn forward(&mut self, re: &mut [f32], im: &mut [f32]) -> Result<(), Error> {
        self.run(re, im, Direction::Forward)
    }

    /// Inverse transform, unscaled (divide by the plan length to round-trip).
    pub fn inverse(&mut self, re: &mut [f32], im: &mut [f32]) -> Result<(), Error> {
        self.run(re, im, Direction::Inverse)
    }

    fn run(&mut self, re: &mut [f32], im: &mut [f32], direction: Direction) -> Result<(), Error> {
        let length = self.len();
        if re.len() < length || im.len() < length {
            return Err(Error::Length { expected: length, actual: re.len().min(im.len()) });
        }
        for ((value, real), imaginary) in self.buffer.iter_mut().zip(re.iter()).zip(im.iter()) {
            *value = Complex::new(*real, *imaginary);
        }
        self.plan.process_in_place(direction, &mut self.buffer)?;
        let scale = if direction == Direction::Inverse { length as f32 } else { 1.0 };
        for ((value, real), imaginary) in self.buffer.iter().zip(re.iter_mut()).zip(im.iter_mut()) {
            *real = value.re * scale;
            *imaginary = value.im * scale;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_naive_dft_and_round_trips() {
        let length = 64;
        let mut fft = Fft::new(length).unwrap();
        let input: Vec<f32> = (0..length).map(|index| ((index * 7 % 13) as f32 - 6.0) / 6.0).collect();
        let mut real = input.clone();
        let mut imaginary = vec![0.0; length];
        fft.forward(&mut real, &mut imaginary).unwrap();
        for bin in 0..length {
            let mut expected = Complex::new(0.0f64, 0.0);
            for (time, &sample) in input.iter().enumerate() {
                let angle = -2.0 * std::f64::consts::PI * (bin * time) as f64 / length as f64;
                expected += Complex::new(angle.cos(), angle.sin()) * sample as f64;
            }
            assert!((real[bin] as f64 - expected.re).abs() < 1e-4);
            assert!((imaginary[bin] as f64 - expected.im).abs() < 1e-4);
        }
        fft.inverse(&mut real, &mut imaginary).unwrap();
        for index in 0..length {
            assert!((real[index] / length as f32 - input[index]).abs() < 1e-6);
            assert!((imaginary[index] / length as f32).abs() < 1e-6);
        }
    }

    #[test]
    fn invalid_arguments_do_not_panic_or_mutate_input() {
        for length in [0, 1, 3, usize::MAX] {
            assert!(Fft::new(length).is_err());
        }
        let mut fft = Fft::new(4).unwrap();
        let mut real = [1.0; 4];
        let mut imaginary = [2.0; 3];
        assert!(fft.forward(&mut real, &mut imaginary).is_err());
        assert_eq!(real, [1.0; 4]);
        assert_eq!(imaginary, [2.0; 3]);
        let mut imaginary = [0.0; 5];
        let mut real = [1.0; 5];
        fft.forward(&mut real, &mut imaginary).unwrap();
        fft.inverse(&mut real, &mut imaginary).unwrap();
        assert_eq!(real, [4.0, 4.0, 4.0, 4.0, 1.0]);
        assert_eq!(imaginary, [0.0; 5]);
    }

    #[test]
    fn processing_including_first_call_does_not_allocate() {
        for length in [64, 128, 2048, 4096, 8192] {
            let mut fft = Fft::new(length).unwrap();
            let mut real = vec![1.0; length];
            let mut imaginary = vec![0.0; length];
            let allocations = allocation_counter::measure(|| {
                fft.forward(&mut real, &mut imaginary).unwrap();
                fft.inverse(&mut real, &mut imaginary).unwrap();
            });
            assert_eq!(allocations.count_total, 0);
        }
    }
}
