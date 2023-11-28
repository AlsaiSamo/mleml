use std::mem::discriminant;

use dasp::frame::Stereo;

use crate::{
    resource::{
        JsonArray, LeftoverSound, Mixer, PremixedSound, ResConfig, ResState, Resource, StringError,
    },
    types::Sound,
};

/// A mixer template that is easy to create and use.
pub struct SimpleMixer<'a> {
    name: String,
    id: String,
    desc: String,
    schema: ResConfig,
    values: ResConfig,
    mix: fn(
        &[(bool, &'a [Stereo<f32>])],
        u32,
        &ResConfig,
        &ResState,
    ) -> Result<(Sound, Box<ResState>, LeftoverSound<'a>), StringError>,
    check_state: fn(&ResState) -> bool,
}

impl<'a> SimpleMixer<'a> {
    /// Create new SimpleMixer.
    pub fn new(
        name: String,
        id: String,
        desc: String,
        schema: ResConfig,
        values: ResConfig,
        mix: fn(
            PremixedSound,
            u32,
            &ResConfig,
            &ResState,
        ) -> Result<(Sound, Box<ResState>, LeftoverSound<'a>), StringError>,
        check_state: fn(&ResState) -> bool,
    ) -> Self {
        SimpleMixer {
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

impl<'a> Resource for SimpleMixer<'a> {
    fn orig_name(&self) -> &str {
        self.name.as_str()
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

impl<'a> Mixer<'a> for SimpleMixer<'a> {
    fn get_config(&self) -> ResConfig {
        self.values.clone()
    }

    fn mix(
        &self,
        channels: PremixedSound<'a>,
        play_time: u32,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>, LeftoverSound<'a>), StringError> {
        (self.mix)(channels, play_time, conf, state)
    }
}

fn json_array_find_deviation(reference: &JsonArray, given: &JsonArray) -> Option<usize> {
    (0..given.len())
        .find(|&i| discriminant(&reference.as_slice()[i]) != discriminant(&given.as_slice()[i]))
}
