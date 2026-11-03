# Shared bitstream and codec migration (#48)

## HEVC and AV1 dependency boundary

- Base: FilmCraft `eb3487a5c24c0d9d03fee9c01c207d52c050d8aa`
  (`algo/48-use-craft-h264`); implementation branch: `algo/48-use-craft-hevc-av1`.
- Shared source: `cloudbring/craft-libs`, branch `algo/48-craft-av1`, locked at
  `58f8a71e1f29d1ff6238e8eed5215873249550c9`. All four shared dependencies
  (`craft-bitstream`, `craft-h264`, `craft-hevc`, `craft-av1`) use this same source, avoiding
  duplicate bitstream package identities. This advances H.264 to its already-hardened version.
- `filmcraft-codecs` consumes `craft-hevc` and `craft-av1`; `filmcraft-platform` consumes
  `craft-hevc`, including the Linux VA-API and Windows NVENC source/test paths. Changes to
  consumers are crate-path renames and import ordering only. No test expectations change.
- Delete `crates/hevc` and `crates/av1`: **69 files, 32,275 lines**. Remove their workspace-layer
  registrations and the local HEVC fixture-generator registration. The `crates/*` membership
  glob needs no explicit member edits. Decoder suites/generators now live in craft-libs.
- Both shared codecs preserve the original `default = ["threads"]` and
  `threads = ["dep:rayon"]` features. Consumer dependency kinds, optionality, target conditions
  and default-feature settings are unchanged; opt-level 3 overrides follow the new names.
- Production decoder implementations match the deleted sources after crate-name normalization.
  HEVC's only other source differences are lint allowances and a test-only adapter replacing
  FilmCraft testkit calls; AV1 adds a lint allowance. FilmCraft retains its own testkit.
- The HEVC hardening on sibling branch `algo/48-hevc-deckcraft-fixes` is deliberately **not**
  included. Revalidate when that branch is merged beneath AV1, and repoint all four shared
  dependencies to `storytold/craft-libs` after upstream merge. Those are separate follow-ups.
- Current evidence: [HEVC/AV1 golden hashes](#hevc-and-av1-byte-parity) and
  [validation counts](#hevc-and-av1-validation). The original H.264 sections below retain
  the earlier migration's evidence and describe its historical pin, not the current one.

## Original H.264 dependency boundary

- Base: FilmCraft `409f5418` (`algo/131-bool-decoder`).
- Shared source: `cloudbring/craft-libs`, branch `algo/48-craft-h264`, locked at
  `935b3523ef56b00f97ff6bba4b989108e3a51a33`.
- All 13 consumer crates use `craft-bitstream`; `codecs` and `platform` also use `craft-h264`.
- The two local crates and their fixture-generator/layer registrations are removed:
  36 files, 11,751 lines. Workspace membership uses `crates/*`, so removing the directories
  removes the members. No FilmCraft testkit dependency was replaced.
- Dependency features, default-feature switches, optionality, dependency kinds and target
  conditions are unchanged. `craft-h264` retains `default`/`threads` (Rayon), and the
  opt-level 3 development override follows its new package name.
- Production implementations match the original after crate-name normalization. The shared
  H.264 crate adds a lint allowance and a test-only adapter; its synthetic tests call that adapter.
- TODO(#48): repoint to `storytold/craft-libs` once craft-h264 merges. The additional deckcraft
  and soundcraft hardening on `algo/48-h264-deckcraft-fixes` is deliberately **not** included
  in this pin; validate again when repointing.

## H.264 byte parity

Generated fixtures are local-only, using external FFmpeg/libx264 (FFmpeg 9.0.2); no media is
committed. The original and shared decoders receive the exact same Annex-B input bytes, call
`decode` followed by `flush`, reject any pending decoder error, and serialize every picture
as Y, U, V plane bytes in output order. Compare SHA-256 of the complete output per fixture.

**40 supported fixtures, 899 frames per mode, 160 matching output hashes / 3,596 decoded frames.**
Modes: 1 and 4 worker threads, each with draft mode off and on. Both thread counts have identical
hashes for each draft setting. Two hostile-input seeds (`h264_fz_10bit_422.h264` and
`h264_fz_interlaced.h264`) return the same unsupported-format error in all four modes.
The table records the matching before/after hashes; fixture input hashes bind the exact media used.

| Fixture | Input SHA-256 | Full decode SHA-256 | Draft decode SHA-256 |
|---|---|---|---|
| `baseline_cif_noise.h264` | `40b3495b87692dfce1dca60f9e5667e14c0e12c7ae618625e3abcc6edd59d69a` | `21027048f25d526c4ba20077974a24f5d52df67861c5aac88b6a77ec2da8daaf` | `21027048f25d526c4ba20077974a24f5d52df67861c5aac88b6a77ec2da8daaf` |
| `baseline_qcif.h264` | `dfc37fc9249cbc091555a6f04c2e70e0e0426f5f607af5929874775a202a5e53` | `b95e8bd7287abb5af637ae0aedda3556c21d52490ad4ce3794686d1f60d155d2` | `b95e8bd7287abb5af637ae0aedda3556c21d52490ad4ce3794686d1f60d155d2` |
| `bench_1080p.h264` | `a2490f66e4eee21ed56f89282baea3d879def976a228274ee9660be2b7cedba4` | `b07903caafc2767332bb250aea0be124c9621b6a03f203653fda28101d9381fa` | `6d7b515a8dd278ce8e36c3bdb6e88b68f4bbad02fb74b20ecee9f51705b25f55` |
| `bpyramid.h264` | `181b9ebbe063a004407920dca6a0d2c611f3954ee5045f3dcf4418aa7c418c45` | `5e1850d2f3410e2e14be096eb76a423a317196024e498eee691091094abe01c7` | `2a9a1e51980c2982292c310553956ea471b877cdd503f4244d29ca6e10d43a3d` |
| `cavlc_b.h264` | `59617930e01f14de40de2c436ea3092d114ba92f75b2563c0463ef0319548955` | `0e7ed84e17942dc01becb7feb72e66a782c5efe5972c7a466425cb78235e5c53` | `d8c417687450211dc2b8d1490b7161ef0f392b3b4c01e72380fc077a437c505e` |
| `cavlc_b_temporal.h264` | `1334f2033fede0d557b4d1e9eacfbbae84e91a52f258534a8eb8f194914ea631` | `cc3d72fc6fe5e1b05b0be8c86800d5ef01175c97935af07ab068a9905a12fda6` | `6a8c5d6c42ca53ad8afc0ae6bba994e5f2ca0bf881485cc7d4d466f62fef66b2` |
| `cavlc_weightp.h264` | `31b882da09128772e3f18bab837ecd14e1b8ba740e2d4d007e83d3a42dd176f1` | `05900d646d473df55c8cb3f4736435a9f32273fbab3009a91f5ae2330abbd271` | `01da70b1f3cea4cdc2cda799f91d1d5bd58cb79499f1ba78127d59fa1fb98f50` |
| `constrained_intra.h264` | `11211ddbee7037c8e8e431b9c107c3d9570f01b08ad442f0853293763e02666e` | `a254dd01c6edc9c889582b6f5945a2a494e0494780bbb7aa4a6fb6db216719e8` | `00388e4ac0903985c6cfba12bd0e7262f3d0beb80c23914c268c4d6cf325bf5d` |
| `cqm_jvt.h264` | `af48fa53738e66a25318a1499f6f57b4c80f081a9b24f5ad7875f25a4c47b46b` | `a2456f17a8fa603593d7cee1ac90ea5ccd67bba466690213841fb21465b0a5d4` | `99ddfaa8d55bd2b1c0b51bd70fa46ffb93a528317265bcde19e4897fced7cd8e` |
| `crop_1918x1078.h264` | `40cd59a64999898a63691a17500a47e5d00b9aeceb75a3c6bbb041c532deee1d` | `08ab9b4bc9cf180f61db165f1d65dea95dc5824ae339e00d695c33ff3465ed06` | `b2cab493efaee4c46491663a0c8bbcf4bee0770bc8756af6b2d855d406cbc2fb` |
| `deblock_m2_2.h264` | `3e4232b9a90304273fd3c2026b5cf5be3512cc66675bf6301e983d7fe156bc90` | `93f75eea3593d7273410d46c5814a385ca917ae84d7fcab21096f485d0dece4f` | `02472a127e7018329513d64723378296cd44000f13ac4474b0854737112c15dd` |
| `direct_spatial.h264` | `ecefd67a2a07bf79d470436a5cc38e9d60f43391f980c5788f7563d1e5b3aa1a` | `d8209d558377fa3f81070f25e8980e7f9432d6f7fe5ac3f1db977cef5c4602c8` | `08756f0224cb2dc932c1282dfcb230b5d3353700ac2feb972cea7545f949ffa5` |
| `direct_temporal.h264` | `93c32d85440e8145c1fe3036478b175a1e0fcc2dbf55e7057240602c96e6c4a3` | `32df96cf35c9d4796ceafa935de81bdcdd8509f83b59ae82d5b26efbe45ae0d9` | `d1c6cebb52d5852e401e4bd641410ee6755590f63d85edd5259fe587af40f2a9` |
| `h264_fz_cabac_b.h264` | `dca267743132d8015f89fcaa832bb43b5fc129e96e6ba66b11f276ea8f1d9b23` | `90e32851222abce6b36985b747fff01eff0ca41507107222e1ff6dcee7cdae56` | `90e32851222abce6b36985b747fff01eff0ca41507107222e1ff6dcee7cdae56` |
| `h264_fz_cavlc.h264` | `e97197142a08d5ff4cd09cd622b1e8d58af819fc49424318e2b497bfbe801dd4` | `61ecd9841753e4d2792d3834a9f28382982788ebd63e009ddd50240c3b8d93ff` | `61ecd9841753e4d2792d3834a9f28382982788ebd63e009ddd50240c3b8d93ff` |
| `h264_fz_multislice.h264` | `f0179fca8bdf132127af42fe629c34ac64cc41d056805adccc09d0f725d005d3` | `02b2999ab0e8fae09f19a5b7470ac5e5655fbe7f8a9bec5a5e420cd31726694d` | `6f5ed21c2b317fa3bed23ebd19bb3f629790bd8307427ffe1f032ed967b828a9` |
| `high_1080p.h264` | `1aa7c93c867c01241cc135f6decebb1ace3ffdf4bd2b0453d1e31649f53e0f77` | `fe00c6cd840484fa09b03589e176532368fd67d8733f162b6727107cc635b8fb` | `614494c8814fc784ecb264e072cf60cfdceeae1c9ba348469fde0b1f4028bfa7` |
| `high_720p.h264` | `ff2227e9819568bbc01705a413bf3be100535122d9de3483892750789191a797` | `a1b7b553ee4f7939e78b709ad894b52e5ae300e7a3a2dd8f76dcf8100ae3f789` | `466164c84b6546bf67c540314306068d4a121544da494f0bfedc590753d1d711` |
| `intra_cavlc.h264` | `594d9ed935499d6c756e6d51e3ee5616fd25be87a9a1aeef5aa63c4daf7056c2` | `53f9081ca1fab14709ae45a01580d4b2bb7e60c5d394240f224461df353a00a9` | `53f9081ca1fab14709ae45a01580d4b2bb7e60c5d394240f224461df353a00a9` |
| `intra_cavlc_8x8.h264` | `1c3078dab81a9982706ebb6a282a70bb874a34622c09db6c31cf73eb51b3aad9` | `9f55c11d392ba77a01bf6fea134f9a15e32426120cea06a1076a45f23d2a977e` | `9f55c11d392ba77a01bf6fea134f9a15e32426120cea06a1076a45f23d2a977e` |
| `intra_cavlc_cqm.h264` | `5c6f0117805f852d5bae7077d6c9fd0067115a971c5b851af9ec45b44c38ac49` | `c1e41ae05f8ceb5d82a26466fa1439ab49fb0140ed95e7319d8fe06352bf652a` | `c1e41ae05f8ceb5d82a26466fa1439ab49fb0140ed95e7319d8fe06352bf652a` |
| `intra_cavlc_noise.h264` | `37010741e06cc6ec495aea5a8575ebf1a830c9edfd37518220e7e2a2705355d5` | `2253334783561aec48aed8f81a2d50311f0d8af8c9bea17d582c9c2178ff5d9f` | `2253334783561aec48aed8f81a2d50311f0d8af8c9bea17d582c9c2178ff5d9f` |
| `keyint10.h264` | `cb192fe195e8054e5cdad619499a987088624517dc0e1e2059e4f9ec6a3b6034` | `887e770b3c285e23a49bb81ce461d30da3e0a87625e900f1c44b3a67895a439c` | `8fb1de7f1252238fa8235674405bf56a95ced9ab94630527733d1dca5b415ba2` |
| `main_cabac_b.h264` | `530dcc847d2c84af8795ea33216b83053e0d57cfd11af8a90eb9acbd49c51d42` | `7735e4b80289681becba461354d7bbfe40a9d0eef6e3d65bd06e420aceed834d` | `c210f9d230e91fdbfd00c3e3f721be6f4342be2913590dfb7f8c2e86b148bc60` |
| `no8x8dct.h264` | `225c156afd938715d9aff7dd066fd26a24f8511142b10051a197bf3bd5a31813` | `c7071306b844367ed3036606a02d888e732799abb5b0a88ccb7a34aa18cc4757` | `97dd011b161893f8fabe7f1b1caab46255e58229677a66716b1f6159972a7178` |
| `no_deblock.h264` | `5ea703afd745c006a6c45b64677d9154c4b7059962fe3782bdb6d75dbfb69805` | `466a2e80c074e85b41acc42ab4c7248f880b8f57cfde7960d8b2429b523ddb43` | `466a2e80c074e85b41acc42ab4c7248f880b8f57cfde7960d8b2429b523ddb43` |
| `open_gop.h264` | `f2ca3079e42a06971ac6ff6ce50d2f69c14a2de99e1fc787b2239ecced3384c1` | `cfc576607ed9c2829e1524f2a32b0a20231b4fd8f7e67027db4745124e0136e7` | `6a6b01e7e0f64a1d393f9a67ae6b043f685a76492501e6d31eeeb07ea8648cfe` |
| `p_cavlc_nodeblock.h264` | `408d041bc19d2127fba69e3d0d9981aede93c388323fb73b2851aa5046cfe439` | `8b46705f6ec5ed06305e9dbb4b5dd02e602c41f476f7d0b4c8f7fb1858c78824` | `8b46705f6ec5ed06305e9dbb4b5dd02e602c41f476f7d0b4c8f7fb1858c78824` |
| `qp1.h264` | `618ee6589c1b2e054ac2f9404859ae5f91f830e13fcaad80cb8f2d8e0b760710` | `a46fe8432f26fc195469f02296adc3df85990f16be7c663375a6259eddedfc19` | `a46fe8432f26fc195469f02296adc3df85990f16be7c663375a6259eddedfc19` |
| `qp1_cavlc.h264` | `a61242588eb64593e6baab6a53bcb99f1829dc18969421e66ba39cfacf3f7c38` | `2ec2f949eb5ce10d76baa2b307cf8f89ff71c2e107aa8fe1d1c28b7a8704f64c` | `2ec2f949eb5ce10d76baa2b307cf8f89ff71c2e107aa8fe1d1c28b7a8704f64c` |
| `qp50.h264` | `23ad8710c8e40ea57f0cead372f075b8802ad7ad0061c148c53f7691eeddbf7e` | `538114ddcc74c437db579ce81ea3a60eb88f080a00a3a3fe596434896edbafaf` | `b3236d8cc35e7b240fb645be221b33f7cdb0fa3f268223c12b52587c032cfc31` |
| `ref4.h264` | `8920fd609203973b2a62d7ee4424607666f25ce2f43adf3aa7df0eab578de9e2` | `d87cd4ff833b81dd2f4492fe9eb09eda95179841098ac90dad106e9dfcd1b2ee` | `3e7faae7c2f5846c8f7b508fc8fb8ce3b6656a33d52ab456eae187f0d4d5e978` |
| `slices4.h264` | `69b56d854e4f08a2baeac55e86c2abad57ec4c844b547175ce3353acd88dd0cb` | `b947744a88fa2236616f10d0e36c8287a620a0ca06d83677442ea0a31fd6c0a5` | `226cd211d9c043eedb893d8820361f5dfa5a2a382c2908a5332046039c2f4049` |
| `slices4_cavlc.h264` | `a0dd15f109a2f68703186830a10de36aa571c38dcf31333a57323c7bae6e394e` | `50243f158a091ecbd7ca603e12d955412231596dc8031e40f2a7a96855b225d1` | `50243f158a091ecbd7ca603e12d955412231596dc8031e40f2a7a96855b225d1` |
| `vt_baseline.h264` | `0f02cff5e07bf4d3d6bbd5ff1d7ae87f9f803e260582006a0cc8c544684f9c44` | `050c0d8a432dd28227aa9a8039031f1c404b786c5f50f4c1ef608f7570a6a532` | `050c0d8a432dd28227aa9a8039031f1c404b786c5f50f4c1ef608f7570a6a532` |
| `vt_high.h264` | `118f3a8f14464080f6f226ef44114c48c811b89a94f1ecba8ae31df170d18e86` | `8d6b1f591f6ddb896c47ac3547207ad973c717c6570098b1badb2a5a8420c8db` | `8d6b1f591f6ddb896c47ac3547207ad973c717c6570098b1badb2a5a8420c8db` |
| `vt_high_1080p.h264` | `404d7f3ee4c55d0979917360f271548ca86fa1f756087bf5dadd7a971e2ceb5e` | `94e8b6f3458a1b1081ce2ff2b45b8cfdde64f055c1d51c665da59ac3f475ff04` | `94e8b6f3458a1b1081ce2ff2b45b8cfdde64f055c1d51c665da59ac3f475ff04` |
| `vt_main.h264` | `81283ab441552f4a716122c0a2de6af71b23de80aba42b3e73ef1d8ae713f931` | `f6d26a89cbdfc201303ce37ad461f819c00acf20347128d6b80543023f93f545` | `f6d26a89cbdfc201303ce37ad461f819c00acf20347128d6b80543023f93f545` |
| `weightb.h264` | `39793818edd9df4a29f3ca8279a9e9c92d5627fbf4c56b34264b1562b04253b8` | `e1074d9495fa941f29bfdc6cbae9e12b87f9a1699f4d75814c746d7668698aed` | `4992031139564345d29cccb626d00cef38bb8ff2395fa1e8f6e55c62e8e73c35` |
| `weightp2.h264` | `39793818edd9df4a29f3ca8279a9e9c92d5627fbf4c56b34264b1562b04253b8` | `e1074d9495fa941f29bfdc6cbae9e12b87f9a1699f4d75814c746d7668698aed` | `4992031139564345d29cccb626d00cef38bb8ff2395fa1e8f6e55c62e8e73c35` |

## Original H.264 validation

Validation uses the rustup stable toolchain (rustc 1.99.0) and the task-local
`CARGO_TARGET_DIR=/Users/emm/ArtCraft/worktrees/filmcraft-48/target-48`.
Counts below include doctests and exclude ignored tests. Baseline test executables were built
before editing and replayed with the same oracle setup as the migrated suites; baseline
doctest results are retained from the initial run. The original and shared H.264/bitstream
suites also have matching counts (shared tests run from task-local copies of the pinned source).

All consumer runs require oracles (`FILMCRAFT_REQUIRE_ORACLES=1`). HEVC/VP9 use the
OSXExperts FFmpeg 9.0.2 build (libkvazaar and high-bit-depth libvpx); AV1 uses the
Martin Riedl FFmpeg 9.0.2 build (libdav1d). These are external test tools only, not dependencies.
The initial Riedl-only attempt could not generate the libkvazaar or 10-bit VP9 fuzz seeds;
both were rerun successfully with the capable oracle, before and after migration.

| Crate (all consumers prefixed `filmcraft-`) | Before passed/failed | After passed/failed | Ignored |
|---|---:|---:|---:|
| `aac` | 35/0 | 35/0 | 0 |
| `apv` | 7/0 | 7/0 | 0 |
| `av1` | 34/0 | 34/0 | 4 |
| `codecs` | 220/0 | 220/0 | 3 |
| `dnx` | 11/0 | 11/0 | 4 |
| `flac` | 13/0 | 13/0 | 0 |
| `h264enc` | 35/0 | 35/0 | 0 |
| `hevc` | 89/0 | 89/0 | 4 |
| `mpa` | 10/0 | 10/0 | 1 |
| `platform` | 72/1 | 72/1 | 0 |
| `prores` | 30/0 | 30/0 | 3 |
| `vp8` | 26/0 | 26/0 | 1 |
| `vp9` | 78/0 | 78/0 | 3 |
| extracted `craft-bitstream` | 12/0 | 12/0 | 0 |
| extracted `craft-h264` | 99/0 | 99/0 | 4 |

The platform failure is pre-existing and reproduced with the original executable:
`hardware_encode::rgba_pictures_encode_like_the_cpu_conversion` fails the VideoToolbox
luma-PSNR comparison. No decoder or test expectation was changed to hide it.

- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo fmt --check`: pass.
- `cargo xtask layers`: pass.
- `cargo xtask assets`: pass.
- `cargo xtask wasm`: pass.
- `cargo test --workspace --no-fail-fast`: 4462 passed, 1 failed, 49 ignored;
  only the same pre-existing VideoToolbox test fails. This broad run permits optional oracle
  skips; the strict per-consumer reruns above separately cover AV1 with libdav1d. No libsvtav1
  failure was observed.

Task-local build products and downloaded oracle tools in `target-48` are removed after validation.
Text logs and the complete 160-entry before/after hash manifests remain locally in the ignored
`target/validation-48/` directory; neither binaries nor fixtures are committed.


## HEVC and AV1 byte parity

Fixtures are generated locally by the original codec suites before removal: HEVC via external
FFmpeg/libx265, libkvazaar and VideoToolbox; AV1 via external FFmpeg/libsvtav1/libaom plus five
AOM conformance vectors. No media is committed. The same task-local Rust harness is compiled
first against the original path dependencies and then against the shared pinned dependencies.
It decodes HEVC Annex-B inputs with `decode` and `flush`, checks `take_error`, and decodes every
AV1 IVF temporal unit with `decode_pts` followed by `flush_result`. Every visible picture is
serialized in output order: HEVC Y/U/V via `to_le_bytes`, AV1 planes as little-endian u16 samples.
SHA-256 covers the complete decoded output; input hashes bind the exact fixture bytes.

| Codec | Successful fixtures | Modes | Matching output hashes | Frames per mode | Total frames |
|---|---:|---:|---:|---:|---:|
| HEVC | 44 | 4 | 176 | 577 | 2,308 |
| AV1 | 33 | 4 | 132 | 224 | 896 |
| Total | 77 | 4 | 308 | 801 | 3,204 |

Modes are 1 and 4 worker threads, each with draft mode off and on. Both worker counts have
identical hashes for each draft setting. Two additional HEVC hostile seeds preserve their
errors in all four modes: `hevc_fz_422_10.hevc` returns
`unsupported: chroma_format_idc 2 (only 4:2:0)` and `hevc_fz_wpp_slices.hevc` returns
`invalid bitstream: empty slice segment data`. Thus all **316 before/after records** match:
308 successful hashes plus eight matching rejections. No failed decode is counted as a golden.

The table lists each successful fixture relative to `target/fixtures`, its frame count, and the
identical before/after hashes. Draft and full entries each represent both worker counts.

| Fixture | Frames | Input SHA-256 | Full SHA-256 | Draft SHA-256 |
|---|---:|---|---|---|
| `av1/aom-test-data/av1-1-b10-23-film_grain-50.ivf` | 10 | `c36cf5ab6a2e9e27c212c06863759b60791e3fa681a0800b5d57fd4192ef29cb` | `4c376ec67b9ef95d194f03a72bf8de2f1caafa7313d2f447c724191d396dc48b` | `4c376ec67b9ef95d194f03a72bf8de2f1caafa7313d2f447c724191d396dc48b` |
| `av1/aom-test-data/av1-1-b8-02-allintra.ivf` | 39 | `5fcd265fd9f9bdd0d3179340b4c4532f1422ca5e5d97741c7481b84cb5dc122f` | `b330c47cf79674ab21f2df074eeafe3c71d5ea6bdb3b4b507a01950812f0f8f1` | `b330c47cf79674ab21f2df074eeafe3c71d5ea6bdb3b4b507a01950812f0f8f1` |
| `av1/aom-test-data/av1-1-b8-05-mv.ivf` | 4 | `222a9050059b254dab17cfb802ff829c778e3f93af18961a622c8268576c1395` | `31a6da9f013ee0d36c7a88b686679243d2ef9b8ead252a77c4bf371bd33e590a` | `31a6da9f013ee0d36c7a88b686679243d2ef9b8ead252a77c4bf371bd33e590a` |
| `av1/aom-test-data/av1-1-b8-06-mfmv.ivf` | 4 | `b59bf9586d8546dfda81dfec4ee4e32ceb502c9d22412ab0b63a2abb534a1f14` | `39405162482b3d24063ee567a3a6e576caca8f86e087ea410476711b8b520d98` | `39405162482b3d24063ee567a3a6e576caca8f86e087ea410476711b8b520d98` |
| `av1/aom-test-data/av1-1-b8-24-monochrome.ivf` | 10 | `3dcc2a3c181f74788bd9575d94f0862d9e914415ac358a4da4ed7d904f0535b7` | `fc48348588091e8b65d9a3b50ba4815e5cbf4d311e418e4bafeaef1a1b22599b` | `fc48348588091e8b65d9a3b50ba4815e5cbf4d311e418e4bafeaef1a1b22599b` |
| `av1/av1_fz_10bit.ivf` | 6 | `92f358430381289c663e773bb36355647a6f2af9be186b7fd1a7a05f5dac9446` | `4934693e3e146f67b454c1fb9963f71824541974fcf3270eb6d4527faccddcb1` | `4934693e3e146f67b454c1fb9963f71824541974fcf3270eb6d4527faccddcb1` |
| `av1/av1_fz_default.ivf` | 6 | `ba939ec6e5d0dc7262fab4c69725dcf946df14bd34f4cca361324dbbdbec16e1` | `30f56f5b742f1501173da30d5b707216498e5f2a3097f748850eaf55c60a83cf` | `b855c0f6d4184ab74f2aa54ee8488188b9ebe726de72e1a3676ae1d1d1d15236` |
| `av1/inter_aom_422.ivf` | 6 | `9a634c6d7503f9303b35e5e47c5c0be20165b29df94ab504aaa159696ea45590` | `ab6d5a9dd8a6da4581c7c9a1284e9dc60bf9cb462a932803b32370ab06d4f50b` | `e5f94e6b682fd0ef5bea4462d241567195e4e40b08636c3215a6eee627f70c17` |
| `av1/inter_aom_444.ivf` | 6 | `551278eb91747ee089ecae5d0d09652bd36dde3da4c0faddb6ad25eb28aded2a` | `36835f602199804ef5dc5b88d680876de82ca5c6e186d5b31e9e121ccd108f5e` | `5a597fc6dc8dab1715a3fa062a428be2a0cf3cb8173b9fc0965bba40c91fb18f` |
| `av1/inter_aom_444_10bit.ivf` | 6 | `8b8440de390de322e1fbc3a7c8c7112804d40f3fd917a495f9418aabdb276e9d` | `90d139d6dc78bfcec1f194ad2b0cd5879af74cc4f589e1fb965440df6df81bd9` | `5ed9e3665ea411f2d5732de4ab263d63bce726eb3a22bc9354be6322b6e37b6e` |
| `av1/inter_film_grain.ivf` | 8 | `167688edc1fa485b8239f53071fecc81369c3aaba2c8818933a889c26e8f768d` | `5a791cd167ae857ea39c3c6d361100e957b25256b20dc04ae763fc4849132f3b` | `0149f7e361228558a577b5ac790422008da5fdbb5fc92464875b52e1cc3f7a8c` |
| `av1/inter_mandel_10bit_p5.ivf` | 16 | `b3ce0fa9064f24e012bbd9f7722eca77205029e416b6dc80d6108f71ba78c33f` | `9d30aab0f33a9170bfa535d9c9541781a0c594fc03dfc90e02d7cc6ab34d3449` | `5a0440b03bf7b6241b82f39c2d0d09cc98d72c043987c7adcf655e9b753c9cae` |
| `av1/inter_noise_odd_p6.ivf` | 12 | `d7ef3d27bc13d13ce85f9369b69a10649ca93d254fdbb21129eccc7d16232353` | `0e2ceb66b485357ecb67a647a84299207b8b39a79a3091c464e816fc4a7189a9` | `023bf6065731b7b183c5bd19ac90e853dc94b2ca1d71380d6d5fbe6f551ff413` |
| `av1/inter_qmatrix_10bit.ivf` | 8 | `8b798a2737a296c9de28b6219cefe41c320480beeeed53982ea2c72e6c4ec146` | `5815c3347b694c1634338c1e39c10489c0d7464697a61cad4176cb4fd19f31d1` | `ffa2aac0b58e19a64fd4a40c240e9ae67257fae36c16f2b74870f82f4f754868` |
| `av1/inter_superres.ivf` | 8 | `e6ed24016e27e2a63856506f4916e98b07b9885d61254a209e16643c7c4997de` | `88029f37b62ecedfafc3928acac4f70d924836dff5473fae56c358f344daef8b` | `098d960478e03d5cefe88577e99b3330b323bb063a7901ad74f391b181dc424f` |
| `av1/inter_testsrc_p3.ivf` | 24 | `4c32be75a20118b8a71f3d9c7b045332f0bd073c467dd7b477dd871651832b1b` | `cfb15be2a2626140545e7c02be9b03fdaf6b12dd049fe7be3d2c2f72457956c8` | `cfb15be2a2626140545e7c02be9b03fdaf6b12dd049fe7be3d2c2f72457956c8` |
| `av1/inter_testsrc_p8.ivf` | 10 | `9d5e93865f3ef7b1ce23a32f741bbfd999d6860dca6889a4d16b07a8ae10c0b5` | `7913ce74f4b2db1f2f5ff5fab1c0a07fc6c3214484d411e390e16594270ca76b` | `15b2c5652deffdd710ba870c7f1ed2f89b91a9999392708816f1c4ef8d6bf109` |
| `av1/inter_tiles_4x2.ivf` | 8 | `362ba2b8a99aba9b2806e63418f531b3159460a9121e01d76999ca45757b937b` | `47ff5c228813bf6ef211fe0b81ec960631158612eae26306f7ff11b0d41d59ef` | `f033b70195b642a3e7bfb46b8e34006d0b9615b468566488e925911dccedc5c7` |
| `av1/inter_tiles_odd_10bit.ivf` | 8 | `c04431cf29562419e57e4ec3e0937ae34f59737d4bd79d7bc3a9361a3b934e37` | `11a08fb67be6b232dca5765c5bcdfc92245d3696b7bb5774631cf08fd7bd6531` | `12dfc55713359067ac8e0538920a76a7f7ff980fc5bbe5836d3140901a5cfcd6` |
| `av1/intra_all_10bit.ivf` | 2 | `6efba8babc604e04fc4eeee9a1dc5b413bdbbee7bed67f3fd32f6f7eb760d798` | `0c37f785cbcc68b3d19d6ea401ce3bed4c295e3bd3bcb4765928b61ecf9fee28` | `0c37f785cbcc68b3d19d6ea401ce3bed4c295e3bd3bcb4765928b61ecf9fee28` |
| `av1/intra_all_grad_noise_10bit.ivf` | 1 | `78883ba107ab669b985f0bb0b2b06f9ca5600273b74f8f3ea3b719ad58b0f56c` | `80b9c9b855014e94d9b6b5bfbb13554fa94ba354ce171105f93e57121f66915a` | `80b9c9b855014e94d9b6b5bfbb13554fa94ba354ce171105f93e57121f66915a` |
| `av1/intra_all_noise_odd.ivf` | 2 | `d89844e1c9263f834d13b7947ab0ebb1211c4536e62632cb6cfe743197dde973` | `442af7c013529f911f80e708b2add490866b8308db4979dbf9cc80fb90c51413` | `442af7c013529f911f80e708b2add490866b8308db4979dbf9cc80fb90c51413` |
| `av1/intra_all_testsrc.ivf` | 2 | `73959e2b2f82cc8e8070310496de0aa56568e88bcbfa33b0918e1c7c06318c31` | `983ed29ce5a3ed04ec419f56d5e21b220a0752077de06e34a8d076e1a6d48d9b` | `983ed29ce5a3ed04ec419f56d5e21b220a0752077de06e34a8d076e1a6d48d9b` |
| `av1/intra_cdef_testsrc.ivf` | 2 | `bbbe214bf9cb51a4c8306d43f226c023bef296d92994f3f93824895453c4aa1d` | `20928fad10987572d9e49b11061b4ab86f4b304d83f52d3bb874daf695cd0d58` | `20928fad10987572d9e49b11061b4ab86f4b304d83f52d3bb874daf695cd0d58` |
| `av1/intra_dlf_mandel.ivf` | 2 | `ad47a1bfd3430d810304b027136d141b9e58bf54aa3f5a0d0cae682091751ba4` | `22703f8331ae80d588ee487936776d8f7af2004740ef9b94a90d7b7b673d88b2` | `22703f8331ae80d588ee487936776d8f7af2004740ef9b94a90d7b7b673d88b2` |
| `av1/intra_lr_mandel.ivf` | 2 | `89f782d4936ef61b97308387a974876f047b4348c70c50ac679648e921693944` | `d82cc13ce73bfe58b613b78cd44898c4d3a065b998ab656fe9482329d1627cab` | `d82cc13ce73bfe58b613b78cd44898c4d3a065b998ab656fe9482329d1627cab` |
| `av1/intra_nofilt_10bit_p4.ivf` | 2 | `646f395ff590d730f3a4de97b62744631a093f03e4277fb5c65b22234f0bf955` | `26a27aa5a193a829cde719024d589fede891173dc22c9486ef3605292272e720` | `26a27aa5a193a829cde719024d589fede891173dc22c9486ef3605292272e720` |
| `av1/intra_nofilt_720p_p8.ivf` | 1 | `2b2ffb372ca2ce49e2886b8fb3bbfd982761e4240c491bd672445bbce4e0af41` | `4f0c1930f2f3246d3a9132b91cbe40442dfd18438dcc646636547a4aa1243ed7` | `4f0c1930f2f3246d3a9132b91cbe40442dfd18438dcc646636547a4aa1243ed7` |
| `av1/intra_nofilt_mandel_352x288_p4.ivf` | 2 | `c75a2e0e8258afbe04992c0ac3af9f6712317ebff863f3d32e6bedb8d264891f` | `869dc845a92e3b933c356c806418f9a3bea816531c9fd0cb88190faf7336d6db` | `869dc845a92e3b933c356c806418f9a3bea816531c9fd0cb88190faf7336d6db` |
| `av1/intra_nofilt_noise_p6.ivf` | 2 | `7ccdb3f8e41b1b8babc5b8ba8327b95222577f2765bc400aee2126e5a501b734` | `04340b1326e40594c505130ff45bce5bf4b7118da1eaf4b8da58c687d2622473` | `04340b1326e40594c505130ff45bce5bf4b7118da1eaf4b8da58c687d2622473` |
| `av1/intra_nofilt_testsrc_128.ivf` | 1 | `96545aae1ab40b72657677c0f9b8670a0338e2b328ce04adbeedd7c5c8cc9c42` | `e903ff9967f72b7b3fedcb95019ca27018de3951578c0d8204a6c9e502967578` | `e903ff9967f72b7b3fedcb95019ca27018de3951578c0d8204a6c9e502967578` |
| `av1/intra_nofilt_testsrc_odd_p2.ivf` | 2 | `8410d326ef209af7403f982b9967bb5694f52118a3c82786b9c0d511271e5f5c` | `c42896c4dcdccd7e7e86c50c342e33b51c7c7356f8c6e5c92dfa6a0a2f44e6b5` | `c42896c4dcdccd7e7e86c50c342e33b51c7c7356f8c6e5c92dfa6a0a2f44e6b5` |
| `av1/intra_qmatrix_noise.ivf` | 2 | `df771db812bc2daf2054c2fc0ea0091a8f5129f0c85237a37852c468f09d2ba2` | `9e5b6353809a029bedcfa9e946e1cd2976eedf90ba4ec652e61b0a31ebbb1507` | `9e5b6353809a029bedcfa9e946e1cd2976eedf90ba4ec652e61b0a31ebbb1507` |
| `hevc/bframes.hevc` | 30 | `febe0e41d5fd597843363e641cffc4e924d5ab642720c4e7eb28d6c61f2fea27` | `a3fe53db32104854103b778f721ee6e26f84b320a668480bb5f0b6b7f0886516` | `cb1b4045c28d87d998bf5c5eb6b21d21e71db03621c3086a085b100b5c8535ad` |
| `hevc/constrained_intra.hevc` | 12 | `9ce844caef4954d20e700497274ad45ac8425bf983d5165c3d71f9ef49b69b36` | `cf3b76a37bf1d6c795576eb66208cf42703b84b5124d165a28948871b1055e29` | `5b843a426a6a2118fe68cd8a0db957b7e9c6cad1094a1f3507e2af3dd8404338` |
| `hevc/crf0.hevc` | 6 | `d1651041a12e79b26bfd36de2fa099e2bc8e3dab1d331842cace6450891ddf9e` | `7a14b97a23abd92224a0528ed78ae80e2443f74df9e1621618fa490953fd31f4` | `9e6f03896df35ac1b0c6c35f7724be1cf316605f07cc686967d65521718d6376` |
| `hevc/crf51.hevc` | 12 | `14e3a1fa2a9d065eacec352d70ad04d36c5939bd5a82076b8361d597a3fdb4c4` | `7187dd9aa6aeef2664ef38374dfc04b2172d1bacfe7350cd87fd2010fe4b6f34` | `1ddbd47beeac98387c2ce1e8921962cfe6e8e073348b94af89cfe3b9a6df468d` |
| `hevc/ctu16.hevc` | 12 | `56e88c937dfd8842a6ea39ecd6d6f2da421ce4a220495fa16a5da18c238249d5` | `a2cb0f53bf788b970b4f920cce9005dec4763b21f8972eec080378a08e3ea728` | `8635f70c067ac2215f5be3d552cba2af2dc3c83346cfc172e743be8dc58e902f` |
| `hevc/ctu32_tu4.hevc` | 12 | `f8443ebcab1cea5e99b8658e9b0ec6bfeb7d1ab8d8125617f0a69b0d2055eb51` | `6afa77088343b2e5ca50beb6f61cf533186efa6fc78ce8d6e9ed408914dcf44f` | `aee8f446022e0ec78bedb7f5390c1ea27957037ab44dbe2090b8be657aced6f1` |
| `hevc/cu_lossless.hevc` | 6 | `0bf76a67ada26277a2f23486c8d7073bda11ab19e26c329e3d4a898f5dfacb39` | `a5ada8c0db5c12642a06eaf497663cf47f7d5dfb1e05dcfabf4098c3b57bdc9e` | `efeb5fe9aefce94acbba6530832938098860dcd7f2c353c9a460ed08a9530462` |
| `hevc/deblock_m3_3.hevc` | 12 | `99637654fdc370e46e4e3c9c0ac6c01bf731ea8cf97ce61ee57abe9145e7311a` | `fb6107a7e181dd4de552b1a33a377fac8b7093679e6b419641f6160d5699b8eb` | `1953f504d8e96d19224134ab24e7474c55d01b1705738ac854748e12db6aec66` |
| `hevc/hd_1080p.hevc` | 3 | `c0f9d62aaa56c41c19438c69bbcb97ade4f565b9d2968e19bc580fce0bb2c15a` | `fee302ca88ed4b126209ad609e1256f7a8ba3dc191073996a9036d3b72aecd72` | `a4602f5172aeaed1ede6dc2b06fe623384434ed2830a2bc3a2098befa6c8df82` |
| `hevc/hevc_fz_main10.hevc` | 7 | `81bc196cc6d0f2eac325f91debfcb0aaefeb95a2e6931ae755ae9db8d0affd11` | `00a9487972dc6663132e69bef0b71549f9ddaaa750a34b0ce5f704c9bcf89d0b` | `00a9487972dc6663132e69bef0b71549f9ddaaa750a34b0ce5f704c9bcf89d0b` |
| `hevc/hevc_fz_main_b.hevc` | 7 | `c79c5113bdd380e136f0811de84e818cb095f99ab38e2baf21aa38092f20c9bb` | `284f93a6a744a1ffe94ec59e8cb7723a8a5522bbfbc7f0efbd92d47dd99c4708` | `284f93a6a744a1ffe94ec59e8cb7723a8a5522bbfbc7f0efbd92d47dd99c4708` |
| `hevc/hevc_fz_tiles_kvazaar.hevc` | 7 | `e905a37a48b01a7a16ed073ebb9e10aa8938dcfbd5a11d29f4146ef6a11f0b41` | `dc6e71fee0389987ff1fae52160a37467ae860995acf7aa34fa11bc1df610445` | `dc6e71fee0389987ff1fae52160a37467ae860995acf7aa34fa11bc1df610445` |
| `hevc/intra_main.hevc` | 3 | `bade969fd44ccb473b4f28a59468a2edd8b649df505f440213e4d075bda97dee` | `195939910107abf0890682c6a8f63c7f4849a7980937c08527452a65b6614f20` | `195939910107abf0890682c6a8f63c7f4849a7980937c08527452a65b6614f20` |
| `hevc/intra_main10.hevc` | 3 | `7929f83c26a2fbe760c80ec2cd29ad17ba438df1a033e2f4413c2d0dc7e2f10b` | `f4bcf58fd2efe3726b4c8c6b2ed1bee882852d40bea2a1a8e639427de5c77f29` | `f4bcf58fd2efe3726b4c8c6b2ed1bee882852d40bea2a1a8e639427de5c77f29` |
| `hevc/intra_nosao_nodbk.hevc` | 2 | `2fd25bc114cf7160e1c22ddb162325d855c3e6b002a630e62f5fd5ee7a9c3407` | `980ee7c4b62faaf0627ef5eba20fe321451d5cca58cf82ca9536fd7dfc5e0635` | `980ee7c4b62faaf0627ef5eba20fe321451d5cca58cf82ca9536fd7dfc5e0635` |
| `hevc/keyint5_idr.hevc` | 20 | `ac503cf3255db7db07a4f0b45fe1d14caf0b84aa460c7668bb859378eabb69a6` | `83101a6b080edfc9fc78617f6c240702bff8e15c58b90a4e927b40e02a8cdb8c` | `4f1cd34ffc8704f76b6a3abe3cb439386f5a0cac9144ebf3029d611696d5630d` |
| `hevc/lossless.hevc` | 6 | `ac089b58b7dc5f419565510c573cbf766853bcc304a31acf8a368311d087bb55` | `d26c38a683a394e55ba1be8a618e0597f39165def677362dfdafec124bdc4db4` | `d26c38a683a394e55ba1be8a618e0597f39165def677362dfdafec124bdc4db4` |
| `hevc/max_merge1.hevc` | 12 | `3412861e8e148f0e4f112b1bb1f8a7302939087baaa79a31748896bb10aaec4d` | `3fa382b8fcf5bcf11a63cdfd276926bd0c30ee1d38b009785760c2edf238ddf7` | `01c1ed90028112dbb74cc202958d6866b5c767c82de261f25f89d3934a9c12f8` |
| `hevc/medium.hevc` | 20 | `bddf26da3e4b996e0f88f33096afe34cccab4a2fa284f0131ac44f559915e474` | `52c60407a8aa9dbe85c1e70703dcf14691df2ee511c77329df4c9a655cff1fdf` | `af618f60bf0b0888b89c69240f328f4f1647074f5715d2bcbd2650cc5a6e4793` |
| `hevc/medium_main10.hevc` | 20 | `e32c0b693e7c25628a854cb9272982cc37dc1158095dea1a3deae4c2900ee887` | `e14ac259e4223d37acd9b561a76dd22f362c3df88127b1dcd46edca6f7b69cc5` | `a830ac55abcaaf8f5d1b35f07271204a55c4eeb7cd0ab79cb07494dacb75a1cf` |
| `hevc/min_cu16.hevc` | 12 | `057abaade9d23cf9be0f26fa6840b777014c86b47463237acf147e99a939ab5d` | `2d5e1d1de6bac8adbd5d25599025fd9289d2c4631dc5a17e38f889dd3f99099b` | `03435e71644d73cc093b2ab579a21c60ea947629bb67cfe39fdafe8e9dab416a` |
| `hevc/no_deblock.hevc` | 16 | `04d8c67328ecad299085632f3425465c81eb7327d01ac12bea0c7867ac184634` | `2a1b74f0babee3bf7b8ad481abfc217c02da317d315c063149e9398f3bf7481d` | `78406a0c13f577f0746a90b243e2abe3977c86c18e28d1dceade43d4ff769ddb` |
| `hevc/no_sao.hevc` | 16 | `fbaa72883f0167beea436bebfff6ca9b92838efca372799f711f463579682ba3` | `d31ff01a3d62f52b63a86784095fb95f03419f573824926e7283c585f868e872` | `87c781bb24c806f410cebeb5c0431538f92995380bc2799b646ed66b92577530` |
| `hevc/no_signhide.hevc` | 12 | `4bcc71ca12c675c453e0390b25f2c8b7a262ef1cd8b4e78551b8289fe874d770` | `34a290f4e7a3de829bbf5d6a9d53381a766ebb8aa36a04b681db69a780468b5c` | `77ae6b9205879b24bfe8742781e1ab0c3cb31c744e4af5e64deba69cedc372cb` |
| `hevc/no_strong_intra.hevc` | 6 | `d51685d8e8e09d8891d9906d41cd6d386a85f151dd8d9a697ad879e392bddc30` | `ec609b1c44eca62eda8c8066c144a660e04a5bf8d9bf3a80cdab63dab4da45dd` | `ec609b1c44eca62eda8c8066c144a660e04a5bf8d9bf3a80cdab63dab4da45dd` |
| `hevc/no_tmvp.hevc` | 12 | `b6812369994d688df8b70b1c3de32f0aa2369cac80c66e83868fdeef60c5ea1b` | `9a537eed7d48141cc2efead1126eafba025a49d4f01b813b6a49186644c730d2` | `7d49ba0bc9a470edbf2157deb59991dfd77bb007489d5241b0fa46167a8b32ca` |
| `hevc/no_wpp.hevc` | 12 | `175cdbe873e4c91fe878e5518ca627f44c9fe6f603f0b26d7549df5367b96e31` | `9db43c3eba3c7badee0f73b3a434424efecd4752da71461de97bbe9179974528` | `568d2ed5f12121baf003936765ffbfea214ffdba559b68dc3a26ed58f666e037` |
| `hevc/odd_1918x1078.hevc` | 3 | `41adfd99f51a6719b258fb90115da28907ca0476cb843cfcc3e12feacc5eec7d` | `596ad33a3f70b866d43603bbe4e9c2f4ff3f36f47efb7de64e4cedbc95690449` | `2287b945ec1d7df3ca5e86f83005cdc8e0628a7deef5e0e9fae63c31cd017f26` |
| `hevc/open_gop.hevc` | 40 | `894db9613f8ff8189a8f69a1beeb5da26cc534fc4f9998014a811ab938dd3bd9` | `e2837029acfc1bb21f725f8a87c592daaecdca626d9292e3e946bf4a6289dd0f` | `cbc21ca118c094bfde9aa1733f24bb7d9a24d4d0e372c8f466011e4d7b78813e` |
| `hevc/p_only.hevc` | 20 | `8cd917afc108ef6035cc28d9a1a06041485694ee098236e3b1a667fd4189b94e` | `76cf8862bec1dd6be92e74afae7390f29c9f94cae77132b17f6b45b0a36ff1e1` | `76cf8862bec1dd6be92e74afae7390f29c9f94cae77132b17f6b45b0a36ff1e1` |
| `hevc/qp1_main10.hevc` | 4 | `73585209ef98e32bb3e49244d50772e02490e11770114cc9d067f4b64303f1cc` | `97949b55a56744ea4cba37db283e22c4ed0b1ec70a16b45d910bd5a62d184530` | `e5adc11707604768955bf23adb7eee21e2fdec415758eb78dfd0303f181ff247` |
| `hevc/rect_amp.hevc` | 16 | `609ab84e99dc338ad84d5c127a241c1289634437f7e7cba62a8fc9f69713334a` | `23fac9494be01b6dfbb46bf5a1c76b88215ba24be43f8b5254ae822a10e3701c` | `14e74432f7e2a0bdf94582117ff896a904e7ea56f823161ca83e28415d08ee98` |
| `hevc/scaling_list.hevc` | 12 | `076c9d58434e3b9749e9941c663f0905ebf738e7bec3e6815ba345336ccce1fe` | `dab5c3d8ee38d0ef7d7a26afaabd847e47dc496149c143e58f66e89436f67a16` | `3defe09219317caf3ee77010e7a60f28f2ebc7ab05fe342eb7330ec353b737f6` |
| `hevc/slices4.hevc` | 12 | `0c47ea651c936f8043921e647092c854e0dc9f63084589898d76dd73761f2816` | `2f273c5bca98ccb64fe52172b43744a294c7b3a0c3cb30978afd77baae10facd` | `4e22d6a8c9a34fca957cadbd1b778b375dd70f54c26cb28760726a593559fd2f` |
| `hevc/slow.hevc` | 16 | `d9b4dd6d36f0345af7ee7a9271024ff1a993aa982a69c8025b5482aeca1d63d7` | `88a68957e0e6c958a37c0ab7dec48721bb2bced5f3adce3ae3ffb7e7c2c806cf` | `3305ad8a2bbef1014a867e27a1b6ffcfe057868d036f5c3b225b0f96173979cd` |
| `hevc/slow_main10.hevc` | 12 | `793777402fa82e8959c36fe54ec31dbf1c051c186d49e03b2a7fb2ca1eeda606` | `f4306a505a34bf5af9e45ad801b523a36b824521765298e877feb36438804131` | `37704f7f82de613994b444538e8419347def48df1f1fce15a289e0a6b14c3f08` |
| `hevc/tskip.hevc` | 12 | `8d8bd2e0c9d5cbdaca1e559dd5dd5e0724b58d4fb093ff9ea354c57700cffa4c` | `f3e2e7646b2f14042553a7c76d64053ea65751e3eddbd28997d582b59cf4ea63` | `0d27229d61cc580ccb96aaf1c2dbeaacab461334d93864c24c0c908ac9a58357` |
| `hevc/uhd_4k_main10.hevc` | 2 | `66a11ae5bd9f1e78fc601b3fb55e21cf03d04671ee25c96256e3e64dceeeebda` | `11ce8f842a4560511c48641a01f99e9f1e67345416f93ab61401cfa9220a261e` | `11ce8f842a4560511c48641a01f99e9f1e67345416f93ab61401cfa9220a261e` |
| `hevc/ultrafast.hevc` | 20 | `d39da6fdc4cf81b7f8d13f9342c9e3905afe06d32462d9fea6f9d5a2986a10b0` | `c9310d66ff6416195c5d0ddf1a0682d7ed4b4d5fdb5a98d00192817d7702bd64` | `c56913fd5f2888eebd7292e85aa981dd6f1900f43a4c3455a3b480cae99ee617` |
| `hevc/veryfast.hevc` | 20 | `827a20607386b545e2eccdf5521e2f832c34ffb67b91c210df41ba953c609f1f` | `1ec1d0927f2f43c8ae6bf0592e53e80b2543677a01c6df3c9e4c6eed16ff3788` | `2f45d5ee8eedd99c1ca41715d15cb242df2c3c26781f3ffeb3fc476c5a0ae4dd` |
| `hevc/vt_main.hevc` | 20 | `e32fd25d38451df38901cac1d8a1164626d104ba6cfab2b368bfb523463b7c9f` | `8629942b9e6791a6c91ac37b527da59dd7d9c59771f1ca1873a1da5eb2d5771d` | `8629942b9e6791a6c91ac37b527da59dd7d9c59771f1ca1873a1da5eb2d5771d` |
| `hevc/vt_main10.hevc` | 20 | `8cb281114a2581a08ea36f2dd609616b11ec40fedd7d79108be7bfcc9af44c5a` | `baa0625334daf47fd3bf0c66eb7b5c6141fa88edcbf9abd372ed9db3066099a0` | `baa0625334daf47fd3bf0c66eb7b5c6141fa88edcbf9abd372ed9db3066099a0` |
| `hevc/weightp.hevc` | 30 | `db3050b21033343495c1749c0edc8615685c6aa28368eb9dc1e933a5fced95dd` | `ae3b2068d278e34309ec1e706f16255a1885f6cdd5bc00780bc92aace6d8d6e8` | `2bfcb11b87db028ffbd2fd7013b313b1657f372b6e95177419c6db7331cf8663` |
| `hevc/weightp_main10.hevc` | 20 | `063cea2fe15ac97418233d302d15fe4223df1ad5e7d31251cb76eb16f475c727` | `cfb29d32fdd49962db4627282f92222849cbcf27b97b2b0e75cc3648bb429c0f` | `92c0dc37a36fc9af163774a9693aca0b42c5e98eeef37fc3fb36771acb920ed4` |


## HEVC and AV1 validation

Validated on macOS arm64 with rustup stable (rustc 1.99.0). Every authoritative Cargo command
uses the rustup toolchain directory first on `PATH`, with
`CARGO_TARGET_DIR=/Users/emm/ArtCraft/worktrees/filmcraft-48-hevc-av1/target-48`.
The broad gates use `CARGO_BUILD_JOBS=4` and `CARGO_INCREMENTAL=0` to bound build storage.
Counts include doctests and exclude ignored tests. Before/after consumers use the same
fixtures and oracle configuration, with `FILMCRAFT_REQUIRE_ORACLES=1`.

OSXExperts FFmpeg 9.0 provides HEVC/libkvazaar and high-bit-depth VP9 fixture generation;
Homebrew ffmpeg-full 9.0.2 provides AV1/libaom/libsvtav1/libdav1d. They are external oracle
tools only. The first exploratory default-FFmpeg attempt was superseded by the strict,
capable-oracle runs recorded here; its environment failures are not migration regressions.

The HEVC/AV1 direct consumers are `codecs` and `platform`. Because the shared branch also
advances bitstream/H.264, every remaining direct consumer of those dependencies was rerun;
`export` is included as additional downstream coverage. No consumer test count changes.

| Crate (consumer names prefixed `filmcraft-`) | Before passed/failed | After passed/failed | Ignored |
|---|---:|---:|---:|
| `aac` | 35/0 | 35/0 | 0 |
| `apv` | 7/0 | 7/0 | 0 |
| `codecs` | 220/0 | 220/0 | 3 |
| `dnx` | 11/0 | 11/0 | 4 |
| `export` | 147/0 | 147/0 | 0 |
| `flac` | 13/0 | 13/0 | 0 |
| `h264enc` | 35/0 | 35/0 | 0 |
| `mpa` | 10/0 | 10/0 | 1 |
| `platform` | 72/1 | 72/1 | 0 |
| `prores` | 30/0 | 30/0 | 3 |
| `vp8` | 26/0 | 26/0 | 1 |
| `vp9` | 78/0 | 78/0 | 3 |
| `filmcraft-hevc` → `craft-hevc` | 89/0 | 89/0 | 4 |
| `filmcraft-av1` → `craft-av1` | 34/0 | 34/0 | 4 |

Shared suites run from task-local copies of the pinned crate sources, with only the enclosing
validation workspace's unrelated vendor patches omitted and development optimization set.
`CRAFT_MEDIA_REQUIRE_ORACLES=1` and `CRAFT_MEDIA_FIXTURES_DIR` point them at the same local
fixtures as the original suites. `craft-bitstream` passes 12 tests; hardened `craft-h264`
passes 112 with four ignored. The latter's additional 13 regressions are intentional upstream
hardening relative to the historical 99-test H.264 pin, not added FilmCraft consumer tests.

The only direct-consumer failure before and after is
`hardware_encode::rgba_pictures_encode_like_the_cpu_conversion` in `filmcraft-platform`:
the existing VideoToolbox luma-PSNR comparison. No test was ignored or weakened to hide it.

Reproduction commands (use the appropriate oracle environment described above):

```sh
export PATH="$(dirname "$(rustup which cargo)"):$PATH"
export CARGO_TARGET_DIR=/Users/emm/ArtCraft/worktrees/filmcraft-48-hevc-av1/target-48
FILMCRAFT_REQUIRE_ORACLES=1 cargo test \
  -p filmcraft-aac -p filmcraft-apv -p filmcraft-codecs -p filmcraft-dnx \
  -p filmcraft-export -p filmcraft-flac -p filmcraft-h264enc -p filmcraft-mpa \
  -p filmcraft-platform -p filmcraft-prores -p filmcraft-vp8 -p filmcraft-vp9 \
  --no-fail-fast
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask layers
cargo xtask assets
cargo xtask wasm
cargo test --workspace --no-fail-fast
```

The baseline additionally runs `filmcraft-hevc` and `filmcraft-av1` at `eb3487a5` before
removal. The shared decoder suites and their no-default-feature checks run in the pinned
craft-libs validation workspace, not as FilmCraft members.

### Final migration gates

- `cargo fmt --check`: passes.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `cargo xtask layers` and `cargo xtask assets`: pass.
- `cargo xtask wasm`: passes for all 45 checked crates.
- `cargo test --workspace --no-fail-fast`: **4,339 passed, 1 failed, 41 ignored**.
  The sole failure is the same pre-existing VideoToolbox comparison named above; exit 101
  is expected for this baseline, not a clean workspace-test pass. Removing the two local
  decoder suites accounts for 123 passed tests and eight ignored tests moving to craft-libs.
- Shared HEVC and AV1 also pass with `--no-default-features`: respectively **89 passed,
  four ignored** and **34 passed, four ignored**, unchanged from their default-feature runs.
  Both shared decoders pass `cargo check --no-default-features --target wasm32-unknown-unknown`.
- Before/after golden manifests match byte-for-byte: **316 records**, comprising **308
  successful hashes** over 77 fixtures and eight matching rejections over two hostile HEVC
  fixtures. The four mode combinations decode **3,204 frames** in total.

Local text logs, the comparison harness and both golden manifests are retained under ignored
`target/validation-48/`. The separate `target-48` build directory, including temporary shared
source copies and downloaded oracle binaries, is removed after validation; rerunning requires
recreating those local inputs. No generated media, oracle binaries or build output is committed.
