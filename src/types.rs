//! Main data types that the library uses.

use dasp::frame::Stereo;
use slice_dst::SliceWithHeader;
use std::num::{NonZeroI8, NonZeroU8};

/// Note, defined in abstract, platform-defined values.
#[derive(Clone, Default)]
#[repr(C)]
pub struct Note {
    /// Note length in ticks.
    ///
    /// May be unspecified.
    pub len: Option<NonZeroU8>,

    /// Note's pitch in semitones relative to C.
    ///
    /// If None, then this is a rest.
    pub pitch: Option<NonZeroI8>,

    ///One cent is 1/100th of a semitone.
    pub cents: i8,

    /// Flag to indicate that the note is intended to be natural (its pitch should not
    /// be affected by the key signature).
    pub natural: bool,

    //TODO: MIDI uses 7 bits for velocity. Should I adhere to that?
    /// Velocity of a note.
    ///
    /// Default is 128 (defined by `dasp` as u8::EQUILIBRIUM).
    pub velocity: u8,
}

/// Note, defined in SI units.
#[derive(Clone, Default)]
pub struct ReadyNote {
    /// Length of a note in seconds.
    pub len: f32,

    /// Length of the sound generated while it decays, in seconds.
    pub decay_time: f32,

    /// Pitch of a note in Hz. None indicates a rest.
    pub pitch: Option<f32>,

    /// Velocity of a note. Default is 128 (defined by `dasp` as u8::EQUILIBRIUM).
    pub velocity: u8,
}

/// Immutable slice of PCM (Stereo, 32 bit float) data with sampling rate.
pub struct Sound(Box<SliceWithHeader<u32, Stereo<f32>>>);

impl Sound {
    /// Create new sound.
    pub fn new(data: Box<[Stereo<f32>]>, sampling_rate: u32) -> Sound {
        Sound(SliceWithHeader::new(sampling_rate, data.into_vec()))
    }

    /// Get sampling rate.
    pub fn sampling_rate(&self) -> u32 {
        self.0.header
    }
    /// Get data.
    pub fn data(&self) -> &[Stereo<f32>] {
        self.0.slice.as_ref()
    }
}

impl std::convert::AsRef<[Stereo<f32>]> for Sound {
    fn as_ref(&self) -> &[Stereo<f32>] {
        self.data()
    }
}
