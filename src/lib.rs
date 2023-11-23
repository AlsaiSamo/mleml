#![warn(missing_docs)]
#![feature(ptr_from_ref)]
#![cfg_attr(feature = "extra", feature(hash_set_entry))]
//TODO: I am using equal temperament here, so mention that
//also that some parts stick to MIDI.

//!A library for working with music written in MML (music macro language) fashion.
//!
//!(WIP)
//!
//!It strives to be flexible enough to allow making an output closely resembling
//!that of a given sound chip (platform) if desired.
//!
//!The library is in development. It currently works in a fixed-function way,
//!only allowing note-consuming instruments. The library also currently does not
//!support dynamically loading C ABI libraries - every [`Resource`][crate::resource::Resource] has to be compiled
//!into the program.
//!
//!# Overview
//!Sound chips have multiple channels, and each channel individually plays sounds.
//!Logically, however, some channels may be split (like rhytm channel in PMDMML),
//!and so their ordering (during processing and in time) is important.
//!Thus, the library represent the channels as [`ChannelState`][crate::channel::ChannelState] structs,
//!which represents one channel the moment a note is played on it.
//!
//!Logic for mixing sounds coming from the channels is available through [`Platform`][crate::resource::Platform],
//!which also provides information about the imitated sound chip.
//!
//!Resources (Platform included) may require configuration and state to operate.
//!These are separated into [`ResConfig`][crate::resource::ResConfig] and [`ResState`][crate::resource::ResState] respectively.
//!Configuration is done via flat JSON array (in the type [`JsonArray`][crate::resource::JsonArray]),
//!while state is kept as a `[u8]` and is intended to be opaque outside of the resource using it.
//!
//!The functions that the resources contain need to be pure for reproducibility reasons,
//!which should help with more complex programs (such as those that support rewinding)
//!
//!# Logic example
//!Suppose that a program is receiving a stream of instructions of some sort,
//!some of which may indicate that one or multiple channels need to be played,
//!and the program simply needs to produce the resulting music.
//!
//!The program would set up the required number of `ChannelState` and one `Platform`.
//!It would then read instructions and, if they change something about a channel
//!(like the current octave), apply the changes, or, if they play a note,
//!use `ChannelState` to generate the sound of the note and put it into the mixing
//!function, indicating that this sound is new. Resource state will be generated when using
//!`ChannelState` and the mixer - it needs to be given back to their creators.
//!Mixer will also output parts of the sounds that it did not use, which also
//!need to be given back and also need to be marked as "not new".
//!
//!After that, the program simply needs to collect sounds mixed by the function
//!and put them together into a music track.
//TODO: should I put this into Platform doc?
// //! Mixing function outputs both
// //!the mixed sound and portions of the sounds that were not completely used -
// //!if we play a full note on channel 1 and a quarter note on channel 2,
// //!mixing function will return three quarter notes worth of sound from channel 1 back.

pub mod channel;
pub mod resource;
pub mod types;

#[cfg(feature = "extra")]
pub mod extra;
