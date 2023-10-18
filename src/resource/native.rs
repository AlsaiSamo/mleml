//! Resources that are built into the library.

use std::borrow::Cow;
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
        //compare_json_array(&self.schema, conf).ok_or
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

pub struct SimplePlatform<'msg> {
    name: String,
    id: String,
    schema: ResConfig,
    values: PlatformValues,
    description: String,
    mix: fn(&[Sound], &ResConfig, &ResState) -> Result<(Sound, Box<ResState>), Cow<'msg, str>>,
}

impl<'msg> SimplePlatform<'msg> {
    pub fn new(
        name: String,
        id: String,
        schema: ResConfig,
        values: PlatformValues,
        description: String,
        mix: fn(&[Sound], &ResConfig, &ResState) -> Result<(Sound, Box<ResState>), Cow<'msg, str>>,
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

impl<'msg> Resource for SimplePlatform<'msg> {
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

impl<'msg> Platform<'msg> for SimplePlatform<'msg> {
    fn get_vals(&self) -> PlatformValues {
        self.values.clone()
    }

    fn mix(
        &self,
        channels: &[Sound],
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>), Cow<'msg, str>> {
        (self.mix)(channels, conf, state)
    }

    fn description(&self) -> String {
        todo!()
    }
}
