use std::{mem::{Discriminant, discriminant}, borrow::Cow};

use crate::resource::{ResConfig, ModData, ResState, StringError, Resource, JsonArray, Mod};

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

fn json_array_find_deviation(reference: &JsonArray, given: &JsonArray) -> Option<usize> {
    (0..given.len())
        .find(|&i| discriminant(&reference.as_slice()[i]) != discriminant(&given.as_slice()[i]))
}
