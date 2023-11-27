use std::{
    borrow::Cow,
    mem::{discriminant, Discriminant},
};

use crate::{
    resource::{Mod, ModData, ResConfig, ResState, Resource, StringError},
    types::{Note, ReadyNote},
};

/// Mod to convert Note into ResNote.
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
            "argument 1 (frequency of C-1) is not float".to_string(),
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
            let decay_time = (conf[3].as_i64().unwrap() as f64 * tick_length) as f32;
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
                decay_time,
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
