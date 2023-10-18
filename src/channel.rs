#![warn(missing_docs)]
//!Types that provide functionality of channels - isolated sound generators.
//!
//!A channel is represented with a stream of instructions or a sequence of channel's states.
//!Channels cannot affect each other directly, but their actions may be accounted for
//!during mixing.
use std::borrow::Cow;

use crate::{
    resource::{InstrumentLump, NoteModLump, PlatformValues, ResState, SoundModLump},
    types::{Note, ReadyNote, Sound},
};

//TODO: replace types everywhere with *size where appropriate

pub struct ChannelStateChanges {
    pub note_states: Vec<Box<[u8]>>,
    pub instrument_state: Box<[u8]>,
    pub sound_states: Vec<Box<[u8]>>,
}

//TODO: replace mods with a pipeline?
//Note to self: check some prev. commit for the pipeline code
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

    ///Default length for a note.
    ///
    ///Used if note's length is None.
    pub length: u8,

    ///Instrument (Mod<ReadyNote, Sound).
    //TODO: do I replace this witha an option? and if None, error that there is no instrument
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
            note_mods: Vec::from(note_mods),
            sound_mods: Vec::from(sound_mods),
        }
    }

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
            post_release: note.post_release as f32 * self.tick_length,
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

        let mut ins_state: Box<[u8]>;
        let mut sound: Sound;

        (sound, ins_state) = self.instrument.apply(&note)?;

        let mut sound_states: Vec<Box<[u8]>> = Vec::new();

        for i in self.sound_mods.iter() {
            let new_state: Box<[u8]>;
            (sound, new_state) = i.apply(&sound)?;
            sound_states.push(new_state);
        }

        let states = ChannelStateChanges {
            note_states,
            instrument_state: ins_state,
            sound_states,
        };

        Ok((sound, states))
    }
}
