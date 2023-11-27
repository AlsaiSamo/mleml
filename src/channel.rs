#![warn(missing_docs)]
//! This module provides Pipeline, Channel, and related items.

use std::{mem::Discriminant, rc::Rc};

use thiserror::Error;

use crate::resource::{Mod, ModData, ResConfig, ResState, Resource, StringError};

/// Error type for pipeline
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Index outside range
    #[error("index outside of range")]
    IndexOutsideRange,

    /// Pipeline is broken (output of a mod does not match input of the next mod)
    #[error("pipeline broken at mod {0}")]
    PipelineBroken(usize),

    //TODO: add information (allowed input and output, position)
    /// Inserting the mod will break the pipeline
    #[error("inserting mod will break the pipeline")]
    InsertBreaksPipeline,
}

/// Trait that extends Vec<Rc<dyn Mod>> with helpful functions
pub trait Pipeline {
    /// Insert a mod into pipeline while not altering how it transforms types and
    /// keeping it valid.
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError>;

    /// Check that the pipeline is valid (each mod produces type that the next mod accepts)
    fn is_valid(&self) -> Result<(), PipelineError>;

    /// Get all type changes that happen in the pipeline
    fn type_flow(&self) -> Result<Vec<Discriminant<ModData>>, PipelineError>;

    //TODO: get indices of mods that change types

    /// Get input type of the first mod in the pipeline
    fn input_type(&self) -> Option<Discriminant<ModData>>;

    /// Get output typee of the last mod in the pipeline
    fn output_type(&self) -> Option<Discriminant<ModData>>;
}

impl Pipeline for Vec<Rc<dyn Mod>> {
    // TODO: allow insertion of the item in a place where it would fix a broken pipeline.
    // TODO: can if else chain be replaced to look nicer?
    fn insert_checked(&mut self, index: usize, item: Rc<dyn Mod>) -> Result<(), PipelineError> {
        if item.input_type() != item.output_type() {
            //A mod that changes data's type breaks or alters a valid pipeline
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

    fn is_valid(&self) -> Result<(), PipelineError> {
        for i in 0..self.len() - 1 {
            if self[i].output_type() != self[i + 1].input_type() {
                return Err(PipelineError::PipelineBroken(i));
            }
        }
        Ok(())
    }

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

/// Type to hold every newly created state when the pipeline is used
pub type PipelineStateChanges = Vec<Box<ResState>>;

/// Channels are expected to pass their input through a pipeline of mods.
pub trait Channel: Resource {

    /// Pass the data through the channel
    fn play(
        &self,
        item: ModData,
        state: &ResState,
        config: &ResConfig,
    ) -> Result<(ModData, PipelineStateChanges, Box<ResState>), StringError>;

    /// Type that the channel accepts
    fn input_type(&self) -> Option<Discriminant<ModData>>;

    /// Type that the channel returns
    fn output_type(&self) -> Option<Discriminant<ModData>>;
}
