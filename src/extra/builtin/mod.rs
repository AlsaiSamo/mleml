//! A collection of implementations of mods, channels, and mixers.

mod channel;
mod mixer_template;
mod mod_template;
mod synth;
mod utility_mods;

pub use channel::SimpleChannel;
pub use mixer_template::SimpleMixer;
pub use mod_template::SimpleMod;
pub use synth::FourOpFm;
pub use utility_mods::ConvertNote;
