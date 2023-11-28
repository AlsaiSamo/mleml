#![feature(closure_lifetime_binder)]

use dasp::{
    frame::Stereo,
    interpolate::linear::Linear,
    signal,
    slice::{add_in_place, map_in_place},
    Frame, Signal,
};
use mleml::{
    extra::builtin::{SimpleMixer, SimpleMod},
    resource::{JsonArray, Mixer, Mod, ModData, ResConfig, ResState, StringError},
    types::{ReadyNote, Sound},
};
use serde_json::json;
use std::{fs::OpenOptions, io::Write, mem::discriminant, path::Path};

//Writes a file with pcm_f32le format
fn main() {
    //Simple square generator
    let square: SimpleMod = SimpleMod::new(
        "Square wave generator".to_owned(),
        //This should be some random string but eh
        "SQUARE".to_owned(),
        "Square wave generator".to_owned(),
        //No config
        JsonArray::new(),
        |input, _conf, _state| -> Result<(ModData, Box<[u8]>), StringError> {
            let input = input
                .as_ready_note()
                .ok_or(StringError("input needs to be a ReadyNote".to_string()))?;
            match input.pitch {
                Some(hz) => {
                    let signal = signal::rate(48000.0).const_hz(hz.into()).square();
                    let data = signal
                        .take((input.len * 48000.0).ceil() as usize)
                        .map(|x: f64| [x as f32, x as f32])
                        .collect();
                    Ok((ModData::Sound(Sound::new(data, 48000)), Box::new([])))
                }
                None => todo!(),
            }
        },
        //No state -> all state is good
        |_| true,
        discriminant(&mleml::resource::ModData::ReadyNote(ReadyNote::default())),
        discriminant(&mleml::resource::ModData::Sound(Sound::new(
            Box::new([]),
            0,
        ))),
    );
    let two_sine: SimpleMod = SimpleMod::new(
        "Sine modulated with sine".to_owned(),
        "TWO_SINES".to_owned(),
        "Sine modulated with another sine".to_owned(),
        //Modulating sine's frequency
        JsonArray::from_vec(vec![json!(440)]).unwrap(),
        |input, conf, _state| -> Result<(ModData, Box<[u8]>), StringError> {
            let input = input
                .as_ready_note()
                .ok_or(StringError("input needs to be a ReadyNote".to_string()))?;
            match input.pitch {
                Some(hz) => {
                    //Modulating wave
                    let s1 = signal::rate(48000.0)
                        .const_hz(hz.into())
                        .sine()
                        .scale_amp(0.5)
                        .offset_amp(1.0);
                    //Carrier wave
                    let s2 = signal::rate(48000.0)
                        .const_hz(conf.as_slice()[0].as_f64().unwrap())
                        .sine()
                        .scale_amp(0.5)
                        .offset_amp(1.0);
                    let interp = Linear::new(0.0, 1.0);
                    let out = s2
                        .mul_hz(interp, s1)
                        .take((input.len * 48000.0).ceil() as usize)
                        .map(|x| [x as f32, x as f32])
                        .collect();
                    Ok((ModData::Sound(Sound::new(out, 48000)), Box::new([])))
                }
                None => todo!(),
            }
        },
        |_| true,
        discriminant(&mleml::resource::ModData::ReadyNote(ReadyNote::default())),
        discriminant(&mleml::resource::ModData::Sound(Sound::new(
            Box::new([]),
            0,
        ))),
    );
    let mixer = SimpleMixer::new(
        "Two channel addition".to_owned(),
        "MIXER".to_owned(),
        "Adds two channels together crudely".to_owned(),
        JsonArray::new(),
        JsonArray::from_vec(
            json!([8.0, 0.00028, 96, 150.0, 255])
                .as_array()
                .unwrap()
                .to_owned(),
        )
        .unwrap(),
        for<'a, 'b, 'c, 'd, 'e> |input: &'b [(bool, &'e [Stereo<f32>])],
                                 _play: u32,
                                 _conf: &'c ResConfig,
                                 _state: &'d ResState|
                                 -> Result<
            (Sound, Box<[u8]>, Box<[Option<&'a [Stereo<f32>]>]>),
            StringError,
        > {
            if input.len() != 2 {
                Err(StringError("mixer needs exactly two channels".to_owned()))
            } else {
                let mut out = input[0].1.to_owned();
                add_in_place(&mut out, input[1].1);
                map_in_place(&mut out, |x| x.mul_amp([0.5, 0.5]));
                Ok((
                    Sound::new(out.into(), 48000),
                    Box::new([]),
                    Box::new([None, None]),
                ))
            }
        },
        |_| true,
    );
    let note = ModData::ReadyNote(ReadyNote {
        len: 2.0,
        decay_time: 0.0,
        pitch: Some(440.0),
        velocity: 128,
    });
    let square_note = square.apply(&note, &JsonArray::new(), &[]).unwrap().0;
    // let square_note: Sound = todo!();
    let sines_note = two_sine
        .apply(&note, &JsonArray::from_vec(vec![json!(256)]).unwrap(), &[])
        .unwrap()
        .0;
    // let sines_note: Sound = todo!();
    let premix = vec![
        (true, square_note.as_sound().unwrap().as_ref()),
        (true, sines_note.as_sound().unwrap().as_ref()),
    ];
    let res = mixer
        .mix(premix.as_slice(), 9999, &JsonArray::new(), &[])
        .unwrap();
    let synthesized: Vec<u8> = res
        .0
        .data()
        .iter()
        .flatten()
        .flat_map(|x| x.to_le_bytes())
        .collect();

    let path = Path::new("one_sound.pcm");
    let mut file = match OpenOptions::new().write(true).create(true).open(path) {
        Ok(file) => file,
        Err(e) => panic!("couldn't open {}: {}", path.display(), e),
    };
    file.write_all(&synthesized).unwrap();
}
