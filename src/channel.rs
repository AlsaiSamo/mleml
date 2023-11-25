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
    resource::{Mod, ModData, ResConfig, ResState, StringError, Resource},
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

pub type PipelineStateChanges = Vec<Box<ResState>>;

pub trait Channel: Resource {
    fn play(
        &self,
        item: ModData,
        state: &ResState,
        config: &ResConfig
    ) -> Result<(ModData, PipelineStateChanges, Box<ResState>), StringError>;

    fn input_type(&self) -> Option<Discriminant<ModData>>;

    fn output_type(&self) -> Option<Discriminant<ModData>>;
}
