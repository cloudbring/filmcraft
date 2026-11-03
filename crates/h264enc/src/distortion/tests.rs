use super::{sad, sad8, sad16, satd, sse};
use std::hint::black_box;
use std::time::Instant;

const SHAPES: [(usize, usize); 9] = [(4, 4), (4, 8), (4, 16), (8, 4), (8, 8), (8, 16), (16, 4), (16, 8), (16, 16)];

fn reference_sad(first: &[u8], first_stride: usize, second: &[u8], second_stride: usize, width: usize, height: usize) -> u32 {
    let mut total = 0;
    for row in 0..height {
        for column in 0..width {
            total += u32::from(first[row * first_stride + column].abs_diff(second[row * second_stride + column]));
        }
    }
    total
}

fn reference_sse(first: &[u8], first_stride: usize, second: &[u8], second_stride: usize, width: usize, height: usize) -> u64 {
    let mut total = 0;
    for row in 0..height {
        for column in 0..width {
            let difference = i32::from(first[row * first_stride + column]) - i32::from(second[row * second_stride + column]);
            total += (difference * difference) as u64;
        }
    }
    total
}

fn hadamard(values: [i32; 4]) -> [i32; 4] {
    let sum01 = values[0] + values[1];
    let diff01 = values[0] - values[1];
    let sum23 = values[2] + values[3];
    let diff23 = values[2] - values[3];
    [sum01 + sum23, diff01 + diff23, sum01 - sum23, diff01 - diff23]
}

fn reference_satd(first: &[u8], first_stride: usize, second: &[u8], second_stride: usize, width: usize, height: usize) -> u32 {
    let mut total = 0;
    for block_row in (0..height).step_by(4) {
        for block_column in (0..width).step_by(4) {
            let mut transformed = [[0; 4]; 4];
            for row in 0..4 {
                transformed[row] = hadamard(std::array::from_fn(|column| {
                    i32::from(first[(block_row + row) * first_stride + block_column + column])
                        - i32::from(second[(block_row + row) * second_stride + block_column + column])
                }));
            }
            for column in 0..4 {
                total += hadamard(std::array::from_fn(|row| transformed[row][column])).iter().map(|value| value.unsigned_abs()).sum::<u32>();
            }
        }
    }
    total / 2
}

fn check(first: &[u8], first_stride: usize, second: &[u8], second_stride: usize, width: usize, height: usize) {
    let expected = reference_sad(first, first_stride, second, second_stride, width, height);
    assert_eq!(sad(first, first_stride, second, second_stride, width, height), expected, "SAD {width}x{height}");
    if width == 8 {
        assert_eq!(sad8(first, first_stride, second, second_stride, height), expected);
    }
    if width == 16 {
        assert_eq!(sad16(first, first_stride, second, second_stride, height), expected);
    }
    assert_eq!(
        sse(first, first_stride, second, second_stride, width, height),
        reference_sse(first, first_stride, second, second_stride, width, height),
        "SSE {width}x{height}"
    );
    assert_eq!(
        satd(first, first_stride, second, second_stride, width, height),
        reference_satd(first, first_stride, second, second_stride, width, height),
        "SATD {width}x{height}"
    );
}

#[test]
fn all_byte_pairs() {
    for left in 0..=255 {
        for right in 0..=255 {
            let first = [left; 256];
            let second = [right; 256];
            for (width, height) in SHAPES {
                check(&first, 16, &second, 16, width, height);
            }
        }
    }
}

#[test]
fn random_strides_offsets_and_exact_lengths() {
    let mut seed = 0x1234_5678u32;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as u8
    };
    for (width, height) in SHAPES {
        for iteration in 0..1024 {
            let first_stride = width + iteration % 17;
            let second_stride = width + iteration % 23;
            let first_offset = iteration % 16;
            let second_offset = (iteration / 16) % 16;
            let first: Vec<_> = (0..first_offset + (height - 1) * first_stride + width).map(|_| random()).collect();
            let second: Vec<_> = (0..second_offset + (height - 1) * second_stride + width).map(|_| random()).collect();
            check(&first[first_offset..], first_stride, &second[second_offset..], second_stride, width, height);
        }
    }
}

#[test]
fn extremes_checkerboards_and_impulses() {
    let zero = [0; 256];
    let full = [255; 256];
    let checker: [u8; 256] = std::array::from_fn(|index| if (index / 16 + index % 16) % 2 == 0 { 255 } else { 0 });
    let inverse = checker.map(|value| 255 - value);
    for (width, height) in SHAPES {
        for first in [&zero, &full, &checker, &inverse] {
            for second in [&zero, &full, &checker, &inverse] {
                check(first, 16, second, 16, width, height);
            }
        }
        for row in 0..height {
            for column in 0..width {
                let mut impulse = zero;
                impulse[row * 16 + column] = 255;
                check(&impulse, 16, &zero, 16, width, height);
                check(&full, 16, &impulse, 16, width, height);
            }
        }
    }
    assert_eq!(sad16(&full, 16, &zero, 16, 16), 65_280);
    assert_eq!(sse(&full, 16, &zero, 16, 16, 16), 16_646_400);
}

#[test]
fn satd_retains_four_by_four_tiling() {
    let first = [10; 256];
    let second = [0; 256];
    assert_eq!(satd(&first, 4, &second, 4, 4, 4), 80);
    assert_eq!(satd(&first, 8, &second, 8, 8, 8), 320);
    for width in [4, 8, 12, 16] {
        for height in [4, 8, 12, 16] {
            assert_eq!(satd(&first, 16, &second, 16, width, height), reference_satd(&first, 16, &second, 16, width, height));
        }
    }
}

#[test]
fn rejects_invalid_geometry_without_partial_scores() {
    let pixels = [255; 256];
    for (stride, width, height) in [(0, 4, 4), (3, 4, 4), (16, 0, 4), (16, 4, 0), (16, 4, 17), (usize::MAX, 4, 4), (16, usize::MAX, 4), (16, 4, usize::MAX)] {
        assert_eq!(sad(&pixels, stride, &pixels, 16, width, height), u32::MAX);
        assert_eq!(satd(&pixels, stride, &pixels, 16, width, height), u32::MAX);
        assert_eq!(sse(&pixels, stride, &pixels, 16, width, height), u64::MAX);
    }
    for (width, height) in SHAPES {
        let length = (height - 1) * 16 + width;
        for short in [&pixels[..0], &pixels[..length - 1]] {
            for (first, second) in [(short, &pixels[..]), (&pixels[..], short)] {
                assert_eq!(sad(first, 16, second, 16, width, height), u32::MAX);
                assert_eq!(satd(first, 16, second, 16, width, height), u32::MAX);
                assert_eq!(sse(first, 16, second, 16, width, height), u64::MAX);
            }
        }
    }
}

#[test]
#[ignore = "release-only distortion microbenchmark; run with --nocapture"]
fn benchmark_kernels() {
    let first: [u8; 256] = std::array::from_fn(|index| (index * 73 + 19) as u8);
    let second: [u8; 256] = std::array::from_fn(|index| (index * 31 + 237) as u8);
    for (width, height) in SHAPES {
        for (name, vectorized) in [("scalar", false), ("wide", true)] {
            let started = Instant::now();
            for _ in 0..100_000 {
                let first = black_box(&first[..]);
                let second = black_box(&second[..]);
                if vectorized {
                    black_box((
                        sad(first, 16, second, 16, width, height),
                        satd(first, 16, second, 16, width, height),
                        sse(first, 16, second, 16, width, height),
                    ));
                } else {
                    black_box((
                        reference_sad(first, 16, second, 16, width, height),
                        reference_satd(first, 16, second, 16, width, height),
                        reference_sse(first, 16, second, 16, width, height),
                    ));
                }
            }
            eprintln!("{name} {width}x{height}: {:.1} ns/SAD+SATD+SSE", started.elapsed().as_nanos() as f64 / 100_000.0);
        }
    }
}
