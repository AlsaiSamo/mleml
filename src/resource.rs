#![warn(missing_docs)]
//! This module provides Mod and Mixer traits.
//TODO: check notes.org

use crate::types::{Note, ReadyNote, Sound};
use dasp::frame::Stereo;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_vec};
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    mem::{discriminant, Discriminant},
};
use thiserror::Error;

type JsonValue = serde_json::Value;

///Flat JSON array of arbitrary values.
#[derive(Clone, Serialize, Deserialize)]
pub struct JsonArray(JsonValue);

impl Default for JsonArray {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonArray {
    //TODO: as_mut_slice?

    /// Create new JSON array.
    pub fn new() -> Self {
        Self(json!([]))
    }

    /// Convert vector of JSON values into JSON array, as long as no value is an array
    /// or an object.
    pub fn from_vec(items: Vec<JsonValue>) -> Option<Self> {
        match items.iter().any(|x| !(x.is_array() | x.is_object())) {
            true => Some(Self(items.into())),
            false => None,
        }
    }

    /// Wrap JSON value as JsonArray as long as it is an array with no nested arrays
    /// or objects.
    pub fn from_value(item: JsonValue) -> Option<Self> {
        match item
            .as_array()?
            .iter()
            .any(|x| !(x.is_array() | x.is_object()))
        {
            true => Some(Self(item)),
            false => None,
        }
    }

    /// Returns a reference to the inner JSON value.
    pub fn get(&self) -> &JsonValue {
        &self.0
    }

    /// Returns a mutable reference to the inner JSON value.
    pub fn get_mut(&mut self) -> &mut JsonValue {
        &mut self.0
    }

    /// Returns a slice of contained JSON values.
    pub fn as_slice(&self) -> &[JsonValue] {
        self.0.as_array().unwrap().as_slice()
    }

    /// Get array's length.
    pub fn len(&self) -> usize {
        self.0.as_array().unwrap().len()
    }

    /// Check if the array is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Serialize into byte vector.
    fn as_byte_vec(&self) -> Vec<u8> {
        to_vec(&self.0).unwrap()
    }

    /// Push item into the array as long as the item is not an array or an object.
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

/// Configuration of a resource
pub type ResConfig = JsonArray;

/// Errors that can be encountered while building configuration.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigBuilderError {
    /// Provided type does not match the type defined in the schema.
    #[error("type mismatch at {0}: expected {1:?}, got {2:?}")]
    TypeMismatch(usize, Discriminant<JsonValue>, Discriminant<JsonValue>),

    /// Extra value is supplied to a configuration that is already fully built.
    #[error("value outside schema")]
    ValueOutsideSchema,
}

/// State of configuration builder in which the config is not fully built yet.
pub struct ConfBuilding<'a> {
    ///Schema of the module.
    schema: &'a ResConfig,

    ///Configuration that is being built.
    config: ResConfig,
}

/// Configuration builder.
///
/// Validates all provided values and their count against the schema, comparing
/// the types.
pub enum ConfigBuilder<'a> {
    /// Configuration is still building.
    Builder(ConfBuilding<'a>),

    /// Configuration is fully built and can be used.
    Config(ResConfig),
}

impl<'a> ConfigBuilder<'a> {
    /// Create new config builder from given schema.
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

    /// Append items from a given source to the configuration that is being built.
    ///
    /// The function finishes when the configuration is finished building, all items
    /// were used, or an error occurs.
    //TODO: current approach silently discards values that did not fit but
    //returns error on attempt to append to a finished config.
    //Should it return error on extra values always? Or should it return Ok(0)?
    pub fn inject<T>(&mut self, values: T) -> Result<usize, ConfigBuilderError>
    where
        T: AsRef<[JsonValue]>,
    {
        if let ConfigBuilder::Config(_) = self {
            return Err(ConfigBuilderError::ValueOutsideSchema);
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
    /// Checks and appends one item to the unfinished configuration. Ok(true)
    /// signals that the config is full.
    fn append(&mut self, value: &JsonValue) -> Result<bool, ConfigBuilderError> {
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

/// Resource's state.
///
/// Data inside of the state is opaque.
pub type ResState = [u8];

/// Configuration error.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// A value has an unexpected type.
    //TODO: discriminant's debug output is Discriminant(int). Replace with something else.
    #[error("type mismatch at {0}: expected {1:?}, got {2:?}")]
    BadValue(u32, Discriminant<JsonValue>, Discriminant<JsonValue>),

    /// Configuration has incorrect length.
    #[error("length mismatch: expected {0}, got {1}")]
    BadLength(u32, u32),
}

//TODO: use Cow? Would this be significant?
/// Arbitrary error message for resources.
#[derive(Error, Debug)]
#[error("resource error: {0}")]
pub struct StringError(pub String);

/// Base trait for any resource.
//TODO: change description to be like the name?
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

/// Type to hold unused bits of sound.
pub type LeftoverSound<'a> = Box<[Option<&'a [Stereo<f32>]>]>;

/// Input type for the mixer.
///
/// Each sound has a flag to indicate whether it is a new sound or not.
pub type PremixedSound<'a> = &'a [(bool, &'a [Stereo<f32>])];

/// Mixer combines multiple sounds into one, returning it together with unused sound pieces.
pub trait Mixer<'a>: Resource {
    /// Get mixer values.
    fn get_config(&self) -> ResConfig;

    /// Mix provided sound samples.
    ///
    /// It is expected that the leftover sound bits from before are not shuffled around,
    /// as the mixer may depend on their position.
    fn mix(
        &self,
        channels: PremixedSound<'a>,
        play_time: u32,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>, LeftoverSound<'a>), StringError>;
}

/// Types that the mods can process.
pub enum ModData {
    /// String
    String(String),

    /// Note
    Note(Note),

    /// ReadyNote
    ReadyNote(ReadyNote),

    /// Sound
    Sound(Sound),
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

    //TODO: write docstrings (not a priority)
    #[allow(missing_docs)]
    pub fn as_string(&self) -> Option<&String> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[allow(missing_docs)]
    pub fn as_note(&self) -> Option<&Note> {
        if let Self::Note(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[allow(missing_docs)]
    pub fn as_ready_note(&self) -> Option<&ReadyNote> {
        if let Self::ReadyNote(v) = self {
            Some(v)
        } else {
            None
        }
    }

    #[allow(missing_docs)]
    pub fn as_sound(&self) -> Option<&Sound> {
        if let Self::Sound(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Mods are used to produce new data from given data.
pub trait Mod: Resource {
    /// Apply mod to data.
    fn apply(
        &self,
        input: &ModData,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(ModData, Box<ResState>), StringError>;

    /// Discriminant of type that this mod expects to receive.
    fn input_type(&self) -> Discriminant<ModData>;

    /// Discriminant of type that this mod will produce.
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
        assert!(conf_building
            .append(&json!("Very silent"))
            .is_ok_and(|x| !x));
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
        assert!(conf_building
            .append(&json!("Very silent"))
            .is_ok_and(|x| !x));
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
                assert_eq!(
                    e,
                    ConfigBuilderError::TypeMismatch(1, expected_disc, given_disc)
                );
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
