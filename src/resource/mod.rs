#![warn(missing_docs)]
//!Resources provide stateless functions that can be configured.
//!
//!Configuration is done via flat JSON array. Function's state between calls
//! is given, received and stored by the program, so that the function can be pure.
//!
//!Two types of resources are currently provided: `Mod<'msg, I, O>` transforms
//!`I` into `O` (used to create note -> sound pipeline), while `Platform` provides
//!constraints and sound mixing.
//!
//TODO: explain what modifiers can be used for what (transposing and key signature in
// note mods, panning, LFO and other things in sound mods)

//TODO: use rc_slice2 crate? It allows creating subslices which can also be Rc's,
// which would probably simplify platform mixer.

use crate::types::{Note, ReadyNote, Sound};
use dasp::frame::Stereo;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_vec};
use thiserror::Error;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    mem::{discriminant, Discriminant},
    rc::Rc,
};


type JsonValue = serde_json::Value;


///Flat JSON array of arbitrary values.
///
///Array's flatness makes it much easier to parse.
#[derive(Clone, Serialize, Deserialize)]
pub struct JsonArray(JsonValue);

impl Default for JsonArray {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonArray {
    ///Create new, empty JSON array.
    pub fn new() -> Self {
        Self { 0: json!([]) }
    }

    ///Convert vector of JSON values into JSON array, as long as no value is an array
    /// or an object.
    pub fn from_vec(items: Vec<JsonValue>) -> Option<Self> {
        match items.iter().any(|x| !(x.is_array() | x.is_object())) {
            true => Some(Self(items.into())),
            false => None
        }
    }

    /// Provides a reference to the inner value.
    pub fn get(&self) -> &JsonValue {
        &self.0
    }

    ///Get elements of the array as a slice.
    pub fn as_slice(&self) -> &[JsonValue] {
        self.0.as_array().unwrap().as_slice()
    }

    ///Get array's length.
    pub fn len(&self) -> usize {
        self.0.as_array().unwrap().len()
    }

    ///Serialize into byte vector.
    fn as_byte_vec(&self) -> Vec<u8> {
        to_vec(&self.0).unwrap()
    }

    ///Push item into the array as long as the item is not an array or an object.
    fn push(&mut self, item: JsonValue) -> Option<()> {
        match item.is_array() | item.is_object() {
            true => None,
            _ => {
                self.0.as_array_mut().unwrap().push(item);
                Some(())
            }
        }
    }
}

impl Hash for JsonArray {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_byte_vec().hash(state);
    }
}

///Resource's configuration.
pub type ResConfig = JsonArray;

///Error encountered while building configuration.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigBuilderError {
    ///Provided type does not match the type defined in the schema.
    #[error("type mismatch at {0}: expected {1:?}, got {2:?}")]
    TypeMismatch(usize, Discriminant<JsonValue>, Discriminant<JsonValue>),

    ///Extra value is supplied to configuration that is already fully built.
    #[error("value outside schema")]
    ValueOutsideSchema,
}

///Unfinished configuration builder (not all values were provided).
pub struct ConfBuilding<'a> {
    ///Schema of the module.
    schema: &'a ResConfig,

    ///Configuration that is being built.
    config: ResConfig,
}

///Configuration builder.
///
///Validates all provided values and their count against the schema.
pub enum ConfigBuilder<'a> {
    ///Config is unfinished.
    Builder(ConfBuilding<'a>),

    ///Config is finished and can be used.
    Config(ResConfig),
}

impl<'a> ConfigBuilder<'a> {
    ///Create new config builder from given schema.
    pub fn new(schema: &'a ResConfig) -> ConfigBuilder {
        if schema.as_slice().is_empty() {
            return ConfigBuilder::Config(ResConfig::new());
        } else {
            return ConfigBuilder::Builder(ConfBuilding {
                schema,
                config: ResConfig::new(),
            });
        }
    }

    ///Append items from given iterable until configuration is built,
    ///all items were appended, or an error occurs.
    ///
    ///If iterable is longer than what is needed, extra values will be unused.
    ///If it is shorter instead, the builder will continue to be a builder.
    //TODO: current approach silently discards values that did not fit but
    //returns error on attempt to append to a finished config.
    //SHould it return error on extra values always? Or should it return Ok(0)?
    pub fn inject<T>(&mut self, values: T) -> Result<usize, ConfigBuilderError>
        where T: AsRef<[JsonValue]>,
    {
        if let ConfigBuilder::Config(_) = self {
            return Err(ConfigBuilderError::ValueOutsideSchema)
        }
        let mut values = values.as_ref().iter();
        let mut count = 0;
        while let ConfigBuilder::Builder(build) = self {
            let val = values.next();
            match val.is_none() {
                true => return Ok(count),
                false => {
                    count += 1;
                    match build.append(val.unwrap())? {
                        //While this is a copy, it should not hurt too much.
                        //TODO: see if this can be improved
                        true => *self = ConfigBuilder::Config(build.config.to_owned()),
                        false => {}
                    }
                }
            }
        }
        Ok(count)
    }

    /// Returns `true` if the config builder is [`Builder`].
    ///
    /// [`Builder`]: ConfigBuilder::Builder
    #[must_use]
    pub fn is_builder(&self) -> bool {
        matches!(self, Self::Builder(..))
    }

    /// Returns `true` if the config builder is [`Config`].
    ///
    /// [`Config`]: ConfigBuilder::Config
    #[must_use]
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config(..))
    }
}

impl<'a> ConfBuilding<'a> {
    ///Checks and appends one item to the unfinished configuration. Ok(true)
    ///signals that the config is full.
    fn append(&mut self, value: &JsonValue) -> Result<bool, ConfigBuilderError>
    {
        if self.schema.as_slice().len() == self.config.as_slice().len() {
            return Err(ConfigBuilderError::ValueOutsideSchema);
        }
        let position = self.config.as_slice().len();
        let current_type = discriminant(&self.schema.as_slice()[position]);
        let given_type = discriminant(value);
        if current_type != given_type {
            return Err(ConfigBuilderError::TypeMismatch(
                position,
                current_type,
                given_type,
            ));
        };
        self.config.push(value.clone()).unwrap();
        if position == self.schema.as_slice().len() - 1 {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

///Resource's state.
///
///Data inside of the state is opaque to everyone except its user.
pub type ResState = [u8];

///Configuration error.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum ConfigError {
    ///Unexpected type of value.
    //TODO: discriminant's debug output is Discriminant(int). Replace with something else.
    #[error("type mismatch at {0}: expected {1:?}, got {2:?}")]
    BadValue(u32, Discriminant<JsonValue>, Discriminant<JsonValue>),

    ///Incorrect length.
    #[error("length mismatch: expected {0}, got {1}")]
    BadLength(u32, u32),
}

//TODO: use Cow?
/// Arbitrary error message for resources.
#[derive(Error, Debug)]
#[error("resource error: {0}")]
pub struct StringError(pub String);

///Base trait for any resource.
///
///Resources of any type need to conform to these constraints:
/// 1. Provide unique ID
/// 2. Only provide pure functions (state is given and returned when needed)
///
///Resources are also named, but association of names to IDs needs to be done
///externally.
pub trait Resource {
    ///Resource's original name.
    fn orig_name(&self) -> Option<Cow<'_, str>>;

    ///Unique ID of the resource.
    fn id(&self) -> &str;

    ///Verify that the given config can be used by the resource.
    fn check_config(&self, conf: &ResConfig) -> Result<(), StringError>;

    ///Verify that the given state can be used by the resource.
    fn check_state(&self, state: &ResState) -> Option<()>;

    ///Get resource's description.
    fn description(&self) -> &str;
}

impl Hash for dyn Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

///Resource that defines the quirks and limitations of a given platform
///(usually a sound chip).
///
///As an example of a platform quirk, PMDMML plays SSG drums, defined in K channel,
/// on I channel, which means that it is not possible to just mix sound.
///
///It should be noted that platform cannot influence what selection of modules
///is used for any of the channels, or their order. Please be careful when
///mimicking output of a sound chip.
pub trait Platform<'a>: Resource {
    ///Get platform values.
    fn get_config(&self) -> ResConfig;

    ///Mix provided sound samples.
    ///
    ///Sound samples are expected to come in the same order the channels that
    ///have produced them do, and their number must match the number of channels
    ///expected by the platform.
    fn mix(
        &self,
        channels: &[(bool, &'a [Stereo<f32>])],
        play_time: u32,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>, Box<[Option<&'a [Stereo<f32>]>]>),StringError>;
}

///Types that the mods can process.
pub enum ModData {
    String(String),
    Note(Note),
    ReadyNote(ReadyNote),
    Sound(Sound)
}

impl ModData {
    /// Returns `true` if the mod data is [`String`].
    ///
    /// [`String`]: ModData::String
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }

    /// Returns `true` if the mod data is [`Note`].
    ///
    /// [`Note`]: ModData::Note
    #[must_use]
    pub fn is_note(&self) -> bool {
        matches!(self, Self::Note(..))
    }

    /// Returns `true` if the mod data is [`ReadyNote`].
    ///
    /// [`ReadyNote`]: ModData::ReadyNote
    #[must_use]
    pub fn is_ready_note(&self) -> bool {
        matches!(self, Self::ReadyNote(..))
    }

    /// Returns `true` if the mod data is [`Sound`].
    ///
    /// [`Sound`]: ModData::Sound
    #[must_use]
    pub fn is_sound(&self) -> bool {
        matches!(self, Self::Sound(..))
    }

    pub fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_note(&self) -> Option<&Note> {
        if let Self::Note(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_ready_note(&self) -> Option<&ReadyNote> {
        if let Self::ReadyNote(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_sound(&self) -> Option<&Sound> {
        if let Self::Sound(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// A resource that is used in data transformations.
pub trait Mod: Resource {

    ///Pure transformation function.
    fn apply(
        &self,
        input: &ModData,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(ModData, Box<ResState>), StringError>;

    ///Discriminant of type that this mod expects to receive.
    fn input_type(&self) -> Discriminant<ModData>;

    ///Discriminant of type that this mod will produce.
    fn output_type(&self) -> Discriminant<ModData>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_json_array() -> JsonArray {
        let mut arr = JsonArray::new();
        arr.push(json!(22.5)).unwrap();
        arr.push(json!("precacious")).unwrap();
        arr.push(json!(true)).unwrap();
        arr
    }

    #[test]
    fn json_array_good_types_are_pushed() {
        let mut arr = JsonArray::new();
        arr.push(json!(22.5)).unwrap();
        arr.push(json!("precacious")).unwrap();
        arr.push(json!(true)).unwrap();
    }

    #[test]
    fn json_array_push_keeps_flatness() {
        let mut arr = example_json_array();
        assert!(arr.push(json!([])).is_none());
        assert!(arr.push(json!({"a": true})).is_none());
    }

    #[test]
    fn json_array_as_slice() {
        let arr = example_json_array();
        let sliced = [json!(22.5), json!("precacious"), json!(true)];
        assert_eq!(arr.as_slice(), &sliced)
    }

    #[test]
    fn json_array_as_byte_vec() {
        let arr = example_json_array();
        assert_eq!(arr.as_byte_vec(), r#"[22.5,"precacious",true]"#.as_bytes());
    }

    #[test]
    fn config_builder_returns_empty_config_on_empty_schema() {
        let schema = JsonArray::new();
        let builder = ConfigBuilder::new(&schema);
        if let ConfigBuilder::Builder(_) = builder {
            panic!("Schema is empty but the builder did not immediately return")
        }
    }

    #[test]
    fn append_to_config_building_works() {
        let schema = example_json_array();
        let mut conf_building = ConfBuilding {
            schema: &schema,
            config: JsonArray::new(),
        };
        //Correct type is Number, and this is not the last element
        assert!(conf_building.append(&json!(30.3)).is_ok_and(|x| !x));
        //Correct type is String, and this is not the last element
        assert!(conf_building.append(&json!("Very silent")).is_ok_and(|x| !x));
        //Correct type is Bool, and this is the last element of the config
        assert!(conf_building.append(&json!(false)).is_ok_and(|x| x));
    }

    #[test]
    fn append_to_config_building_extra() {
        let schema = example_json_array();
        let mut conf_building = ConfBuilding {
            schema: &schema,
            config: JsonArray::new(),
        };
        //Correct type is Number, and this is not the last element
        assert!(conf_building.append(&json!(30.3)).is_ok_and(|x| !x));
        //Correct type is String, and this is not the last element
        assert!(conf_building.append(&json!("Very silent")).is_ok_and(|x| !x));
        //Correct type is Bool, and this is the last element of the config
        assert!(conf_building.append(&json!(false)).is_ok_and(|x| x));
        assert!(conf_building
            .append(&json!("extra"))
            .is_err_and(|x| x == ConfigBuilderError::ValueOutsideSchema));
    }

    #[test]
    fn append_to_config_building_type_mismatch() {
        let schema = example_json_array();
        let mut conf_building = ConfBuilding {
            schema: &schema,
            config: JsonArray::new(),
        };
        let given_disc = discriminant(&json!("a"));
        let expected_disc = discriminant(&json!(8));
        assert!(conf_building
            .append(&json!("teehee"))
            .is_err_and(|x| x == ConfigBuilderError::TypeMismatch(0, expected_disc, given_disc)));
    }

    #[test]
    fn config_builder_inject_typical_use() {
        let schema = example_json_array();
        let mut conf_build = ConfigBuilder::new(&schema);
        let items = vec![json!(2500), json!("merged"), json!(false)];

        match conf_build.inject(items) {
            Ok(count) => {
                //Count has to be three because 3 items were inserted
                assert_eq!(count, 3);
                //Builder has to be finished
                assert!(conf_build.is_config())
            }
            //Provided items match the schema, so Err(_) is impossible
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn config_builder_inject_longer() {
        let schema = example_json_array();
        let mut conf_build = ConfigBuilder::new(&schema);
        //There are more items than needed
        let items = vec![json!(2500), json!("merged"), json!(false), json!("extra")];

        match conf_build.inject(items) {
            Ok(count) => {
                //Count has to be three because 3 items were inserted
                assert_eq!(count, 3);
                //Builder has to be finished
                assert!(conf_build.is_config())
            }
            //Provided items match the schema (the last one is dropped), so Err(_) is impossible
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn config_builder_inject_two_small() {
        let schema = example_json_array();
        let mut conf_build = ConfigBuilder::new(&schema);
        //Both vectors are smaller than schema
        let it1 = vec![json!(2500), json!("merged")];
        let it2 = vec![json!(false), json!("extra")];

        match conf_build.inject(it1) {
            Ok(count) => {
                assert_eq!(count, 2);
                //Builder has to be unfinished
                assert!(conf_build.is_builder())
            }
            Err(_) => unreachable!(),
        }

        match conf_build.inject(it2) {
            Ok(count) => {
                //Schema is of length 3 and two items were inserted earlier, only one
                //needs to be taken.
                assert_eq!(count, 1);
                assert!(conf_build.is_config())
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn config_builder_inject_wrong() {
        let schema = example_json_array();
        let mut conf_build = ConfigBuilder::new(&schema);
        //Second value is not a string
        let items = vec![json!(7), json!(0xF00F), json!(false)];
        let given_disc = discriminant(&json!(0xF00F));
        let expected_disc = discriminant(&json!("bee"));

        match conf_build.inject(items) {
            Ok(_) => panic!("config builder created a config that does not match the schema"),
            //Other test proves that extra values will not be accepted,
            //eliminating ValueOutsideSchema possibility.
            Err(e) => {
                assert_eq!(e, ConfigBuilderError::TypeMismatch(1, expected_disc, given_disc));
            }
        }
    }

    #[test]
    fn config_builder_inject_into_full() {
        let schema = example_json_array();
        let mut conf_build = ConfigBuilder::new(&schema);
        let it1 = vec![json!(2500), json!("merged"), json!(false)];
        let it2 = vec![json!("extra")];

        //Other test proves that this does not panic.
        conf_build.inject(it1).unwrap();
        match conf_build.inject(it2) {
            Ok(_) => panic!("config builder accepted a value that does not fit into the schema"),
            Err(e) => assert_eq!(e, ConfigBuilderError::ValueOutsideSchema),
        }
    }
}
