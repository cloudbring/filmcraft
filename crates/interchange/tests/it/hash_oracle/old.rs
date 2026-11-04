//! OLD hash code, vendored verbatim for the #188 old-vs-new equivalence test.
//!
//! Source: https://github.com/storytold/filmcraft @ eb59ec09daaea519c9c8fffa404a594ae50d00e0
//! (merge-base of cloudbring/filmcraft `algo/37-craft-hash` with `origin/main`). Each item names its
//! file; bodies are unchanged (only made `pub` and renamed where two files used the same name).
//! These are private to their crates (a git dependency could not reach them), hence vendored.

/// crates/interchange/src/aaf/write.rs and omf/write.rs: seeded FNV-1a fold.
pub fn fnv(s: &[u8], h: u64) -> u64 {
    s.iter().fold(h, |h, &b| (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01B3))
}

/// crates/engine/src/graphic_templates.rs (font dedupe) and ui-egui menus.rs / ime.rs: the plain fold.
pub fn fnv_fonts(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100_0000_01b3))
}

pub fn fnv_clipboard(rgba: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in rgba {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// crates/mpegts/src/ts.rs: byte-wise table for the MSB-first CRC-32 (polynomial 0x04C11DB7).
const CRC_TABLE: [u32; 256] = {
    let mut t = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = (i as u32) << 24;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000_0000 != 0 { (crc << 1) ^ 0x04C1_1DB7 } else { crc << 1 };
            bit += 1;
        }
        t[i] = crc;
        i += 1;
    }
    t
};

/// crates/mpegts/src/ts.rs
pub fn crc32_mpeg(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = (crc << 8) ^ CRC_TABLE[((crc >> 24) as u8 ^ b) as usize];
    }
    crc
}

/// crates/mxf/src/write.rs
pub fn hash128(seed: &[u8]) -> [u8; 16] {
    let mut a = 0xcbf2_9ce4_8422_2325u64;
    let mut b = 0x84222325_cbf29ce4u64 ^ seed.len() as u64;
    for &x in seed {
        a = (a ^ x as u64).wrapping_mul(0x0000_0100_0000_01B3);
        b = (b ^ x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(23);
    }
    let mix = |mut z: u64| {
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut out = [0u8; 16];
    out[..8].copy_from_slice(&mix(a ^ b.rotate_left(17)).to_be_bytes());
    out[8..].copy_from_slice(&mix(b ^ a.rotate_left(41)).to_be_bytes());
    out
}

/// crates/render/src/preview.rs: 128-bit FNV-1a over the serde JSON form of values.
pub struct Fnv128(u128);

impl Fnv128 {
    const OFFSET: u128 = 0x6c62272e07bb014262b821756295c58d;
    const PRIME: u128 = 0x0000000001000000000000000000013B;
    pub fn new() -> Self {
        Self(Self::OFFSET)
    }
    pub fn json<T: serde::Serialize + ?Sized>(&mut self, v: &T) {
        let _ = serde_json::to_writer(&mut *self, v);
        self.0 ^= 0xff; // value separator
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }
    pub fn hex(&self) -> String {
        format!("{:032x}", self.0)
    }
}

impl std::io::Write for Fnv128 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &b in buf {
            self.0 ^= b as u128;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// crates/speech/src/models.rs `valid_file` and crates/tts/src/kokoro/weights.rs: SHA-256 as lowercase hex
/// through the `sha2` crate (`format!("{:x}", hasher.finalize())`).
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut &data[..], &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}
