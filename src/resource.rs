//!Configurable, stateful providers of pure functions.
//!
//!Configuration is done via flat JSON array.
//!
//!Two types of resources are currently provided: `Mod<'msg, I, O>` transforms
//!`I` into `O` (used to create note -> sound pipeline), while `Platform` provides
//!constraints, sound mixing, and so on.

//TODO: actualy deliver on these promises.
//Resources can be dynamically loaded or built-in. They are expected to be able to
//verify correctness of provided config and state, as well as provide the config schema.
//They may also provide a name for themselves.
//

//TODO: write about config, state, Mod, FFI, ResLump

//TODO: constructor for ExtResource
use crate::types::{Note, ReadyNote, ResSound};
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

///Unfinished configuration builder (not all values were provided).
pub struct ConfBuilding<'a> {
    schema: &'a ResConfig,
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
    ///Checks and appends one item to the unfinished configuration. Ok(true)
    ///signals that the config is full.
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

///Configuration error.
pub enum ConfigError {
    //TODO: make JsonValues into refs. And maybe display something else?
    ///Unexpected type of value.
    BadValue(u32, JsonValue, JsonValue),

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
    fn check_config(&self, conf: ResConfig) -> Result<(), Cow<'_, str>>;

    ///Verify that the given state can be used by the resource.
    fn check_state(&self, state: ResState) -> Option<()>;

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
pub trait Platform: Resource {
    ///Get platform values
    fn get_vals() -> PlatformValues;

    //TODO: explain somewhere that platform works with channels by their order,
    //which needs to be upheld in interpreter
    //TODO: what should I pass in? Something that is also easily FFI-convertable
    // (can I use variadics for this?)
    // ///Mix sound from provided channels
    //fn mix(&[])-> todo!();
    //TODO: provide "how to use" string? Would help the end user with channel
    //ordering, and other things.
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
///
///Functions like (T, Return<[i8], [i8]>)
#[repr(C)]
struct ResReturn<T: Sized> {
    ///Is the response OK or some kind of an error.
    is_ok: bool,
    ///Returned item.
    item: *const T,
    ///Length of a message.
    msg_len: usize,
    ///Message, interpretation of which depends on `is_ok`.
    msg: *const i8,
}

//I was told this is good
#[repr(C)]
struct NoItem([u8; 0]);

//TODO: wrap dealloc?
///Mod that is loaded at a runtime as a C library.
pub struct ExtMod<I, O> {
    ///Unique ID.
    id: String,

    ///Schema.
    schema: ResConfig,

    ///Pure transformation function.
    apply: extern "C" fn(
        input: *const I,
        conf_size: usize,
        conf: *const u8,
        state_size: usize,
        state: *const u8,
    ) -> ResReturn<O>,

    ///Notify the module that the message can be deallocated safely.
    ///
    ///This is required because the module may have been compiled to use
    ///a different allocator than the library (like jemalloc), which will lead to
    ///issues if Rust side was to deallocate items created by the loaded library.
    dealloc: extern "C" fn(),

    ///Original name of the module.
    orig_name: extern "C" fn() -> *const i8,

    ///Check configuration.
    check_config: extern "C" fn(size: usize, conf: *const u8) -> ResReturn<NoItem>,

    ///Check state.
    check_state: extern "C" fn(size: usize, state: *const u8) -> ResReturn<NoItem>,
    //TODO: this needs to be used during resource creation, it is not necessary
    // to keep around.
    //config_schema: extern "C" fn() -> (u32, *const u8),
}

//TODO: look into making these safe or document how they can mess up.
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
        let ret = (self.check_config)(conf.len(), conf.as_ptr());
        if ret.is_ok {
            return Ok(());
        } else {
            unsafe {
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

//TODO: same as for the prev. block
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
