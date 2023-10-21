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

use super::{JsonArray, Mod, Platform, PlatformValues, ResConfig, ResState, Resource};
use crate::types::Sound;

fn json_array_find_deviation(reference: &JsonArray, given: &JsonArray) -> Option<usize> {
    for i in 0..given.len() {
        if discriminant(&reference.as_slice()[i]) != discriminant(&given.as_slice()[i]) {
            return Some(i);
        }
    }
    None
}

///Simple implementation of a module that is easy to initialise and use.
pub struct SimpleMod<'msg, I, O> {
    name: String,
    id: String,
    desc: String,
    schema: ResConfig,
    apply: fn(
        input: &I,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(O, Box<ResState>), Cow<'msg, str>>,
    check_state: fn(&ResState) -> bool,
}

impl<'msg, I, O> SimpleMod<'msg, I, O> {
    pub fn new(
        name: String,
        id: String,
        desc: String,
        schema: ResConfig,
        apply: fn(&I, &ResConfig, &ResState) -> Result<(O, Box<ResState>), Cow<'msg, str>>,
        check_state: fn(&ResState) -> bool,
    ) -> Self {
        SimpleMod {
            name,
            id,
            desc,
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
        match json_array_find_deviation(&self.schema, conf) {
            Some(i) => Err(Cow::Owned(format!("type mismatch at index {}", i))),
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
///It cannot change the provided values.
pub struct SimplePlatform<'a, 'msg> {
    name: String,
    id: String,
    desc: String,
    schema: ResConfig,
    values: PlatformValues,
    description: String,
    mix: fn(
        &[(bool, &'a [Stereo<f32>])],
        u32,
        &ResConfig,
        &ResState,
    )
        -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), Cow<'msg, str>>,
    check_state: fn(&ResState) -> bool,
}

impl<'a, 'msg> SimplePlatform<'a, 'msg> {
    pub fn new(
        name: String,
        id: String,
        desc: String,
        schema: ResConfig,
        values: PlatformValues,
        description: String,
        mix: fn(
            &[(bool, &'a [Stereo<f32>])],
            u32,
            &ResConfig,
            &ResState,
        )
            -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>), Cow<'msg, str>>,
        check_state: fn(&ResState) -> bool,
    ) -> Self {
        SimplePlatform {
            name,
            id,
            desc,
            schema,
            values,
            description,
            mix,
            check_state
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
        match json_array_find_deviation(&self.schema, conf) {
            Some(i) => Err(Cow::Owned(format!("type mismatch at index {}", i))),
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
}
