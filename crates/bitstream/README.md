# filmcraft-bitstream

L0 bitstream foundation: re-exports `craft_media::bitstream` and owns the allocation-free
`BoolDecoder` used by both VP8 and VP9. No unsafe code or codec dependencies.

## Boolean decoder (#131)

Implemented from RFC 6386 section 7 (VP8) and the *VP9 Bitstream & Decoding Process
Specification*, v0.6, 31 March 2016, sections 9.2–9.3. No third-party codec source was used.
The 64-bit value window refills seven or eight bytes at a time with safe slice access;
symbol decoding uses masked range selection and leading-zero-count normalisation, with
no input indexing outside refill. Probability adaptation remains in each codec.

- `new` starts a VP8 partition, preserving zero extension beyond its end.
- `new_vp9` rejects empty partitions and nonzero marker bits.
- `finish_vp9` checks actual consumption rather than prefetched bytes, rejects overreads,
  and checks all remaining buffered/unread padding bits for zero.
- Literals, equiprobable flags, signed fields and tree/subtree reads share the same state.
  Literal and signed widths are capped to their return types. Malformed trees have bounded
  traversal, return zero and record an error reported by `finish_vp9`.

Tests exhaust every initial byte and probability (including 0 and 255), compare a million
random symbols per codec with a bit-at-a-time reference, sweep all truncations through
1024 bytes, and cover literals, trees, signed fields, markers, padding and long overreads.
The RFC-style VP8 test encoder also round-trips all probabilities against this decoder.

Run `cargo test -p filmcraft-bitstream -p filmcraft-vp8 -p filmcraft-vp9`.
The codec suites compare generated fixtures byte-for-byte with ffmpeg's external decoder;
VP8's optional conformance corpus asserts its existing per-frame MD5 golden hashes.
Set `FILMCRAFT_FFMPEG` for an external oracle outside the standard installation paths.
Decode-only release benchmarks: VP8 `--test it perf::perf_1080p` and VP9
`--test it perf::perf_4k`, both with `-- --ignored --nocapture`.
