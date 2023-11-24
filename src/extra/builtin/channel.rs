//TODO: rewrite to use Channel trait
pub struct SimpleChannel {
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

impl SimpleChannel {
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
        SimpleChannel {
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
    ) -> Result<(Sound, PipelineStateChanges), StringError> {
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
