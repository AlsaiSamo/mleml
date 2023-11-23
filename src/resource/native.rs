#![warn(missing_docs)]
//! Resources that are built into the library.
//!
//! Currently, only `SimpleMod` and `SimplePlatform` are provided, which may not offer
//! some functionality, like changing PlatformValues.

use dasp::frame::Stereo;
use std::{
    borrow::Cow,
    mem::{discriminant, Discriminant},
};

use super::{
    JsonArray, Mod, ModData, Platform, PlatformValues, ResConfig, ResState, Resource, StringError,
};
use crate::types::{Note, ReadyNote, Sound};

fn json_array_find_deviation(reference: &JsonArray, given: &JsonArray) -> Option<usize> {
    (0..given.len())
        .find(|&i| discriminant(&reference.as_slice()[i]) != discriminant(&given.as_slice()[i]))
}

///Simple implementation of a module that is easy to initialise and use.
pub struct SimpleMod {
    name: String,
    id: String,
    desc: String,
    schema: ResConfig,
    apply: fn(
        input: &ModData,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(ModData, Box<ResState>), StringError>,
    check_state: fn(&ResState) -> bool,
    input_type: Discriminant<ModData>,
    output_type: Discriminant<ModData>,
}

impl SimpleMod {
    ///Create new SimpleMod.
    pub fn new(
        name: String,
        id: String,
        desc: String,
        schema: ResConfig,
        apply: fn(&ModData, &ResConfig, &ResState) -> Result<(ModData, Box<ResState>), StringError>,
        check_state: fn(&ResState) -> bool,
        input_type: Discriminant<ModData>,
        output_type: Discriminant<ModData>,
    ) -> Self {
        SimpleMod {
            name,
            id,
            desc,
            schema,
            apply,
            check_state,
            input_type,
            output_type,
        }
    }
}

impl Resource for SimpleMod {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.name.as_str()))
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError> {
        match json_array_find_deviation(&self.schema, conf) {
            Some(i) => Err(StringError(format!("type mismatch at index {}", i))),
            None => Ok(()),
        }
    }

    fn check_state(&self, state: &ResState) -> Option<()> {
        (self.check_state)(state).then_some(())
    }

    fn description(&self) -> &str {
        self.desc.as_str()
    }
}

impl Mod for SimpleMod {
    fn apply(
        &self,
        input: &ModData,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(ModData, Box<ResState>), StringError> {
        if discriminant(input) != self.input_type {
            Err(StringError("incorrect input type".to_string()))
        } else {
            (self.apply)(input, conf, state)
        }
    }

    fn input_type(&self) -> Discriminant<ModData> {
        self.input_type
    }

    fn output_type(&self) -> Discriminant<ModData> {
        self.output_type
    }
}

///Simple implementation of a platform.
///
///It cannot change the provided values.
pub struct SimplePlatform<'a> {
    name: String,
    id: String,
    desc: String,
    schema: ResConfig,
    values: PlatformValues,
    mix: fn(
        &[(bool, &'a [Stereo<f32>])],
        u32,
        &ResConfig,
        &ResState,
    ) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), StringError>,
    check_state: fn(&ResState) -> bool,
}

impl<'a> SimplePlatform<'a> {
    ///Create new SimplePlatform.
    pub fn new(
        name: String,
        id: String,
        desc: String,
        schema: ResConfig,
        values: PlatformValues,
        mix: fn(
            &[(bool, &'a [Stereo<f32>])],
            u32,
            &ResConfig,
            &ResState,
        )
            -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), StringError>,
        check_state: fn(&ResState) -> bool,
    ) -> Self {
        SimplePlatform {
            name,
            id,
            desc,
            schema,
            values,
            mix,
            check_state,
        }
    }
}

impl<'a> Resource for SimplePlatform<'a> {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.name.as_str()))
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError> {
        match json_array_find_deviation(&self.schema, conf) {
            Some(i) => Err(StringError(format!("type mismatch at index {}", i))),
            None => Ok(()),
        }
    }

    fn check_state(&self, state: &ResState) -> Option<()> {
        (self.check_state)(state).then_some(())
    }

    fn description(&self) -> &str {
        self.desc.as_str()
    }
}

impl<'a> Platform<'a> for SimplePlatform<'a> {
    fn get_vals(&self) -> PlatformValues {
        self.values.clone()
    }

    fn mix(
        &self,
        channels: &[(bool, &'a [Stereo<f32>])],
        play_time: u32,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), StringError> {
        (self.mix)(channels, play_time, conf, state)
    }
}

//TODO: how to best explain that this one is special?
//TODO: maybe this hsould be moved out?
pub struct ConvertNote();

impl Resource for ConvertNote {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed("Prepare note for playing"))
    }

    fn id(&self) -> &str {
        "BUILTIN_CONVERT_NOTE"
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError> {
        //TODO: consider turning to_result() into a macro and use it in other places
        //TODO: write somewhere how the schema needs to be defined? or have the user simply see the code?

        fn to_result(input: bool, msg: String) -> Result<(), StringError> {
            match input {
                true => Ok(()),
                false => Err(StringError(msg)),
            }
        }

        let conf = conf.as_slice();

        to_result(conf.len() == 5, "incorrect config length".to_string())?;
        to_result(
            conf[0].is_f64(),
            "argument 1 (frequency of note 0) is not float".to_string(),
        )?;
        to_result(
            conf[1].is_f64(),
            "argument 2 (length of one tick) is not float".to_string(),
        )?;
        to_result(
            conf[2].is_i64() && conf[2].as_i64().unwrap() >= 0,
            "argument 3 (octave) is not nonnegative integer".to_string(),
        )?;
        to_result(
            conf[3].is_i64(),
            "argument 4 (length of sound post key release) is not integer".to_string(),
        )?;
        to_result(
            conf[4].is_i64(),
            "argument 5 (added cents) is not integer".to_string(),
        )?;
        Ok(())
    }

    //No state
    fn check_state(&self, _state: &ResState) -> Option<()> {
        Some(())
    }

    fn description(&self) -> &str {
        "Built-in mod to prepare the note for playing"
    }
}

//TODO: verify
impl Mod for ConvertNote {
    fn apply(
        &self,
        input: &ModData,
        conf: &ResConfig,
        _state: &ResState,
    ) -> Result<(ModData, Box<ResState>), StringError> {
        self.check_config(conf)?;
        if discriminant(input) != self.input_type() {
            Err(StringError("incorrect type provided".to_string()))
        } else {
            let conf = conf.as_slice();
            let input = input.as_note().unwrap();
            let tick_length = conf[1].as_f64().unwrap();

            let len = (input
                .len
                .ok_or(StringError("length of the note is unspecified".to_string()))?
                .get() as f64
                * tick_length) as f32;
            let post_release = (conf[3].as_i64().unwrap() as f64 * tick_length) as f32;
            let pitch = input.pitch.map(|semitones| {
                conf[0].as_f64().unwrap() as f32
                    * 2.0_f32.powf(
                        1.0 + (semitones.get() as f32) / 12.0
                            + (conf[4].as_i64().unwrap() as f32) / 1200.0
                            + conf[2].as_i64().unwrap() as f32,
                    )
            });
            let velocity = input.velocity;

            let out = ReadyNote {
                len,
                post_release,
                pitch,
                velocity,
            };
            Ok((ModData::ReadyNote(out), Box::new([])))
        }
    }

    fn input_type(&self) -> Discriminant<ModData> {
        discriminant(&ModData::Note(Note::default()))
    }

    fn output_type(&self) -> Discriminant<ModData> {
        discriminant(&ModData::ReadyNote(ReadyNote::default()))
    }
}
