//TODO: switch to deny
#![warn(missing_docs)]
#![feature(ptr_from_ref)]
#![cfg_attr(feature = "extra", feature(hash_set_entry))]

//TODO: create example platform and mods, feature-gated, and preferably in
//Rust and C.

//TODO: I am using equal temperament here, so mention that
//also that some parts stick to MIDI.
//TODO: when I add loading C libs, mention that here.

//!(WIP)
//!
//!A library for working with music written in MML (music macro language) fashion.
//!
//!It strives to be flexible enough to allow making an output closely resembling
//!that of some chip (platform).

pub mod types;
pub mod resource;
pub mod channel;

#[cfg(feature = "extra")]
pub mod extra;

