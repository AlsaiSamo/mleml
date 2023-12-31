//! Builder for configurations, represented as flat [JSON arrays][crate::resource::JsonArray],
//! that uses a schema.

use std::mem::{discriminant, Discriminant};

use thiserror::Error;

use crate::resource::{JsonValue, ResConfig};

/// Errors that [`ConfigBuilder`] can produce.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigBuilderError {
    //TODO: change from displaying discriminant to displaying a type
    /// Provided type does not match the type defined in the schema.
    #[error("type mismatch at {0}: expected {1:?}, got {2:?}")]
    TypeMismatch(usize, Discriminant<JsonValue>, Discriminant<JsonValue>),

    /// Extra value is supplied to a configuration that is already fully built.
    #[error("value outside schema")]
    ValueOutsideSchema,
}

/// State of [`ConfigBuilder`] in which the config is not fully built yet.
#[derive(Debug)]
pub struct ConfBuilding<'a> {
    /// Schema against which the configuration is being built.
    schema: &'a ResConfig,

    /// Configuration that is being built.
    config: ResConfig,
}

/// Configuration builder.
///
/// Validates all provided values and their count against the schema, making sure
/// that the types match.
#[derive(Debug)]
pub enum ConfigBuilder<'a> {
    /// Configuration is still being built.
    Builder(ConfBuilding<'a>),

    /// Configuration is fully built and can be used.
    Config(ResConfig),
}

impl<'a> ConfigBuilder<'a> {
    /// Create new [`ConfigBuilder`] from given schema.
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

    /// Append items from a given source of JSON values to the configuration that is being built
    /// and returns the number of appended values.
    ///
    /// The function finishes when the configuration is finished building, all items
    /// were used, or an error occurs.
    ///
    /// # Errors
    ///
    /// If the configuration had already been built,
    /// [`ValueOutsideSchema`][crate::extra::config_builder::ConfigBuilderError::ValueOutsideSchema]
    /// is returned.
    ///
    /// If an item from the source has an incorrect type,
    /// [`TypeMismatch`][crate::extra::config_builder::ConfigBuilderError::TypeMismatch]
    /// is returned. Everything before this item will remain in the configuration.
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::{json, Value};
    /// # use mleml::extra::config_builder::{ConfigBuilder, ConfigBuilderError};
    /// # use mleml::resource::{ResConfig, JsonArray};
    /// # fn main() -> Result<(), ConfigBuilderError> {
    /// let schema: ResConfig = ResConfig::from_value(json!([5, "six"])).expect("failed to create resource config");
    /// let mut builder: ConfigBuilder = ConfigBuilder::new(&schema);
    /// let source: JsonArray = JsonArray::from_value(json!([12, "lime"])).expect("failed to create JSON array");
    ///
    /// // Number of values that were taken from the source
    /// let taken: usize = builder.inject(source.as_slice())?;
    /// assert_eq!(taken, 2);
    ///
    /// // Finished config is taken from the builder
    /// let config: ResConfig = match builder {
    ///     ConfigBuilder::Builder(..) => unreachable!(),
    ///     ConfigBuilder::Config(conf) => conf
    /// };
    ///
    /// # Ok(())
    /// # }
    /// ```
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
                        //TODO: figure out if this is expensive
                        true => *self = ConfigBuilder::Config(build.config.to_owned()),
                        false => continue,
                    }
                }
            }
        }
        Ok(count)
    }

    /// If the configuration is unfinished, checks and appends one item to it.
    /// `Ok(true)` means that the config is fully built.
    ///
    /// # Errors
    ///
    /// If the configuration had already been built,
    /// [`ValueOutsideSchema`][crate::extra::config_builder::ConfigBuilderError::ValueOutsideSchema]
    /// is returned.
    ///
    /// If the inserted item has an incorrect type,
    /// [`TypeMismatch`][crate::extra::config_builder::ConfigBuilderError::TypeMismatch]
    /// is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::{json, Value};
    /// # use mleml::extra::config_builder::{ConfigBuilder, ConfigBuilderError};
    /// # use mleml::resource::{ResConfig, JsonArray};
    /// # fn main() -> Result<(), ConfigBuilderError> {
    /// let schema: ResConfig = ResConfig::from_value(json!([5, "six"])).expect("failed to create resource config");
    /// let mut builder: ConfigBuilder = ConfigBuilder::new(&schema);
    /// let number: Value = json!(31);
    /// let string: Value = json!("many");
    ///
    /// let is_finished: bool = builder.append(&number)?;
    /// assert_eq!(is_finished, false);
    /// let is_finished: bool = builder.append(&string)?;
    /// assert_eq!(is_finished, true);
    /// # Ok(())
    /// # }
    /// ```
    pub fn append(&mut self, value: &JsonValue) -> Result<bool, ConfigBuilderError> {
        match self {
            ConfigBuilder::Builder(builder) => builder.append(value),
            ConfigBuilder::Config(_) => Err(ConfigBuilderError::ValueOutsideSchema),
        }
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::resource::JsonArray;

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
    fn append_to_config_builder_works() {
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
    fn append_to_config_builder_extra() {
        let schema = example_json_array();
        let mut conf_builder = ConfigBuilder::new(&schema);

        //Correct type is Number, and this is not the last element
        assert!(conf_builder.append(&json!(30.3)).is_ok_and(|x| !x));
        //Correct type is String, and this is not the last element
        assert!(conf_builder.append(&json!("Very silent")).is_ok_and(|x| !x));
        //Correct type is Bool, and this is the last element of the config
        assert!(conf_builder.append(&json!(false)).is_ok_and(|x| x));
        assert!(conf_builder
            .append(&json!("extra"))
            .is_err_and(|x| x == ConfigBuilderError::ValueOutsideSchema));
    }

    #[test]
    fn append_to_config_building_type_mismatch() {
        let schema = example_json_array();
        let mut conf_builder = ConfigBuilder::new(&schema);

        let given_disc = discriminant(&json!("a"));
        let expected_disc = discriminant(&json!(8));
        assert!(conf_builder
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
