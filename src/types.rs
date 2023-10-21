//!Main data types that the library uses.
//!
//! Typically, the program will create a `Note` and use it with [`ChannelState`][crate::channel::ChannelState]
//! to create a `Sound`. `ReadyNote` is created during the process as it is a
//! platform-value-agnostic representation of the given `Note`.

use dasp::frame::Stereo;
use slice_dst::SliceWithHeader;
use std::num::{NonZeroU8, NonZeroI8};

/// Note, defined in platform-friendly values.
///
/// This is designed to be used with [`ChannelState`][crate::channel::ChannelState].
#[derive(Clone)]
#[repr(C)]
pub struct Note {
    /// Note's length in ticks. If None, then the length needs to be provided from
    /// `ChannelState`
    pub len: Option<NonZeroU8>,

    ///Note's pitch in semitones relative to C, or the note's number in MIDI mod 12. If None,
    ///then this is a rest.
    pub pitch: Option<NonZeroI8>,

    ///One cent is 1/100th of a semitone.
    pub cents: i8,

    ///Flag to indicate that the note is intended to be natural (its pitch should not
    /// be affected by the key signature).
    pub natural: bool,

    //TODO: MIDI uses 7 bits for velocity. Should I adhere to that?
    //TODO: should I make it an Option so that the ChannelState can provide a default?
    ///Velocity of a note. Default is 128 (defined by `dasp` as u8::EQUILIBRIUM).
    pub velocity: u8,
}

///Note, defined in SI units.
#[derive(Clone)]
pub struct ReadyNote {
    ///Length of a note in seconds.
    pub len: f32,

    ///Length of the sound generated while it decays (fourth stage of ADSR envelope),
    /// in seconds.
    pub post_release: f32,

    ///Pitch of a note in Hz. None means that this is a rest.
    pub pitch: Option<f32>,

    ///Velocity of a note. Default is 128 (defined by `dasp` as u8::EQUILIBRIUM).
    pub velocity: u8,
}

///Immutable slice of PCM (Stereo, 32 bit float) data.
///
/// Also contains sampling rate of the data.
pub struct Sound(Box<SliceWithHeader<u32, Stereo<f32>>>);

impl Sound {
    ///Create new sound.
    pub fn new(data: Box<[Stereo<f32>]>, sampling_rate: u32) -> Sound {
        Sound(SliceWithHeader::new(sampling_rate, data.into_vec()))
    }

    ///Get sampling rate.
    pub fn sampling_rate(&self) -> u32 {
        self.0.header
    }
    ///Get data.
    pub fn data(&self) -> &[Stereo<f32>] {
        self.0.slice.as_ref()
    }
}

impl std::convert::AsRef<[Stereo<f32>]> for Sound {
    fn as_ref(&self) -> &[Stereo<f32>] {
        self.data()
    }
}
