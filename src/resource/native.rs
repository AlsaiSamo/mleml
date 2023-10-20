#![warn(missing_docs)]
//! Resources that are built into the library.

use std::borrow::Cow;
use dasp::frame::Stereo;

use crate::types::Sound;
use super::{JsonArray, Mod, PlatformValues, ResConfig, ResState, Resource, Platform};

fn compare_json_array(reference: &JsonArray, given: &JsonArray) -> bool {
    todo!()
}

///Simple implementation of a module that is easy to initialise and use.
pub struct SimpleMod<'msg, I, O> {
    name: String,
    id: String,
    schema: ResConfig,
    apply: fn(
        input: &I,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(O, Box<ResState>), Cow<'msg, str>>,
    check_state: fn(ResConfig) -> bool,
}

impl<'msg, I, O> SimpleMod<'msg, I, O> {
    pub fn new(
        name: String,
        id: String,
        schema: ResConfig,
        apply: fn(&I, &ResConfig, &ResState) -> Result<(O, Box<ResState>), Cow<'msg, str>>,
        check_state: fn(ResConfig) -> bool,
    ) -> Self {
        SimpleMod {
            name,
            id,
            schema,
            apply,
            check_state,
        }
    }
}

impl<'msg, I, O> Resource for SimpleMod<'msg, I, O> {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.name.as_str()))
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), Cow<'_, str>> {
        todo!()
    }

    fn check_state(&self, state: &ResState) -> Option<()> {
        todo!()
    }
}

impl<'msg, I, O> Mod<'msg, I, O> for SimpleMod<'msg, I, O> {
    fn apply(
        &self,
        input: &I,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(O, Box<ResState>), Cow<'msg, str>> {
        (self.apply)(input, conf, state)
    }
}

///Simple implementation of a platform.
///
///It cannot change the provided values, but it does allow custom mixing functions
pub struct SimplePlatform<'a, 'msg> {
    name: String,
    id: String,
    schema: ResConfig,
    values: PlatformValues,
    description: String,
    mix: fn(&[(bool, &'a [Stereo<f32>])], u32, &ResConfig, &ResState) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), Cow<'msg, str>>,
}

impl<'a, 'msg> SimplePlatform<'a, 'msg> {
    pub fn new(
        name: String,
        id: String,
        schema: ResConfig,
        values: PlatformValues,
        description: String,
        mix: fn(&[(bool, &'a [Stereo<f32>])], u32, &ResConfig, &ResState) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), Cow<'msg, str>>,
    ) -> Self {
        SimplePlatform {
            name,
            id,
            schema,
            values,
            description,
            mix
        }
    }
}

impl<'a, 'msg> Resource for SimplePlatform<'a, 'msg> {
    fn orig_name(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Borrowed(self.name.as_str()))
    }

    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn check_config(&self, conf: &ResConfig) -> Result<(), Cow<'_, str>> {
        todo!()
    }

    fn check_state(&self, state: &ResState) -> Option<()> {
        todo!()
    }
}

impl<'a, 'msg> Platform<'a, 'msg> for SimplePlatform<'a, 'msg> {
    fn get_vals(&self) -> PlatformValues {
        self.values.clone()
    }

    fn mix(
        &self,
        channels: &[(bool, &'a [Stereo<f32>])],
        play_time: u32,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), Cow<'msg, str>> {
        (self.mix)(channels, play_time, conf, state)
    }

    fn description(&self) -> String {
        todo!()
    }
}
