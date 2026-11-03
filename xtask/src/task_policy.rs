use std::path::PathBuf;

use craft_xtask::{Cargo, Configs, layers};

const L0_FOUNDATION: &[&str] = &["testkit", "time", "geom", "color", "bitstream"];
const L0_CODECS: &[&str] = &[
    "isobmff", "avi", "matroska", "mxf", "cfb", "mpegts", "mpeg2v", "ac3", "mpa", "ogg", "riff", "h264", "h264enc", "hevc", "vp8", "vp9", "prores", "mjpeg",
    "dnx", "apv", "dng", "av1", "aac", "flac", "opus",
];
const LAYERS: &[(&str, &[&str])] = &[
    ("L1", &["frame", "media", "project", "audio-dsp", "text"]),
    ("L2", &["edit", "codecs", "keyframe", "effects", "audio", "captions", "speech", "tts", "tts-text", "interchange"]),
    ("L3", &["render", "gpu", "golden", "scopes", "playback", "export", "format"]),
    ("L4", &["engine"]),
    ("L5", &["platform", "ui-egui", "automation"]),
    ("L6", &["filmcraft", "cli", "web"]),
];
const SAME_LAYER: &[(&str, &str)] = &[
    ("media", "frame"),
    ("project", "media"),
    ("project", "frame"),
    ("effects", "keyframe"),
    ("audio", "keyframe"),
    ("playback", "render"),
    ("playback", "gpu"),
    ("export", "render"),
    ("scopes", "gpu"),
    ("gpu", "render"),
    ("cli", "filmcraft"),
    ("tts", "tts-text"),
];
const UI_ONLY: &[&str] = &["craft-ui", "egui", "eframe", "egui-wgpu", "winit", "rfd", "cpal", "muda"];

pub fn pkg(short: &str) -> String {
    if short == "filmcraft" || short == "xtask" { short.into() } else { format!("filmcraft-{short}") }
}

pub fn short(name: &str) -> &str {
    name.strip_prefix("filmcraft-").unwrap_or(name)
}

pub fn configs(root: PathBuf) -> Configs {
    let mut configs = Configs::for_root(&root);
    configs.version.cargo = Cargo { root: root.clone(), ..Cargo::default() };
    let l0: Vec<String> = L0_FOUNDATION.iter().chain(L0_CODECS).map(|s| pkg(s)).collect();
    let mut layers = vec![("L0".into(), l0.clone())];
    layers.extend(LAYERS.iter().map(|(l, names)| ((*l).into(), names.iter().map(|s| pkg(s)).collect())));
    let mut allowed_edges: Vec<(String, String)> = SAME_LAYER.iter().map(|(a, b)| (pkg(a), pkg(b))).collect();
    allowed_edges.extend(L0_CODECS.iter().map(|c| (pkg(c), pkg("bitstream"))));
    allowed_edges.extend(L0_FOUNDATION.iter().flat_map(|a| l0.iter().map(move |b| (pkg(a), b.clone()))));
    let allowed_from: Vec<String> = LAYERS[4..].iter().flat_map(|(_, names)| names.iter().map(|s| pkg(s))).collect();
    configs.layers = layers::Config {
        root,
        layers,
        allowed_edges,
        exempt: vec!["xtask".into()],
        standalone: L0_CODECS.iter().map(|s| pkg(s)).collect(),
        ignore_dev: true,
        external_rules: vec![layers::ExternalRule { dependencies: UI_ONLY.iter().map(|s| (*s).into()).collect(), allowed_from, allowed_dev_from: Vec::new() }],
        ..layers::Config::default()
    };
    configs
}
