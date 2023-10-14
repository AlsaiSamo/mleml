//!Types that provide functionality of channels - isolated sound generators.
//!
//!A channel is represented with a stream of instructions or a sequence of channel's states.
//!Channels cannot affect each other directly, but their actions may be accounted for
//!during mixing.
use crate::resource::{InstrumentLump, NoteModLump, SoundModLump};

//TODO: understand how to best make it and write
//Note to self: check some prev. commit for the pipeline code
///Channel's state at a given point of time, expressed in MML/platform frinedly values.
pub struct ChannelState {
    ///Length of one tick in seconds
    tick_length: f32,

    ///Volume of the sound in platform's units
    volume: u8,

    ///Number of octaves above C-1.
    octave: u8,

    ///Default length for a note.
    ///
    ///Used if note's length is None.
    length: u8,

    ///Note's default velocity.
    velocity: u8,

    instrument: InstrumentLump,
    note_mods: Vec<NoteModLump>,
    sound_mods: Vec<SoundModLump>,
}
