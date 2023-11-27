#![warn(missing_docs)]
#![feature(ptr_from_ref)]
#![cfg_attr(feature = "extra", feature(hash_set_entry))]
//TODO: for all docs, make links to items

//! A library for working with music written in MML (music macro language) fashion.
//!
//! It strives to be flexible enough to:
//! 1. allow creating sound output closely resembling that of a given platform
//! 2. allow any type of usage, from a basic MML compiler to a DAW-like environment.
//!
//! # Overview
//! Base trait for other traits is [`Resource`][crate::resource::Resource], which
//! provides a name, a description, and a unique ID. It also provides configuration,
//! which is a **flat** JSON array, and state, which is a byte-slice.
//!
//! Every resource has at least one function that, besides other inputs and outputs,
//! takes config and state and returns new state. This allows making the functions pure.
//!
//! These resources are defined:
//! - Mod: essentially is a function that takes some piece of data and produces a new
//! piece of data.
//! - Channel: takes a piece of data and is expected to pass it through multiple
//! mods, respecting their configs and states.
//! - Mixer: takes sounds (or pieces of sounds) and combines them into a new sound,
//! which is returned along with unused pieces.
//!
//! Vec<Rc<dyn Mod>> is extended with Pipeline trait, adding functions to help with
//! constructing a valid senquence of mods and examining how and when the data type changes.
//!
//! # Logic example
//! Suppose that a program is receiving a stream of instructions of some sort,
//! some of which may indicate that one or multiple channels need to be played,
//! and the program simply needs to produce the resulting music.
//!
//! The program would create a necessary number of channels that produce sound from notes
//! and a mixer. It would then apply state changes (for example, octave shifts)
//! to channels' states, and, when an instructions asks to play a note,
//! do that on the required channel, and then pass the newly created sound, along with
//! previously left unused pieces of sound, to the mixer. Mixed sound is appended
//! to the resulting music, and leftover pieces are reused in the next invocation.
//! On all invocations of a channel or a mixer, their output state is reused,
//! like mixer's leftover sounds.

pub mod channel;
pub mod resource;
pub mod types;

//Feature-gating is in extra/mod.rs
pub mod extra;
