#![warn(missing_docs)]
//!Configurable, stateful providers of pure functions.
//!
//!Configuration is done via flat JSON array.
//!
//!Two types of resources are currently provided: `Mod<'msg, I, O>` transforms
//!`I` into `O` (used to create note -> sound pipeline), while `Platform` provides
//!constraints, sound mixing, and so on.

//mod ext;
mod native;

use crate::types::{Note, ReadyNote, Sound};
use core::fmt;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_vec};
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    mem::{discriminant, Discriminant},
    rc::Rc,
};

type JsonValue = serde_json::Value;


///Flat JSON array of arbitrary values.
#[derive(Clone, Serialize, Deserialize)]
pub struct JsonArray(JsonValue);

impl JsonArray {
    ///Create new, empty JSON array.
    fn new() -> Self {
        Self { 0: json!([]) }
    }

    ///Get elements in a slice.
    fn as_slice(&self) -> &[JsonValue] {
        self.0.as_array().unwrap().as_slice()
    }

    ///Serialize into byte vector.
    fn as_byte_vec(&self) -> Vec<u8> {
        to_vec(&self.0).unwrap()
    }

    ///Push item into the array as long as the item is not an array or a map.
    fn push(&mut self, item: JsonValue) -> Option<()> {
        match item.is_array() | item.is_object() {
            true => None,
            _ => {
                self.0.as_array_mut().unwrap().push(item);
                return Some(());
            }
        }
    }
}

impl Hash for JsonArray {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_byte_vec().hash(state);
    }
}

type ResConfig = JsonArray;

///Error encountered while building configuration.
#[derive(Eq, PartialEq, Debug)]
pub enum ConfigBuilderError {
    ///Provided type does not match the type defined in the schema.
    TypeMismatch(usize, Discriminant<JsonValue>, Discriminant<JsonValue>),

    ///Configuration's length matches the schema's, so the provided value
    ///cannot fit.
    ValueOutsideSchema,
}

impl fmt::Display for ConfigBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TypeMismatch(l, e, g) => {
                write!(f, "Type mismatch at {l}: expected {:?}, got {:?}", e, g)
            }
            Self::ValueOutsideSchema => {
                write!(f, "Value is not needed as the config is comlete already")
            }
        }
    }
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
        if schema.as_slice().len() == 0 {
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
    ///If it is shorter instead, the builder will remain a builder.
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
                        //I think this will not be time consuming
                        true => *self = ConfigBuilder::Config(build.config.to_owned()),
                        false => {}
                    }
                }
            }
        }
        return Ok(count);
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
        //where T: AsRef<JsonValue> + Clone
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

type ResState = [u8];

///Configuration error.
#[derive(Eq, PartialEq)]
pub enum ConfigError {
    ///Unexpected type of value.
    //TODO: discriminant's debug output is Discriminant(int). Replace with something else.
    BadValue(u32, Discriminant<JsonValue>, Discriminant<JsonValue>),

    ///Incorrect length.
    BadLength(u32, u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadValue(l, e, g) => {
                write!(f, "Type mismatch at {l}: expected {:?}, got {:?}", e, g)
            }
            Self::BadLength(e, g) => {
                write!(f, "Length mismatch: expected {}, got {}", e, g)
            }
        }
    }
}

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
    fn check_config(&self, conf: &ResConfig) -> Result<(), Cow<'_, str>>;

    ///Verify that the given state can be used by the resource.
    fn check_state(&self, state: &ResState) -> Option<()>;

    //fn get_config_schema(&self) -> &ResConfig;
}

impl Hash for dyn Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

///Values provided by the platform, such as maixmum channel count,
///frequency of C-1, acceptable volume range, and more.
///
///The platform may, for example, allow more channels, if configured to do so.
#[derive(Clone)]
#[repr(C)]
pub struct PlatformValues {
    ///Frequency of C-1. All other note frequencies are derived from it.
    ///
    ///For reference, A440 standard makes C-1 equal to 8.175799.
    pub cccc: f32,

    ///Length of one tick in seconds.
    pub tick_len: f32,

    ///Length of a whole note in ticks.
    pub zenlen: u32,

    ///Number of ticks per beat.
    pub tempo: f32,

    ///What number denotes maximum volume setting.
    pub max_volume: u32,

    ///Maximum number of sound-producing channels.
    pub channels: u32,
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
pub trait Platform<'msg>: Resource {
    ///Get platform values.
    fn get_vals(&self) -> PlatformValues;

    ///Mix provided sound samples.
    ///
    ///Sound samples are expected to come in the same order the channels that
    ///have produced them do, and their number must match the number of channels
    ///expected by the platform.
    fn mix(
        &self,
        channels: &[Sound],
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(Sound, Box<ResState>), Cow<'msg, str>>;

    //TODO: move this to Resource?
    ///Get platform's description.
    ///
    ///This may be used to provide any message, for example, the order of channels,
    ///and how they are going to be mixed.
    fn description(&self) -> String;
}

///A resource that is used in data transformations.
///
///Currently, it is used to define note and sound modifiers and instruments,
///which convert from notes to sounds.
pub trait Mod<'msg, I, O>: Resource {

    ///Pure transformation function.
    fn apply(
        &self,
        input: &I,
        conf: &ResConfig,
        state: &ResState,
    ) -> Result<(O, Box<ResState>), Cow<'msg, str>>;
}

///Mod, along with its configuration and state bundled together for ease of use.
///
///This is a stub version of resource/config/state storage. It is likely to be
///replaced in the future.
#[derive(Clone)]
pub struct ResLump<I, O> {
    #[allow(missing_docs)]
    pub module: Rc<dyn for<'a> Mod<'a, I, O>>,
    #[allow(missing_docs)]
    pub conf: Rc<ResConfig>,
    #[allow(missing_docs)]
    pub state: Rc<[u8]>,
}

impl<'msg, I, O> ResLump<I, O> {
    ///Use mod's apply() with bundled state and config.
    pub fn apply(&self, input: I) -> Result<(O, Box<ResState>), Cow<'msg, str>> {
        self.module.apply(&input, &self.conf, &self.state)
    }
}

#[allow(missing_docs)]
pub type NoteModLump = ResLump<Note, Note>;
#[allow(missing_docs)]
pub type SoundModLump = ResLump<Sound, Sound>;
#[allow(missing_docs)]
pub type InstrumentLump = ResLump<ReadyNote, Sound>;

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
    //TODO: external mod/platform tests
}
