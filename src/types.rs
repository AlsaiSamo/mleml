//!Types that the library operates on.

use dasp::frame::Stereo;
use slice_dst::SliceWithHeader;
use std::num::NonZeroU8;

///Note, defined in platform-friendly values.
#[derive(Clone)]
#[repr(C)]
pub struct Note {
    ///Note's length in ticks. If None, then length needs to be provided externally.
    pub len: Option<NonZeroU8>,

    ///Note's pitch in semitones above C, or the note's number in MIDI mod 12. If None,
    ///then this is a rest.
    pub pitch: Option<NonZeroU8>,

    ///1/100th of a semitone.
    pub cents: i8,

    ///True if the note needs to be natural.
    pub natural: bool,

    ///Velocity of a note. Default is 128 (u8::EQUILIBRIUM).
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
