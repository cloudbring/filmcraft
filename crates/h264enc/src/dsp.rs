//! Block statistics for lookahead.

/// Variance*N (sum of squares minus square of sum / N) of a block.
pub fn block_var(a: &[u8], sa: usize, w: usize, h: usize) -> u32 {
    let mut s = 0u32;
    let mut ss = 0u32;
    for r in 0..h {
        for &v in &a[r * sa..r * sa + w] {
            s += v as u32;
            ss += (v as u32) * (v as u32);
        }
    }
    ss - ((s as u64 * s as u64) / (w * h) as u64) as u32
}
