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
    mem::Discriminant,
};
use thiserror::Error;

#[allow(missing_docs)]
pub(crate) type JsonValue = serde_json::Value;

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
    pub fn as_byte_vec(&self) -> Vec<u8> {
        to_vec(&self.0).unwrap()
    }

    /// Push item into the array as long as the item is not an array or an object.
    pub fn push(&mut self, item: JsonValue) -> Option<()> {
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

