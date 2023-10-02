#![feature(hash_set_entry)]
#![feature(ptr_from_ref)]

pub mod platform {
    //TODO: soundmod

    //Octocontra C, or C-1. All other frequencies are derived from it;
    // this value makes A above middle C equal to 440Hz.
    pub const CCCC: f32 = 8.175799;
    pub const MAX_TICK: u32 = 256;
    pub const MAX_VOLUME: u32 = 100;
    //TODO: clarify how this is used (what length is considered for tempo)
    pub const MAX_TEMPO: f32 = 256.0;
    pub const MAX_CHANNELS: u32 = 256;
}

use std::collections::HashSet;
pub use std::{hash::Hash, num::NonZeroU8, rc::Rc};

pub trait SetRc<T> {
    //Removes all solitary Rc's (strong count is 1) as they are not used anywhere
    fn trim(&mut self);
    fn wrap(&mut self, value: T) -> Rc<T>;
}

impl<T: Eq + Hash> SetRc<T> for HashSet<Rc<T>> {
    fn trim(&mut self) {
        self.retain(|r| Rc::strong_count(r) == 1);
    }

    fn wrap(&mut self, value: T) -> Rc<T> {
        let new = Rc::new(value);
        self.get_or_insert_owned(&new).clone()
    }
}

pub mod resource {
    use crate::types::{Note, ResSound};
    use core::{fmt, slice};
    use serde::{Deserialize, Serialize};
    use serde_json::{json, to_vec};
    use std::{
        borrow::Cow,
        ffi::CStr,
        hash::{Hash, Hasher},
        mem::{discriminant, Discriminant},
        ptr,
        rc::Rc,
    };

    type JsonValue = serde_json::Value;
    #[derive(Clone, Serialize, Deserialize)]
    //Contains a flat array
    pub struct JsonArray(JsonValue);

    impl JsonArray {
        fn new() -> Self {
            Self { 0: json!([]) }
        }
        fn as_slice(&self) -> &[JsonValue] {
            self.0.as_array().unwrap().as_slice()
        }
        fn as_byte_vec(&self) -> Vec<u8> {
            to_vec(&self.0).unwrap()
        }
        //Maintains array's flatness
        fn push(&mut self, item: JsonValue) -> Option<()> {
            match item.is_array() | item.is_object() {
                True => None,
                _ => {
                    self.0.as_array_mut().unwrap().push(item);
                    return Some(());
                }
            }
        }
    }

    impl Hash for JsonArray {
        fn hash<H: Hasher>(&self, state: &mut H) {
            to_vec(self.as_slice()).unwrap().hash(state);
        }
    }

    type ResConfig = JsonArray;

    pub enum ConfigBuilderError {
        BadSchema,
        TypeMismatch(usize, Discriminant<JsonValue>, Discriminant<JsonValue>),
    }

    impl fmt::Display for ConfigBuilderError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::BadSchema => write!(f, "Schema is not a flat array"),
                Self::TypeMismatch(l, e, g) => {
                    write!(f, "Type mismatch at {l}: expected {:?}, got {:?}", e, g)
                }
            }
        }
    }

    pub struct ConfigBuilder<'a> {
        schema: &'a ResConfig,
        config: ResConfig,
    }

    pub enum BuilderState<'a> {
        Builder(ConfigBuilder<'a>),
        Config(ResConfig),
    }

    impl<'a> BuilderState<'a> {
        pub fn new(schema: &'a ResConfig) -> BuilderState {
            if schema.as_slice().len() == 0 {
                return BuilderState::Config(ResConfig::new());
            } else {
                return BuilderState::Builder(ConfigBuilder {
                    schema,
                    config: ResConfig::new(),
                });
            }
        }
        //Appends items from the iterator, until a wrong one is found,
        //or the iterator ends, or the config is complete. Count of taken elements is
        //returned.
        //TODO: verify that this is a good approach
        pub fn inject<I>(&mut self, values: I) -> Result<usize, ConfigBuilderError>
        where
            I: IntoIterator<Item = JsonValue>,
        {
            let mut values = values.into_iter();
            let mut count = 0;
            //if let BuilderState::Builder(build) = self {
            while let BuilderState::Builder(build) = self {
                let val = values.next();
                match val.is_none() {
                    true => return Ok(count),
                    false => {
                        count += 1;
                        match build.append(val.unwrap())? {
                            //I think this will not be time consuming
                            true => *self = BuilderState::Config(build.config.to_owned()),
                            false => {}
                        }
                    }
                }
            }
            return Ok(count);
        }
    }

    impl<'a> ConfigBuilder<'a> {
        //true == full
        fn append(&mut self, value: JsonValue) -> Result<bool, ConfigBuilderError> {
            let position = self.config.as_slice().len();
            let current_type = discriminant(&self.schema.as_slice()[position]);
            let given_type = discriminant(&value);
            if current_type != given_type {
                return Err(ConfigBuilderError::TypeMismatch(
                    position + 1,
                    current_type,
                    given_type,
                ));
            };
            self.config
                .push(value)
                .ok_or_else(|| ConfigBuilderError::BadSchema)?;
            if position == self.schema.as_slice().len() {
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    //pub struct ResState(Rc<[u8]>);
    type ResState = Rc<[u8]>;

    pub enum ConfigError {
        BadValue(u32, JsonValue, JsonValue),
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

    //A (possibly dynamically loaded) resource (a library that provides a function)
    pub trait Resource {
        //Constant name defined in the resource
        fn orig_name(&self) -> Option<Cow<'_, str>>;
        //ID of a resource is unique and cannot be changed
        fn id(&self) -> &str;
        fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>>;
        //We cannot look into ResState so we can only check that it is correct
        fn check_state(&self, state: ResState) -> Option<()>;
        //fn get_config_schema(&self) -> &ResConfig;
    }

    impl Hash for dyn Resource {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.id().hash(state)
        }
    }

    pub trait Mod<'msg, I, O>: Resource {
        fn apply(
            &self,
            input: &I,
            conf: &ResConfig,
            state: ResState,
        ) -> Result<(O, ResState), Cow<'msg, str>>;
    }

    //Return type for loadable resources
    #[repr(C)]
    struct ResReturn<T: Sized> {
        is_ok: bool,
        item: *const T,
        //If ok, it is state. If not, it is error.
        msg_len: usize,
        msg: *const i8,
    }

    #[repr(C)]
    struct NoItem([u8; 0]);

    pub struct ExtResource<I, O> {
        id: String,
        //In this format here, comes from a deser. string given by the resource
        schema: ResConfig,
        apply: extern "C" fn(
            input: *const I,
            conf_size: usize,
            conf: *const u8,
            state_size: usize,
            state: *const u8,
        ) -> ResReturn<O>,
        //It is fine to deallocate the message
        dealloc: extern "C" fn(),
        orig_name: extern "C" fn() -> *const i8,
        check_config: extern "C" fn(size: usize, conf: *const u8) -> ResReturn<NoItem>,
        check_state: extern "C" fn(size: usize, state: *const u8) -> ResReturn<NoItem>,
        //TODO: this needs to be used during resource creation, it is not necessary
        // to keep around.
        //config_schema: extern "C" fn() -> (u32, *const u8),
    }

    impl<I, O> Resource for ExtResource<I, O> {
        fn orig_name(&self) -> Option<Cow<'_, str>> {
            unsafe {
                match (self.orig_name)() {
                    ptr if ptr.is_null() => None,
                    ptr => Some(CStr::from_ptr(ptr).to_string_lossy()),
                }
            }
        }

        fn id(&self) -> &str {
            return self.id.as_str();
        }

        fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>> {
            let conf = conf.as_byte_vec();
            unsafe {
                let ret = (self.check_config)(conf.len(), conf.as_ptr());
                if ret.is_ok {
                    return Ok(());
                } else {
                    return Err(CStr::from_ptr(ret.msg).to_string_lossy());
                }
            }
        }

        fn check_state(&self, state: ResState) -> Option<()> {
            (self.check_state)(state.len(), state.as_ptr())
                .is_ok
                .then_some(())
        }
    }

    impl<'msg, I, O> Mod<'msg, I, O> for ExtResource<I, O> {
        fn apply(
            &self,
            input: &I,
            conf: &ResConfig,
            state: ResState,
        ) -> Result<(O, ResState), Cow<'msg, str>> {
            let conf = conf.as_byte_vec();
            unsafe {
                let ret = (self.apply)(
                    ptr::from_ref(input),
                    conf.len(),
                    (conf).as_ptr(),
                    state.len(),
                    state.as_ptr(),
                );
                match ret.is_ok {
                    true => Ok((
                        (ret.item as *const O).read(),
                        Rc::from(slice::from_raw_parts(ret.msg as *const u8, ret.msg_len)),
                    )),
                    false => Err(CStr::from_ptr(ret.msg).to_string_lossy()),
                }
            }
        }
    }

    pub type ExtNoteMod = ExtResource<Note, Note>;
    pub type ExtSoundMod = ExtResource<ResSound, ResSound>;
    pub type ExtInstrument = ExtResource<Note, ResSound>;

    //Module, conf and state bundled together for ease of use.
    #[derive(Clone)]
    pub struct ResLump<I, O> {
        pub module: Rc<dyn for<'a> Mod<'a, I, O>>,
        pub conf: Rc<ResConfig>,
        pub state: Rc<[u8]>,
    }

    impl<'msg, I, O> ResLump<I, O> {
        pub fn apply(&self, input: I) -> Result<(O, Rc<[u8]>), Cow<'msg, str>> {
            self.module.apply(&input, &self.conf, self.state.clone())
        }
    }

    pub type NoteModLump = ResLump<Note, Note>;
    pub type SoundModLump = ResLump<ResSound, ResSound>;
    pub type InstrumentLump = ResLump<Note, ResSound>;
}

pub mod types {
    use dasp::frame::Stereo;
    use std::num::NonZeroU8;

    //Length in ticks, pitch in semitones.
    //Unspecified length means that the channel's default length will be used.
    //Unspecified pitch means that the note is actually a rest.
    #[derive(Clone, Copy)]
    #[repr(C)]
    pub struct Note {
        pub len: Option<NonZeroU8>,
        pub pitch: Option<NonZeroU8>,
        pub cents: i8,
        pub natural: bool,
        //Equilibrium is 128, or u8::EQUIIBRIUM (from dasp)
        pub velocity: u8,
    }

    //Length in seconds, pitch in Hz, velocity and pitch=None retain meaning from Note
    pub struct ReadyNote {
        pub len: f32,
        pub pitch: Option<f32>,
        pub velocity: u8,
    }

    //An immutable slice of PCM data.
    pub struct Sound {
        sampling_rate: u32,
        data: Box<[Stereo<f32>]>,
    }

    #[repr(C)]
    pub struct ResSound {
        sampling_rate: u32,
        data_len: u32,
        data: *const Stereo<f32>,
    }

    impl Sound {
        pub fn new(data: Box<[Stereo<f32>]>, sampling_rate: u32) -> Sound {
            Sound {
                data,
                sampling_rate,
            }
        }
        pub fn sampling_rate(&self) -> u32 {
            self.sampling_rate
        }
        pub fn data(&self) -> &[Stereo<f32>] {
            self.data.as_ref()
        }
    }
}

pub mod channel {
    use crate::platform::CCCC;
    use crate::resource::{InstrumentLump, NoteModLump, SoundModLump};
    use crate::types::ReadyNote;
    use crate::{
        resource::Mod,
        types::{Note, Sound},
    };
    use std::rc::Rc;

    //Note to self: check some prev. commit for the pipeline code
    pub struct ChannelState {
        tick_length: f32,
        volume: u8,
        octave: u8,
        length: u8,
        velocity: u8,
        instrument: InstrumentLump,
        note_mods: Vec<NoteModLump>,
        sound_mods: Vec<SoundModLump>,
    }
}
