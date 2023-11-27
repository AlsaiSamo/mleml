use crate::{
    resource::{Mod, ModData, ResConfig, ResState, Resource, StringError},
    types::{ReadyNote, Sound},
};
use dasp::{
    interpolate::linear::Linear,
    signal::{self, ConstHz, FromIterator, MulAmp, Saw, Sine, Take, UntilExhausted},
    Frame, Signal,
};
use serde_json::Value as JsonValue;
use std::{
    borrow::{self},
    iter::{self, Chain, FromFn},
    mem::{discriminant, Discriminant},
};

//dasp allows generalising over impl Signal, but I couldn't use that, this
//enum is used instead.
enum Wave {
    Sine(Sine<ConstHz>),
    Saw(Saw<ConstHz>),
}

impl Signal for Wave {
    type Frame = f64;

    fn next(&mut self) -> Self::Frame {
        match self {
            Wave::Sine(w) => w.next().map(clamp_f64_to_i8),
            Wave::Saw(w) => w.next().map(clamp_f64_to_i8),
        }
    }
}

//Same as Wave
enum IterSignal<S: Signal> {
    Take(Take<S>),
    All(UntilExhausted<S>),
}

impl<S: Signal> Iterator for IterSignal<S> {
    type Item = <S as Signal>::Frame;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterSignal::Take(s) => s.next(),
            IterSignal::All(s) => s.next(),
        }
    }
}

/// Example four-operator FM synthesizer.
pub struct FourOpFm();

impl Resource for FourOpFm {
    fn orig_name(&self) -> Option<borrow::Cow<'_, str>> {
        Some(borrow::Cow::Borrowed("Simple FM"))
    }

    fn id(&self) -> &str {
        "FOUR_OPERATOR_FM"
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError> {
        let conf = conf.as_slice();
        let len = conf.len();
        if len != 34 {
            return Err(StringError(format!(
                "wrong number of values: expected 34, got {len}"
            )));
        }
        get_int_value(&conf[0], 0, 7)?;
        get_bool_value(&conf[1])?;
        for op in 0..4 {
            get_int_value(&conf[2 + 8 * op], 0, 511)?;
            get_int_value(&conf[3 + 8 * op], 0, 511)?;
            get_int_value(&conf[4 + 8 * op], 0, 511)?;
            get_int_value(&conf[5 + 8 * op], 0, 511)?;
            get_int_value(&conf[6 + 8 * op], 0, 127)?;
            get_int_value(&conf[7 + 8 * op], 0, 127)?;
            get_int_value(&conf[8 + 8 * op], 0, 31)?;
            get_int_value(&conf[9 + 8 * op], -511, 511)?;
        }
        Ok(())
    }

    fn check_state(&self, _: &ResState) -> Option<()> {
        Some(())
    }

    fn description(&self) -> &str {
        "Simple four operator FM."
    }
}

impl Mod for FourOpFm {
    fn apply(
        &self,
        input: &ModData,
        conf: &ResConfig,
        _: &[u8],
    ) -> Result<(ModData, Box<ResState>), StringError> {
        let input = input
            .as_ready_note()
            .ok_or(StringError("input has to be a ReadyNote".to_string()))?;
        if input.pitch.is_none() {
            let len = ((input.len + input.decay_time) * 48000.0) as usize;
            let data: Box<[[f32; 2]]> = vec![[0.0, 0.0]; len].into_boxed_slice();
            return Ok((ModData::Sound(Sound::new(data, 48000)), Box::new([])));
        }

        let conf = conf.as_slice();
        //Algorhitm to chain operators. Taken from YM2608 datasheet.
        let alg = get_int_value(&conf[0], 0, 7)? as i8;
        //Should the first operator be sawtooth or not
        let saw = get_bool_value(&conf[1])?;
        let mut op_params = <[FnParams; 4]>::default();
        for op in 0..4 {
            op_params[op].ar = get_int_value(&conf[2 + 8 * op], 0, 511)? as i16;
            op_params[op].dr = get_int_value(&conf[3 + 8 * op], 0, 511)? as i16;
            op_params[op].sr = get_int_value(&conf[4 + 8 * op], 0, 511)? as i16;
            op_params[op].rr = get_int_value(&conf[5 + 8 * op], 0, 511)? as i16;
            op_params[op].sl = get_int_value(&conf[6 + 8 * op], 0, 127)? as i8;
            op_params[op].tl = get_int_value(&conf[7 + 8 * op], 0, 127)? as i8;
            op_params[op].ml = get_int_value(&conf[8 + 8 * op], 0, 31)? as i8;
            op_params[op].dt = get_int_value(&conf[9 + 8 * op], -511, 511)? as i16;
        }
        let op0 = play_fn_operator(&op_params[0], input, saw);
        let op1 = play_fn_operator(&op_params[1], input, false);
        let op2 = play_fn_operator(&op_params[2], input, false);
        let op3 = play_fn_operator(&op_params[3], input, false);

        match alg {
            //Operators are chained one after another
            0 => {
                let op1 = op1.mul_hz(linear(), op0.offset_amp(1.0));
                let op2 = op2.mul_hz(linear(), op1.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op2.offset_amp(1.0));
                let out = op3.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //Operators 0 and 1 modulate 2, which goes into 3
            1 => {
                let op2 = op2.mul_hz(linear(), op0.offset_amp(1.0));
                let op2 = op2.mul_hz(linear(), op1.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op2.offset_amp(1.0));
                let out = op3.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //Operator 1 modulates 2, 0 and 2 go into 3
            2 => {
                let op2 = op2.mul_hz(linear(), op1.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op0.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op2.offset_amp(1.0));
                let out = op3.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //Operator 0 modulates 1, 1 and 2 go into 3
            3 => {
                let op1 = op1.mul_hz(linear(), op0.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op1.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op2.offset_amp(1.0));
                let out = op3.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //Two lines (0 into 1, 2 into 3)
            4 => {
                let op1 = op1.mul_hz(linear(), op0.offset_amp(1.0));
                let op3 = op3.mul_hz(linear(), op2.offset_amp(1.0));
                let out = op3.add_amp(op1);
                let out = out.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //0 goes into 1, 2 and 3
            5 => {
                //FIXME: because FromIterator (or is it FnMut inside?) doesn't impl Clone,
                // I cannnnot clone op0. Naive approach is to make it 3 times,
                // as shown here. It would be better to use Fork.
                let op0_1 = play_fn_operator(&op_params[0], input, saw);
                let op0_2 = play_fn_operator(&op_params[0], input, saw);

                let op1 = op1.mul_hz(linear(), op0.scale_amp(0.5).offset_amp(0.5));
                let op2 = op2.mul_hz(linear(), op0_1.scale_amp(0.5).offset_amp(0.5));
                let op3 = op3.mul_hz(linear(), op0_2.scale_amp(0.5).offset_amp(0.5));
                let out = op3.add_amp(op1).add_amp(op2).scale_amp(0.333);
                let out = out.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //0 goes into 1
            6 => {
                let op1 = op1.mul_hz(linear(), op0.scale_amp(0.5).offset_amp(0.5));
                let out = op3.add_amp(op1).add_amp(op2).scale_amp(0.333);
                let out = out.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            //No modulation
            7 => {
                let out = op3.add_amp(op1).add_amp(op2).add_amp(op0).scale_amp(0.25);
                let out = out.map(|x| [x as f32, x as f32]);
                let time = ((input.len + input.decay_time) * 48000.0) as usize;
                Ok((
                    ModData::Sound(Sound::new(
                        out.take(time).map(clamp_frame_to_i8).collect(),
                        48000,
                    )),
                    Box::new([]),
                ))
            }
            _ => unreachable!(),
        }
    }

    fn input_type(&self) -> Discriminant<ModData> {
        discriminant(&ModData::ReadyNote(ReadyNote::default()))
    }

    fn output_type(&self) -> Discriminant<ModData> {
        discriminant(&ModData::Sound(Sound::new(Box::new([]), 0)))
    }
}

#[derive(Default, Clone)]
struct FnParams {
    //Attack rate
    pub ar: i16,
    //Decay rate
    pub dr: i16,
    //Sustain rate (max. time the sound is allowed to be sustained)
    pub sr: i16,
    //Release rate
    pub rr: i16,
    //Sustain level
    pub sl: i8,
    //Total level
    pub tl: i8,
    //Multiplier
    pub ml: i8,
    //Detune
    pub dt: i16,
}

//With current approach to envelope the return type has to be this big.
// It can be made nicer if instead of four small iterators there was one that is complex.
fn play_fn_operator(
    params: &FnParams,
    note: &ReadyNote,
    saw: bool,
) -> MulAmp<
    Wave,
    FromIterator<
        iter::Map<
            Chain<
                Chain<
                    IterSignal<
                        FromIterator<
                            Chain<
                                Chain<
                                    FromFn<impl FnMut() -> Option<f64>>,
                                    FromFn<impl FnMut() -> Option<f64>>,
                                >,
                                FromFn<impl FnMut() -> Option<f64>>,
                            >,
                        >,
                    >,
                    FromFn<impl FnMut() -> Option<f64>>,
                >,
                iter::Repeat<f64>,
            >,
            impl FnMut(f64) -> f64,
        >,
    >,
> {
    //Frequency multipler
    let multiplier = match params.ml {
        ml if ml < 0 => unreachable!(),
        0 => 0.5,
        ml => ml as f64,
    };

    //Detune is treated as 1/32 of a cent.
    let detune = 2.0_f64.powf(params.dt as f64 / 3200.0);
    //Wave's frequency.
    let native: signal::ConstHz =
        signal::rate(48000.0).const_hz(note.pitch.unwrap() as f64 * multiplier * detune);
    //Used for envelope calculation.
    let sustain_mul = (127 - params.sl) as f64 / 127.0;
    //Note's length in frames.
    let len_frames = (note.len * 48000.0) as usize;
    //Sound level during sustain.
    let sustain_level = params.sl as f64 / 127.0;

    //Lengths of envelope parts.
    let attack_frames = 2.0_f64.powf(params.ar as f64 / 16.0);
    let decay_frames = 2.0_f64.powf(params.dr as f64 / 16.0);
    let sustain_frames = 2.0_f64.powf(params.sr as f64 / 16.0);
    let release_frames = 2.0_f64.powf(params.rr as f64 / 16.0);

    //Find sound level when release needs to happen.
    let release_level = match len_frames {
        //If note is released during attack.
        x if x <= attack_frames as usize => x as f64 / attack_frames,
        //If note is released during decay.
        x if x <= (attack_frames + decay_frames) as usize => {
            (x - attack_frames as usize) as f64 / decay_frames * sustain_mul
        }
        //Anything else.
        _ => sustain_level,
    };

    //Parts of the envelope:
    //Attack
    let mut count = 0;
    let attack = iter::from_fn(move || {
        count += 1;
        if count >= attack_frames as usize {
            None
        } else {
            Some(count as f64 / attack_frames)
        }
    });

    //Decay
    let mut count = 0;
    let decay = iter::from_fn(move || {
        count += 1;
        if count >= decay_frames as usize {
            None
        } else {
            Some(1.0 - count as f64 / decay_frames * sustain_mul)
        }
    });

    //Sustain
    let mut count = 0;
    let sustain = iter::from_fn(move || {
        count += 1;
        if count >= sustain_frames as usize {
            None
        } else {
            Some(sustain_level)
        }
    });

    //Release
    let mut count = release_frames as usize;
    let release = iter::from_fn(move || {
        count -= 1;
        if count == 0 {
            None
        } else {
            Some(count as f64 / release_frames * release_level)
        }
    });

    //First 3 stages of the envelope happen up until the key is released,
    //or until they end on their own.
    let ads_len = (attack_frames + decay_frames + sustain_frames) as usize;
    let ads = if ads_len <= len_frames {
        IterSignal::All(signal::from_iter(attack.chain(decay).chain(sustain)).until_exhausted())
    } else {
        IterSignal::Take(signal::from_iter(attack.chain(decay).chain(sustain)).take(ads_len))
    };
    let total_level = params.tl as f64 / 127.0;
    let envelope = signal::from_iter(
        ads.chain(release)
            .chain(iter::repeat(0.0))
            .map(move |x| x * total_level),
    );

    match saw {
        true => Wave::Saw(native.saw()).mul_amp(envelope),
        false => Wave::Sine(native.sine()).mul_amp(envelope),
    }
}

fn linear() -> Linear<f64> {
    Linear::new(0.0, 1.0)
}

fn get_int_value(val: &JsonValue, lower: i64, upper: i64) -> Result<i64, StringError> {
    match val.as_i64() {
        Some(x) => match x {
            x if (x < lower) || (x > upper) => Err(StringError(format!(
                "value {} is outside of range {} - {}",
                x, lower, upper
            ))),
            _ => Ok(x),
        },
        None => Err(StringError("extracted value is not integer".to_string())),
    }
}

fn get_bool_value(val: &JsonValue) -> Result<bool, StringError> {
    match val.as_bool() {
        Some(x) => Ok(x),
        None => Err(StringError("extracted value is not bool".to_string())),
    }
}

//Could just divide, truncate, and multiply back
fn clamp_f64_to_i8(f: f64) -> f64 {
    ((f * 512.0) as i8) as f64 / 512.0
}

fn clamp_frame_to_i8(f: [f32; 2]) -> [f32; 2] {
    [
        ((f[0] * 512.0) as i8) as f32 / 512.0,
        ((f[1] * 512.0) as i8) as f32 / 512.0,
    ]
}
