#[path = "../src/task_policy.rs"]
mod task_policy;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn task(args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask")).current_dir(root()).args(args).output().unwrap();
    assert!(output.status.success(), "{args:?}: {}", String::from_utf8_lossy(&output.stderr));
    output
}

fn stdout(args: &[&str]) -> String {
    String::from_utf8(task(args).stdout).unwrap()
}

fn temporary(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("filmcraft-xtask-70-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn version_matches_original_and_rewrites_temp_copy() {
    assert_eq!(stdout(&["version"]), include_str!("golden/version.stdout"));
    let temp = temporary("version");
    let manifest = temp.join("Cargo.toml");
    let input = include_str!("golden/version.input.toml");
    let updated = include_str!("golden/version.updated.toml");
    std::fs::write(&manifest, input).unwrap();
    let mut config = task_policy::configs(temp.clone()).version;
    config.refresh_lockfile = false;
    config.manifest_paths = vec!["Cargo.toml".into()];
    assert_eq!(craft_xtask::version::read(&std::fs::read_to_string(&manifest).unwrap()).unwrap(), "0.1.0");
    let plan = craft_xtask::version::compute(&config, &craft_xtask::version::Args::Set("0.2.0-rc.1".into())).unwrap();
    assert_eq!(craft_xtask::version::render(&plan), include_str!("golden/version-set.stdout"));
    assert_eq!(plan.edits.len(), 1);
    assert_eq!(plan.edits[0].2, updated);
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), input);
    assert_eq!(craft_xtask::version::rewrite(&config, input, "0.2.0-rc.1").unwrap(), updated);
    craft_xtask::version::run(&config, &["set".into(), "0.2.0-rc.1".into()]).unwrap();
    assert_eq!(std::fs::read_to_string(&manifest).unwrap(), updated);
    let crlf = input.replace('\n', "\r\n");
    assert_eq!(craft_xtask::version::replace(&crlf, "0.2.0-rc.1").unwrap(), updated.replace('\n', "\r\n"));
    for ok in ["0.1.0", "1.20.300", "1.0.0-rc.1", "1.0.0-alpha", "1.0.0-x-y.2"] {
        assert!(craft_xtask::version::validate(ok).is_ok(), "{ok}");
    }
    for bad in ["1.0", "1.0.0.0", "v1.0.0", "01.0.0", "1.0.0-", "1.0.0-a..b", "1.0.0+meta", "1.a.0", ""] {
        assert!(craft_xtask::version::validate(bad).is_err(), "{bad}");
    }
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn layers_matches_original_report_and_policy() {
    assert_eq!(stdout(&["layers"]), include_str!("golden/layers.stdout"));
    let config = task_policy::configs(root()).layers;
    assert_eq!(task_policy::short("filmcraft-geom"), "geom");
    assert_eq!(task_policy::pkg("filmcraft"), "filmcraft");
    assert_eq!(task_policy::pkg("geom"), "filmcraft-geom");
    use craft_xtask::layers::{Crate, Dep, DepKind::*};
    let edge = |from: &str, to: &str, kind, workspace| {
        craft_xtask::layers::check(&config, &[Crate { name: from.into(), deps: vec![Dep { name: to.into(), kind, workspace }] }]).unwrap()
    };
    assert!(edge("filmcraft-frame", "filmcraft-time", Normal, true).is_empty());
    assert!(edge("filmcraft-media", "filmcraft-frame", Normal, true).is_empty());
    assert!(edge("filmcraft-cli", "filmcraft", Normal, true).is_empty());
    assert!(!edge("filmcraft-frame", "filmcraft-media", Normal, true).is_empty());
    assert!(!edge("filmcraft-time", "filmcraft-frame", Normal, true).is_empty());
    assert!(edge("filmcraft-h264", "filmcraft-bitstream", Normal, true).is_empty());
    assert!(!edge("filmcraft-h264", "filmcraft-time", Normal, true).is_empty());
    assert!(edge("filmcraft-testkit", "filmcraft-h264", Normal, true).is_empty());
    assert!(edge("filmcraft-h264", "filmcraft-testkit", Dev, true).is_empty());
    assert!(!edge("filmcraft-engine", "egui", Normal, false).is_empty());
    assert!(edge("filmcraft-ui-egui", "egui", Normal, false).is_empty());
    assert!(!craft_xtask::layers::check(&config, &[Crate { name: "filmcraft-mystery".into(), deps: Vec::new() }]).unwrap().is_empty());
}

#[test]
fn ico_matches_original_bytes() {
    let temp = temporary("ico");
    let out = temp.join("test.ico");
    let output = stdout(&[
        "ico",
        out.to_str().unwrap(),
        "assets/app-icon/hicolor/16x16/apps/ai.storyteller.filmcraft.png",
        "assets/app-icon/hicolor/32x32/apps/ai.storyteller.filmcraft.png",
    ]);
    assert_eq!(output.replace(out.to_str().unwrap(), "/tmp/filmcraft70.ico"), include_str!("golden/ico.stdout"));
    let hex = include_str!("golden/ico.hex").trim();
    let expected: Vec<u8> = hex.as_bytes().as_chunks::<2>().0.iter().map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()).collect();
    assert_eq!(std::fs::read(&out).unwrap(), expected);
    let committed = std::fs::read(root().join("assets/app-icon/filmcraft.ico")).unwrap();
    assert_eq!(&committed[..4], &[0, 0, 1, 0]);
    assert!(u16::from_le_bytes([committed[4], committed[5]]) >= 4);
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn parity_matches_original_inventory_and_report() {
    assert_eq!(stdout(&["parity", "--ours", "xtask/tests/golden/parity-ours.json"]), include_str!("golden/parity-inventory.stdout"));
    let temp = temporary("parity");
    let out = temp.join("parity.md");
    let output = stdout(&[
        "parity",
        "--ours",
        "xtask/tests/golden/parity-ours.json",
        "--reference",
        "xtask/tests/golden/parity-ref.json",
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(output.replace(out.to_str().unwrap(), "/tmp/filmcraft70-parity.md"), include_str!("golden/parity.stdout"));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), include_str!("golden/parity.md"));
    std::fs::remove_dir_all(temp).unwrap();
}

#[test]
fn help_includes_shared_task_syntax() {
    let text = stdout(&["--help"]);
    for expected in ["layers", "assets", "wasm", "parity", craft_xtask::version::cli_help(), craft_xtask::ico::cli_help(), craft_xtask::layers::cli_help()] {
        assert!(text.contains(expected), "missing {expected}");
    }
}
