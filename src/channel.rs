#![warn(missing_docs)]
//!A channel is an isolated sound generator.
//!
//!A channel is represented with a stream of instructions or a sequence of channel's states.
//!Channels cannot affect each other directly, but their actions may be accounted for
//!during mixing.

use std::{
    mem::{discriminant, Discriminant},
    rc::Rc,
};

use thiserror::Error;

use crate::{
    resource::{Mod, ModData, PlatformValues, ResConfig, ResState, StringError},
    types::{Note, Sound},
};

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("index outside of range")]
    IndexOutsideRange,
    #[error("pipeline broken at mod {0}")]
    PipelineBroken(usize),
    //TODO: add information (allowed input and output, position)
    #[error("inserting mod will break the pipeline")]
    InsertBreaksPipeline,
}

//TODO: maybe rename to "ModVec" or similar?
pub trait Pipeline {
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError>;

    fn is_valid(&self) -> Result<(), PipelineError>;

    // vector of discriminants describing how data is transformed
    // or "pipeline broken"
    fn type_flow(&self) -> Result<Vec<Discriminant<ModData>>, PipelineError>;

    fn input_type(&self) -> Option<Discriminant<ModData>>;

    fn output_type(&self) -> Option<Discriminant<ModData>>;
}

impl Pipeline for Vec<Rc<dyn Mod>> {
    //fails if the mod changes the type (because this would break the pipeline or
    // alter it)
    //TODO: can if else chain be replaced to look nicer?
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError> {
        if item.input_type() != item.output_type() {
            Err(PipelineError::InsertBreaksPipeline)
        } else if index > self.len() {
            return Err(PipelineError::IndexOutsideRange);
        } else if self.is_empty() {
            self.push(item);
            return Ok(());
        } else if (index == 0) && (item.input_type() == self[0].input_type()) {
            self.insert(0, item);
            return Ok(());
        } else if (index == self.len()) && (item.input_type() == self.last().unwrap().output_type())
        {
            self.push(item);
            return Ok(());
        } else if item.output_type() == self[index].input_type() {
            self.insert(index, item);
            return Ok(());
        } else {
            Err(PipelineError::InsertBreaksPipeline)
        }
    }

    //checks that each O matches the next I
    fn is_valid(&self) -> Result<(), PipelineError> {
        for i in 0..self.len() - 1 {
            if self[i].output_type() != self[i + 1].input_type() {
                return Err(PipelineError::PipelineBroken(i));
            }
        }
        Ok(())
    }

    //the discriminants are added when output type changes
    // So, the result may be empty, if there were no transformations.
    fn type_flow(&self) -> Result<Vec<Discriminant<ModData>>, PipelineError> {
        self.is_valid()?;

        let mut out: Vec<Discriminant<ModData>> = Vec::new();
        for i in self {
            if i.input_type() != i.output_type() {
                out.push(i.output_type());
            }
        }
        Ok(out)
    }

    fn input_type(&self) -> Option<Discriminant<ModData>> {
        let item = self.first()?;
        Some(item.input_type())
    }

    fn output_type(&self) -> Option<Discriminant<ModData>> {
        let item = self.last()?;
        Some(item.output_type())
    }
}

pub type ChannelStateChanges = Vec<Box<ResState>>;

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

    pub mods: Vec<Rc<dyn Mod>>,

    pub states: Vec<Rc<ResState>>,

    pub configs: Vec<Rc<ResConfig>>,
}

impl ChannelState {
    ///Create new ChannelState
    pub fn new(
        tick_length: f32,
        volume: u8,
        octave: u8,
        length: u8,
        post_release: u8,
        mods: Vec<Rc<dyn Mod>>,
        states: Vec<Rc<ResState>>,
        configs: Vec<Rc<ResConfig>>,
    ) -> Self {
        ChannelState {
            tick_length,
            volume,
            octave,
            length,
            post_release,
            mods,
            states,
            configs,
        }
    }

    pub fn play(
        &self,
        note: Note,
        //Should I instead only pass in cccc?
        _vals: &PlatformValues,
    ) -> Result<(Sound, ChannelStateChanges), StringError> {
        if (self.mods.len() != self.states.len()) || (self.mods.len() != self.states.len()) {
            return Err(StringError(
                "number of mods, configs and states is not equal".to_owned(),
            ));
        }

        let mut item = ModData::Note(note);
        let mut state_changes: Vec<Box<ResState>> = Vec::new();

        for i in 0..self.mods.len() {
            //TODO: check for ID of Note -> ResNote and process it differently
            //TODO: also could differently process a "comment" mod
            if discriminant(&item) == self.mods[i].input_type() {
                match self.mods[i].apply(&item, &self.configs[i], &self.states[i]) {
                    Ok((new, state)) => {
                        item = new;
                        state_changes.push(state);
                    }
                    Err(what) => return Err(StringError(format!("mod error at {i}: {}", what))),
                }
            } else {
                return Err(StringError(format!(
                    "pipeline broken at {i} (type mismath)"
                )));
            }
        }

        match item {
            ModData::Sound(out) => Ok((out, state_changes)),
            _ => Err(StringError("pipeline produced incorrect type".to_string())),
        }
    }
}
