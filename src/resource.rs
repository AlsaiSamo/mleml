//! This module provides Mod and Mixer traits.

use crate::types::{Note, ReadyNote, Sound};
use dasp::frame::Stereo;
use sealed::sealed;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_vec};
use std::{
    hash::{Hash, Hasher},
    mem::Discriminant,
    rc::Rc,
};
use thiserror::Error;

pub(crate) type JsonValue = serde_json::Value;

///Flat JSON array of arbitrary values.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(transparent)]
pub struct JsonArray(JsonValue);

impl Default for JsonArray {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonArray {
    /// Construct a new, empty JSON array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use mleml::resource::JsonArray;
    /// let mut conf: JsonArray = JsonArray::new();
    /// ```
    pub fn new() -> Self {
        Self(json!([]))
    }

    /// Convert an ordered collection of JSON values into JSON array, as long as no value is an array
    /// or an object.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// let vec: Vec<Value> = vec![json!(5), json!("six")];
    /// let conf: JsonArray = JsonArray::from_values(vec).expect("Vector contains an array or an object");
    /// ```
    pub fn from_values<I: AsRef<[JsonValue]>>(items: I) -> Option<Self> {
        match items
            .as_ref()
            .iter()
            .any(|x| !(x.is_array() | x.is_object()))
        {
            true => Some(Self(items.as_ref().into())),
            false => None,
        }
    }

    /// Wrap JSON value as JsonArray as long as it is an array with no nested arrays
    /// or objects.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// let val: Value = json!([5, "six"]);
    /// let conf: JsonArray = JsonArray::from_value(val).expect("JSON value was not a flat array");
    /// ```
    //TODO: accept borrowed and to_owned() them
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

    /// Serialize into [`Vec<u8>`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::{json, Value};
    /// # use mleml::resource::JsonArray;
    /// let conf: JsonArray = JsonArray::from_value(json!([5, "six"])).
    /// expect("JSON value was not a flat array");
    /// assert_eq!(conf.as_byte_vec(), r#"[5,"six"]"#.as_bytes())
    /// ```
    pub fn as_byte_vec(&self) -> Vec<u8> {
        to_vec(&self.0).unwrap()
    }

    /// Push `item` into the array as long as the item is not
    /// an [`Array`][serde_json::Value::Array] or an [`Object`][serde_json::Value::Object] and
    /// returns `Some(())` to indicate success, or `None` to indicate failure.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_json::json;
    /// # use mleml::resource::JsonArray;
    /// let mut conf: JsonArray = JsonArray::new();
    /// conf.push(json!(5)).expect("Somehow 5 was not pushed in");
    /// assert_eq!(conf.as_slice()[0].as_i64(), Some(5));
    /// ```
    pub fn push(&mut self, item: JsonValue) -> Option<()> {
        match item.is_array() | item.is_object() {
            true => None,
            false => {
                self.0.as_array_mut().unwrap().push(item);
                Some(())
            }
        }
    }

    /// Calls [`Vec::pop()`].
    pub fn pop(&mut self) -> Option<JsonValue> {
        self.0.as_array_mut().unwrap().pop()
    }

    /// Checks that `element` is not [`Array`][serde_json::Value::Array] or
    /// [`Object`][serde_json::Value::Object] and calls [`Vec::insert()`],
    /// returning `Some(())`, otherwise returns `None`.
    pub fn insert(&mut self, index: usize, element: JsonValue) -> Option<()> {
        if element.is_array() | element.is_object() {
            return None;
        }
        self.0.as_array_mut().unwrap().insert(index, element);
        Some(())
    }

    /// Calls [`Vec::remove()`].
    pub fn remove(&mut self, index: usize) -> JsonValue {
        self.0.as_array_mut().unwrap().remove(index)
    }

    // Mention that it will return how many elements were inserted and whether it failed or not
    /// Clones and pushes each item from `items` into the array,
    /// checking that they are not an [`Array`][serde_json::Value::Array]
    /// or an [`Object`][serde_json::Value::Object]. Returns the number of items pushed.
    ///
    /// # Errors
    ///
    /// If an item turns out to be an `Array` or `Object`, an `Err` is returned.
    ///
    /// See [`Vec::extend_from_slice()`].
    pub fn extend_from_slice<T>(&mut self, items: T) -> Result<usize, usize>
    where
        T: AsRef<[JsonValue]>,
    {
        let target = self.0.as_array_mut().unwrap();
        let source = items.as_ref().iter();
        let source_len = source.len().clone();
        for (index, item) in source.enumerate() {
            if item.is_array() | item.is_object() {
                return Err(index);
            } else {
                target.push(item.clone())
            }
        }
        Ok(source_len)
    }

    /// Consumes the `JsonArray` and returns inner [`Value`][serde_json::Value].
    pub fn into_inner(self) -> JsonValue {
        self.0
    }
}

impl AsRef<JsonValue> for JsonArray {
    fn as_ref(&self) -> &JsonValue {
        &self.0
    }
}

impl Hash for JsonArray {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_byte_vec().hash(state);
    }
}

/// Configuration of a resource
pub type ResConfig = JsonArray;

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
#[derive(Error, Debug, Default, Clone)]
#[error("resource error: {0}")]
pub struct StringError(pub String);

/// Base trait for any resource.
pub trait Resource {
    ///Resource's original name.
    fn orig_name(&self) -> &str;

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
    /// Get mixer values as JSON array.
    fn get_values(&self) -> ResConfig;

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
    ) -> Result<(Box<Sound>, Box<ResState>, LeftoverSound<'a>), StringError>;
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
    Sound(Box<Sound>),
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

    /// If the value is a String, returns it, otherwise returns None.
    pub fn as_string(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v.as_str())
        } else {
            None
        }
    }

    /// If the value is a Note, returns it, otherwise returns None.
    pub fn as_note(&self) -> Option<&Note> {
        if let Self::Note(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// If the value is a ReadyNote, returns it, otherwise returns None.
    pub fn as_ready_note(&self) -> Option<&ReadyNote> {
        if let Self::ReadyNote(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// If the value is a Sound, returns it, otherwise returns None.
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

/// Error type for pipeline.
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Index outside range
    #[error("index outside of range")]
    IndexOutsideRange,

    /// Pipeline is broken (output of a mod does not match input of the next mod)
    #[error("pipeline broken at mod {0}")]
    PipelineBroken(usize),

    //TODO: should additional info be given? (allowed input and output, position)
    /// Inserting the mod will break the pipeline
    #[error("inserting mod will break the pipeline")]
    InsertBreaksPipeline,
}

/// Trait that extends Vec<Rc<dyn Mod>> with helpful functions
#[sealed]
pub trait Pipeline {
    /// Insert a [`Mod`] into the pipeline, making sure that it does not break the pipeline or
    /// alter pipeline's input and output types.
    //TODO: usage example (will require multiple mods)
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError>;

    /// Check that the pipeline is valid (each mod produces the type that the next mod accepts).
    fn is_valid(&self) -> Result<(), PipelineError>;

    /// Get all type changes that happen in the pipeline.
    fn type_flow(&self) -> Result<Vec<Discriminant<ModData>>, PipelineError>;

    //TODO: get indices of mods that change types

    /// Get input type of the first mod in the pipeline.
    fn input_type(&self) -> Option<Discriminant<ModData>>;

    /// Get output typee of the last mod in the pipeline.
    fn output_type(&self) -> Option<Discriminant<ModData>>;
}

#[sealed]
impl Pipeline for Vec<Rc<dyn Mod>> {
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError> {
        match () {
            // Outside of the range
            _ if index > self.len() => Err(PipelineError::IndexOutsideRange),

            // There are no mods that would restrict the available types
            _ if self.is_empty() => {
                self.push(item);
                Ok(())
            }

            // The mod fits in the middle of the pipeline
            _ if (index < self.len())
                && (index > 0)
                && (item.input_type() == self[index - 1].output_type())
                && (item.output_type() == self[index].input_type()) =>
            {
                self.insert(index, item);
                Ok(())
            }

            // If the mod did not fit in the middle, then it is being inserted at an
            // edge of the pipeline, and so must preserve pipeline's I/O type
            _ if item.input_type() != item.output_type() => {
                Err(PipelineError::InsertBreaksPipeline)
            }

            // Mod is inserted at the start
            _ if (index == 0) == (item.input_type() == self[0].input_type()) => {
                self.insert(0, item);
                Ok(())
            }

            // Mod is inserted at the end
            _ if (index == self.len())
                && (item.input_type() == self.last().unwrap().output_type()) =>
            {
                self.push(item);
                Ok(())
            }

            _ => Err(PipelineError::InsertBreaksPipeline),
        }
    }

    fn is_valid(&self) -> Result<(), PipelineError> {
        for i in 0..self.len() - 1 {
            if self[i].output_type() != self[i + 1].input_type() {
                return Err(PipelineError::PipelineBroken(i));
            }
        }
        Ok(())
    }

    fn type_flow(&self) -> Result<Vec<Discriminant<ModData>>, PipelineError> {
        self.is_valid()?;

        let mut out: Vec<Discriminant<ModData>> = Vec::new();
        for i in self {
            if i.input_type() != i.output_type() {
                out.push(i.output_type());
            }
        }
        Ok(out)
    }

    fn input_type(&self) -> Option<Discriminant<ModData>> {
        let item = self.first()?;
        Some(item.input_type())
    }

    fn output_type(&self) -> Option<Discriminant<ModData>> {
        let item = self.last()?;
        Some(item.output_type())
    }
}

/// Type to hold every newly created state when the pipeline is used
pub type PipelineStateChanges = Vec<Box<ResState>>;

/// Channels are expected to pass their input through a pipeline of mods.
pub trait Channel: Resource {
    /// Pass the data through the channel
    fn play(
        &self,
        item: ModData,
        state: &ResState,
        config: &ResConfig,
    ) -> Result<(ModData, PipelineStateChanges, Box<ResState>), StringError>;

    /// Type that the channel accepts
    fn input_type(&self) -> Discriminant<ModData>;

    /// Type that the channel returns
    fn output_type(&self) -> Discriminant<ModData>;
}

/// What note to play on what channel.
#[derive(Debug, Default, Clone)]
pub struct ChannelNumberAndNote {
    /// Channel number to play the note on.
    pub channel_number: usize,

    /// Note to play.
    pub note: Note,
}

/// This is a controller of a chip, which is a combination of Note->Sound channels
/// and a mixer.
pub trait Chip: Resource {
    /// Start playing note(s) on chip and get the next sound bit.
    ///
    /// Note: returned sound is expected to be a sound generated during period without
    /// keyon/keyoff events, which may be shorter than the note(s) that were given.
    fn play(
        &mut self,
        notes: &[ChannelNumberAndNote],
        state: &ResState,
        config: &ResConfig,
    ) -> Result<(Box<Sound>, Box<ResState>), StringError>;

    /// Get the last sound bit - up until `ticks` after last keyoff event.
    fn flush(ticks: usize) -> Result<(Box<Sound>, Box<ResState>), StringError>;

    /// Reset chip's state
    fn reset(&mut self);
}
