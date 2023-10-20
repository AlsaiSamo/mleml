#![warn(missing_docs)]
//!Types that provide functionality of channels - isolated sound generators.
//!
//!A channel is represented with a stream of instructions or a sequence of channel's states.
//!Channels cannot affect each other directly, but their actions may be accounted for
//!during mixing.
use std::borrow::Cow;

use crate::{
    resource::{InstrumentLump, NoteModLump, PlatformValues, SoundModLump},
    types::{Note, ReadyNote, Sound},
};

///Intermediary type that holds updated states of the resources in the channel.
pub struct ChannelStateChanges {
    #[allow(missing_docs)]
    pub note_states: Vec<Box<[u8]>>,
    #[allow(missing_docs)]
    pub instrument_state: Box<[u8]>,
    #[allow(missing_docs)]
    pub sound_states: Vec<Box<[u8]>>,
}

///Channel's state at a given point of time.
///
///This keeps necessary information so that the user does not need to remember anything
///more to play the note.
pub struct ChannelState {
    ///Length of one tick in seconds
    pub tick_length: f32,

    ///Volume of the sound in platform's units
    pub volume: u8,

    ///Number of octaves above C-1.
    pub octave: u8,

    ///Default length for a note, in ticks.
    ///
    ///Used if note's length is None.
    pub length: u8,

    ///Duration of the sound after the note has been released, in ticks.
    pub post_release: u8,

    ///Instrument (Mod<ReadyNote, Sound).
    pub instrument: InstrumentLump,
    ///Note mods (Mod<Note, Note>)
    pub note_mods: Vec<NoteModLump>,
    ///Sound mods (Mod<Sound, Sound>)
    pub sound_mods: Vec<SoundModLump>,
}

impl ChannelState {
    ///Create new ChannelState
    pub fn new(
        tick_length: f32,
        volume: u8,
        octave: u8,
        length: u8,
        post_release: u8,
        instrument: InstrumentLump,
        note_mods: Box<[NoteModLump]>,
        sound_mods: Box<[SoundModLump]>,
    ) -> Self {
        ChannelState {
            tick_length,
            volume,
            octave,
            length,
            instrument,
            post_release,
            note_mods: Vec::from(note_mods),
            sound_mods: Vec::from(sound_mods),
        }
    }

    ///Play a note on the channel
    pub fn play(
        &self,
        note: Note,
        vals: &PlatformValues,
    ) -> Result<(Sound, ChannelStateChanges), Cow<'_, str>> {
        let mut note = note;
        let mut note_states: Vec<Box<[u8]>> = Vec::new();
        for i in self.note_mods.iter() {
            let new_state: Box<[u8]>;
            (note, new_state) = i.apply(&note)?;
            note_states.push(new_state);
        }

        let note = ReadyNote {
            len: match note.len {
                Some(t) => t.get() as f32 * self.tick_length,
                None => self.length as f32 * self.tick_length,
            },
            post_release: self.post_release as f32 * self.tick_length,
            pitch: note.pitch.and_then(|semitones| {
                Some(
                    vals.cccc
                        * 2.0_f32.powf(
                            1.0 + (semitones.get() as f32) / 12.0
                                + (note.cents as f32) / 1200.0
                                + self.octave as f32,
                        ),
                )
            }),
            velocity: note.velocity,
        };

        let instrument_state: Box<[u8]>;
        let mut sound: Sound;
        (sound, instrument_state) = self.instrument.apply(&note)?;

        let mut sound_states: Vec<Box<[u8]>> = Vec::new();
        for i in self.sound_mods.iter() {
            let new_state: Box<[u8]>;
            (sound, new_state) = i.apply(&sound)?;
            sound_states.push(new_state);
        }

        let states = ChannelStateChanges {
            note_states,
            instrument_state,
            sound_states,
        };
        Ok((sound, states))
    }
}
