//TODO: switch to deny
#![warn(missing_docs)]
#![feature(hash_set_entry)]
#![feature(ptr_from_ref)]

//TODO: create examlpe platform and mods, feature-gated, and preferably in
//Rust and C.

//TODO: I am using equal temperament here, so mention that
//also that some parts stick to MIDI.
//TODO: when I add loading C libs, mention that here.
//!(WIP)
//!
//!A library for working with music written in MML (music macro language) fashion.
//!
//!It strives to be flexible enough to allow making an output closely resembling
//!that of some chip (platform).

/////Stub verstion of platform code.
/////
/////This module (or, rather, what it will be reworked into) is used to
/////provide constraints and quirks that exist on a platform that is being written
/////music for (for example, YM2608 chip).
/////
/////Platform code cannot constrain sound-producing mods, so for genuine mimicking
/////of a platform one needs to also select the correct mods.
//TODO: remove
// pub mod platform {
//     //TODO: mods for every part of platform step
//     //TODO: understand what goes here

//     ///Frequency of C-1. All other note frequencies are derived from it.
//     ///
//     ///For reference, A440 standard makes C-1 equal to 8.175799.
//     pub const CCCC: f32 = 8.175799;
//     ///Maximum tick value permitted for a note.
//     pub const MAX_TICK: u32 = 256;
//     ///Full volume value
//     pub const MAX_VOLUME: u32 = 100;
//     //TODO: clarify how this is used (what length is considered for tempo)
//     pub const MAX_TEMPO: f32 = 256.0;
//     ///Maximum number of channels permitted
//     pub const MAX_CHANNELS: u32 = 256;
// }

//TODO: feature-gate
///Extra things to help with storing data
pub mod extra {
    use std::collections::HashSet;
    use std::{hash::Hash, rc::Rc};

    ///Trait for sets that contain Rc<T>
    pub trait SetRc<T> {
        ///Remove unused Rc's.
        fn trim(&mut self);
        ///Return an Rc that already has T or create a new one.
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

}

///Provides resources (currently of one type) along with the necessary types.
//TODO: write about config, state, Mod, FFI, ResLump
pub mod resource {
    //TODO: resource constructor for ExtResource
    use crate::types::{Note, ResSound, ReadyNote};
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

    ///Flat JSON array of arbitrary values.
    #[derive(Clone, Serialize, Deserialize)]
    pub struct JsonArray(JsonValue);

    impl JsonArray {
        ///Create new, empty JSON array
        fn new() -> Self {
            Self { 0: json!([]) }
        }
        ///Get elements in a slice
        fn as_slice(&self) -> &[JsonValue] {
            self.0.as_array().unwrap().as_slice()
        }
        ///Serialize into byte vector
        fn as_byte_vec(&self) -> Vec<u8> {
            to_vec(&self.0).unwrap()
        }
        ///Push item into the array as long as the item is not an array or a map
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
            to_vec(self.as_slice()).unwrap().hash(state);
        }
    }

    type ResConfig = JsonArray;

    ///Error encountered while building configuration.
    pub enum ConfigBuilderError {
        ///Schema provided by module cannot be used.
        BadSchema,
        ///Provided type does not match the type defined in the schema.
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

    ///Unfinished configuration builder
    pub struct ConfBuilding<'a> {
        schema: &'a ResConfig,
        config: ResConfig,
    }

    ///Configuration builder
    pub enum ConfigBuilder<'a> {
        ///Config is unfinished
        Builder(ConfBuilding<'a>),
        ///Config is finished
        Config(ResConfig),
    }

    impl<'a> ConfigBuilder<'a> {
        ///Create new config builder from given schema
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
        pub fn inject<I>(&mut self, values: I) -> Result<usize, ConfigBuilderError>
        where
            I: IntoIterator<Item = JsonValue>,
        {
            let mut values = values.into_iter();
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
    }

    impl<'a> ConfBuilding<'a> {
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

    type ResState = Rc<[u8]>;

    ///Configuration error
    pub enum ConfigError {
        //TODO: make JsonValues into refs
        ///Unexpected type of value
        BadValue(u32, JsonValue, JsonValue),
        ///Incorrect length
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

    ///Resources of any type need to conform to these constraints:
    /// 1. Provide unique ID
    /// 2. Only provide pure functions (state is given and returned when needed)
    ///
    ///Resources are also named, but association of names to IDs needs to be done
    ///externally.
    pub trait Resource {
        ///Resource's original name
        fn orig_name(&self) -> Option<Cow<'_, str>>;
        ///Unique ID of the resource
        fn id(&self) -> &str;
        ///Verify that the given config can be used by the resource
        fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>>;
        ///Verify that the given state can be used by the resource
        fn check_state(&self, state: ResState) -> Option<()>;
        //fn get_config_schema(&self) -> &ResConfig;
    }

    impl Hash for dyn Resource {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.id().hash(state)
        }
    }

    //TODO: platform. PlatformValues, struct that has functions, incl. mixer.

    ///Values provided by the platform, such as maixmum channel count,
    ///frequency of C-1, acceptable volume range, and more.
    ///
    ///The platform may, for example, allow more channels, if configured to do so.
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
        pub channels: u32
    }

    ///Resource that defines the quirks and limitations of a given platform
    ///(usually a sound chip).
    ///
    ///As an example of a platform quirk, PMDMML plays SSG drums, defined in K channel,
    /// on I channel, which means that it is not possible to just mix sound.
    pub trait Platform: Resource {
        ///Get platform values
        fn get_vals() -> PlatformValues;
        //TODO: explain somewhere that platform works with channels by their order,
        //which needs to be upheld in interpreter
        //TODO: what should I pass in? Something that is also easily FFI-convertable
        // (can I use variadics for this?)
        // ///Mix sound from provided channels
        //fn mix(&[])-> todo!();
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
            state: ResState,
        ) -> Result<(O, ResState), Cow<'msg, str>>;
    }

    ///FFI-friendly return type for all kinds of messages.
    #[repr(C)]
    struct ResReturn<T: Sized> {
        is_ok: bool,
        item: *const T,
        //If ok, it is state. If not, it is error.
        msg_len: usize,
        msg: *const i8,
    }

    //I was told this is good
    #[repr(C)]
    struct NoItem([u8; 0]);

    ///Mod that is loaded at a runtime as a C library.
    pub struct ExtMod<I, O> {
        ///Unique ID
        id: String,
        //In this format here, comes from a deser. string given by the resource
        ///Schema
        schema: ResConfig,
        ///Transformation function
        apply: extern "C" fn(
            input: *const I,
            conf_size: usize,
            conf: *const u8,
            state_size: usize,
            state: *const u8,
        ) -> ResReturn<O>,
        //It is fine to deallocate the message
        ///Notify that the message can be deallocated safely
        dealloc: extern "C" fn(),
        ///Original name
        orig_name: extern "C" fn() -> *const i8,
        ///Check configuration
        check_config: extern "C" fn(size: usize, conf: *const u8) -> ResReturn<NoItem>,
        ///Check state
        check_state: extern "C" fn(size: usize, state: *const u8) -> ResReturn<NoItem>,
        //TODO: this needs to be used during resource creation, it is not necessary
        // to keep around.
        //config_schema: extern "C" fn() -> (u32, *const u8),
    }

    impl<I, O> Resource for ExtMod<I, O> {
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

    impl<'msg, I, O> Mod<'msg, I, O> for ExtMod<I, O> {
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

    //TODO: will I use these?
    // ///External note -> note mod
    // pub type ExtNoteMod = ExtMod<Note, Note>;
    // ///External sound -> sound mod
    // pub type ExtSoundMod = ExtMod<ResSound, ResSound>;
    // ///External note -> sound mod
    // pub type ExtInstrument = ExtMod<Note, ResSound>;

    ///Mod, along with its configuration and state bundled together for ease of use.
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
        ///Use mod's apply() with bundled state and config
        pub fn apply(&self, input: I) -> Result<(O, Rc<[u8]>), Cow<'msg, str>> {
            self.module.apply(&input, &self.conf, self.state.clone())
        }
    }

    #[allow(missing_docs)]
    pub type NoteModLump = ResLump<Note, Note>;
    #[allow(missing_docs)]
    pub type SoundModLump = ResLump<ResSound, ResSound>;
    #[allow(missing_docs)]
    pub type InstrumentLump = ResLump<ReadyNote, ResSound>;
}

///Types used throughout the library.
pub mod types {
    use dasp::frame::Stereo;
    use std::num::NonZeroU8;

    //Length in ticks, pitch in semitones.
    //Unspecified length means that the channel's default length will be used.
    //Unspecified pitch means that the note is actually a rest.
    ///Note, defined in platform-friendly values.
    #[derive(Clone)]
    #[repr(C)]
    pub struct Note {
        ///Note's length. If None, then length needs to be provided elsewhere
        pub len: Option<NonZeroU8>,
        ///Note's pitch in semitones above C-1, or the note's number in MIDI. If None,
        ///then this is a rest.
        pub pitch: Option<NonZeroU8>,
        ///1/100th of a semitone.
        pub cents: i8,
        ///True if the note needs to be natural.
        pub natural: bool,
        //Equilibrium is 128, or u8::EQUIIBRIUM (from dasp)
        ///Velocity of a note. Default is 128.
        pub velocity: u8,
    }

    ///Note, defined in SI units.
    #[derive(Clone)]
    pub struct ReadyNote {
        ///Length of a note in seconds.
        pub len: f32,
        ///Length of sound generated while it decays (fourth stage of ADSR envelope),
        /// in seconds.
        pub post_release: f32,
        ///Pitch of a note in Hz. None means that this is a rest.
        pub pitch: Option<f32>,
        ///Velocity of a note. Default is 128.
        pub velocity: u8,
    }

    ///Immutable slice of PCM data.
    pub struct Sound {
        sampling_rate: u32,
        data: Box<[Stereo<f32>]>,
    }

    ///FFI-friendly immutable slice of PCM data.
    #[repr(C)]
    pub struct ResSound {
        sampling_rate: u32,
        data_len: u32,
        data: *const Stereo<f32>,
    }

    impl Sound {
        ///Create new sound.
        pub fn new(data: Box<[Stereo<f32>]>, sampling_rate: u32) -> Sound {
            Sound {
                data,
                sampling_rate,
            }
        }
        ///Get sampling rate.
        pub fn sampling_rate(&self) -> u32 {
            self.sampling_rate
        }
        ///Get data.
        pub fn data(&self) -> &[Stereo<f32>] {
            self.data.as_ref()
        }
    }
}

///Types that provide functionality of channels - isolated sound generators.
///
///A channel is represented with a stream of instructions or a sequence of channel's states.
///Channels cannot affect each other until their outputs are mixed by platform code.
pub mod channel {
    use crate::resource::{InstrumentLump, NoteModLump, SoundModLump};

    //TODO: understand how to best make it and write
    //Note to self: check some prev. commit for the pipeline code
    ///Channel's state at a given point of time, expressed in MML-frinedly values.
    pub struct ChannelState {
        ///Length of one tick in seconds
        tick_length: f32,
        #[allow(missing_docs)]
        volume: u8,
        ///Number of octaves above C-1.
        octave: u8,
        ///Default length for a note
        length: u8,
        ///Note's velocity
        velocity: u8,
        //TODO: should I replace this with a "pipeline" structure? This would
        //allow me to add non-mod resounres, like monitors
        instrument: InstrumentLump,
        note_mods: Vec<NoteModLump>,
        sound_mods: Vec<SoundModLump>,
    }
}
