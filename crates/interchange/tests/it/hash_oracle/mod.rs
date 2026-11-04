//! #188: OLD (inline FNV / CRC-32 / SHA-256) vs NEW (`craft-hash`) digests across filmcraft.
//!
//! OLD is vendored in `old.rs` (filmcraft @ eb59ec09daaea519c9c8fffa404a594ae50d00e0). NEW is
//! `craft_hash`, called the way each swapped site calls it (the sites are private to their crates,
//! so each `new_*` below restates the call exactly as the tree has it, with its location):
//! seeded `Fnv64` (interchange aaf/omf write.rs), `fnv1a64` (engine graphic_templates.rs, ui-egui
//! menus.rs / ime.rs), `crc32_mpeg2` (mpegts ts.rs), the `hash128` rewrite (mxf write.rs),
//! `ContentHash` behind the render preview's JSON hasher, and `Sha256::hex` (speech models.rs,
//! tts weights.rs). Inputs: random bytes at every chunking, plus corpus files.

mod old;

use proptest::prelude::*;
use std::path::PathBuf;

// ---- NEW, as the tree has it ------------------------------------------------------------------

/// interchange/src/aaf/write.rs and omf/write.rs
fn new_fnv(s: &[u8], h: u64) -> u64 {
    let mut hash = craft_hash::Fnv64::from_state(h);
    hash.update(s);
    hash.finish()
}

/// mxf/src/write.rs
fn new_hash128(seed: &[u8]) -> [u8; 16] {
    let a = craft_hash::fnv1a64(seed);
    let mut b = 0x84222325_cbf29ce4u64 ^ seed.len() as u64;
    for &x in seed {
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

/// render/src/preview.rs
struct NewFnv128(craft_hash::ContentHash);

impl NewFnv128 {
    fn new() -> Self {
        Self(craft_hash::ContentHash::new())
    }
    fn json<T: serde::Serialize + ?Sized>(&mut self, v: &T) {
        let _ = serde_json::to_writer(&mut *self, v);
        self.0.update(&[0xff]);
    }
    fn hex(&self) -> String {
        format!("{:032x}", self.0.finish())
    }
}

impl std::io::Write for NewFnv128 {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// speech/src/models.rs `valid_file`
fn new_sha256_hex(data: &[u8]) -> String {
    let mut hasher = craft_hash::Sha256::new();
    std::io::copy(&mut &data[..], &mut hasher).unwrap();
    hasher.hex()
}

fn check_all(bytes: &[u8], seed: u64) {
    assert_eq!(new_fnv(bytes, seed), old::fnv(bytes, seed), "aaf/omf fnv, seed {seed:#x}");
    assert_eq!(craft_hash::fnv1a64(bytes), old::fnv_fonts(bytes), "fnv fonts/thumbnail");
    assert_eq!(craft_hash::fnv1a64(bytes), old::fnv_clipboard(bytes), "fnv clipboard key / ime stamp");
    assert_eq!(craft_hash::crc32_mpeg2(bytes), old::crc32_mpeg(bytes), "mpegts crc32");
    assert_eq!(new_hash128(bytes), old::hash128(bytes), "mxf hash128");
    assert_eq!(new_sha256_hex(bytes), old::sha256_hex(bytes), "sha256 hex");
}

/// Up to `max` corpus files (each at most 6 MB), spread evenly over the sorted file list.
fn corpus_sample(max: usize) -> Vec<(PathBuf, Vec<u8>)> {
    let root = PathBuf::from(std::env::var("ARTCRAFT_CORPUS").unwrap_or_else(|_| "/Users/emm/ArtCraft/corpus".into()));
    let mut files = vec![];
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if e.metadata().is_ok_and(|m| m.len() > 0 && m.len() <= 6 << 20) {
                files.push(p);
            }
        }
    }
    files.sort();
    if files.is_empty() {
        eprintln!("SKIP corpus: no files under {}", root.display());
        return vec![];
    }
    let step = (files.len() / max).max(1);
    files.into_iter().step_by(step).take(max).filter_map(|p| std::fs::read(&p).ok().map(|b| (p, b))).collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn random_bytes_digest_identically(bytes in prop::collection::vec(any::<u8>(), 0..70_000), seed in prop_oneof![Just(0u64), Just(0xcbf2_9ce4_8422_2325), Just(u64::MAX), any::<u64>()]) {
        check_all(&bytes, seed);
    }

    #[test]
    fn short_inputs_and_block_boundaries(len in prop_oneof![0usize..200, 63usize..66, 183usize..190], fill in any::<u8>(), seed in any::<u64>()) {
        let bytes: Vec<u8> = (0..len).map(|i| fill.wrapping_add(i as u8).wrapping_mul(31)).collect();
        check_all(&bytes, seed);
    }

    /// Feeding the same bytes in pieces gives the same seeded FNV as one call.
    #[test]
    fn seeded_fnv_is_chunk_independent(bytes in prop::collection::vec(any::<u8>(), 0..3000), seed in any::<u64>(), cut in 1usize..200) {
        let mut state = seed;
        for piece in bytes.chunks(cut) {
            state = new_fnv(piece, state);
        }
        prop_assert_eq!(state, old::fnv(&bytes, seed));
    }

    /// The render preview hasher over JSON values (strings, numbers, nested), incl. multi-byte text.
    #[test]
    fn preview_json_hash_matches(values in prop::collection::vec(("[ -~🦀é]{0,30}", any::<i64>(), any::<bool>()), 0..12)) {
        let (mut old_h, mut new_h) = (old::Fnv128::new(), NewFnv128::new());
        for v in &values {
            old_h.json(v);
            new_h.json(v);
            prop_assert_eq!(old_h.hex(), new_h.hex());
        }
    }
}

#[test]
fn corpus_files_digest_identically() {
    let files = corpus_sample(80);
    let mut total = 0u64;
    for (i, (_path, bytes)) in files.iter().enumerate() {
        check_all(bytes, 0xcbf2_9ce4_8422_2325 ^ i as u64);
        total += bytes.len() as u64;
    }
    eprintln!("corpus: {} files, {} bytes digested identically", files.len(), total);
}
