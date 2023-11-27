//! A collection of implementations of mods, channels, and mixers.

mod synth;
mod mod_template;
mod mixer_template;
mod utility_mods;
mod channel;

pub use synth::FourOpFm;
pub use utility_mods::ConvertNote;
pub use mod_template::SimpleMod;
pub use mixer_template::SimpleMixer;
pub use channel::SimpleChannel;
