use dasp::{
    interpolate::linear::Linear,
    signal,
    slice::{add_in_place, map_in_place},
    Frame, Signal,
};
use mleml::{
    resource::{
        native::{SimpleMod, SimplePlatform},
        PlatformValues,
        Platform
    },
    resource::{JsonArray, Mod},
    types::{ReadyNote, Sound},
};
use serde_json::json;
use std::{borrow::Cow, fs::OpenOptions, path::Path, io::Write};

//Writes a file with pcm_f32le format
fn main() {
    //Simple square generator
    let square: SimpleMod<ReadyNote, Sound> = SimpleMod::new(
        "Square wave generator".to_owned(),
        //This should be some random string but eh
        "SQUARE".to_owned(),
        //No config
        //JsonArray::from_vec(vec![]).unwrap(),
        JsonArray::new(),
        |input, _conf, _state| match input.pitch {
            Some(hz) => {
                let signal = signal::rate(48000.0).const_hz(hz.into()).square();
                let data = signal
                    .take((input.len * 48000.0).ceil() as usize)
                    .map(|x| [x as f32, x as f32])
                    .collect();
                Ok((Sound::new(data, 48000), Box::new([])))
            }
            None => todo!(),
        },
        //No state -> all state is good
        |_| true,
    );
    let two_sine: SimpleMod<ReadyNote, Sound> = SimpleMod::new(
        "Sine modulated with sine".to_owned(),
        "TWO_SINES".to_owned(),
        //Modulating sine's frequency
        JsonArray::from_vec(vec![json!(440)]).unwrap(),
        |input, conf, _state| -> Result<(Sound, Box<[u8]>), std::borrow::Cow<'_, str>> {
            match input.pitch {
                Some(hz) => {
                    //Modulating wave
                    let s1 = signal::rate(48000.0).const_hz(hz.into()).sine();
                    //Carrier wave
                    let s2 = signal::rate(48000.0)
                        .const_hz(conf.as_slice()[0].as_f64().unwrap())
                        .sine();
                    let interp = Linear::new(0.0, 1.0);
                    let out = s2
                        .mul_hz(interp, s1)
                        .take((input.len * 48000.0).ceil() as usize)
                        .map(|x| [x as f32, x as f32])
                        .collect();
                    Ok((Sound::new(out, 48000), Box::new([])))
                }
                None => todo!(),
            }
        },
        |_| true,
    );
    let mixer = SimplePlatform::new(
        "Two channel addition".to_owned(),
        "MIXER".to_owned(),
        JsonArray::new(),
        PlatformValues {
            cccc: 8.0,
            tick_len: 0.00028,
            zenlen: 96,
            tempo: 150.0,
            max_volume: 255,
            channels: 2,
        },
        "Just adds two channels together".to_owned(),
        |input, _conf, _state| -> Result<(Sound, Box<[u8]>), Cow<'_, str>> {
            if input.len() != 2 {
                Err(Cow::Borrowed("platform needs exactly two channels"))
            } else {
                let mut out = input[0].data().to_owned();
                add_in_place(&mut out, input[1].data());
                map_in_place(&mut out, |x| x.mul_amp([0.5, 0.5]));
                Ok((Sound::new(out.into(), 48000), Box::new([])))
            }
        },
    );
    let note =
        ReadyNote {
            len: 2.0,
            post_release: 0.0,
            pitch: Some(962.0),
            velocity: 128,
        };
    let square_note = square.apply(
        &note,
        &JsonArray::new(),
        &[],
    ).unwrap().0;
    let sines_note = two_sine.apply(
        &note,
        &JsonArray::from_vec(vec![json!(256)]).unwrap(),
        &[]
    ).unwrap().0;
    let res = mixer.mix(&[&square_note, &sines_note], &JsonArray::new(), &[]).unwrap();
    let synthesized: Vec<u8> = res.0.data().iter().flatten().flat_map(|x| x.to_le_bytes()).collect();

    let path = Path::new("one_sound.pcm");
    let mut file = match OpenOptions::new().write(true).create(true).open(&path) {
        Ok(file) => file,
        Err(e) => panic!("couldn't open {}: {}", path.display(), e),
    };
    file.write_all(&synthesized).unwrap();
}
