use mleml::{
    resource::{Mod, ResConfig},
    types::ReadyNote, extra::example_resources::FourOpFm,
};
use serde_json::json;
use std::{fs::OpenOptions, io::Write, path::Path};

fn main() {
    let fop = FourOpFm{};
    let note = ReadyNote { len: 3.0, post_release: 2.0, pitch: Some(256.0), velocity: 64 };
    let conf = ResConfig::from_vec(json!([4, false,
    0, 0, 210, 511, 110, 127, 12, 192,
    0, 140, 200, 260, 110, 30, 4, 192,
    0, 0, 210, 511, 110, 127, 4, 180,
    0, 140, 200, 260, 110, 30, 4, 180
    ]).as_array().unwrap().to_owned()).unwrap();
    let state: Vec<u8> = Vec::new();
    let out = fop.apply(&note, &conf, state.as_slice()).unwrap().0;
    let synthesized: Vec<u8> = out
        .data()
        .iter()
        .map(|x| x[0])
        .flat_map(|x| x.to_le_bytes())
        .collect();

    let path = Path::new("example_fm.pcm");
    let mut file = match OpenOptions::new().write(true).create(true).open(&path) {
        Ok(file) => file,
        Err(e) => panic!("couldn't open {}: {}", path.display(), e),
    };
    file.write_all(&synthesized).unwrap();
}
