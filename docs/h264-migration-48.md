# Shared bitstream and H.264 migration (#48)

## Dependency boundary

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

## Validation

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
