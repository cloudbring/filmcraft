# FilmCraft xtask

The `xtask` binary dispatches `version`, `ico`, and `layers` to `craft-xtask`.
FilmCraft's L0–L6 crate layering table, L0 codec/container `bitstream`-only
isolation, allowed same-layer edges, and L0–L4 UI/OS dependency restrictions live
in `src/task_policy.rs`.

`parity` remains local in `src/parity.rs`: as recorded in `craft-xtask`'s
`SURVEY.md`, FilmCraft's parity command inventories shipped menus, effects, and
transitions against a reference tree and reports presence vs. placement rather
than scoring a static weighted checklist. `assets`, `wasm`, `web`, `fixtures`,
`i18n-report`, `bench`, `bench-playback`, and `ci` also remain app-specific.

`tests/golden/` captures the original command outputs on the pre-migration tree
(2026-11-03), including `version` dry-run `Plan` and manifest rewrite checks on a
temporary copy, `ico` binary output (stored as hex so `cargo xtask assets` does
not treat test fixtures as unattributed app icons), `layers` output, and local
`parity` inventory and checklist output.
