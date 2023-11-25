use std::{rc::Rc, mem::{discriminant, Discriminant}, borrow::Cow};

use serde_json::json;

use crate::{resource::{ResState, Mod, ResConfig, StringError, ModData, Resource, JsonArray}, types::{Note, Sound}, channel::{PipelineStateChanges, Channel}};

pub struct SimpleChannel {
    ///Length of one tick in seconds
    pub tick_length: f32,

    ///Volume of the sound in platform's units
    pub volume: u8,

    ///Number of octaves above C-1.
    pub octave: u8,

    ///Default length for a note, in ticks.
    ///
    ///Used if note's length is None.
    pub length: u8,

    ///Duration of the sound after the note has been released, in ticks.
    pub post_release: u8,

    pub mods: Vec<Rc<dyn Mod>>,

    pub states: Vec<Rc<ResState>>,

    pub configs: Vec<Rc<ResConfig>>,
}

impl SimpleChannel {
    ///Create new ChannelState
    pub fn new(
        tick_length: f32,
        volume: u8,
        octave: u8,
        length: u8,
        post_release: u8,
        mods: Vec<Rc<dyn Mod>>,
        states: Vec<Rc<ResState>>,
        configs: Vec<Rc<ResConfig>>,
    ) -> Self {
        SimpleChannel {
            tick_length,
            volume,
            octave,
            length,
            post_release,
            mods,
            states,
            configs,
        }
    }
}

impl Resource for SimpleChannel {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed("Simple channel"))
    }

    fn id(&self) -> &str {
        "SIMPLE_CHANNEL"
    }

    //[cccc, tick_len, zenlen, tempo, max_volume]
    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError> {
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
            "argument 2 (Length of one tick) is not float".to_string(),
        )?;

        to_result(
            conf[2].is_i64(),
            "argument 3 (number of ticks in one whole note) is not integer".to_string(),
        )?;

        to_result(
            conf[3].is_f64(),
            "argument 4 (ticks per beat) is not float".to_string(),
        )?;

        to_result(
            conf[4].is_i64(),
            "argument 5 (maximum volume setting) is not integer".to_string(),
        )?;

        Ok(())
    }

    fn check_state(&self, _state: &ResState) -> Option<()> {
        Some(())
    }

    fn description(&self) -> &str {
        "A simple channel that auto-configures a builtin Note -> ResNote converter. Requires
        configuration through serialized PlatformValues."
    }
}

impl Channel for SimpleChannel {
    fn play(
        &self,
        item: ModData,
        state: &ResState,
        config: &ResConfig
    ) -> Result<(ModData, PipelineStateChanges, Box<ResState>), StringError> {
        if (self.mods.len() != self.states.len()) || (self.mods.len() != self.states.len()) {
            return Err(StringError(
                "number of mods, configs and states is not equal".to_owned(),
            ));
        }

        if !item.is_note() {
            return Err(StringError("channel expects a Note".to_string()))
        }

        let mut item = item;
        let mut state_changes: Vec<Box<ResState>> = Vec::new();

        for i in 0..self.mods.len() {
            if self.mods[i].id() == "BUILTIN_CONVERT_NOTE" {
                let cccc = config.get().get(0).unwrap().as_f64().unwrap();
                let tick_len = config.get().get(1).unwrap().as_f64().unwrap();
                let conf = JsonArray::from_value(json!([cccc, tick_len, self.octave, self.post_release, 0])).unwrap();
                match self.mods[i].apply(&item, &conf, &self.states[i]) {
                    Ok((new, state)) => {
                        item = new;
                        state_changes.push(state);
                    }
                    Err(what) => return Err(StringError(format!("mod error at {i}: {}", what))),
                }
                continue
            };
            if discriminant(&item) == self.mods[i].input_type() {
                match self.mods[i].apply(&item, &self.configs[i], &self.states[i]) {
                    Ok((new, state)) => {
                        item = new;
                        state_changes.push(state);
                    }
                    Err(what) => return Err(StringError(format!("mod error at {i}: {}", what))),
                }
            } else {
                return Err(StringError(format!(
                    "pipeline broken at {i} (type mismath)"
                )));
            }
        }

        match item {
            ModData::Sound(out) => Ok((ModData::Sound(out), state_changes, Box::new([]))),
            _ => Err(StringError("pipeline produced incorrect type".to_string())),
        }
    }

    fn input_type(&self) -> Option<Discriminant<ModData>> {
        Some(discriminant(&ModData::Note(Note::default())))
    }

    fn output_type(&self) -> Option<Discriminant<ModData>> {
        Some(discriminant(&ModData::Sound(Sound::new(Box::new([]), 0))))
    }
}
