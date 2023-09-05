#![feature(hash_set_entry)]

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

use dasp::frame::Stereo;
use serde_json;
use std::{
    collections::HashSet, ffi::CString, hash::Hash, num::NonZeroU32, rc::Rc,
};

use crate::platform::CCCC;

//Ticks
type Length = NonZeroU32;
//100 cents, or 1/12th of an octave
type Pitch = NonZeroU32;
type Octave = u32;
type Volume = u32;
type ModName = String;

//Length in ticks, pitch in semitones.
//Unspecified length means that the channel's default length will be used.
//Unspecified pitch means that the note is actually a rest.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Note {
    pub len: Option<Length>,
    pub pitch: Option<Pitch>,
    pub cents: i32,
    pub natural: bool,
    //TODO: velocity?
}

//Length in seconds, pitch in Hz, volume is in percentages
#[repr(C)]
pub struct ReadyNote {
    pub len: f32,
    pub pitch: Option<f32>,
    pub volume: f32,
}

//A slice of PCM data
#[repr(C)]
pub struct Sound {
    pub data: Box<[Stereo<f32>]>,
    sampling_rate: u32,
}

impl Sound {
    pub fn new(rate: u32) -> Sound {
        Sound {
            data: Box::new([]),
            sampling_rate: rate,
        }
    }
    pub fn sampling_rate(&self) -> u32 {
        self.sampling_rate
    }
}

//TODO: serde_json::Value is not Hash. Implement it by hashing string
// representation of JSON.
type JsonValue = serde_json::Value;

//TODO: decide what the config needs to look like
//Array is good because of ordering
//type ResConfig = Vec<JsonValue>;
pub struct ResConfig(Vec<JsonValue>);

impl ResConfig {
    pub fn try_new(value: JsonValue) -> Option<Self>{
        //TODO: to_vec() produces vec<u8>. I need to use as_array, which returns
        // references. I need to rewrite the code to rely on references.
        //value.as_array().and_then(|v| Some(ResConfig {0: v.to_vec()}))
        // match value.is_array() {
        //     True => Some(ResConfig{0: value.as_array()}),
        //     False => None
        // }
        todo!()
    }
}

pub struct ConfigBuilder<'a> {
    //Used to validate the passed types
    schema: &'a JsonValue,
    config: JsonValue,
}

//Typestate for ConfigBuilder
pub enum BuilderState<'a> {
    Builder(ConfigBuilder<'a>),
    Config(ResConfig),
}

//TODO: better error type
impl<'a> ConfigBuilder<'a> {
    pub fn append(&self, value: JsonValue) -> Result<BuilderState<'a>, String> {
        //let index = self.config.as_object().ok_or("Config is not an object")?;
        // let index = self
        //     //TODO: I will not ned to write this here if I were to change ResConfig
        //     // to a vector of values
        //     .config
        //     .as_array()
        //     .ok_or("Config is not an object")?
        //     .len();
        todo!()
    }
}

#[derive(Clone)]
pub struct ResState(JsonValue);

impl ResState {
    pub fn new(state: JsonValue) -> Self {
        ResState(state)
    }
}

pub trait SetRc<T> {
    //Removes all solitary Rc's (strong count is 1) as they are not used anywhere
    fn trim(&mut self);
    fn wrap_and_return(&mut self, value: T) -> Rc<T>;
}

impl<T: Eq + Hash> SetRc<T> for HashSet<Rc<T>> {
    fn trim(&mut self) {
        self.retain(|r| Rc::strong_count(r) == 1);
    }

    fn wrap_and_return(&mut self, value: T) -> Rc<T> {
        let new = Rc::new(value);
        self.get_or_insert(new).clone()
    }
}

//A resource
pub trait Resource {
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn check_config(&self, conf: ResConfig) -> Result<(), String>;
    fn check_state(&self, state: ResState) -> Result<(), String>;
    fn get_config(&self) -> JsonValue;
    fn get_state(&self) -> JsonValue;
    fn get_config_schema(&self) -> JsonValue;
}

//Note -> Note
pub trait NoteMod: Resource {
    fn apply(&self, note: Note) -> Result<Note, String>;
}
//Sound -> Sound
pub trait SoundMod: Resource {
    fn apply(&self, sound: Sound) -> Result<Sound, String>;
}
//Note -> Sound
pub trait Instrument: Resource {
    fn apply(&self, note: ReadyNote) -> Result<Sound, String>;
}

//NoteMod that is external
pub struct ExternNoteMod {
    name: String,
    id: String,
    //Second and third argument are created by converting JSON into something else
    //TODO: return something that is representable in C
    //TODO: strings need to be repr(C). These are not..?
    apply: extern "C" fn(Note, CString, CString) -> Result<(Note, CString), CString>,
}

impl NoteMod for ExternNoteMod {
    fn apply(&self, note: Note) -> Result<Note, String> {
        todo!();
    }
}

impl Resource for ExternNoteMod {
    //TODO: give reference to string?
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn check_config(&self, conf: ResConfig) -> Result<(), String> {
        todo!()
    }

    fn check_state(&self, state: ResState) -> Result<(), String> {
        todo!()
    }

    fn get_config(&self) -> JsonValue {
        todo!()
    }

    fn get_state(&self) -> JsonValue {
        todo!()
    }

    fn get_config_schema(&self) -> JsonValue {
        todo!()
    }
}

pub struct ExternSoundMod {
    name: String,
    id: String,
    //Second and third argument are created by converting JSON into something else
    apply: extern "C" fn(Sound, String, String) -> Result<(Stereo<f32>, String), String>,
}

impl SoundMod for ExternSoundMod {
    fn apply(&self, note: Sound) -> Result<Sound, String> {
        todo!();
    }
}

impl Resource for ExternSoundMod {
    //TODO: give reference to string?
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn check_config(&self, conf: ResConfig) -> Result<(), String> {
        todo!()
    }

    fn check_state(&self, state: ResState) -> Result<(), String> {
        todo!()
    }

    fn get_config(&self) -> JsonValue {
        todo!()
    }

    fn get_state(&self) -> JsonValue {
        todo!()
    }

    fn get_config_schema(&self) -> JsonValue {
        todo!()
    }
}

pub struct ExternInstrument {
    name: String,
    id: String,
    //Second and third argument are created by converting JSON into something else
    apply: extern "C" fn(ReadyNote, String, String) -> Result<(Sound, String), String>,
}

impl Instrument for ExternInstrument {
    fn apply(&self, note: ReadyNote) -> Result<Sound, String> {
        todo!();
    }
}

impl Resource for ExternInstrument {
    //TODO: give reference to string?
    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn check_config(&self, conf: ResConfig) -> Result<(), String> {
        todo!()
    }

    fn check_state(&self, state: ResState) -> Result<(), String> {
        todo!()
    }

    fn get_config(&self) -> JsonValue {
        todo!()
    }

    fn get_state(&self) -> JsonValue {
        todo!()
    }

    fn get_config_schema(&self) -> JsonValue {
        todo!()
    }
}

//State of the channel at the start of a note/rest
pub struct ChannelState {
    //Length of one tick in seconds
    tick_length: f32,
    //Platform-defined volume setting
    volume: Volume,
    note: Note,
    octave: Octave,
    length: Length,
    instrument: Rc<dyn Instrument>,
    note_modifiers: Vec<Rc<dyn NoteMod>>,
    sound_modifiers: Vec<Rc<dyn SoundMod>>,
    //TODO: state and config information for the resources
}

impl ChannelState {
    pub fn play(&self) -> Result<Sound, String> {
        //TODO: pass in config and state of the mods
        let note = self
            .note_modifiers
            .iter()
            .fold(self.note, |a, f| f.apply(a).unwrap());
        let note = ReadyNote {
            len: self.tick_length * (note.len.unwrap_or(self.length)).get() as f32,
            //TODO:
            pitch: note.pitch.and_then(|_| {
                Some(
                    CCCC * 2.0_f32.powf(
                        (note.pitch.unwrap().get() as f32 + note.cents as f32 / 100.0) / 12.0
                            + self.octave as f32,
                    ),
                )
            }),
            volume: self.volume as f32,
        };
        //Apply everything
        //TODO: pass in state, config, and volume
        let mut sound = self.instrument.apply(note)?;
        sound = self
            .sound_modifiers
            .iter()
            .fold(sound, |a, f| f.apply(a).unwrap());
        //TODO: apply platform's sound mod
        Ok(sound)
    }
}

pub struct TrackState {
    //Platform-defined ticks since start
    tick: usize,
    channels: Vec<ChannelState>,
    //TODO: state and config for platform code
}

impl TrackState {
    //TODO: play(), new(), etc.
}

//All of the commands that can be executed on a channel
pub enum Instruction {
    //Play or not play a new sound
    Play(Note),
    //TODO: Enter macro
    //macro: Macro,
    //Set instrument, add/remove sound/note mod
    Instrument(ModName),
    AddNoteMod(ModName),
    RemNoteMod(ModName),
    AddSoundMod(ModName),
    RemSoundMod(ModName),
    //Set volume
    Volume(Volume),
    //Set octave
    Octave(Octave),
    //Set note's default length
    Length(Length),
}
